use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temporary_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "links-logs-vault-cli-{}-{nonce}.lino",
        std::process::id()
    ))
}

#[test]
fn cli_ingests_searches_and_summarizes_records() {
    let database = temporary_path();
    let binary = env!("CARGO_BIN_EXE_links-logs-vault");

    let ingest = Command::new(binary)
        .args([
            "ingest",
            "--db",
            database.to_str().expect("temporary path should be UTF-8"),
            "--level",
            "warning",
            "--channel",
            "api",
            "--message",
            "latency high",
            "--field",
            "region=eu",
            "--timestamp",
            "42",
        ])
        .output()
        .expect("ingest command should run");
    assert!(
        ingest.status.success(),
        "{}",
        String::from_utf8_lossy(&ingest.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&ingest.stdout), "stored record 1\n");

    let search = Command::new(binary)
        .args([
            "search",
            "--db",
            database.to_str().expect("temporary path should be UTF-8"),
            "--min-level",
            "warning",
            "--contains",
            "LATENCY",
        ])
        .output()
        .expect("search command should run");
    assert!(
        search.status.success(),
        "{}",
        String::from_utf8_lossy(&search.stderr)
    );
    let output = String::from_utf8_lossy(&search.stdout);
    assert!(output.contains("warning api latency high"), "{output}");
    assert!(output.contains("region=eu"), "{output}");

    let stats = Command::new(binary)
        .args([
            "stats",
            "--db",
            database.to_str().expect("temporary path should be UTF-8"),
        ])
        .output()
        .expect("stats command should run");
    assert!(
        stats.status.success(),
        "{}",
        String::from_utf8_lossy(&stats.stderr)
    );
    assert!(String::from_utf8_lossy(&stats.stdout).contains("warning=1"));

    let _ = fs::remove_file(&database);
    let _ = fs::remove_file(database.with_extension("links"));
}
