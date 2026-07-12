# Requirements

This matrix translates issue #1 into testable initial-release requirements.
“Initial” means the basic single-process behavior requested by the issue;
distributed scaling and a browser UI are explicitly staged follow-ups.

| ID | Requirement | Initial-release acceptance | Status / evidence |
| --- | --- | --- | --- |
| R1 | Collect issue-related evidence | Preserve issue, PR, related-project, and upstream-product evidence under `docs/case-studies/issue-1/`. | Complete; see case study and `github-data/`. |
| R2 | Deep case-study analysis | Compare the four referenced products, associative projects, constraints, risks, and solution boundaries. | Complete; see case study. |
| R3 | Inventory every issue requirement | Maintain this traceable matrix and a plan for work outside the initial boundary. | Complete. |
| R4 | Elasticsearch basics | Append durable typed events, assign ids, filter text/fields/channel/time/severity, limit results, and aggregate severity counts. | Complete; `Vault::append`, `Vault::search`, `Vault::count_by_level`. |
| R5 | Logstash basics | Accept events, apply ordered transforms/drop filters, and send passing events to durable output. | Complete; `Pipeline`, `Filter`, `MinimumLevel`, `AddField`, CLI `ingest`. |
| R6 | Kibana basics | Let an operator inspect matching events and a compact aggregate summary. | Complete for CLI/API; CLI `search` and `stats`. Browser visualization is R12. |
| R7 | Monolog basics | Support standard levels, logger channels, text messages, arbitrary context, processors, and severity handling. | Complete; eight `LogLevel`s, fields, `Filter` processors. |
| R8 | Associative stack | Project records into deduplicated doublets using Link Foundation libraries rather than an unrelated database. | Complete; `link-cli::LinkStorage` and `UnicodeStringStorage`. |
| R9 | Links Notation | Use LiNo for an inspectable representation and validate it with the official parser. | Complete; text codec and `links_notation::parse_lino`. |
| R10 | Text and binary stores | Preserve identical text and arbitrary binary events through both durable formats. | Complete; round-trip test covers both formats and non-UTF-8 bytes. |
| R11 | Library and CLI | Publish composable Rust APIs plus a runnable operator interface. | Complete; `[lib]`, `[[bin]]`, example, CLI integration test. |
| R12 | Advanced Elastic/Kibana parity | Distributed indices, query DSL, full-text indexing, dashboards, alerts, retention, authentication, and multi-tenancy. | Planned after the initial release; detailed below. |

## Detailed continuation plan

The issue asks for “most basic features,” so the initial release intentionally
does not claim production parity with the four mature upstream systems. The
following plan addresses every deferred surface without forcing unstable APIs
into the first version.

### Search and indexing

1. Add inverted token links from normalized terms to record roots, while
   retaining snapshot scans as the correctness oracle.
2. Add boolean (`must`, `should`, `must_not`), prefix, existence, and numeric
   field predicates to a serializable query AST.
3. Add sorted pagination with opaque cursors, then time-bucket and field-value
   aggregations.
4. Partition immutable segments by time; compact segments without changing
   public record ids; benchmark ingest, recovery, and representative queries.
5. Add a compatibility HTTP layer for document creation, search, and bulk
   ingestion only after the internal semantics have conformance tests.

### Ingestion and handlers

1. Add streaming file/stdin and TCP/HTTP inputs with bounded channels.
2. Add JSON, NDJSON, syslog, and multiline codecs before domain-specific
   filters.
3. Add composable outputs and handler bubbling so alerts or remote sinks can
   coexist with vault persistence.
4. Add dead-letter records, retry policies, backpressure metrics, and graceful
   shutdown tests.

### Operator experience

1. Expose read-only search, aggregation, and health endpoints.
2. Build a static web client with saved queries, a time histogram, severity
   chart, live tail, and record detail view.
3. Add threshold alerts with durable rule state and pluggable notification
   handlers.
4. Add export/import, retention previews, and integrity/rebuild commands before
   destructive maintenance operations.

### Production boundaries

1. Define crash-consistent snapshot/graph commits with a write-ahead journal.
2. Add checksums, format migration, backups, and corrupt-tail recovery.
3. Add authentication, authorization, tenant isolation, field redaction, and
   audit events before network listeners are enabled by default.
4. Add retention quotas and explicit resource limits for event size, field
   count, query cost, and binary allocations.
5. Add replication only after deterministic segment and transaction semantics
   are proven in fault-injection tests.
