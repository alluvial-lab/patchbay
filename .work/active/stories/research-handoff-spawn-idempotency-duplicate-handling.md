---
id: research-handoff-spawn-idempotency-duplicate-handling
kind: story
stage: implementing
tags: [adapter, protocol, verification]
parent: research-handoff-spawn
depends_on: [spawn-delivery-atomic-claim-idempotency-generation]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Spawn retry, duplicate, and ambiguous-outcome handling

## Checkpoint

Make all duplicate categories explicit without claiming adapter-side exactly-once execution that is not proven. An exact caller retry returns the existing Operation and generation claim. An intentional fresh spawn or continuation uses a new command id/key. Duplicate delivery of one accepted spawn is suppressed by durable result/claim evidence where possible. If the adapter may have created a runtime before losing acknowledgement, Patchbay reports `execution_outcome_unknown` and presents retry risk from the adapter's declared idempotency strength.

## Design

**Files**
- `core/src/acceptance/pipeline.rs` and `core/src/storage/port.rs` — retain per-target boundary dedup and return the persisted spawn claim/current state on duplicate.
- `core/src/acceptance/index.rs` — expose accepted claim plus deferred-success/no-redelivery evidence.
- `server/src/adapter_service.rs` — reconstruct delivery only for a claim that has neither terminal nor qualifying durable external-success evidence.
- `pi-adapter/src/spawn_journal.ts` (new) — adapter-local durable journal keyed by authority domain + spawn command id, recording `received`, `external_identity_known`, `reported` without treating memory as authority.
- `pi-adapter/src/spawn_supervisor.ts` — reconcile a repeated delivery through the journal before any external create/continue call.
- `contracts/proto/patchbay/adapter.proto` and `docs/PROTOCOL.md` — keep `idempotency_strength` honest; do not promote `end_to_end` unless the Pi journal plus external identity lookup proves it.
- `core/tests/acceptance_pipeline.rs`, `server/tests/spawn_completion.rs`, and `pi-adapter/tests/spawn.test.ts` — duplicate, response-loss, store-unavailable, and restart traces.

```ts
export interface SpawnJournal {
  lookup(operationId: string): Promise<SpawnJournalRecord | undefined>;
  claim(operationId: string, claim: SpawnGenerationClaim): Promise<"claimed" | "existing">;
  recordExternalIdentity(operationId: string, runtimeSessionId: string): Promise<void>;
  recordReported(operationId: string): Promise<void>;
}
```

The journal is an adapter-side execution aid, not Patchbay ordering or authority. The core log remains the accepted-Operation source of truth. If journal durability or external lookup cannot prove whether create happened, the adapter must fail with `execution_outcome_unknown`; it must not launch another process automatically.

## Acceptance evidence

- [ ] Same command id/key/target/payload returns the existing command state and exact persisted claim; no second accepted record or generation claim is created.
- [ ] Same key with changed payload rejects; same key on a different target follows the canonical per-target rule; a deliberate duplicate requires new ids.
- [ ] Reoffered delivery after adapter/core restart consults durable success/journal evidence and does not blindly recreate a runtime.
- [ ] Crash-before-external-id and effect-before-response-loss traces surface `execution_outcome_unknown` with retry safety derived from declared capability.
- [ ] Journal unavailable/corrupt fails closed and does not silently execute.
- [ ] `end_to_end` is declared only if a mutation that removes journal/external-identity dedup is killed; otherwise Pi remains honestly `at_patchbay_boundary`.
- [ ] The `duplicate-continuation`, `crash-before-ack`, and `bounded-dedup` executable vectors preserve the boundary-versus-external distinction.

## Ordering constraint

Depends on the atomic generation claim. Restart orchestration consumes this duplicate policy.
