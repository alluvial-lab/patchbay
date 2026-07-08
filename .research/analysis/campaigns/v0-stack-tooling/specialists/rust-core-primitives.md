---
provenance: agent-synthesis
updated: 2026-07-07
campaign: v0-stack-tooling
facet: rust-core-primitives
---

# Rust core primitives for Patchbay v0

## Executive position

Default the v0 durable log backend to SQLite through a Rust wrapper, with Patchbay owning the storage port, LSN allocation invariant, replay semantics, and snapshot prefix rules in core code rather than treating any database as the protocol model. SQLite supplies the needed single-writer-friendly durable substrate: its docs say ordinary writes are serialized with only one writer at a time, WAL commits are special records appended to the WAL, and WAL writers append sequentially while readers can proceed concurrently [sqlite-isolation]{2} [sqlite-wal]{2} [sqlite-wal]{4} [sqlite-wal]{5}. The storage design still must make gap-free monotonic LSN an explicit Patchbay invariant: none of the fetched storage sources claims to provide Patchbay's `(authority_domain_id, LSN)` log contract directly {inferred: fit}.

Use `rusqlite` as the initial embedded-store binding unless an implementation-wide async database abstraction becomes a binding local constraint. Rusqlite is a direct ergonomic SQLite wrapper, exposes transactions, rolls transactions back by default unless explicitly committed, exposes WAL commit hooks, and statically prevents simultaneous use of one `Connection` across threads via Rust `Send`/`Sync` behavior [rusqlite]{1} [rusqlite]{3} [rusqlite]{4} [rusqlite]{6} [rusqlite]{9}. SQLx remains a viable SQLite wrapper if Patchbay wants async query ergonomics, but its SQLite driver still rests on `libsqlite3-sys` and defers journaling/synchronous semantics to SQLite documentation [sqlx-sqlite]{1} [sqlx-sqlite]{2} [sqlx-sqlite]{7} [sqlx-sqlite]{8}.

Do not adopt a Rust event-sourcing framework as the durable core log. `cqrs-es` is useful evidence for aggregate-level event-sourcing patterns, but its attested event uniqueness key is `aggregate_type + aggregate_id + sequence`, with `sequence` defined per aggregate instance, not a global gap-free authority-domain LSN [cqrs-es]{3} [cqrs-es]{8} [cqrs-es]{9}. That makes it a domain-pattern library candidate, not the storage primitive for Patchbay's protocol log {inferred: fit}.

Use `tokio` as the async runtime and `tonic` as the Rust-side gRPC server candidate for the internal seam. Tokio documents a non-blocking async runtime with task, I/O, timer, synchronization, filesystem, process, and signal support [tokio]{1} [tokio]{2} [tokio]{3} [tokio]{4} [tokio]{5}. Tonic documents a Rust gRPC-over-HTTP/2 implementation with async/await support, transport built on hyper/tower/tokio, server support, and streaming request/response types [tonic]{1} [tonic]{2} [tonic]{3} [tonic]{5} [tonic]{6} [tonic]{10}.

Use `proptest` for registry-level and replay-level property tests. Its guide says it generates arbitrary inputs, shrinks failures to minimal reproductions, composes per-value generation/shrinking, and includes state-machine testing in its guide surface [proptest]{1} [proptest]{2} [proptest]{3} [proptest]{8}.

## Current fetched crate-doc versions

The fetched docs.rs latest pages identified these documented versions: `rusqlite-0.40.1`, `sqlx-0.9.0`, `libsql-0.9.30`, `sled-0.34.7`, `cqrs-es-0.5.0`, `statig-0.4.1`, `smlang-0.8.0`, `tokio-1.52.3`, `proptest-1.11.0`, and `tonic-0.14.6` [rusqlite]{10} [sqlx-sqlite]{9} [libsql-rust]{9} [sled]{11} [cqrs-es]{10} [statig]{9} [smlang]{9} [tokio]{10} [proptest-docs]{1} [tonic]{13}.

