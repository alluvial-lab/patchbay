---
id: authority-descendant-grant-completion-crash-safe-writer
kind: story
stage: done
tags: [security]
parent: authority-descendant-grant-completion
depends_on: [authority-descendant-grant-completion-contract-fold]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Defer spawn terminalization and execute the crash-safe writer

## Checkpoint
Implement the parent feature's Unit 2. Successful spawn results become durable completion evidence (`CompletionDeferred`), and `SpawnCompletionDriver` executes audit → descendant grant → completed transition while holding the shared `CoreDecisionGate` and folding every committed result from storage.

This is a durable acceptance/crash checkpoint inside the parent feature bundle. It does not introduce a spawn-specific storage primitive and does not absorb generic authority-writer correctness.

## Acceptance evidence
- `CommandSnapshot` exposes the generated `OperationKind`; successful non-spawn results retain their existing transition behavior.
- A successful spawn result is appended without a terminal transition or completion audit.
- The driver requires delivered/running state, accepted verified provenance, success evidence, and registration/bump before acting.
- The driver records one durable `CommandCompleted/spawn_completion` audit, feeds its id into `ingest_descendant_grant`, observes that grant, and raw-appends `completed` last.
- The completion audit and grant are not externally readable as a partial completion because the full chain holds the composition-root gate.
- Earlier rejected/failed/expired/cancelled/superseded transitions suppress issuance. A legacy completed transition permits only missing audit/grant repair.
- Fresh-driver crash-prefix tests converge from evidence-only, audit-only, audit+grant, and complete prefixes without duplicate audit, grant, or terminal transition.
- Any corrupt fact, non-durable audit receipt, or write/fold failure returns a typed driver error rather than advancing past the failed spawn.

## Ordering constraint
Depends on `authority-descendant-grant-completion-contract-fold`; it consumes the action fold, required audit linkage, and generated generation-bump correlation. It must finish before production wiring.

## Verification

```bash
cargo test -p patchbay-core --test acceptance_observation
cargo test -p patchbay-core-server --test spawn_completion
cargo test -p patchbay-core --test authority_spawn_tail --test authority_ingest
```

## Implementation notes
- Execution capability: Sol xhigh (explicit autopilot caller selection for security/provenance, lifecycle exposure, and restart repair); one direct owner, no nested agents or peers.
- Review weight: thorough (explicit caller selection; parent feature stops at review for fresh review).
- Files changed: command snapshot/observation ingestion and fixtures; `server/src/spawn_completion.rs`; server exports and adapter result mapping; `server/tests/spawn_completion.rs`; the durable fold's completed-restart source selection.
- Tests added/updated: successful-spawn `CompletionDeferred` versus unchanged non-spawn completion, evidence-only/audit-only/audit+grant/full crash-prefix convergence, diagnostic-only audit rejection, restart idempotence, and a barrier-controlled proof that the shared `CoreDecisionGate` hides an audit+grant intermediate prefix.
- Simplification: one driver reuses `Storage`, `AuditSink`, `AuthorityRegistry`, `SpawnDescendantTail`, and `ingest_descendant_grant`; no spawn-specific storage transaction, callback mesh, or optimistic fold mutation was added.
- Discrepancies from design: none. `SpawnCompletionError` is implemented without adding a server dependency; production `AuditedStorage` supplies the ordinary grant-created audit, while the final raw completion transition intentionally does not emit a second completion audit.
- Adjacent issues parked: none (generic concurrent descendant-writer conflict/no-op behavior remains in the excluded `authority-writer-correctness` scope).
