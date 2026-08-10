---
id: authority-descendant-grant-completion-live-composition
kind: story
stage: implementing
tags: [security]
parent: authority-descendant-grant-completion
depends_on: [authority-descendant-grant-completion-crash-safe-writer]
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Wire startup repair and continuous descendant completion

## Checkpoint
Implement the parent feature's Unit 3. The production composition root bootstraps `SpawnCompletionDriver` to quiescence before service projections/listeners, then runs its continuous durable-tail loop as a fail-fast peer of the network and admin servers. Complete the real adapter/server/restart evidence.

This closes the production path; no service-local completion hook or SQLite-specific shortcut is allowed.

## Acceptance evidence
- Main creates one shared `CoreDecisionGate`, bootstraps the completion driver before constructing services/binding listeners, and gives every service/driver the same gate.
- Control and adapter service projections rebuild from the repaired prefix.
- The continuous driver catches committed facts without a later RPC trigger, sleeps only while quiescent and outside the gate, and exits the process serving set on unexpected error.
- Adapter observation ingress maps `CompletionDeferred` to its durable Observation event id without directly invoking completion.
- Real `AuditedStorage<RusqliteStorage>` tests prove registration and generation-bump completion, exact audit-id linkage, restart idempotence, verified actor/endpoint provenance, subsequent descendant authorization, and independent parent/child revocation.
- A barrier-controlled reader sees only the pre-decision or final prefix, never audit/grant-only state.
- No descendant grant, completion audit, or terminal transition is duplicated after restart.
- Existing model/vector checks remain green while `SpawnCreatesDescendantGrant` remains honestly stated-normative.

## Ordering constraint
Depends on `authority-descendant-grant-completion-crash-safe-writer`. This is the last child checkpoint; green verification advances it directly to done and makes the parent eligible for thorough integrated review.

## Verification

```bash
cargo test -p patchbay-core-server --test spawn_completion --test grpc_smoke
cargo test --workspace
npm --prefix contracts/ts run build
npm --prefix contracts/ts run check:models
npm --prefix contracts/ts run check:vectors
cargo fmt --all -- --check
```
