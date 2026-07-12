# Case study: initial associative logs vault

## Executive summary

Issue #1 asks for an initial log-management product inspired by Elasticsearch,
Logstash, Kibana, and Monolog, but grounded in Link Foundation's associative
stack and capable of using text and binary stores. It also asks for evidence
collection, a complete requirement inventory, online research, deep analysis,
and a detailed plan.

The repository provided no product implementation: it was an unchanged Rust
sum-example template. The initial release therefore establishes one coherent
vertical slice rather than thin placeholders for four mature products:

1. normalized text or binary records with standard logging metadata;
2. ordered ingestion filters;
3. durable LiNo or binary snapshots;
4. a deduplicated doublet projection using `link-cli`;
5. composable search and a severity aggregation;
6. Rust library and CLI operator surfaces.

The full acceptance matrix is in [`REQUIREMENTS.md`](../../../REQUIREMENTS.md)
and design details are in [`ARCHITECTURE.md`](../../../ARCHITECTURE.md).

## Collected first-party issue evidence

- [`github-data/issue-1.md`](github-data/issue-1.md): issue metadata and body;
- [`github-data/issue-1-comments.md`](github-data/issue-1-comments.md): empty
  paginated comment result;
- [`github-data/pr-2.md`](github-data/pr-2.md): prepared PR and all three PR
  feedback surfaces before implementation.

The absence of comments means the issue body is the complete authoritative
scope. No screenshots or attachments were present.

## Related Link Foundation work

Research was performed against the current repositories and their latest
relevant merged work on 2026-07-12 UTC.

### links-notation

