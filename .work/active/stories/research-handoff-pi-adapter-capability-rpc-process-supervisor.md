---
id: research-handoff-pi-adapter-capability-rpc-process-supervisor
kind: story
stage: implementing
tags: [adapter, protocol, security]
parent: research-handoff-pi-adapter-capability
depends_on: [research-handoff-pi-adapter-capability-manifest-profile, research-handoff-spawn-restart-continuation-orchestration, research-handoff-spawn-idempotency-duplicate-handling, deployment-authority-workspace-scoped-revocable-keys]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Per-generation Pi RPC process supervisor

## Checkpoint

Replace the production in-process SDK execution path with one supervised `pi --mode rpc` child per managed runtime generation while retaining the SDK `ModelRuntime` seam for deterministic unit fixtures. The adapter consumes the core-prepared generation claim; it never allocates `current + 1`.

For continuation, the supervisor serializes by logical target, stops delivery to the prior generation, aborts and waits for `agent_settled` when work is active, preserves and verifies the exact persisted JSONL path, terminates the process group, respawns with the explicit session path, reconciles before reporting live, and reports the exact claim/status. It never uses reload as a runtime upgrade and never auto-restarts after crash.

## Design

**Files**
- New `pi-adapter/src/rpc_client.ts` — strict bounded LF-delimited JSONL request/event client; correlated responses and asynchronous notifications remain distinct.
- New `pi-adapter/src/pi_process.ts` — injected process-launch/terminate port using an absolute executable, argv array, sanitized environment, process groups, and bounded TERM→KILL escalation.
- New `pi-adapter/src/session_file.ts` — canonical allowed-root and JSONL-header verifier returning an opaque integrity seal; paths never enter forwarded diagnostics.
- New `pi-adapter/src/spawn_supervisor.ts` — sole fresh/continuation owner and per-logical-target mutex.
- `pi-adapter/src/pi_session.ts`, `session_registry.ts`, `delivery.ts`, `main.ts`, and `core_client.ts` — bind a generation-scoped RPC runtime, consume generated spawn claims/target specs, report exact crash/connectivity evidence, and remove adapter-owned generation increments.
- New `contracts/proto/patchbay/pi_adapter.proto` — generated Pi target spec and typed reconfigure/reload payloads.

Crash evidence maps narrowly: unexpected nonzero/signal process exit → connectivity `failed` and activity `unknown`; RPC loss without conclusive process exit → `stale`/`unknown`; expected clean exit or confirmed normal exit → `offline`/`unknown`. None changes generation. Running work without a proved terminal outcome becomes `failed(execution_outcome_unknown)`; accepted/delivered work follows the core generation-transition policy and is never executed by the fenced child.

## Acceptance evidence

- [ ] Fresh spawn launches one child and reports core-claimed generation `1`; continuation launches only exact `N+1` from the accepted claim.
- [ ] The managed continuation path uses canonical `--session <path>`; `--continue` is allowed only after proving one unambiguous candidate and verifying the resulting exact path/id.
- [ ] Active work is aborted/quiesced with a bounded wait; unresolved effects are reported unknown before forced termination.
- [ ] A changed/missing/truncated/wrong-id/wrong-cwd session file blocks respawn and cannot be logged raw.
- [ ] Old child callbacks, stdout, process handles, and delivery paths are inert after fencing; new live state appears only after reconciliation.
- [ ] Explicit crash, unexplained transport loss, and clean exit produce `failed`, `stale`, and `offline` respectively without generation allocation or automatic restart.
- [ ] SDK `ModelRuntime` remains a test fixture seam, not a second production lifecycle implementation.

## Ordering constraint

Consumes spawn continuation, duplicate/journal, and adapter-owned project/deployment resolution contracts; the manifest declaration must exist before this substrate advertises itself.
