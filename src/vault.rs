use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};

use link_cli::{LinkStorage, UnicodeStringStorage};
use thiserror::Error;

use crate::{LogLevel, LogPayload, LogRecord, Query, StorageFormat, StoredRecord};

const BINARY_MAGIC: &[u8; 5] = b"LLVB\x01";

/// Errors returned by durable vault operations.
#[derive(Debug, Error)]
pub enum VaultError {
    /// File-system operation failed.
    #[error("storage I/O failed: {0}")]
    Io(#[from] std::io::Error),
    /// JSON record encoding failed.
    #[error("record encoding failed: {0}")]
    Json(#[from] serde_json::Error),
    /// A snapshot or path does not satisfy the storage contract.
    #[error("invalid vault data: {0}")]
    InvalidData(String),
    /// The associative link layer rejected an operation.
    #[error("associative storage failed: {0}")]
    Associations(String),
}

/// Persistent log records plus a deduplicated associative doublet graph.
pub struct Vault {
    path: PathBuf,
    format: StorageFormat,
    records: Vec<StoredRecord>,
    associations: LinkStorage,
}

impl Vault {
    /// Opens or creates a vault. The graph is stored beside the snapshot as `.links`.
    pub fn open(path: impl AsRef<Path>, format: StorageFormat) -> Result<Self, VaultError> {
        let path = path.as_ref().to_path_buf();
        let records = if path.exists() {
            match format {
                StorageFormat::Text => read_text(&path)?,
                StorageFormat::Binary => read_binary(&path)?,
            }
        } else {
            Vec::new()
        };
        let graph_path = path.with_extension("links");
        let graph_path = graph_path
            .to_str()
            .ok_or_else(|| VaultError::InvalidData(String::from("vault path is not UTF-8")))?;
        let associations = LinkStorage::new(graph_path, false)
            .map_err(|error| VaultError::Associations(error.to_string()))?;
        let mut vault = Self {
            path,
            format,
            records,
            associations,
        };

        if !vault.records.is_empty() && vault.associations.all().is_empty() {
            vault.rebuild_associations()?;
        }
        Ok(vault)
    }

    /// Stores one event and returns its monotonically increasing identifier.
    pub fn append(&mut self, record: LogRecord) -> Result<u64, VaultError> {
        let id = self.records.last().map_or(1, |entry| entry.id + 1);
        let graph_id = add_to_graph(&mut self.associations, &record)?;
        self.associations
            .save()
            .map_err(|error| VaultError::Associations(error.to_string()))?;
        self.records.push(StoredRecord {
            id,
            graph_id,
            record,
        });
        if let Err(error) = self.save_snapshot() {
            self.records.pop();
            return Err(error);
        }
        Ok(id)
    }

    /// Searches records in insertion order using all supplied filters.
    #[must_use]
    pub fn search(&self, query: &Query) -> Vec<&StoredRecord> {
        let needle = query.contains.as_ref().map(|value| value.to_lowercase());
        self.records
            .iter()
            .filter(|entry| {
                query
                    .channel
                    .as_ref()
                    .is_none_or(|value| entry.record.channel == *value)
            })
            .filter(|entry| {
                query
                    .min_level
                    .is_none_or(|value| entry.record.level >= value)
            })
            .filter(|entry| {
                query
                    .from_timestamp
                    .is_none_or(|value| entry.record.timestamp >= value)
            })
            .filter(|entry| {
                query
                    .to_timestamp
                    .is_none_or(|value| entry.record.timestamp <= value)
            })
            .filter(|entry| {
                needle.as_ref().is_none_or(|value| {
                    entry
                        .record
                        .message()
                        .is_some_and(|message| message.to_lowercase().contains(value))
                })
            })
            .filter(|entry| {
                query
                    .fields
                    .iter()
                    .all(|(key, value)| entry.record.fields.get(key) == Some(value))
            })
            .take(query.limit.unwrap_or(usize::MAX))
            .collect()
    }

    /// Counts stored events by severity, suitable for a dashboard summary.
    #[must_use]
    pub fn count_by_level(&self) -> BTreeMap<LogLevel, usize> {
        let mut counts = BTreeMap::new();
        for entry in &self.records {
            *counts.entry(entry.record.level).or_default() += 1;
        }
        counts
    }

    /// Number of durable events.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.records.len()
    }

    /// Whether the vault contains no events.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Number of deduplicated doublets in the associated link graph.
    #[must_use]
    pub fn association_count(&self) -> usize {
        self.associations.all().len()
    }

    fn save_snapshot(&self) -> Result<(), VaultError> {
        match self.format {
            StorageFormat::Text => write_text(&self.path, &self.records),
            StorageFormat::Binary => write_binary(&self.path, &self.records),
        }
    }

    fn rebuild_associations(&mut self) -> Result<(), VaultError> {
        for index in 0..self.records.len() {
            let graph_id = add_to_graph(&mut self.associations, &self.records[index].record)?;
            self.records[index].graph_id = graph_id;
        }
        self.associations
            .save()
            .map_err(|error| VaultError::Associations(error.to_string()))?;
        self.save_snapshot()
    }
}

fn add_to_graph(storage: &mut LinkStorage, record: &LogRecord) -> Result<u32, VaultError> {
    let mut strings = UnicodeStringStorage::new(storage)
        .map_err(|error| VaultError::Associations(error.to_string()))?;
    let record_type = strings
        .get_or_create_type("LogRecord")
        .map_err(|error| VaultError::Associations(error.to_string()))?;
    let payload_kind = match &record.payload {
        LogPayload::Text(_) => "TextPayload",
        LogPayload::Binary(_) => "BinaryPayload",
    };
    let payload = match &record.payload {
        LogPayload::Text(value) => value.clone(),
        LogPayload::Binary(value) => encode_hex(value),
    };
    let payload_value = typed_value(&mut strings, payload_kind, &payload)?;
    let root = strings.links_mut().create(record_type, payload_value);

    for (kind, value) in [
        ("Timestamp", record.timestamp.to_string()),
        ("Level", record.level.to_string()),
        ("Channel", record.channel.clone()),
    ] {
        let value = typed_value(&mut strings, kind, &value)?;
        strings.links_mut().get_or_create(root, value);
    }
    for (key, value) in &record.fields {
        let key = typed_value(&mut strings, "FieldKey", key)?;
        let value = typed_value(&mut strings, "FieldValue", value)?;
        let field = strings.links_mut().get_or_create(key, value);
        strings.links_mut().get_or_create(root, field);
    }
    Ok(root)
}

fn typed_value(
    strings: &mut UnicodeStringStorage<'_>,
    kind: &str,
    value: &str,
) -> Result<u32, VaultError> {
    let value_type = strings
        .get_or_create_type(kind)
        .map_err(|error| VaultError::Associations(error.to_string()))?;
    let value = strings
        .create_string(value)
        .map_err(|error| VaultError::Associations(error.to_string()))?;
    Ok(strings.links_mut().get_or_create(value_type, value))
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(
        String::with_capacity(bytes.len().saturating_mul(2)),
        |mut output, byte| {
            write!(output, "{byte:02x}").expect("writing to a String cannot fail");
            output
        },
    )
}

fn write_text(path: &Path, records: &[StoredRecord]) -> Result<(), VaultError> {
    let mut output = String::new();
    for record in records {
        let json = serde_json::to_string(record)?;
        output.push_str("(record: '");
        output.push_str(&json.replace('\'', "''"));
        output.push_str("')\n");
    }
    fs::write(path, output)?;
    Ok(())
}

fn read_text(path: &Path) -> Result<Vec<StoredRecord>, VaultError> {
    let input = fs::read_to_string(path)?;
    if !input.trim().is_empty() {
        links_notation::parse_lino(&input)
            .map_err(|error| VaultError::InvalidData(format!("invalid Links Notation: {error}")))?;
    }
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let line = line.trim();
            let quoted = line
                .strip_prefix("(record: ")
                .and_then(|value| value.strip_suffix(')'))
                .ok_or_else(|| VaultError::InvalidData(format!("invalid record line: {line}")))?;
            let json = quoted
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
                .ok_or_else(|| {
                    VaultError::InvalidData(String::from("record value must be quoted"))
                })?
                .replace("''", "'");
            serde_json::from_str(&json).map_err(VaultError::from)
        })
        .collect()
}

