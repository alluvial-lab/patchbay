---
id: story-approval-response-conformance-vectors
kind: story
stage: done
tags: [protocol, verification, foundation]
parent: feature-v0-approval-response-contract
depends_on: [story-approval-response-proto-message]
release_binding: null
gate_origin: null
created: 2026-07-19
updated: 2026-07-20
---

# Story: Conformance vectors for the approval response contract

Checkpoint for `feature-v0-approval-response-contract` Unit 4. Pins the two
committed decisions + three rejection classes as executable conformance vectors.

## Deliverable

Five conformance vectors in `contracts/vectors/`, using the existing envelope:

**Acceptance vectors** (property_id: `ElicitationInvalidResponseRejected` —
exercise the valid side):
1. `approval-response-approved.json` — APPROVED → `Answered`.
2. `approval-response-denied.json` — DENIED → `Declined` (the load-bearing
   vector: pins the decision-driven terminal mapping).

**Rejection vectors** (property_id: `boundary-validation` — descriptive draft):
3. `approval-response-invalid-unspecified-decision.json` — UNSPECIFIED →
   `validation_failed`.
4. `approval-response-invalid-reserved-decision.json` — ALLOW_ONCE (reserved) →
   `validation_failed`.
5. `approval-response-invalid-wrong-content-type.json` — JSON content_type →
   `validation_failed`.

## Acceptance evidence

- [ ] Five vectors exist, pass `check:vectors`.
- [ ] `proto_fields_constrained` references real Unit-1 fields:
      `patchbay.ApprovalDecision`, `patchbay.ApprovalResponsePayload.decision`,
      `patchbay.Operation.payload`, `patchbay.PayloadEnvelope.content_type`.
- [ ] Vectors carry structurally-complete `response_operation` (sender,
      target_scope, idempotency_key) + canonical `SubmissionResult` expectations
      (per the Important-1 fix from the elicitation-response-contract review).
- [ ] The DENIED vector's `expected_outcome` asserts the Elicitation terminalizes
      as `declined` (the decision-driven mapping).
- [ ] VERIFICATION.md traceability table lists the new vectors.

## Notes

- Same limitation as the question side: `check-vectors` validates envelope +
  property-id but does not execute the core's validation against the input (no
  invariant checker registered). The Rust unit tests (Unit 2) are the
  executable check today. Flagged in the feature's Risks.
- `property_id` choices: acceptance vectors reference the stated-normative
  `ElicitationInvalidResponseRejected` (already in the `check-vectors`
  registry); rejection vectors reference the descriptive `boundary-validation`
  id (draft-only).

## Implementation notes

- Execution capability: direct inline implementation; the five vector envelopes mirrored the reviewed question-response fixtures and required no new checker behavior.
- Review weight: standard (project default); review is deferred to the feature boundary because this is a child-story checkpoint.
- Files changed: five `contracts/vectors/approval-response-*.json` files and the generated conformance traceability block in `docs/VERIFICATION.md`.
- Tests added/removed: five draft vectors covering APPROVED, DENIED, unspecified, reserved ALLOW_ONCE, and wrong content type. No vectors removed.
- Simplification: approval contracts carry only `contract_kind`; no question body or handwritten approval contract body was added. The wrong-content vector reuses valid protobuf bytes and changes only the discriminator, isolating that boundary rule.
- Discrepancies from design: none. Accepted outcomes add the explicit `elicitation_terminal_state` field so APPROVED→Answered and the load-bearing DENIED→Declined mapping are reviewable in the vector itself.
- Adjacent issues parked: none; the known envelope-only vector checker limitation remains documented by the feature.
- Verification: `cd contracts/ts && npm run check:vectors` passed (24 vectors), and direct generated-schema decoding confirmed payload bytes map to decisions 1, 2, 0, and 100 as intended.
