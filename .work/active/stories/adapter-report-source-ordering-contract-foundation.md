---
id: adapter-report-source-ordering-contract-foundation
kind: story
stage: done
tags: [protocol, foundation]
parent: adapter-report-source-ordering
depends_on: []
release_binding: v0.2.0
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Define the session-report source cursor and atomic wire event

## Checkpoint

Make Protobuf the single source for the adapter report, its
`(adapter_generation, revision)` source cursor, the atomic equal-generation
`SessionReportApplied` durable event, and the snapshot watermark. Regenerate
Rust and TypeScript, then roll the protocol/security/verification/glossary/Pi
adapter assertions and extension-seam classification forward.

## Acceptance evidence

- `patchbay.SessionReport` is moved, not copied, so adapter ingress and the
  durable event reuse one generated type; generated artifacts are never
  hand-edited.
- Fresh report ingress can require a present cursor, current adapter generation,
  and positive uint64 revision; `Session.last_source_cursor` remains distinct
  from core `last_authoritative_lsn`.
- Registration and runtime-generation replacement carry their initial source
  cursor; equal-generation reports have one atomic full-report event with the
  previous applied cursor.
- Existing delta tags remain readable for durable legacy history and
  core-authored degradation, but the contract does not require dual-writing.
- Foundation prose records committed single-producer source ordering, reserved
  multi-producer/per-field merge policy, and rejection of arrival-LSN,
  wall-clock, or promise-tail ordering authority.
- Buf generation, Rust/TypeScript contract builds, and final generated-drift
  verification pass.

## Ordering constraints

This is the first checkpoint. Core enforcement and Pi emission consume these
generated types; conformance promotion must use the settled field paths and
property id.

## Current-HEAD reconciliation (2026-08-10)

- `SessionGenerationBumped.spawn_origin` now occupies tag 11, so the generated source cursor uses the next stable tag 12 there. The design sketch's tag 11 is superseded; no existing tag is reused.
- `SessionReport` still lives only in `adapter_control.proto` with tags 1–11. Moving it to `sessions.proto` preserves the package-qualified `patchbay.SessionReport` identity and adds `source_cursor = 12`.
- Existing session delta variants/tags are durable data and stay readable. The new `report_applied = 8` variant is additive; generated Rust/TypeScript files remain outputs only.
- Current model/vector traceability is generated from live registries, so this checkpoint updates normative prose/property registration without hand-editing either generated table.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`, maximum reasoning (caller-selected for a cross-language durability/security contract); direct-read single-owner delivery, no nested agents or peeragent.
- Review weight: `thorough` (explicit caller override); child checkpoint review is not applicable.
- Files changed: `contracts/proto/patchbay/{sessions,adapter_control}.proto`; regenerated `contracts/rust/src/gen/patchbay/patchbay.rs` and `contracts/ts/src/gen/patchbay/{sessions,adapter_control}_pb.ts`; `docs/{PROTOCOL,SECURITY,VERIFICATION,GLOSSARY,ADAPTER-PI}.md`.
- Tests added/removed: no implementation-bound serialization tests; generation plus both generated-language builds protect the schema boundary.
- Simplification: moved the sole package-qualified `SessionReport` definition instead of copying it; the new atomic event reuses that generated report shape, and legacy delta tags remain intact without a dual-write contract.
- Discrepancies from design: `SessionGenerationBumped.source_cursor` uses current-HEAD tag 12 because `spawn_origin` already owns tag 11; model/vector registry and generated traceability promotion remain intentionally with the conformance checkpoint.
- Adjacent issues parked: none.

## Verification evidence

- `PATH="$HOME/.npm-global/bin:$PATH"; (cd contracts && buf generate && buf build proto)` — passed.
- `cargo test -p patchbay-contracts` — passed.
- `npm --prefix contracts/ts run build` — passed.
- Post-commit `npm --prefix contracts/ts run check:drift` — passed; generated Rust and TypeScript are byte-current with the proto source.
- `git diff --check` — passed before the implementation commit.
