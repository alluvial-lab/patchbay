---
id: story-protocol-idl-conformance-vectors
kind: story
stage: review
tags: [protocol, verification, foundation]
parent: feature-protocol-idl-and-conformance
depends_on: [story-protocol-idl-proto-package]
created: 2026-07-06
updated: 2026-07-06
gate_origin: null
release_binding: null
---

# Story: Author the v0 conformance vectors

Implements Unit 3 of `feature-protocol-idl-and-conformance`.

## Scope

Author `contracts/vectors/*.json` — one JSON file per vector, with the structured envelope (model property id, promotion status, `.proto` fields constrained, input, expected outcome). Required vectors (from the feature brief's acceptance criteria):

- Command acceptance (valid submission → `accepted`).
- Reply correlation (response Operation → command/message id via typed correlation).
- Snapshot reconciliation (stale snapshot rejected; cursor replay returns only `LSN > cursor`).
- Terminal-commit race: completion before cancellation; cancellation before completion; expiration before late completion; retry after terminal returns existing; late terminal candidate as audit/reconciliation only; replay of the same committed prefix is idempotent.
- Failure/outcome mapping: unknown OperationKind → `validation_failed`; missing grant → `authorization_denied`; missing target → `target_not_found`.

Vectors reference `.proto` message types and enum values by fully-qualified name. See the feature body's Unit 3 for the format and acceptance criteria.

## Acceptance criteria

- [ ] All required vector cases above exist as JSON files.
- [ ] Each vector carries the structured envelope (property id, promotion status, `.proto` fields).
- [ ] Vectors reference real `.proto` types (no forward references to types that don't exist).
- [ ] `contracts/vectors/README.md` documents the vector format.

## Review (2026-07-06)

**Verdict**: Approve (fast-lane via feature review)

**Notes**: Reviewed as part of the feature-protocol-idl-and-conformance deep-lane review (gpt-5.5 fresh context). Initial review returned Request changes (3 important findings: failure-vector operation_state contradiction, reply-correlation mis-typing, missing drift check); all fixed in commit 9a2854f; targeted re-review returned READY. Builds pass (cargo build, npm run build); check-vectors.mjs passes (12 vectors); check:drift detects generated-code modifications. Story advanced implementing → review; rolled up to feature.
