# links-logs-vault

An associative log vault for collecting, retaining, searching, and summarizing
structured or unstructured events. Every record is projected into deduplicated
doublet links while remaining available through a human-readable Links Notation
snapshot or a compact binary snapshot.

The initial release takes the smallest coherent feature set from Elasticsearch,
Logstash, Kibana, and Monolog without trying to reproduce their distributed or
browser-based surfaces.

## Features

- append-only text and arbitrary binary log payloads;
- Monolog-compatible levels, channels, and structured context fields;
- exact channel/field filters, inclusive timestamp and severity ranges,
  case-insensitive text matching, and result limits;
- counts by severity for a dashboard-style summary;
- ordered Logstash-style filters that can enrich or drop events;
- LiNo text snapshots (`.lino`) and length-prefixed binary snapshots (`.llvb`);
- a sibling `.links` graph backed by [`link-cli`](https://github.com/link-foundation/link-cli),
  including link-backed Unicode string deduplication;
- LiNo syntax validation through
  [`links-notation`](https://github.com/link-foundation/links-notation) when a
  text snapshot is opened;
- a reusable Rust library and `links-logs-vault` CLI.

## Quick start

```bash
cargo run -- ingest \
  --db app.lino \
  --level warning \
  --channel api \
  --message "request was slow" \
  --field region=eu

cargo run -- search \
  --db app.lino \
  --min-level warning \
  --contains slow \
  --field region=eu

cargo run -- stats --db app.lino
```

Use `--format binary` with an `.llvb` path for a binary snapshot. Ingest an
arbitrary file with `--binary path/to/event.bin` instead of `--message`.

## Library example

```rust
use links_logs_vault::{LogLevel, LogRecord, Query, StorageFormat, Vault};

# fn run() -> Result<(), Box<dyn std::error::Error>> {
let mut vault = Vault::open("app.lino", StorageFormat::Text)?;
vault.append(
    LogRecord::text(1_700_000_000, LogLevel::Info, "api", "request complete")
        .with_field("region", "eu"),
)?;

let results = vault.search(&Query {
    channel: Some(String::from("api")),
    contains: Some(String::from("complete")),
    ..Query::default()
});
assert_eq!(results.len(), 1);
# Ok(())
# }
```

For ingestion filters, construct a `Pipeline`, then add `MinimumLevel`,
`AddField`, or a custom implementation of the public `Filter` trait.

## Storage model

For a snapshot at `logs.lino` or `logs.llvb`, the vault maintains
`logs.links`. The snapshot provides deterministic recovery of complete typed
records. The graph is the associative projection used to deduplicate shared
types, strings, field keys, field values, channels, levels, and payloads.

Each event is represented by a `LogRecord` root pointing to a typed payload and
typed timestamp, level, channel, and field associations. Binary payloads are
hex-encoded only inside the graph so every byte remains lossless in the primary
snapshot. If a sidecar is missing, opening the snapshot rebuilds it.

| Borrowed concept | Initial vault surface |
| --- | --- |
| Elasticsearch | durable event ids, structured fields, range/text filters, counts |
| Logstash | input through the CLI/library, ordered `Filter`s, vault output |
| Kibana | search listing and `stats` summary for inspection/automation |
| Monolog | eight levels, channels, context, enrichment and severity filtering |

The detailed feature boundary and follow-up plan are in
[`REQUIREMENTS.md`](REQUIREMENTS.md). Design rationale and recovery behavior are
in [`ARCHITECTURE.md`](ARCHITECTURE.md). The evidence collected for issue #1 is
in [`docs/case-studies/issue-1/`](docs/case-studies/issue-1/README.md).

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features
cargo test --all-features --verbose
cargo test --doc --verbose
rust-script scripts/check-file-size.rs
rust-script scripts/check-crate-size.rs
```

The package keeps the repository template's automated changelog-driven release
version. User-facing changes belong in `changelog.d/`; pull requests must not
edit the version field directly.

## License

[Unlicense](LICENSE)
