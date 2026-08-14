---
id: research-handoff-spawn-runtime-evidence-promotion-contract
kind: story
stage: review
tags: [protocol, security, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-logical-target-identity-contract, research-handoff-spawn-continuation-payload-authority-contract, research-handoff-spawn-claim-registry-contract, research-handoff-spawn-crash-external-effect-evidence-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-13
---

# Runtime evidence quarantine and atomic promotion contract

## Checkpoint

Define the durable replay envelopes that make claimed-successor staging, stale-evidence quarantine, and authority-bearing promotion structural. This is the last contract leaf; no spawn operation begins until it and the cursor leaf are complete.

`ClaimedSuccessor` validates exact Operation provenance + expected prior + claimed generation. It routes only a SessionReport to staged evidence; it does not make the successor current. Stale/unknown/mismatched candidates persist, when useful, only inside `QuarantinedRuntimeEvidence`, never as raw authoritative Observations.

`SpawnPromotionCommitted` is one semantic event consumed by session, claim, authority, and command projections. It contains or references complete success evidence and the descendant Grant. Its source and audit commit atomically; replay cannot observe a candidate-live/grantless prefix.

## Design

**Files**
- `contracts/proto/patchbay/{common,observations,sessions,authority}.proto` — classifier and self-contained event envelopes; generated Rust/TypeScript artifacts follow from the schema.
- `contracts/proto/patchbay/common.proto` stored-event registry — dedicated staged/quarantined/promotion kinds.
- `core/src/storage/port.rs` plus backend contract tests — atomic audited promotion append that stamps exact event/audit linkage in one transaction.
- Replay-dispatch and generated contract tests.

```rust
pub enum RuntimeGenerationDisposition {
    Current,
    ClaimedSuccessor {
        claim_operation_id: CommandId,
        expected_prior: Option<RuntimeGenerationRef>,
        claimed_generation: Generation,
    },
    Tombstoned { superseded_at_lsn: u64 },
    Unknown,
    IdentityMismatch,
}
```

The quarantine envelope uses a generated `oneof` for each admitted candidate family (Observation, SessionReport, delivery acknowledgement, transcript/status evidence, Elicitation mutation), plus authenticated source binding, candidate target, classifier context, and canonical reason. It is not an arbitrary payload/schema escape hatch. Normal projections switch on the outer stored kind and cannot unwrap quarantine as normal authoritative ingress.

Promotion readiness requires accepted compound provenance, delivered/running lifecycle, successful Result, staged successor, exact external-runtime uniqueness reservation, exact generation transition, live-at-promotion exact-prior replacement Grant for continuation, completion audit linkage, and descendant Grant. Promotion installs authority before session current/live and command completed within the event fold order.

## Acceptance evidence

- [ ] Legitimate first fresh/N+1 reports classify `ClaimedSuccessor` only on exact durable claim provenance.
- [ ] Wrong Operation, expected prior, adapter/deployment, logical id, runtime id, or generation never stages.
- [ ] Every quarantined candidate is carried by an admitted generated `oneof`; unknown/untyped candidates reject, and nested Observation/report/transcript/ack/Elicitation evidence cannot mutate any normal projection on hot fold or replay.
- [ ] Promotion source is a self-contained replay unit carrying the complete descendant Grant; its distinct audit record commits in the same storage transaction, and no crash prefix publishes N+1 without authority.
- [ ] Every consuming projection validates the same promotion event and exact pre-state; disagreement fails closed.
- [ ] Revoked/expired exact-prior replacement authority before promotion suppresses promotion.
- [ ] Mutations dispatching quarantine as raw evidence or exposing staged successor as current fail.

## Ordering constraint

Final invariant contract leaf. Target resolution waits on this leaf and the parallel cursor replacement leaf, ensuring all operations consume settled shared contracts.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` — caller-selected strongest worker for the atomic promotion, authority fence, and replay-quarantine security boundary.
- Review weight: `thorough` (caller); implementation stops at `review` for independent convergence review and does not self-approve the promotion/quarantine/fence invariants.
- Files changed: generated contract sources/artifacts in `contracts/proto/patchbay/{common,observations,sessions,authority}.proto`, `contracts/{rust,ts}/src/gen/**`; runtime fence and ordered projection folds in `core/src/{session,authority,acceptance}/`; dedicated storage transaction in `core/src/storage/`; replay dispatch allowlists; focused contract tests.
- Tests added: `core/tests/runtime_evidence_promotion.rs` covers exact `ClaimedSuccessor`, wrong-operation rejection, staging-not-current, direct claim consumption, nested quarantine inertness, mandatory atomic quarantine audit, promotion id/audit stamping, and generic-promotion append rejection. The prior Leaf-6 guard test now explicitly rejects only the forbidden legacy two-event promotion shape.
- Atomic promotion mechanism: `Storage::append_spawn_promotion_audited` is fail-closed by default and implemented by the SQLite single-writer actor as one transaction. It predicts the two consecutive LSNs, stamps `promotion_event_id`, `completion_audit_event_id`, and the nested descendant Grant's `audit_id` before encoding, reserves the descendant Grant identity, inserts the one promotion source, inserts its source-linked audit, verifies both assigned ids, and commits once. Generic/unaudited promotion appends reject.
- Promotion un-guard wiring: `SpawnClaimRegistry` consumes `STORED_EVENT_KIND_SPAWN_PROMOTION_COMMITTED` directly, validates every embedded fact against the referenced durable prefix and exact active/poisoned claim, then sets `promoted` and clears the fence. The old disposition-event path cannot promote. `fold_spawn_promotion_ordered` stages clones and applies authority → session → claim → command before publishing any projection.
- Quarantine rationale: the generated admitted-family `oneof` has no bytes/Any/opaque arm; normal command, Elicitation, diagnostics, authority, session, and completion dispatch recognizes only the outer quarantine kind and never recursively applies its candidate. Un-audited quarantine append rejects.
- Simplification: reused the existing SQLite writer transaction and audit index instead of introducing a batch-prefix protocol; one semantic promotion source remains the only authoritative replay unit.
- Discrepancies from design: none. The implementation keeps the operation driver and target-selection work downstream; this leaf supplies envelopes, validation/folds, and storage atomicity only.
- Adjacent issues parked: none.

## Verification evidence

- `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`
- `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
- `cd operator-domain && npm run build && npm test`
- `cd pi-adapter && npm test`

All commands passed on 2026-08-13.
