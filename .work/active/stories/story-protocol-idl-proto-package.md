---
id: story-protocol-idl-proto-package
kind: story
stage: done
tags: [protocol, foundation]
parent: feature-protocol-idl-and-conformance
depends_on: []
created: 2026-07-06
updated: 2026-07-06
gate_origin: null
release_binding: null
---

# Story: Author the v0 .proto package

Implements Unit 1 of `feature-protocol-idl-and-conformance`.

## Scope

Author `contracts/proto/patchbay/{common,operations,observations,elicitations,sessions,authority,adapter}.proto` + `contracts/proto/buf.yaml`. Map every committed registry in `docs/PROTOCOL.md` to Protobuf enums/messages. Payloads are `bytes` + `PayloadContentType` enum (opaque, per Q1). Reserved kinds/contracts are wire-present enum values marked reserved. Package split by concern (Q2). `buf.yaml` configures the `patchbay` package with lint + breaking rules per `feature-research-contract-tooling`.

See the feature body's Unit 1 for the per-file content and acceptance criteria.

## Acceptance criteria

- [ ] Every committed registry in `docs/PROTOCOL.md` has a corresponding Protobuf enum/message.
- [ ] Reserved kinds/contracts are enum values marked reserved, not omitted.
- [ ] Payloads are `bytes` + `PayloadContentType`, not contract_kind-specific schemas.
- [ ] `buf lint` passes on the package.
- [ ] `.proto` derives names from `docs/PROTOCOL.md` registries (no parallel vocabulary).

## Review (2026-07-06)

**Verdict**: Approve (fast-lane via feature review)

**Notes**: Reviewed as part of the feature-protocol-idl-and-conformance deep-lane review (gpt-5.5 fresh context). Initial review returned Request changes (3 important findings: failure-vector operation_state contradiction, reply-correlation mis-typing, missing drift check); all fixed in commit 9a2854f; targeted re-review returned READY. Builds pass (cargo build, npm run build); check-vectors.mjs passes (12 vectors); check:drift detects generated-code modifications. Story advanced implementing → review; rolled up to feature.
