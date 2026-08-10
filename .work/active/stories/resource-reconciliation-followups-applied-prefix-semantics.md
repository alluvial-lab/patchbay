---
id: resource-reconciliation-followups-applied-prefix-semantics
kind: story
stage: done
tags: [adapter, protocol]
parent: resource-reconciliation-followups
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Apply resource events against one validated authority-domain prefix

## Checkpoint
Replace the resource fold's per-record obsolete checks with one whole-event
prefix decision at the `ResourceRegistry` boundary. The registry must know which
contiguous authority-domain prefix it has observed, advance that prefix for
owned and sibling durable event kinds, and classify a structurally valid
resource event at or below the prefix as an inert redelivery before generation
or `from_revision_lsn` checks. A new lower-generation event remains corrupt.

The prefix is projection metadata reconstructed from the durable log, not a new
wire field or persistence store. One `ResourceRegistry` represents one
authority-domain log, so it carries one domain-qualified cursor; source-adapter
generation remains checked inside the new-event resource fold. Roll the
corresponding assertions in `docs/PROTOCOL.md`, `docs/ARCHITECTURE.md`, and
`docs/GLOSSARY.md` forward before describing the implementation evidence.

## Primary files
- `core/src/resource/registry.rs`
- `core/src/resource/replay.rs`
- `core/src/resource/ingest.rs`
- `core/tests/resource_state.rs`
- `core/tests/resource_replay.rs`
- `docs/PROTOCOL.md`
- `docs/ARCHITECTURE.md`
- `docs/GLOSSARY.md`

## Acceptance evidence
- A lower-generation `ResourceStateEvent` whose LSN is prefix-covered returns
  success and leaves the complete registry, including its cursor, unchanged.
- The same lower-generation payload at the next uncovered LSN fails with
  `ResourceError::CorruptLog` and does not advance the cursor or partially fold.
- A gap, wrong domain, zero/missing LSN, unknown/unspecified durable kind, or
  malformed owned payload cannot establish prefix coverage.
- Known sibling events advance the resource projection's prefix without
  changing resource/view state; full replay still rejects gaps and duplicate
  storage rows rather than laundering them as catch-up redelivery.
- Production report ingestion catches the projection up through the durable
  tail before normalization; after append, the new resource event is the next
  cursor candidate under the existing `CoreDecisionGate` serialization.
- The per-view/per-record `revision_lsn >= event_lsn` skip branches are removed;
  obsolete handling has one whole-event source of truth.

## Ordering constraint
This semantic checkpoint must land before the generated reconciliation evidence
so the generator tests a fixed protocol rule rather than selecting the rule by
what current code happens to do.

## Implementation notes
- Execution capability: `openai-codex/gpt-5.6-sol`; caller-selected because the protocol replay prefix and rejected-state atomicity are load-bearing.
- Review weight: `thorough` from the explicit caller selection; feature review is intentionally deferred to a fresh reviewer at `stage: review`.
- Files changed: `core/src/resource/{registry,replay,ingest}.rs`, `core/tests/{resource_state,resource_replay,resource_ingest}.rs`, `docs/{PROTOCOL,ARCHITECTURE,GLOSSARY}.md`.
- Tests added/removed: added covered-vs-next lower-generation regression, sibling framing/gap/owned-payload prefix validation, strict replay-gap rejection, and report-ingress durable-tail synchronization; removed no tests.
- Simplification: removed the per-view and per-record obsolete-LSN branches; whole-event applied-prefix classification is the single redelivery rule.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Verification evidence: `cargo test -p patchbay-core --test resource_state --test resource_replay --test resource_ingest --test resource_reconciliation`; `cargo test -p patchbay-core --tests`; `cargo clippy -p patchbay-core --lib --test resource_state --test resource_replay --test resource_ingest --test resource_reconciliation -- -D warnings`.
