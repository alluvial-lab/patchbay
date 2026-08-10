---
id: authority-descendant-grant-completion-crash-safe-writer
kind: story
stage: implementing
tags: [security]
parent: authority-descendant-grant-completion
depends_on: [authority-descendant-grant-completion-contract-fold]
release_binding: null
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
