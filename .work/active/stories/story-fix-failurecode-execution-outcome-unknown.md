---
id: story-fix-failurecode-execution-outcome-unknown
kind: story
stage: drafting
tags: [protocol, verification, bug, foundation]
parent: epic-foundation-hardening
depends_on: []
created: 2026-07-08
updated: 2026-07-09
gate_origin: null
release_binding: null
---

# Fix: FailureCode proto missing `execution_outcome_unknown`

`docs/PROTOCOL.md` failure vocabulary (line ~356) includes `execution_outcome_unknown` (added by `feature-idempotency-ambiguous-execution`, commit `60784e0`), but `contracts/proto/patchbay/operations.proto` `FailureCode` enum does not include `FAILURE_CODE_EXECUTION_OUTCOME_UNKNOWN`. Generated contracts and the normative registry disagree — a Generated Contracts + SSOT violation.

## Origin

Surfaced during the `feature-observability-operator-admin` deep review (pass 4) as a generated-contracts derivability drift. Parked rather than bundled into the observability feature — it predates that feature (introduced by idempotency) and belongs to the contracts/proto substrate, not observability.

## Fix

Add `FAILURE_CODE_EXECUTION_OUTCOME_UNKNOWN = 14;` to the `FailureCode` enum in `contracts/proto/patchbay/operations.proto`, regenerate (`buf generate`), and verify the drift check (`check-generated-drift.mjs` / `npm run check:drift`) passes. Update the conformance vectors if any vector exercises this failure code.

## Acceptance criteria

- [ ] `FailureCode` enum in `operations.proto` includes `FAILURE_CODE_EXECUTION_OUTCOME_UNKNOWN`.
- [ ] `buf generate` regenerates Rust + TS contracts; `check:drift` passes.
- [ ] `docs/PROTOCOL.md` failure vocabulary and the proto enum agree (no drift).
- [ ] Any conformance vector covering `execution_outcome_unknown` is updated if one exists.

## Routing

Small fix story — routes through `implement` (single file + regen + drift check). No design gate.
