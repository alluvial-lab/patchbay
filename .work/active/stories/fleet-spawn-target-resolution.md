---
id: fleet-spawn-target-resolution
kind: story
stage: review
tags: [adapter, protocol, security]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-cursor-authoritative-replacement-contract, research-handoff-spawn-runtime-evidence-promotion-contract]
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-14
---

# Operation-aware spawn target and compound authority resolution

## Redesign disposition

Rewritten after the 2026-08-12 adversarial review. The historical `fleet` id is retained for reference stability, but this checkpoint does not select a fleet. It consumes all early contract leaves rather than defining logical-target, claim, or continuation types downstream.

## Checkpoint

Resolve a `spawn` against one canonical attached adapter. Fresh spawn selects one live adapter-scoped `spawn` Grant. Continuation additionally resolves the exact current prior generation and selects a live exact-generation `session-management` Grant for the same verified subject/endpoint/domain, using the same sampled decision time under `CoreDecisionGate`.

Both selected Grant ids and the exact prior are returned by the compound resolution port for Unit 2's atomic accepted envelope. Until that writer exists, continuation submit is guarded before ordinary durable acceptance. Adapter spawn authority alone never authorizes continuation. Runtime-session/resource/fleet/authority-domain spawn targets reject before durable acceptance; broadcast remains excluded.

## Design

**Files**
- `core/src/acceptance/ports.rs` — operation-aware target/authority result port.
- `core/src/target.rs` — one explicit adapter target plus exact-prior lookup.
- `core/src/authority/check.rs` — deterministic two-Grant compound selection/rejection.
- `core/src/acceptance/pipeline.rs` — validate generated payload, preserve fresh-spawn acceptance, and guard continuation before the ordinary writer until Unit 2 persists resolver-produced provenance.
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

For continuation, direct resolution fails if the payload prior is not the exact current generation, if an active/poisoned claim already consumes N+1, or if either Grant is missing/revoked/expired/mismatched. The submit boundary remains guarded until Unit 2 can persist that complete result atomically. Promotion later rechecks the exact replacement Grant's liveness; no other Grant id may silently replace accepted provenance.

## Acceptance evidence

- [x] Fresh spawn resolves with one adapter-spawn Grant and generation-1 claim.
- [x] Continuation resolves only with adapter-spawn + exact-prior session-management Grants for one verified subject/endpoint/domain.
- [x] Missing/revoked/expired/wrong-generation replacement Grant rejects before accepted append/delivery.
- [x] Runtime/resource/fleet/domain/malformed/mixed spawn targets reject before acceptance; unsupported adapter shape remains delivery-layer `unsupported_command`.
- [x] Restart replay preserves durable adapter routing eligibility without fabricating a live attachment.
- [x] Mutations accepting continuation on the broad spawn Grant alone or substituting another replacement Grant fail.
- [x] Continuation submit is guarded with canonical `unsupported_command` and zero durable events until Unit 2's atomic writer; fresh spawn still accepts through resolved single-Grant authorization.

## Ordering constraint