The [links-notation repository](https://github.com/link-foundation/links-notation)
describes LiNo as references and links, supports N-tuples and nested/indented
syntax, and publishes a Rust parser. Crates.io reported Rust crate 0.13.0. The
vault uses that exact public parser to reject malformed text snapshots rather
than inventing an unrelated “text” label.

### link-cli

The [link-cli repository](https://github.com/link-foundation/link-cli) models
CRUD as substitutions over links and publishes both a CLI and reusable Rust
library. Its Rust API exposes `LinkStorage` and `UnicodeStringStorage`, allowing
this project to use persistent doublets and link-backed string sequences. The
latest relevant merged PR was
[#93, public library packages alongside the CLI](https://github.com/link-foundation/link-cli/pull/93),
which specifically made reuse by projects such as this one an intended surface.
Earlier relevant work includes
[#68, Rust query and Unicode sequence parity](https://github.com/link-foundation/link-cli/pull/68)
and [#72, supported-feature and architecture documentation](https://github.com/link-foundation/link-cli/pull/72).
Crates.io reported `link-cli` 0.2.7.

### formal-ai and adjacent projects

The [formal-ai repository](https://github.com/link-assistant/formal-ai) uses an
append-only event memory, treats Links Notation as a deterministic recovery and
migration projection, exposes the same core through CLI/library/UI adapters,
and keeps destructive maintenance explicit. The applicable practices are:

- one domain model behind every interface;
- append-only events and deterministic export/replay;
- clear native-store versus LiNo-projection responsibilities;
- full-memory export before destructive maintenance;
- public library APIs rather than logic trapped in the CLI.

The [lino-cache repository](https://github.com/link-foundation/lino-cache)
demonstrates inspectable `.lino` persistence, while
[`dedup-log`](https://github.com/link-foundation/dedup-log) identifies
deduplicated LiNo and lazy logging as adjacent goals. Both were still early
template-derived projects, so they were treated as intent evidence, not copied
as production implementations.

## Upstream product research

Only upstream/official documentation was used for technical product behavior.
Pages were checked on 2026-07-12.

### Elasticsearch

Elastic describes Elasticsearch as a near-real-time engine to search, index,
store, and analyze data, with structured query languages and aggregations
([reference](https://www.elastic.co/guide/en/elasticsearch/reference/current/)).
Its document API assigns or accepts ids and can apply an ingest pipeline
([index API](https://www.elastic.co/guide/en/elasticsearch/reference/current/docs-index_.html/)).
Data streams are append-oriented time-series collections whose events include
a timestamp
([data-stream setup](https://www.elastic.co/guide/en/elasticsearch/reference/current/set-up-a-data-stream.html)).

Initial implications: immutable ids, timestamps, structured fields, combined
filters, and at least one aggregation are more fundamental than distributed
shards or a complete Query DSL.

### Logstash

The official pipeline model is inputs → filters → outputs. Inputs create
events, ordered filters transform/drop them, and outputs deliver them; codecs
separate transport from serialization
([pipeline overview](https://www.elastic.co/guide/en/logstash/current/pipeline.html/),
[configuration structure](https://www.elastic.co/guide/en/logstash/current/configuration-file-structure.html)).

Initial implications: a filter can pass, transform, or drop an event; filters
must preserve order; the vault is one output. Network inputs and codec plugins
can grow around that stable contract.

### Kibana

Kibana Discover operates on raw data, while dashboards combine panels and
interactive filters; alerting watches conditions and reporting shares results
([Kibana overview](https://www.elastic.co/guide/en/kibana/current/get-started.html),
[dashboards](https://www.elastic.co/guide/en/kibana/current/dashboard.html/)).

Initial implications: searchable event inspection and a summary aggregate are
the minimum operator loop. A web dashboard is valuable but should consume a
tested query API rather than define the storage model.

### Monolog

Monolog loggers have channels and handler stacks; records traverse handlers,
which can stop propagation. Context adds per-record data and processors enrich
records
([usage](https://seldaek.github.io/monolog/doc/01-usage.html)). Its record
structure includes message, level, context, and channel
([record structure](https://seldaek.github.io/monolog/doc/message-structure.html)).
Official handler documentation covers level filters, grouping, sampling,
deduplication, buffering, and diverse outputs
([handlers, formatters, processors](https://seldaek.github.io/monolog/doc/02-handlers-formatters-processors.html)).

Initial implications: use the eight standard levels, channel, message/context,
and processor-like filters. Full handler bubbling and remote notification
outputs remain compatible follow-ups.

## Requirements derived from the issue

1. Preserve issue-related evidence inside `docs/case-studies/issue-1`.
2. Perform a deep comparison with online facts, not only repository guesses.
3. Enumerate every requirement and propose a detailed solution for each.
4. Provide basic Elasticsearch-style durable indexing, filtering, search, and
   analysis.
5. Provide basic Logstash-style ingestion, ordered filtering, and output.
6. Provide basic Kibana-style inspection and summarized analysis.
7. Provide basic Monolog-style records, levels, channels, context, and
   processors/handlers.
8. Implement storage on an associative stack.
9. Build directly on links-notation and link-cli.
10. Support both text and binary stores and both text and binary log content.
11. Deliver the work and its verification in PR #2.

Each item maps to a status, acceptance test, and continuation plan in the root
requirement matrix.

## Design decisions

### One record model, two codecs

Separate text and binary implementations would drift. Both snapshot codecs
therefore encode the same `StoredRecord` schema. Tests append a UTF-8 message
and non-UTF-8 bytes, reopen each codec, and assert identical ids, fields, and
payload bytes.

### Snapshot as recovery authority, graph as associative projection

`link-cli`'s current storage persists doublets but does not natively serialize
the complete typed log record. The snapshot preserves exact application types;
the sidecar projects shared values into typed doublets. This is honest about
the initial query implementation while providing a migration path to graph
indices. A missing sidecar rebuilds from the snapshot.

### A narrow but complete query contract

The first query combines exact channel/fields, inclusive severity/time ranges,
case-insensitive message substring, and limit. All predicates use AND. This is
enough for real operational questions and small enough to specify and test.

### CLI inspection before browser UI

Search and stats prove the full producer → store → reopen → query loop on every
CI operating system. A later UI can call the same domain API. Calling the CLI
“Kibana parity” would be misleading, so the advanced visualization requirement
is explicitly staged.

## Risks and mitigations

| Risk | Initial mitigation | Required follow-up |
| --- | --- | --- |
| Snapshot and sidecar diverge after a crash | Save graph first; rebuild a missing graph | Journaled atomic commit and orphan compaction |
| Linear scans degrade | Query semantics are tested independently of index layout | Inverted term/field links and immutable segments |
| Malformed binary length triggers allocation | Validate count, length, remaining bytes, and trailing data | Checksums and corrupt-tail recovery |
| Concurrent writers overwrite snapshots | Document single-process boundary | File locking or server-owned writer |
| Sensitive fields leak | No network listener by default | Redaction, authn/authz, tenant isolation, audit |
| Product-comparison scope expands without bound | Define initial acceptance per product | Stage query, ingestion, UI, and production milestones |

## Reproduction and verification

Before implementation, the new tests failed because the expected
`links_logs_vault` library and `links-logs-vault` executable did not exist; the
captured compiler output is in `experiments/issue-1-red-test.log`.

Automated coverage added by this PR verifies:

- both snapshot formats reopen text and binary payloads losslessly;
- graph associations exist and outnumber record roots;
- all query predicates compose correctly;
- severity aggregation returns expected counts;
- ordered filters drop low-severity events and enrich passing events;
- the CLI ingests, reopens, searches, and summarizes a durable vault;
- existing release-pipeline tests remain intact.

The repository's complete formatting, lint, unit, integration, doc-test,
file-size, crate-size, package, and release-script checks are run before PR
finalization.