## Durable log and event-sourcing options

### SQLite through rusqlite

SQLite WAL is a direct fit for a local, single-authoritative-core event log, provided Patchbay treats SQLite as a durable append substrate rather than as the source of protocol semantics. SQLite documents serializable isolation for ordinary separate connections, automatic write serialization, WAL commit records, and one writer at a time in WAL mode [sqlite-isolation]{1} [sqlite-isolation]{2} [sqlite-isolation]{3} [sqlite-wal]{2} [sqlite-wal]{4}. SQLite also documents the durability knob that matters for crash recovery: writers sync the WAL on every transaction commit with `PRAGMA synchronous=FULL`, while `NORMAL` omits that commit sync and may roll back transactions after power loss or hard reset [sqlite-wal]{6} [sqlite-wal]{8}.

A Patchbay SQLite adapter should therefore make these semantics explicit in the storage port contract: enable WAL mode, require `synchronous=FULL` for safety-critical v0 claims unless a weaker durability mode is deliberately classified, append event rows and derived snapshot metadata in transactions, and fail acceptance if the durable append/snapshot-prefix transaction cannot complete {inferred: fit} [sqlite-isolation]{4} [sqlite-wal]{6} [rusqlite]{4}. Rusqlite fits the narrow single-writer shape because it exposes a SQLite connection and transaction API, rolls transactions back by default on drop, can install busy handling for lock contention, and has a WAL hook for commits [rusqlite]{2} [rusqlite]{3} [rusqlite]{4} [rusqlite]{6} [rusqlite]{7}.

### SQLite through SQLx

SQLx's SQLite driver is appropriate if the implementation wants an async database API aligned with a Tokio service, but its own docs point back to SQLite for journaling and synchronous-mode semantics [sqlx-sqlite]{1} [sqlx-sqlite]{7} [sqlx-sqlite]{8}. SQLx also uses `libsqlite3-sys` and can statically link a bundled SQLite build by default, which is useful for deployment reproducibility but does not change the underlying WAL/crash-recovery contract [sqlx-sqlite]{2} [sqlx-sqlite]{4} [sqlx-sqlite]{5}. For a single Rust core with one authoritative writer, SQLx adds async/pool ergonomics; it does not remove the need for a storage-port-level LSN invariant {inferred: fit}.

### libSQL

libSQL's Rust API is based on SQLite and wraps the SQLite C API while adding transparent replication and compatibility with the SQLite ecosystem [libsql-rust]{1} [libsql-rust]{2}. It can connect to local and remote databases, and its feature flags distinguish core local/embedded-replica code from replication support [libsql-rust]{4} [libsql-rust]{5} [libsql-rust]{6} [libsql-rust]{7} [libsql-rust]{8}. For v0's local-first, no-replication storage posture, libSQL is an enriching candidate rather than a necessary default: it imports more future-facing replication surface than the v0 backend needs {inferred: fit}.

### sled

Sled is viable as an embedded key-value substrate only if Patchbay is willing to implement more log shape itself. It documents atomic operations, ACID transactions, serializable transactions, recovery detection, and `flush`/`flush_async` guarantees that previous writes recover after a crash if flush succeeds [sled]{2} [sled]{3} [sled]{5} [sled]{8} [sled]{9} [sled]{10}. But sled's own generated ID facility is explicitly monotonic but not contiguous, which disqualifies that facility as Patchbay's gap-free LSN allocator [sled]{7}. Sled also qualifies durability by saying database state is guaranteed only up to the last `flush` unless periodic sync or IO-buffer rotation has occurred [sled]{6}. That makes sled possible behind the port, but less directly aligned with an SQL append table plus transaction boundary for v0 {inferred: fit}.

### Custom WAL

