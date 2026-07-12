use std::fs;

use links_logs_vault::{LogLevel, LogRecord, Query, StorageFormat, Vault};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::temp_dir().join("links-logs-vault-example.lino");
    let mut vault = Vault::open(&path, StorageFormat::Text)?;
    vault.append(
        LogRecord::text(1_700_000_000, LogLevel::Info, "example", "vault started")
            .with_field("environment", "demo"),
    )?;

    for result in vault.search(&Query {
        contains: Some(String::from("started")),
        ..Query::default()
    }) {
        println!("{}: {:?}", result.id, result.record);
    }

    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(path.with_extension("links"));
    Ok(())
}
