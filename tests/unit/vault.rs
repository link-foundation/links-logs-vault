use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use links_logs_vault::{
    AddField, LogLevel, LogRecord, MinimumLevel, Pipeline, Query, StorageFormat, Vault,
};

fn temporary_path(name: &str, extension: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "links-logs-vault-{name}-{}-{nonce}.{extension}",
        std::process::id()
    ))
}

fn remove_store(path: &PathBuf) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(path.with_extension("links"));
}

#[test]
fn text_and_binary_stores_round_trip_the_same_records() {
    for format in [StorageFormat::Text, StorageFormat::Binary] {
        let extension = if format == StorageFormat::Text {
            "lino"
        } else {
            "llvb"
        };
        let path = temporary_path("round-trip", extension);
        let mut vault = Vault::open(&path, format).expect("vault should open");

        let first_id = vault
            .append(LogRecord::text(
                1_700_000_001,
                LogLevel::Info,
                "api",
                "request's complete",
            ))
            .expect("text record should be stored");
        let binary = LogRecord::binary(
            1_700_000_002,
            LogLevel::Error,
            "worker",
            vec![0, 159, 146, 150, 255],
        )
        .with_field("job", "import");
        let second_id = vault
            .append(binary)
            .expect("binary record should be stored");
        assert_eq!((first_id, second_id), (1, 2));
        drop(vault);
        fs::remove_file(path.with_extension("links"))
            .expect("test should be able to remove the graph sidecar");

        let reopened = Vault::open(&path, format).expect("vault should reopen");
        {
            let all_records = reopened.search(&Query::default());
            assert_eq!(all_records.len(), 2);
            assert_eq!(all_records[0].record.message(), Some("request's complete"));
            let matches = reopened.search(&Query {
                min_level: Some(LogLevel::Error),
                ..Query::default()
            });

            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0].id, 2);
            assert_eq!(
                matches[0].record.payload.as_bytes(),
                &[0, 159, 146, 150, 255]
            );
            assert_eq!(
                matches[0].record.fields.get("job"),
                Some(&String::from("import"))
            );
        }
        assert!(reopened.association_count() > reopened.len());

        drop(reopened);
        remove_store(&path);
    }
}

#[test]
fn malformed_snapshots_are_rejected() {
    let text_path = temporary_path("malformed", "lino");
    fs::write(&text_path, "(record: 'not-json')\n").expect("fixture should be written");
    assert!(Vault::open(&text_path, StorageFormat::Text).is_err());

    let binary_path = temporary_path("malformed", "llvb");
    fs::write(&binary_path, b"LLVB\x01\0\0\0\0\0\0\0\0trailing")
        .expect("fixture should be written");
    assert!(Vault::open(&binary_path, StorageFormat::Binary).is_err());

    remove_store(&text_path);
    remove_store(&binary_path);
}

#[test]
fn search_combines_time_channel_level_text_and_field_filters() {
    let path = temporary_path("search", "lino");
    let mut vault = Vault::open(&path, StorageFormat::Text).expect("vault should open");
    let records = [
        LogRecord::text(10, LogLevel::Debug, "api", "cache miss"),
        LogRecord::text(20, LogLevel::Warning, "api", "request was slow")
            .with_field("region", "eu"),
        LogRecord::text(30, LogLevel::Error, "worker", "request failed").with_field("region", "us"),
    ];
    for record in records {
        vault.append(record).expect("record should be stored");
    }

    let query = Query {
        channel: Some(String::from("api")),
        min_level: Some(LogLevel::Info),
        from_timestamp: Some(15),
        to_timestamp: Some(25),
        contains: Some(String::from("SLOW")),
        fields: BTreeMap::from([(String::from("region"), String::from("eu"))]),
        limit: Some(1),
    };
    {
        let matches = vault.search(&query);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].record.message(), Some("request was slow"));
    }
    assert_eq!(vault.count_by_level().get(&LogLevel::Warning), Some(&1));
    drop(vault);
    remove_store(&path);
}

#[test]
fn pipeline_applies_filters_and_can_drop_noisy_records() {
    let path = temporary_path("pipeline", "llvb");
    let vault = Vault::open(&path, StorageFormat::Binary).expect("vault should open");
    let mut pipeline = Pipeline::new(vault);
    pipeline.add_filter(MinimumLevel::new(LogLevel::Info));
    pipeline.add_filter(AddField::new("service", "billing"));

    assert_eq!(
        pipeline
            .process(LogRecord::text(1, LogLevel::Debug, "app", "details"))
            .expect("pipeline should run"),
        None
    );
    assert_eq!(
        pipeline
            .process(LogRecord::text(2, LogLevel::Error, "app", "failed"))
            .expect("pipeline should run"),
        Some(1)
    );

    {
        let records = pipeline.vault().search(&Query::default());
        assert_eq!(
            records[0].record.fields.get("service"),
            Some(&String::from("billing"))
        );
    }
    drop(pipeline);
    remove_store(&path);
}