A custom WAL has low source-backed leverage in this pass. SQLite's WAL docs show that a correct WAL has commit records, sync-mode tradeoffs, checkpoint ordering, and power-loss caveats [sqlite-wal]{2} [sqlite-wal]{6} [sqlite-wal]{7} [sqlite-wal]{8}. Rebuilding those behaviors inside Patchbay would move crash-safety burden into project code without a fetched source showing a Rust custom-WAL crate that already supplies Patchbay's single-writer, gap-free LSN contract {inferred: fit}. Treat custom WAL as a reserved fallback if SQLite is rejected for a concrete reason.

### Event-sourcing crates

`cqrs-es` is not a durable-log answer for Patchbay's core protocol. Its docs describe CQRS/event sourcing and persistent store integrations, but the attested event envelope uniqueness is aggregate-scoped (`aggregate_type`, `aggregate_id`, `sequence`) and its sequence is for an aggregate instance [cqrs-es]{1} [cqrs-es]{3} [cqrs-es]{4} [cqrs-es]{8} [cqrs-es]{9}. Patchbay needs a total authority-domain log order for terminal races, snapshots, cursors, and replay; using an aggregate framework would still require a separate authoritative log adapter {inferred: fit}.

## State-machine crates

Patchbay should hand-roll or generate registry-driven transition tables for canonical protocol states first, then optionally use a state-machine crate for non-authoritative implementation internals. Statig is designed for dynamic systems where external events arrive in runtime order and are submitted with `handle()`, supports hierarchical state machines, async actions/handlers, and optional macros [statig]{1} [statig]{3} [statig]{4} [statig]{6} [statig]{7}. Smlang provides a procedural macro DSL that generates state, event, and state-machine types, supports guards/actions including async functions, wildcard and multi-state transitions, and state data [smlang]{1} [smlang]{3} [smlang]{4} [smlang]{5} [smlang]{6} [smlang]{7}.

Those properties are useful, but they are also why these crates should not become the first source of truth for `CommandState`, `ElicitationState`, or session connectivity/activity axes. Smlang's DSL generates its own states/events, and statig's macro/state-machine layer is an authoring surface separate from Patchbay's protocol registry [smlang]{3} [statig]{3}. For Patchbay's registry-driven design, the safer shape is: protocol registry table -> generated Rust enum/transition predicates/conformance tests -> optional crate integration only if generated from that registry {inferred: fit}. Statig aligns with a complex hierarchical runtime/session-side machine because its docs explicitly target hierarchical event-driven systems and dynamic external event order [statig]{1} [statig]{6}. Smlang aligns with compact macro-declared transition matrices where generated documentation is helpful [smlang]{1} [smlang]{2}.

## Async runtime, property testing, and tonic

Tokio fits the core service layer because the fetched docs cover the runtime pieces Patchbay needs around gRPC serving, adapters, background observation streams, timers, and process/signal/file-system integration [tokio]{3} [tokio]{4} [tokio]{5}. For the storage path, Tokio does not make blocking SQLite calls magically non-blocking; if rusqlite is used, keep SQLite on a dedicated writer thread or use a controlled blocking boundary while preserving a single-writer append discipline {inferred: integration} [tokio]{9} [rusqlite]{9}.

Proptest fits protocol-registry conformance because it can generate arbitrary inputs, shrink failing cases, compose per-value strategies, and has state-machine testing in scope [proptest]{2} [proptest]{3} [proptest]{5} [proptest]{6} [proptest]{8}. The practical v0 use is to generate operation sequences, terminal-candidate races, snapshot cursors, and replay prefixes from the same registry tables that generate Rust types and conformance vectors {inferred: integration}.

