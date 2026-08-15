---
id: fleet-spawn-target-resolution
kind: story
stage: implementing
tags: [adapter, protocol, security]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-cursor-authoritative-replacement-contract, research-handoff-spawn-runtime-evidence-promotion-contract]
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-12
---

# Operation-aware spawn target and compound authority resolution

## Redesign disposition

Rewritten after the 2026-08-12 adversarial review. The historical `fleet` id is retained for reference stability, but this checkpoint does not select a fleet. It consumes all early contract leaves rather than defining logical-target, claim, or continuation types downstream.

## Checkpoint

Resolve a `spawn` against one canonical attached adapter. Fresh spawn selects one live adapter-scoped `spawn` Grant. Continuation additionally resolves the exact current prior generation and selects a live exact-generation `session-management` Grant for the same verified subject/endpoint/domain, using the same sampled decision time under `CoreDecisionGate`.

Both selected Grant ids and the exact prior are returned for the accepted envelope. Adapter spawn authority alone never authorizes continuation. Runtime-session/resource/fleet/authority-domain spawn targets reject before durable acceptance; broadcast remains excluded.

## Design

**Files**
- `core/src/acceptance/ports.rs` — operation-aware target/authority result port.
- `core/src/target.rs` — one explicit adapter target plus exact-prior lookup.
- `core/src/authority/check.rs` — deterministic two-Grant compound selection/rejection.
- `core/src/acceptance/pipeline.rs` — validate generated payload and persist resolver-produced provenance.
- `server/src/{state,service}.rs` — catch up target/claim/authority projections under one gate before decision.
- Acceptance/authority/resolver tests.

```rust
pub enum TargetBinding {
    SpawnAdapter {
        adapter_id: AdapterId,
        claim: SpawnGenerationClaim,
        continuation_authority: Option<ContinuationAuthorityProvenance>,
    },
    RuntimeSession { /* existing exact target */ },
    Resource(ResourceIdentity),
    AuthorityDomain(AuthorityDomainId),
}
```

For continuation, acceptance fails if the payload prior is not the exact current generation, if an active/poisoned claim already consumes N+1, or if either Grant is missing/revoked/expired/mismatched. Promotion later rechecks the exact replacement Grant's liveness; no other Grant id may silently replace accepted provenance.

## Acceptance evidence

- [x] Fresh spawn resolves with one adapter-spawn Grant and generation-1 claim.
- [x] Continuation resolves only with adapter-spawn + exact-prior session-management Grants for one verified subject/endpoint/domain.
- [x] Missing/revoked/expired/wrong-generation replacement Grant rejects before accepted append/delivery.
- [x] Runtime/resource/fleet/domain/malformed/mixed spawn targets reject before acceptance; unsupported adapter shape remains delivery-layer `unsupported_command`.
- [x] Restart replay preserves durable adapter routing eligibility without fabricating a live attachment.
- [x] Mutations accepting continuation on the broad spawn Grant alone or substituting another replacement Grant fail.

## Ordering constraint

Begins only after every contract leaf is available: the promotion-contract dependency brings the identity, continuation, claim, and crash-evidence leaves; the cursor leaf completes the parallel shared-contract layer.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` for the operation-aware resolver and security-critical compound Grant decision. This pass stops at `review` for independent review.
- `TargetResolver` now receives the complete validated Operation plus optional generated `SpawnRequest`. Spawn admits only one explicitly named attached adapter with declared spawn capability. Fresh spawn derives a deterministic logical-target id from the command id and generation 1; continuation accepts only the exact current logical-target runtime and derives checked `N+1`.
- `ResolvedGrantCheck` carries the first Grant selection, resolved target/claim, and one sampled timestamp into the second authority decision. Production revalidates the adapter-spawn Grant at that timestamp, then selects the lowest exact UTF-8 Grant id among live exact-runtime `session-management` Grants for the verified domain/actor/optional endpoint. Both ids and exact prior remain resolver-produced authority carriage; sender-authored fields cannot substitute.
- The acceptance pipeline's historical continuation guard was removed. Fresh spawn requires no continuation provenance; continuation requires complete `ContinuationAuthorityProvenance` before append. Target/claim/authority projections catch up under the existing `CoreDecisionGate`, and the locked resolver rejects invalid, conflicting, active, or poisoned claim candidates while allowing an exact retry. Atomic durable `SpawnClaimAccepted` construction remains the explicitly dependent Unit 2; no protobuf or generated artifact changed here.
- Resolver coverage rejects stale and one-field-mutated prior identities, cross-adapter continuation, unattached adapters, and runtime/resource/fleet/domain/mixed scopes. Authority coverage rejects spawn-only and replacement-only decisions, expired/revoked/wrong-subject/wrong-endpoint/wrong-generation replacement Grants, and proves canonical selection survives replay.
- Mutation probes: bypassing the replacement-Grant half failed `continuation_compound_authority_requires_exact_live_replacement_and_selects_canonically` at the spawn-only witness; trusting fabricated spawn provenance failed the same test at the replacement-only witness. Both mutants were reverted and the restored focused test passed.
- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, and 38 mutation witnesses.
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS**, 23/23 tests.
- Verification group 4 — `cd pi-adapter && npm test`: **PASS**, 29/29 tests including the real core/adapter restart e2e.
- Additional required gates — `PROPTEST_CASES=256 cargo test --workspace --features proptest`: **PASS**; `./formal/run-model-checks.sh`: **PASS**, 20/20; `cargo fmt --all --check` and `git diff --check`: **PASS**.
- Adjacent issues parked: none.
