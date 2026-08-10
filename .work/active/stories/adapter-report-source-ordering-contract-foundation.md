---
id: adapter-report-source-ordering-contract-foundation
kind: story
stage: implementing
tags: [protocol, foundation]
parent: adapter-report-source-ordering
depends_on: []
release_binding: null
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