Tonic is a plausible Rust-side server half for the internal seam. Its docs position it as gRPC over HTTP/2 with async/await support and as a production-systems building block, and the transport feature provides client/server implementation based on hyper, tower, and tokio [tonic]{2} [tonic]{3} [tonic]{5}. The server builder is described as a batteries-included HTTP/2 gRPC server, and streaming request/response support is explicit through `Streaming` and `IntoStreamingRequest` [tonic]{8} [tonic]{9} [tonic]{10} [tonic]{12}. This facet does not decide the TypeScript/Connect client fit; it only finds the Rust server-side maturity adequate for a prototype internal seam {inferred: scope}.

## Disconfirming analysis

SQLite is not a free durability proof. WAL mode with `synchronous=NORMAL` omits the per-commit sync, and SQLite says transactions in that configuration are no longer durable and may roll back after power failure or hard reset [sqlite-wal]{6} [sqlite-wal]{8}. Therefore any Patchbay safety claim about "no accepted command disappears silently" must pin or test the actual SQLite sync mode and acceptance boundary, not merely say "SQLite WAL" {inferred: risk}.

Sled disconfirms a shortcut implementation that uses its generated IDs as LSNs: sled's docs say generated IDs are monotonic but not contiguous [sled]{7}. Sled also disconfirms assuming every successful write is crash-durable without an explicit flush or configured sync behavior, because its docs guarantee state only up to the last `flush` except for periodic sync / buffer rotation [sled]{6} [sled]{8}.

`cqrs-es` disconfirms replacing the core log with an off-the-shelf aggregate event-source framework: its event envelope uniqueness and sequence are aggregate-instance scoped [cqrs-es]{8} [cqrs-es]{9}. That does not satisfy a total order for first-terminal-commit-wins across commands, observations, snapshots, and audit records {inferred: risk}.

State-machine crates disconfirm a naive "use a state-machine crate for protocol semantics" answer. Statig is made for dynamic runtime event loops, and smlang generates its own state/event types from a DSL [statig]{6} [statig]{7} [smlang]{3}. Patchbay's protocol registries must remain the source of truth; crate DSLs are acceptable only if downstream of the registry or limited to internal machines {inferred: risk}.

Tonic's Rust server evidence is positive, but it does not answer the browser/client transport question. The tonic docs establish Rust gRPC/HTTP2, server transport, and streaming primitives [tonic]{1} [tonic]{5} [tonic]{10}; they do not establish TypeScript Connect interoperability or browser ergonomics {inferred: scope}.

## Contradictions

No direct contradictions were found among fetched sources. The important tensions are qualifiers rather than contradictions:

- SQLite WAL gives a local append/commit model, but durability depends on synchronous mode and checkpoint ordering [sqlite-wal]{2} [sqlite-wal]{6} [sqlite-wal]{7} [sqlite-wal]{8}.
- Sled advertises atomic operations and ACID transactions, while separately qualifying crash recovery by `flush`/sync timing and generated IDs as monotonic-not-contiguous [sled]{2} [sled]{3} [sled]{6} [sled]{7}.
- State-machine crates provide useful generated state/event machinery, but Patchbay's canonical protocol registries must own protocol variants and transition truth {inferred: architecture} [statig]{8} [smlang]{3}.

## Revisit if

- SQLite with WAL plus `synchronous=FULL` cannot meet v0 latency targets once acceptance, snapshot, and replay tests run against realistic hardware.
- The storage port cannot enforce "assign LSN only at durable commit" with a gap-free append table and idempotent retry semantics.
- LibSQL's replication/remote features become a committed v0 requirement rather than a reserved future seam.
- The internal seam chooses an async database/pool style that makes SQLx materially simpler than a dedicated rusqlite writer thread.
- A future crate/source is found that explicitly provides a single-writer, local, crash-recoverable, gap-free, monotonic Rust event log with documented fsync semantics.
- Protocol registries become generated in a way that can emit statig/smlang definitions without creating a second state-machine source of truth.

## Acquisition candidates

None. The facet did not encounter a blocking source-access gap. The only enriching follow-up would be implementation-level benchmarking documents or crash-test reports after a prototype exists; those are not acquisition candidates for this source pass.
