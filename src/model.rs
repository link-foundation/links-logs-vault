use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Monolog-compatible severity levels, ordered from least to most severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    /// Detailed diagnostic information.
    Debug,
    /// Informational events.
    Info,
    /// Normal but significant events.
    Notice,
    /// Exceptional events that do not stop processing.
    Warning,
    /// Runtime errors.
    Error,
    /// Critical conditions.
    Critical,
    /// Conditions requiring immediate action.
    Alert,
    /// System-wide emergencies.
    Emergency,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", format!("{self:?}").to_lowercase())
    }
}

impl FromStr for LogLevel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "debug" => Ok(Self::Debug),
            "info" => Ok(Self::Info),
            "notice" => Ok(Self::Notice),
            "warning" | "warn" => Ok(Self::Warning),
            "error" => Ok(Self::Error),
            "critical" => Ok(Self::Critical),
            "alert" => Ok(Self::Alert),
            "emergency" => Ok(Self::Emergency),
            _ => Err(format!("unknown log level: {value}")),
        }
    }
}

/// Text or arbitrary binary log content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum LogPayload {
    /// UTF-8 text suitable for full-text matching.
    Text(String),
    /// Lossless arbitrary bytes.
    Binary(Vec<u8>),
}

impl LogPayload {
    /// Returns the payload without changing its representation.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Text(value) => value.as_bytes(),
            Self::Binary(value) => value,
        }
    }

    /// Returns text payloads and leaves binary payloads opaque.
    #[must_use]
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(value) => Some(value),
            Self::Binary(_) => None,
        }
    }
}

/// A normalized event accepted by the vault.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogRecord {
    /// Unix timestamp in seconds. Producers may use another documented epoch.
    pub timestamp: u64,
    /// Event severity.
    pub level: LogLevel,
    /// Producer or logical logger name.
    pub channel: String,
    /// Text or binary event body.
    pub payload: LogPayload,
    /// Structured, searchable event context.
    pub fields: BTreeMap<String, String>,
}

impl LogRecord {
    /// Creates a text event.
    #[must_use]
    pub fn text(
        timestamp: u64,
        level: LogLevel,
        channel: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            timestamp,
            level,
            channel: channel.into(),
            payload: LogPayload::Text(message.into()),
            fields: BTreeMap::new(),
        }
    }

    /// Creates a binary event without attempting UTF-8 conversion.
    #[must_use]
    pub fn binary(
        timestamp: u64,
        level: LogLevel,
        channel: impl Into<String>,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            timestamp,
            level,
            channel: channel.into(),
            payload: LogPayload::Binary(payload),
            fields: BTreeMap::new(),
        }
    }

    /// Adds or replaces one structured field.
    #[must_use]
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    /// Returns the message when this is a text record.
    #[must_use]
    pub fn message(&self) -> Option<&str> {
        self.payload.as_text()
    }
}

/// A durable record and its monotonically increasing identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredRecord {
    /// Vault-local identifier.
    pub id: u64,
    /// Root link in the associative graph.
    pub graph_id: u32,
    /// Original event.
    pub record: LogRecord,
}

/// Filters supported by the initial search engine.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Query {
    /// Exact channel match.
    pub channel: Option<String>,
    /// Inclusive severity floor.
    pub min_level: Option<LogLevel>,
    /// Inclusive lower timestamp bound.
    pub from_timestamp: Option<u64>,
    /// Inclusive upper timestamp bound.
    pub to_timestamp: Option<u64>,
    /// Case-insensitive substring for text messages.
    pub contains: Option<String>,
    /// Exact structured-field matches.
    pub fields: BTreeMap<String, String>,
    /// Maximum number of results, in insertion order.
    pub limit: Option<usize>,
}

/// Durable snapshot encoding. Both formats share the same associative graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageFormat {
    /// Line-oriented Links Notation containing readable JSON records.
    Text,
    /// Length-prefixed binary snapshot.
    Binary,
}

impl fmt::Display for StorageFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => formatter.write_str("text"),
            Self::Binary => formatter.write_str("binary"),
        }
    }
}

impl FromStr for StorageFormat {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "text" | "lino" => Ok(Self::Text),
            "binary" | "bin" | "llvb" => Ok(Self::Binary),
            _ => Err(format!("unknown storage format: {value}")),
        }
    }
}
