//! Associative storage, ingestion, and search primitives for log data.

mod model;
mod pipeline;
mod vault;

pub use model::{LogLevel, LogPayload, LogRecord, Query, StorageFormat, StoredRecord};
pub use pipeline::{AddField, Filter, MinimumLevel, Pipeline};
pub use vault::{Vault, VaultError};
