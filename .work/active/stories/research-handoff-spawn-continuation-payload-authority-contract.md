---
id: research-handoff-spawn-continuation-payload-authority-contract
kind: story
stage: implementing
tags: [protocol, security]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-identity-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Continuation payload and compound authority provenance contract

## Checkpoint

Define one generated fresh/continuation payload and the durable compound authority provenance required by continuation. A continuation names the exact prior logical-target/runtime generation. Acceptance requires both an adapter-scoped `spawn` Grant and an exact-prior-generation `session-management` Grant for the same verified subject/endpoint/domain.

The accepted record preserves both selected Grant ids and the exact prior reference. The descendant Grant preserves both provenance links. Authentication/correlation prevents source confusion but is not proof that an authenticated adapter honestly observed an external effect.

## Design

**Files**
- `contracts/proto/patchbay/operations.proto` — `SpawnRequest`, `FreshSpawn`, `SpawnContinuation`, `SpawnTargetSpec`, and continuation-authority provenance carriage only; the downstream claim leaf adds `SpawnGenerationClaim` and claim/effect fields to the accepted envelope.
- `contracts/proto/patchbay/authority.proto` — `ContinuationAuthorityProvenance` and descendant provenance extension.
- Generated Rust/TypeScript artifacts and boundary validation tests.
- Protocol/security contract documentation updated when implemented.

```proto
message SpawnContinuation { RuntimeGenerationRef prior = 1; }
message ContinuationAuthorityProvenance {
  RuntimeGenerationRef exact_prior = 1;
  GrantId replacement_grant_id = 2;
  OperationKind replacement_authority_kind = 3;
}
```

`replacement_authority_kind` must be the canonical generated `session-management` value. It is explicit to make the security-relevant provenance self-describing; implementations do not accept another kind. The existing `authorizing_grant_id` remains the selected adapter-spawn Grant.

## Acceptance evidence

- [ ] Fresh and continuation are one `spawn` kind with disjoint generated payload variants.
- [ ] Continuation cannot omit or wildcard the exact prior logical/runtime generation.
- [ ] Accepted continuation and its descendant provenance carry both Grant ids; fresh spawn carries only the adapter-spawn half.
- [ ] The two provenance Grants must name the same verified subject/endpoint/domain and the replacement Grant must exactly contain the prior generation.
- [ ] Adapter-wide spawn authority alone cannot satisfy or fabricate continuation provenance.
- [ ] Unknown/reserved operation kinds, generation overflow, malformed target spec, and mixed intent reject before stateful work.

## Ordering constraint

Consumes the logical-target identity leaf. The claim registry and target/authority decision consume this contract.
