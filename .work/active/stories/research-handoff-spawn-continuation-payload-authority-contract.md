---
id: research-handoff-spawn-continuation-payload-authority-contract
kind: story
stage: review
tags: [protocol, security]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-identity-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-13
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

- [x] Fresh and continuation are one `spawn` kind with disjoint generated payload variants.
- [x] Continuation cannot omit or wildcard the exact prior logical/runtime generation.
- [x] Accepted continuation and its descendant provenance carry both Grant ids; fresh spawn carries only the adapter-spawn half.
- [x] The two provenance Grants must name the same verified subject/endpoint/domain and the replacement Grant must exactly contain the prior generation.
- [x] Adapter-wide spawn authority alone cannot satisfy or fabricate continuation provenance.
- [x] Unknown/reserved operation kinds, generation overflow, malformed target spec, and mixed intent reject before stateful work.

## Ordering constraint

Consumes the logical-target identity leaf. The claim registry and target/authority decision consume this contract.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` (caller-selected for the security-critical authority contract leaf).
- Review weight: `thorough` (caller override; independent authority review remains orchestrator-owned, so this story stops at `stage: review`).
- Files changed: `contracts/proto/patchbay/{operations,authority}.proto`; generated Rust/TypeScript bindings; `core/src/acceptance/{spawn,mod,pipeline}.rs`; authority descendant-provenance projection/fixtures; `core/tests/spawn_continuation_contract.rs`; the server fresh-spawn fixture; `docs/{PROTOCOL,SECURITY}.md`.
- Tests added: eight mutation-sensitive contract tests cover generated intent disjointness, exact-prior identity/generation omissions, target-envelope bounds, exact schema decoding, two distinct Grant ids, missing replacement provenance, wrong authority kind, exact-prior mismatch, and descendant wire round-trip.
- Simplification: one validator owns decoded spawn shape and one narrower validator owns reusable continuation-provenance structure; neither resolves a target nor selects Grants.
- Design rationale: `SpawnTargetSpec` retains the previously resolved adapter-owned `{shape, adapter_payload, deployment_authority_ref}` seam; bounded validation admits any shape but leaves support to adapter delivery. Existing `AcceptedOperation.authorizing_grant_id` is the adapter-spawn half; the claim leaf will add accepted claim/effect carriage that composes it with this leaf's generated continuation provenance. Descendant provenance already preserves both links.
- Authority boundary rationale: this leaf proves only structural carriage. The downstream target-resolution decision must still prove both selected Grants share the verified subject/endpoint/domain and that the replacement Grant scope exactly contains `exact_prior`; authentication is not evidence of adapter honesty.
- Integration rationale: the live submit path preserves the existing security-lockdown-before-typed-payload precedence, then rejects malformed spawn payload before Grant evaluation, target resolution, or durability. The standalone boundary validator remains immediately structural.
- Discrepancies from design: none; grant selection and accepted claim/effect fields remain explicitly downstream.
- Adjacent issues parked: none.
- Verification: generated drift, conformance vectors/models, workspace all-target build, full workspace tests, and workspace all-target clippy with warnings denied are green.
