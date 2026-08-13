---
id: research-handoff-spawn-claim-registry-contract
kind: story
stage: review
tags: [protocol, security, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-identity-contract, research-handoff-spawn-continuation-payload-authority-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-13
---

# Spawn claim registry and pending-replacement fence contract

## Checkpoint

Define the durable-log-derived exclusive claim state machine before acceptance or delivery consumes it. `CommandState` terminality is not claim release. A claim is keyed by authority domain + logical target + expected prior generation (`None` for fresh) and consumes exactly generation `1` or `N+1`.

Continuation acceptance also activates a durable pending-replacement delivery fence for exact prior N. New N-bound work rejects; never-offered accepted work is explicitly superseded; offered/running work enters quiesce/outcome reconciliation. The fence remains for active or poisoned claims.

## Design

**Files**
- Generated claim/disposition/event shapes under `contracts/proto/patchbay/operations.proto` and `sessions.proto`; the accepted continuation decision carries the fence plus explicit supersession/quiesce effects for affected prior work.
- New `core/src/session/spawn_claim.rs` — claim record, exclusive-key index, fold, query port.
- Claim replay/checkpoint, transition-table, property, and concurrency tests.

```rust
pub enum SpawnClaimDisposition {
    Active,
    ReleasedNoExternalEffect,
    PoisonedPendingReconciliation,
    Promoted,
    TargetAbandoned,
}

pub struct SpawnClaimRecord {
    pub claim: SpawnGenerationClaim,
    pub accepted_lsn: u64,
    pub compound_authority: Option<ContinuationAuthorityProvenance>,
    pub disposition: SpawnClaimDisposition,
    pub pending_replacement: Option<RuntimeGenerationRef>,
}
```

Allowed transitions are `active → released_no_external_effect | poisoned | promoted | target_abandoned`, `poisoned → promoted | released_no_external_effect | target_abandoned` only with later exact evidence, and terminal claim dispositions never return to active. A later claim for the same generation is forbidden after promotion/abandonment and while active/poisoned.

## Acceptance evidence

- [ ] Two distinct commands cannot own one exclusive claim, including after restart replay.
- [ ] Exact retry projects the original claim; changed payload does not mutate it.
- [ ] `failed`, `cancelled`, and `expired` command states alone never release a claim.
- [ ] Only a referenced durable `NoExternalEffectProof` may select `released_no_external_effect`; ambiguous evidence selects poison.
- [ ] Continuation claim activation, exact-N delivery fence, and explicit effects for already accepted N work are one durable accepted-continuation decision.
- [ ] New N work is rejected with canonical `superseded/replacement_pending`; never-offered work is explicitly superseded, offered/running work is explicitly marked for quiesce/outcome reconciliation, and no hidden hold queue exists.
- [ ] Mutation that releases on any terminal state or clears a poisoned fence fails.

## Ordering constraint

Consumes identity and continuation authority leaves. Crash evidence and promotion define its evidence inputs; operational acceptance consumes it only after all contract leaves are complete.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` (caller-selected for the security-critical claim/poison state machine).
- Review weight: `thorough` (caller-selected); implementation stops at `review` for independent deep review and does not self-approve BLOCKER-3 invariants.
- Dispatch rationale: direct-read only; the resolved feature design, adversarial review, generated-contract patterns, and adjacent identity/authority contracts fully specified the bounded leaf.
- Files changed: `contracts/buf.gen.yaml`; `contracts/proto/patchbay/{common,operations,sessions}.proto`; generated Rust/TypeScript bindings; `core/src/session/{mod,spawn_claim}.rs`; `core/tests/spawn_claim_registry.rs`; sibling core projections updated only to enumerate the new schema-owned durable event kind as an inert family.
- Key decisions:
  - One `SpawnClaimEvent` is the durable replay unit for accepted claim + compound authority + exact-N fence + complete prior-work effects, so fence visibility cannot race effect classification.
  - `SpawnClaimDisposition` is independent of `OperationState`; all command-transition events are sibling no-ops, so terminal command state cannot release or clear a fence.
  - Release requires one of three generated closed `NoExternalEffectProof` variants referencing prior durable evidence; continuation release additionally requires exact prior-N liveness evidence.
  - Ambiguous external-effect evidence selects poison; active/poisoned/promoted/abandoned records retain the exclusive generation key, while only proved-no-effect release makes it reusable.
  - The private claim checkpoint preserves records and accepted prior-work effects under exact authority-domain/LSN anchoring; full replay validates the same gap-free log prefix.
- Tests added: generated round-trip, full transition-table, terminal-state inertness, no-effect proof closure, ambiguity poison/reconciliation, exact-runtime promotion, release+liveness, abandonment, active/poisoned exclusivity, exact retry, hot/cold replay, checkpoint recovery, concurrency, property traces, and explicit mutation witnesses for all four required forbidden mutations.
- Simplification: no hidden hold queue, no command-terminal release hook, no adapter-local claim state, and no alternate generation allocator were introduced; one registry owns claim/fence queries.
- Discrepancies from design: none.
- Adjacent issues parked: none (caller prohibited creating or touching other items).

## Verification evidence

- `cd contracts/ts && npm run build`
- `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models`
- `cargo build --workspace --all-targets`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `rustfmt --edition 2021 --check core/src/session/spawn_claim.rs core/tests/spawn_claim_registry.rs`
