use crate::{LogLevel, LogRecord, Vault, VaultError};

/// A Logstash-style event transformation. `None` drops the event.
pub trait Filter {
    /// Transforms, passes, or drops one event.
    fn apply(&mut self, record: LogRecord) -> Option<LogRecord>;
}

/// Drops events below a configured severity.
pub struct MinimumLevel {
    minimum: LogLevel,
}

impl MinimumLevel {
    /// Creates a severity filter.
    #[must_use]
    pub const fn new(minimum: LogLevel) -> Self {
        Self { minimum }
    }
}

impl Filter for MinimumLevel {
    fn apply(&mut self, record: LogRecord) -> Option<LogRecord> {
        (record.level >= self.minimum).then_some(record)
    }
}

/// Enriches every passing event with a fixed field.
pub struct AddField {
    key: String,
    value: String,
}

impl AddField {
    /// Creates an enrichment filter.
    #[must_use]
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl Filter for AddField {
    fn apply(&mut self, mut record: LogRecord) -> Option<LogRecord> {
        record.fields.insert(self.key.clone(), self.value.clone());
        Some(record)
    }
}

/// Ordered filters followed by an associative-vault output.
pub struct Pipeline {
    vault: Vault,
    filters: Vec<Box<dyn Filter>>,
}

impl Pipeline {
    /// Creates a pipeline whose output is `vault`.
    #[must_use]
    pub fn new(vault: Vault) -> Self {
        Self {
            vault,
            filters: Vec::new(),
        }
    }

    /// Appends a filter. Filters run in insertion order.
    pub fn add_filter(&mut self, filter: impl Filter + 'static) {
        self.filters.push(Box::new(filter));
    }

    /// Runs one event through all filters and stores it unless it is dropped.
    pub fn process(&mut self, mut record: LogRecord) -> Result<Option<u64>, VaultError> {
        for filter in &mut self.filters {
            let Some(filtered) = filter.apply(record) else {
                return Ok(None);
            };
            record = filtered;
        }
        self.vault.append(record).map(Some)
    }

    /// Borrows the pipeline output.
    #[must_use]
    pub const fn vault(&self) -> &Vault {
        &self.vault
    }

    /// Mutably borrows the pipeline output.
    #[must_use]
    pub fn vault_mut(&mut self) -> &mut Vault {
        &mut self.vault
    }

    /// Returns the output vault.
    #[must_use]
    pub fn into_vault(self) -> Vault {
        self.vault
    }
}
