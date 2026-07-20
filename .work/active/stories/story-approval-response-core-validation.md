---
id: story-approval-response-core-validation
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

# Story: Core boundary validation + DENIED→Declined terminal mapping

Checkpoint for `feature-v0-approval-response-contract` Unit 2. The
load-bearing unit — this is where the decision-driven design is earned.

## Deliverable

Two changes:

**(a) Validation** — in `core/src/acceptance/elicitation_response.rs`, replace
the approval no-op-pass arm in `validate_response_payload`:
```rust
// was: (ApprovalResponse, Approval) => return Ok(())  // no-op pass
// now: decode + validate the decision (Fail Fast)
(OperationKind::ApprovalResponse, ResponseContractKind::Approval) => {
    let payload = decode_approval_payload(operation)?;  // content_type PROTOBUF check + decode
    match payload.decision {
        ApprovalDecision::Approved | ApprovalDecision::Denied => {}
        ApprovalDecision::Unspecified => return Err("approval response has an unspecified decision".to_owned()),
        _ => return Err(format!("approval decision {:?} is reserved and not validatable in v0.1.0", payload.decision)),
    }
    return Ok(());
}
```
The `content_type` check mirrors the question side (Blocker 2 fix): reject if
not `PAYLOAD_CONTENT_TYPE_PROTOBUF` before decoding.

**(b) Terminal mapping** — in `core/src/acceptance/elicitation.rs`, extend
`terminalize_slot`. On a `Completed` *approval-response* transition, decode the
decision (kind-gated): `APPROVED`→`Answered`, `DENIED`→`Declined`. Question
responses stay payload-opaque (Completed → `Answered`, existing). Non-Completed
terminals leave the slot pending (unchanged). **Remove the deferred comment at
line 301.** Do NOT add a `Rejected`→`Declined` mapping (that was the conflation
this feature fixes).

Full signatures + the kind-gating rationale in the feature body Unit 2.

## Acceptance evidence

`validate_response_payload` rejects:
- [ ] approval with unspecified decision
- [ ] approval with a reserved decision (100-103)
- [ ] approval with wrong content_type (not PROTOBUF)
- [ ] approval against a non-approval contract (kind mismatch, existing)

`validate_response_payload` accepts:
- [ ] APPROVED
- [ ] DENIED

`terminalize_slot`:
- [ ] Completed APPROVED → `Answered`
- [ ] Completed DENIED → `Declined` (the load-bearing assertion)
- [ ] Completed question response → `Answered` (unchanged, payload-opaque)
- [ ] non-Completed terminal → slot stays pending (unchanged)

- [ ] The deferred comment at `elicitation.rs:301` is removed; no
      `Rejected`→`Declined` mapping exists.
- [ ] Idempotent retry of a terminal approval response returns the existing
      command (not `validation_failed`) — regression test (the `winning_response`
      exemption from the elicitation-response-contract fix covers this).
- [ ] `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` pass
      (with `CARGO_HOME=/home/agent/projects/patchbay/.cargo-cache`).

## Notes

- Kind-gating: the decision decode runs **only** for `ApprovalResponse` +
  `Completed`. Question responses stay payload-opaque. This is the bounded
  architectural shift (see feature body Architectural choice).
- The slot layer's `command_operations` map + `winning_response` (added in the
  elicitation-response-contract fix) already hold the response Operation; the
  decision decode reads `response_operation.payload`.
- The idempotent-retry path is unchanged: `winning_response == Some(operation)`
  exempts the exact retry before the terminal mapping runs.

## Implementation notes

- Execution capability: direct inline implementation; this load-bearing core change was cohesive across the boundary validator and its event-log projection.
- Review weight: standard (project default); review is deferred to the feature boundary because this is a child-story checkpoint.
- Files changed: `core/src/acceptance/elicitation_response.rs`, `core/src/acceptance/elicitation.rs`.
- Tests added/removed: table-driven approval validation for both committed decisions, all four reserved decisions, unspecified, wrong content type, and kind mismatch; table-driven terminal mapping for APPROVED, DENIED, question, and machine rejection; corrupt-record fail-closed coverage; exact terminal approval retry coverage. No tests removed.
- Simplification: removed the stale Rejected-to-Declined deferred comment and narrowed terminal payload awareness to completed approval responses only. The private terminalizer now receives the concrete response Operation rather than an unnecessary `Option`.
- Discrepancies from design: prost stores enum fields as `i32`, so validation and projection explicitly convert with `ApprovalDecision::try_from`; unknown numeric decisions fail fast in addition to the specified reserved values. The approval-specific retry regression proves it clears validation, while the existing pipeline regression proves that this exemption reaches storage dedup and returns the existing command record.
- Adjacent issues parked: none.
- Verification: `CARGO_HOME=/home/agent/projects/patchbay/.cargo-cache cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` passed.
