---
id: story-elicitation-response-core-validation
kind: story
stage: implementing
tags: [protocol, verification, foundation]
parent: feature-v0-elicitation-response-contract
depends_on: [story-elicitation-response-proto-messages]
release_binding: null
gate_origin: null
created: 2026-07-19
updated: 2026-07-19
---

# Story: Core boundary validation (Fail-Fast response payload check)

Checkpoint for `feature-v0-elicitation-response-contract` Unit 2. The
load-bearing unit — this is where the "typed contract the core validates"
claim is earned.

## Deliverable

A pure validation function `validate_response_payload(operation, active) ->
Result<(), String>` that rejects a malformed `ELICITATION_RESPONSE` (and
enforces kind/contract-kind correspondence for `APPROVAL_RESPONSE`) against
the active `response_contract` before durable acceptance. Plus the new
`ElicitationContractLookup` port trait + `ActiveElicitation` struct, and the
insertion of the check into `acceptance::pipeline::submit` (after structural
`validate_operation`, before grant check — Fail Fast).

Full signatures in the feature body Unit 2. Files touched:

- `core/src/acceptance/ports.rs` — new `ElicitationContractLookup` trait +
  `ActiveElicitation` struct
- `core/src/acceptance/elicitation_response.rs` — new module with
  `validate_response_payload`
- `core/src/acceptance/pipeline.rs` — `submit` gains a `C:
  ElicitationContractLookup` generic param; the validation branch runs for
  `ElicitationResponse | ApprovalResponse` kinds before grant check
- `core/src/acceptance/mod.rs` — re-export the new port + struct + function

## Acceptance evidence

`validate_response_payload` rejects (each a table-driven test case):
- [ ] missing `ElicitationId` typed correlation on the Operation
- [ ] unknown/wrong-domain elicitation (`active` is `None`)
- [ ] already-terminal elicitation (`active.is_terminal`)
- [ ] kind/contract-kind mismatch (approval-response against a question contract)
- [ ] question contract missing its `QuestionContract` body
- [ ] both `selected_option_id` and `free_text` set (exactly one primary answer)
- [ ] neither `selected_option_id` nor `free_text` set
- [ ] `selected_option_id` not in the contract's `options[].option_id`
- [ ] `free_text` when `question.allow_free_text == false`

`validate_response_payload` accepts:
- [ ] a valid `selected_option_id` matching a contract option
- [ ] a valid `free_text` when `allow_free_text == true`
- [ ] a selected option + EC2 `clarification` (answer-and)
- [ ] an approval response against an approval contract (no-op pass)

Pipeline integration:
- [ ] A malformed response Operation rejects as `validation_failed` with **no**
      grant-check call, **no** target resolution, and **no** durable append
      (Fail Fast — verified by the existing
      `*_reject_before_grant_without_durable_state` test pattern).
- [ ] A well-formed response Operation accepts normally (the validation is a
      pure precondition; the accepted path is unchanged).
- [ ] `cargo test --workspace` passes.
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes.

## Notes

- The check runs for both `ElicitationResponse` and `ApprovalResponse` kinds so
  the kind/contract-kind correspondence (D5) is enforced for approval too.
  Approval's typed body is a no-op pass in v0.1.0 (binary; existing payload
  unchanged).
- A terminal elicitation rejects as `validation_failed` with a clear
  diagnostic — a *different* response op id targeting an already-answered
  elicitation is not an idempotent retry (different idempotency key/command id)
  and must not be treated as one. First-answer-wins structural property is
  unchanged (the slot layer still owns terminalization); this is an early
  rejection.
- The new `C: ElicitationContractLookup` generic on `submit` widens the
  signature — every caller and every `acceptance_pipeline.rs` test needs a
  stub. Non-response tests use a `None`-returning stub; the validation branch
  only runs for response kinds. See Risks in the feature body.
