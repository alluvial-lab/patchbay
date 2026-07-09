---
id: story-fix-failurecode-execution-outcome-unknown
kind: story
stage: done
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

## Implementation notes

- Files changed: `contracts/proto/patchbay/operations.proto` (added `FAILURE_CODE_EXECUTION_OUTCOME_UNKNOWN = 14;` after `FAILURE_CODE_STALE_EVENT = 13;`), `contracts/rust/src/gen/patchbay/patchbay.rs` (regenerated), `contracts/ts/src/gen/patchbay/operations_pb.ts` (regenerated).
- Tests added: none — generated-contract change; verification is the drift check, not a new test.
- Verification performed: `buf generate` (exit 0); `check-generated-drift.mjs` clean (fresh regen matches staged, no drift); `check-vectors.mjs` pass; `check-models.mjs` pass. Confirmed the new value landed in both generated outputs (Rust `ExecutionOutcomeUnknown = 14` + its string round-trip; TS `EXECUTION_OUTCOME_UNKNOWN = 14`).
- Discrepancies from design: none. The fix is exactly as specified in the story body.
- Adjacent issues parked: none. The related Important finding from the epic review (automated checks don't catch proto↔prose drift) is already filed as `idea-proto-prose-registry-consistency-check`.
- Acceptance criteria walk:
  - `FailureCode` enum includes `FAILURE_CODE_EXECUTION_OUTCOME_UNKNOWN` → met (`operations.proto:86`).
  - `buf generate` regenerates Rust + TS; `check:drift` passes → met (clean regen, drift check clean).
  - PROTOCOL.md failure vocabulary and proto enum agree → met (both carry the term; no drift).
  - Any conformance vector covering `execution_outcome_unknown` updated if one exists → met (no vector references it; nothing to update).

## Review (2026-07-09)

**Verdict**: Approve

**Blockers**: none
**Important**: none
**Nits**: none

**Notes**: Fast-lane story review. Implementation verification is green and recorded: `buf generate` exit 0, `check-generated-drift.mjs` clean (fresh regen matches staged), `check-vectors.mjs` pass, `check-models.mjs` pass — re-confirmed green at review time. The fix is a single proto enum value + regeneration, exactly as the story specified; all four acceptance criteria met. The generated-contract drift the epic review caught is now closed. Advanced review → done.