Begins only after every contract leaf is available: the promotion-contract dependency brings the identity, continuation, claim, and crash-evidence leaves; the cursor leaf completes the parallel shared-contract layer.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` for the operation-aware resolver and security-critical compound Grant decision. This pass stops at `review` for independent review.
- `TargetResolver` now receives the complete validated Operation plus optional generated `SpawnRequest`. Spawn admits only one explicitly named attached adapter with declared spawn capability. Fresh spawn derives a deterministic logical-target id from the command id and generation 1; continuation accepts only the exact current logical-target runtime and derives checked `N+1`.
- `ResolvedGrantCheck` carries the first Grant selection, resolved target/claim, and one sampled timestamp into the second authority decision. Production revalidates the adapter-spawn Grant at that timestamp, then selects the lowest exact UTF-8 Grant id among live exact-runtime `session-management` Grants for the verified domain/actor/optional endpoint. Both ids and exact prior remain resolver-produced authority carriage; sender-authored fields cannot substitute.
- The continuation decision remains available through the resolver/authority ports, but the acceptance pipeline is fail-closed for continuation until it can atomically persist the claim and complete provenance. Fresh spawn still traverses target binding plus resolved single-Grant authorization. Target/claim/authority projections catch up under the existing `CoreDecisionGate`, and the locked resolver rejects invalid, conflicting, active, or poisoned claim candidates while allowing an exact retry. Atomic durable `SpawnClaimAccepted` construction and removal of this one submit guard remain the explicitly dependent Unit 2; no protobuf or generated artifact changed here.
- Resolver coverage rejects stale and one-field-mutated prior identities, cross-adapter continuation, unattached adapters, and runtime/resource/fleet/domain/mixed scopes. Authority coverage rejects spawn-only and replacement-only decisions, expired/revoked/wrong-subject/wrong-endpoint/wrong-generation replacement Grants, and proves canonical selection survives replay.
- Mutation probes: bypassing the replacement-Grant half failed `continuation_resolution_round_trips_both_grant_ids_and_exact_prior` at the spawn-only witness; trusting fabricated spawn provenance failed the same test at the replacement-only witness. Both mutants were reverted and the restored focused test passed.
- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, and 38 mutation witnesses.
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS**, 23/23 tests.
- Verification group 4 — `cd pi-adapter && npm test`: **PASS**, 29/29 tests including the real core/adapter restart e2e.
- Additional required gates — `PROPTEST_CASES=256 cargo test --workspace --features proptest`: **PASS**; `./formal/run-model-checks.sh`: **PASS**, 20/20; `cargo fmt --all --check` and `git diff --check`: **PASS**.
- Adjacent issues parked: none.

### Fix round — re-guard continuation until Unit 2

- Review disposition: the single BLOCKER in `.work/active/reviews/fleet-target-resolution-review-2026-08-14.md` was accepted. Unit 1's in-memory compound decision was correct, but the ordinary `AcceptedOperation` writer discarded the exact claim and replacement-Grant provenance, leaving replay, exclusivity, and legacy completion unsafe.
- Mechanism: the acceptance pipeline now enumerates `SpawnContinuation` as one temporary fail-closed guard after generated payload validation. It returns `SubmissionOutcome = rejected`, `FailureCode = unsupported_command`, and canonical reason `unsupported_command` before Grant lookup, target resolution, or durable append. This prevents any managed continuation `Operation` event from reaching `SpawnDescendantTail`; fresh spawn remains enabled and traverses the resolved single-Grant path.
- Sequencing decision: Unit 1 retains the operation-aware resolver, exact-current-prior lookup, one-time compound Grant decision, deterministic selection, and full provenance in the decision-port result. Durable compound carriage, atomic claim/provenance/fence/prior-work persistence, deduplication, and removal of exactly this guard belong to Unit 2 `spawn-delivery-atomic-claim-idempotency-generation`, as directed by the review.
- Boundary oracles: `guarded_continuation_writes_no_event_while_fresh_spawn_uses_resolved_grant_path` uses a compound-ready Grant adapter so removing the guard accepts and writes the unsafe ordinary event; in the guarded build it proves canonical rejection, zero Grant/resolver calls, and an empty durable log, while fresh spawn proves both authority phases run and persists the selected single Grant. `continuation_resolution_round_trips_both_grant_ids_and_exact_prior` compares the production decision port against an independent expected broad Grant, replacement Grant, authority kind, and exact prior both live and after durable authority-log replay.
- Mutation kills, all reverted with `git restore`: removing the restored guard failed the pipeline oracle with actual `Accepted` versus expected `Rejected`; stripping `replacement_grant_id` from the resolution result failed the independent round-trip expectation; bypassing the exact-prior replacement half failed the broad-spawn-only witness. Restored focused tests passed.
- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, and 38 killed mutation witnesses.
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS**, 23/23 tests.
- Verification group 4 — `cd pi-adapter && npm test`: **PASS**, 29/29 tests.
- Hygiene: `cargo fmt --all --check`, `git diff --check`, and restored focused tests are **PASS**. No proto or generated artifact changed; adjacent issues parked: none.
