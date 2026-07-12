# Architecture

## Data flow

```text
producer / CLI
      |
      v
ordered Filter chain ----> dropped event
      |
      v
Vault::append
      |------------------------|
      v                        v
typed snapshot          associative .links graph
(.lino or .llvb)        (link-cli doublets + Unicode strings)
      |
      v
Vault::search / count_by_level / CLI output
```

The snapshot is the recovery authority because it preserves Rust-level types,
including arbitrary bytes. The link graph is an associative projection: shared
strings and typed values converge on the same doublets through `get_or_create`.
Keeping these roles explicit avoids pretending that the initial linear query
engine already has Elasticsearch's indexing behavior.

## Modules

- `model`: levels, payloads, records, query filters, and storage-format types;
- `pipeline`: ordered filters and the durable vault output stage;
- `vault`: codecs, search, aggregate counts, graph projection, and recovery;
- `main`: thin CLI adapter over the public library.

## Snapshot codecs

The text codec writes one `(record: '<json>')` LiNo link per line. Apostrophes
use LiNo's doubled-quote escape. On open, the complete document must parse with
`links-notation` before individual typed records are decoded.

The binary codec starts with `LLVB` plus format version 1, followed by an
unsigned little-endian record count and length-prefixed JSON records. The
reader rejects an invalid header, impossible counts or lengths, truncation, and
trailing bytes. JSON is an internal typed-record codec in both cases, allowing
format evolution to share one schema.

## Associative projection

`link-cli::UnicodeStringStorage` stores and deduplicates Unicode sequences.
Named types distinguish `LogRecord`, payload kind, timestamp, level, channel,
field key, and field value. A record root links the record type to its payload,
then links to each typed attribute and key/value association. Binary payloads
use lowercase hex only in this projection; the snapshot holds original bytes.

The `.links` file is saved before its corresponding snapshot entry. A failed
snapshot write can therefore leave unreachable graph data, but cannot expose a
record whose graph is absent. If an entire sidecar is lost, opening a non-empty
snapshot rebuilds it. Atomic multi-file transactions and orphan compaction are
explicit post-initial-release work in `REQUIREMENTS.md`.

## Query semantics

All predicates combine with logical AND. Timestamps and minimum level are
inclusive. Channel and structured fields use exact case-sensitive equality.
Text matching is a case-insensitive Unicode substring over text payloads only;
binary payloads never undergo lossy conversion. Results retain insertion order
and apply `limit` last.

## Compatibility and safety

- Public APIs contain no unsafe code.
- Files are portable across supported operating systems.
- The manifest declares Rust 1.70; implementation avoids newer standard APIs.
- Binary lengths are checked before allocation/read.
- The CLI treats a downstream broken pipe as a clean exit.
- The first release is local and single-process; concurrent writers are not
  yet supported.