fn write_binary(path: &Path, records: &[StoredRecord]) -> Result<(), VaultError> {
    let mut output = Vec::new();
    output.extend_from_slice(BINARY_MAGIC);
    output.extend_from_slice(&(records.len() as u64).to_le_bytes());
    for record in records {
        let encoded = serde_json::to_vec(record)?;
        output.extend_from_slice(&(encoded.len() as u64).to_le_bytes());
        output.extend_from_slice(&encoded);
    }
    fs::write(path, output)?;
    Ok(())
}

fn read_binary(path: &Path) -> Result<Vec<StoredRecord>, VaultError> {
    let data = fs::read(path)?;
    let mut cursor = Cursor::new(data.as_slice());
    let mut magic = [0; BINARY_MAGIC.len()];
    cursor.read_exact(&mut magic)?;
    if &magic != BINARY_MAGIC {
        return Err(VaultError::InvalidData(String::from(
            "invalid binary header",
        )));
    }
    let count = usize::try_from(read_u64(&mut cursor)?)
        .map_err(|_| VaultError::InvalidData(String::from("record count is too large")))?;
    if count > data.len() / 8 {
        return Err(VaultError::InvalidData(String::from(
            "record count exceeds snapshot size",
        )));
    }
    let mut records = Vec::with_capacity(count);
    for _ in 0..count {
        let length = read_u64(&mut cursor)?;
        let length = usize::try_from(length)
            .map_err(|_| VaultError::InvalidData(String::from("record length is too large")))?;
        let position = usize::try_from(cursor.position())
            .map_err(|_| VaultError::InvalidData(String::from("cursor position is too large")))?;
        let remaining = data.len().saturating_sub(position);
        if length > remaining {
            return Err(VaultError::InvalidData(String::from(
                "truncated binary record",
            )));
        }
        let mut encoded = vec![0; length];
        cursor.read_exact(&mut encoded)?;
        records.push(serde_json::from_slice(&encoded)?);
    }
    if usize::try_from(cursor.position()) != Ok(data.len()) {
        return Err(VaultError::InvalidData(String::from(
            "trailing binary data",
        )));
    }
    Ok(records)
}

fn read_u64(cursor: &mut Cursor<&[u8]>) -> Result<u64, VaultError> {
    let mut bytes = [0; 8];
    cursor.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}
