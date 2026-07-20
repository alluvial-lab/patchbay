---
id: story-elicitation-response-proto-messages
kind: story
stage: done
tags: [protocol, verification, foundation]
parent: feature-v0-elicitation-response-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-19
updated: 2026-07-19
---

# Story: Typed proto messages (question contract + response payload)

Checkpoint for `feature-v0-elicitation-response-contract` Unit 1. This is the
critical-path head: the types Units 2, 3, and 4 depend on.

## Deliverable

Add `ResponseOption`, `QuestionContract`, and `ElicitationResponsePayload`
messages to `contracts/proto/patchbay/elicitations.proto`, and a
`oneof contract_body { QuestionContract question = 8; }` arm on
`ResponseContract`. Then regenerate both codegen targets.

Exact shapes (see the feature body Unit 1 for the full proto):

```proto
message ResponseOption {
  string option_id = 1;
  string label = 2;
}
message QuestionContract {
  repeated ResponseOption options = 1;
  bool allow_free_text = 2;
}
message ElicitationResponsePayload {
  string selected_option_id = 1;   // singular — D2: select-many reserved
  string free_text = 2;
  string clarification = 3;        // EC2 answer-and supplement
}
```

`ResponseContract` gains field 8 `oneof contract_body { QuestionContract question = 8; }`.
`Operation.payload` (`PayloadEnvelope`) is unchanged in shape.

## Implementation steps

1. Edit `contracts/proto/patchbay/elicitations.proto` to add the three messages
   and the `oneof` arm.
2. `buf generate` from `contracts/` to regen both targets:
   - `contracts/rust/src/gen/patchbay/elicitations_pb.rs`
   - `contracts/ts/src/gen/patchbay/elicitations_pb.ts`
3. Verify the TS builds: `cd contracts/ts && npm run build`.
4. Verify vectors still pass: `npm run check:vectors` (envelope integrity — the
   new proto fields don't break existing vectors).
5. Commit the proto + regenerated bindings together (the bindings are the
   checked-in sync surface).

## Acceptance evidence

- [ ] Three new messages exist with the fields above (field numbers as shown).
- [ ] `ResponseContract.question` `oneof` arm at field 8.
- [ ] No `repeated selected_option_ids` field anywhere (D2 reserved-seam guard).
- [ ] `buf generate` succeeds with no generator errors.
- [ ] `contracts/ts` builds clean (`npm run build`).
- [ ] `npm run check:vectors` passes.
- [ ] Regenerated Rust + TS bindings committed alongside the proto change.

## Notes

- `check:drift` is a pre-existing broken repo gap (needs `protoc-gen-prost`,
  not run in CI). A proto change *will* report drift vs the committed Rust
  bindings regardless of correctness — this is expected and not this story's
  concern. The TS build *is* checked and must pass.
- Field numbers: `question = 8` is the next free tag on `ResponseContract`
  (1–7 used). The new messages start fresh from field 1.
- `oneof contract_body` is forward-compatible: a future promoted contract kind
  adds a new `oneof` arm, non-breaking. A v0.1.0 reader ignores unknown arms.

## Implementation notes

Added the three typed messages and the `ResponseContract.question` oneof arm.
Regenerated TypeScript with the canonical `buf generate` target and regenerated
Rust with the repository's `prost-build` build script after discarding the
pre-existing alternate buf Rust output. No multi-select field was added.

Verification: `cargo build --workspace --all-targets`, `cd contracts/ts && npm
run build`, and `npm run check:vectors` all pass. `check:vectors` reports 12
vectors and no traceability-table change.
