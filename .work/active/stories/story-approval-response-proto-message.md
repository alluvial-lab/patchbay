---
id: story-approval-response-proto-message
kind: story
stage: done
tags: [protocol, verification, foundation]
parent: feature-v0-approval-response-contract
depends_on: []
release_binding: v0.1.0
gate_origin: null
created: 2026-07-19
updated: 2026-07-20
---

# Story: Typed proto message (ApprovalDecision + ApprovalResponsePayload)

Checkpoint for `feature-v0-approval-response-contract` Unit 1. Critical-path
head: the types Units 2, 3, 4 depend on.

## Deliverable

Add `ApprovalDecision` enum + `ApprovalResponsePayload` message to
`contracts/proto/patchbay/elicitations.proto`. The decision lives in the
*response Operation's payload*, NOT in the `response_contract` body — no
`oneof contract_body` arm for approval.

```proto
enum ApprovalDecision {
  APPROVAL_DECISION_UNSPECIFIED = 0;
  APPROVAL_DECISION_APPROVED = 1;
  APPROVAL_DECISION_DENIED = 2;
  // reserved, not validatable in v0; submissions reject with validation_failed.
  APPROVAL_DECISION_RESERVED_ALLOW_ONCE = 100;
  // reserved, not validatable in v0; submissions reject with validation_failed.
  APPROVAL_DECISION_RESERVED_ALWAYS = 101;
  // reserved, not validatable in v0; submissions reject with validation_failed.
  APPROVAL_DECISION_RESERVED_POLICY_AMEND = 102;
  // reserved, not validatable in v0; submissions reject with validation_failed.
  APPROVAL_DECISION_RESERVED_MODIFIED_INPUT = 103;
}

message ApprovalResponsePayload {
  ApprovalDecision decision = 1;
}
```

`Operation.payload` (`PayloadEnvelope`) is unchanged in shape — for an
`APPROVAL_RESPONSE`, `content_type` is `PAYLOAD_CONTENT_TYPE_PROTOBUF` and the
bytes are a serialized `ApprovalResponsePayload`.

## Implementation steps

1. Edit `contracts/proto/patchbay/elicitations.proto` to add the enum + message.
2. Regen: `buf generate` (binary at
   `/home/agent/.npm-global/lib/node_modules/@bufbuild/buf/bin/buf`) for TS, then
   `git checkout -- contracts/rust/src/gen` to discard buf's Rust output (wrong
   generator — pre-existing divergence), then `cargo build` (with
   `CARGO_HOME=/home/agent/projects/patchbay/.cargo-cache`) to regen the
   committed Rust via `build.rs`/prost-build.
3. Verify: `cd contracts/ts && npm run build && npm run check:vectors`.
4. Commit proto + regenerated TS + regenerated Rust bindings together.

## Acceptance evidence

- [ ] `ApprovalDecision` enum + `ApprovalResponsePayload` message exist with the
      fields/values above.
- [ ] No `oneof contract_body` arm for approval (decision is response-side).
- [ ] Reserved decision values (100-103) have "reserved, not validatable" comments.
- [ ] `buf generate` + `cargo build` regen both targets; TS builds clean.
- [ ] `npm run check:vectors` passes.

## Notes

- `check:drift` is the pre-existing broken gap (needs `protoc-gen-prost`, not
  run in CI). A proto change reports drift vs the committed Rust bindings
  regardless of correctness — expected, not this story's concern. The TS build
  is the real check.
- Enum values follow the `RESPONSE_CONTRACT_KIND_RESERVED_*` pattern (committed
  low, reserved at 100+).

## Implementation notes

- Execution capability: direct inline implementation; the generated-contract change was bounded and the required regeneration procedure was fully specified.
- Review weight: standard (project default); review is deferred to the feature boundary because this is a child-story checkpoint.
- Files changed: `contracts/proto/patchbay/elicitations.proto`, generated Rust and TypeScript bindings.
- Tests added/removed: none; generated bindings were verified through the workspace build, TypeScript compiler, and existing vector gate.
- Simplification: no approval contract-body arm or parallel handwritten DTO was introduced; the decision remains response-payload-only.
- Discrepancies from design: Rust generation writes the consolidated `contracts/rust/src/gen/patchbay/patchbay.rs` file rather than the older split path named in the story; generation otherwise followed the specified canonical procedure.
- Adjacent issues parked: none. The known `check:drift` generator divergence was intentionally not run or changed.
- Verification: `CARGO_HOME=/home/agent/projects/patchbay/.cargo-cache cargo build --workspace --all-targets`, `cd contracts/ts && npm run build`, and `npm run check:vectors` passed (19 vectors).
