---
source_handle: mission-control-src
fetched: 2026-08-08
source_url: https://github.com/builderz-labs/mission-control/tree/17186288ef28341723999a040b3b7baa55427a2c
provenance: source-direct
---

## Summary

Mission Control is a self-hosted, SQLite-backed control plane over multiple agent runtimes. Its durable model centers on tasks, runs, spawn history, provisioning jobs, audit records, and workspace-scoped governance. Dispatch has atomic task claiming, bounded retries, stale-task requeue, and a deferred-completion reconciler when a runtime returns a usable run identifier. The repository also contains narrow idempotency mechanisms and downstream gateway idempotency keys, but its spawn HTTP route does not expose a caller-supplied operation identity or a durable accepted-operation lifecycle. Runtime depth is represented by an explicit per-adapter capability manifest.

## Structural metadata

- Repository: `builderz-labs/mission-control`
- Commit: `17186288ef28341723999a040b3b7baa55427a2c`
- Commit subject: `feat: declared runtime capability manifests (#900, manifests slice) (#911)`
- License presented by repository: MIT
- Fetched as a local Git clone and inspected at the pinned commit.

## Key passages

1. The README calls Mission Control a self-hosted control plane for dispatching tasks, inspecting runs, reviewing failures, tracking spend, and coordinating runtimes. It says SQLite stores local control-plane state and lists shipped governance surfaces including roles, API keys, approvals, audits, and evals. (`README.md`, “What Mission Control governs”)

2. The orchestration guide defines the durable task lifecycle `inbox → assigned → in_progress → review → done`, with rejection/retry and failure/cancellation branches. Auto-dispatch atomically claims assigned work, failed dispatches increment `dispatch_attempts` and return to `assigned`, and stale `in_progress` tasks assigned to offline agents are requeued up to a bounded failure limit. (`docs/orchestration.md`, “Task Lifecycle,” “Retry Handling,” and “Stale Task Recovery”; `src/lib/task-dispatch.ts`, `requeueStaleTasks` and the atomic `UPDATE ... status = 'assigned'` claim)

3. Deferred task dispatch records `dispatch_session_id`, optional `dispatch_run_id`, and `async_state: pending`. A dispatch accepted without a run id is marked `async_reconciliation: manual_required`; comments state that automatic completion reconciliation cannot safely wait without that identifier. A separate reconciler advances matching `in_progress` tasks when a run can be checked. (`src/lib/task-dispatch.ts`, `reconcileDeferredTaskCompletions` and accepted-without-run-id branches)

4. `POST /api/spawn` generates a new random `spawnId` inside each request. It first calls `sessions_spawn`; only its modern `agent` compatibility fallback receives `idempotencyKey: spawnId`. The response reports success/failure immediately. The same route's `GET` still says a real implementation would store spawn history in a database and currently derives history from logs. (`src/app/api/spawn/route.ts`, `POST` and `GET`)

5. A separate `spawn-history` module and migration define durable SQLite `spawn_history` rows with session, trigger, status, exit, error, duration, workspace, and timestamps. Starting a spawn also creates a durable run; finishing updates both records. The `runs` table records status/outcome, lineage/provenance, steps, cost, task relation, and workspace. (`src/lib/spawn-history.ts`; `src/lib/migrations.ts`, migrations `044_spawn_history` and `046_agent_runs`; `src/lib/runs.ts`)

6. Agent registration is an idempotent upsert by `(name, workspace_id)`. Provisioning jobs carry an `idempotency_key`, but bootstrap and decommission creation generate a fresh UUID internally for each new job rather than accepting a retry key from the caller. A platform PRD explicitly lists “idempotent commands” and spawn-history durability among incomplete or uneven operational areas. (`src/app/api/agents/register/route.ts`; `src/lib/super-admin.ts`, bootstrap/decommission job creation; `docs/plans/2026-03-20-mission-control-platform-cli-tui-prd.md`, “Problem statement”)

7. Agent API keys are stored with `agent_id`, `workspace_id`, JSON scopes, optional expiry, and revocation time. Authentication rejects revoked/expired keys, binds the key to an agent within its workspace, and derives viewer/operator/admin role from scopes. The global API key remains an admin path and logs a suggestion to prefer agent-scoped keys. (`src/lib/auth.ts`, global and agent-scoped API-key paths)

8. Strict-workspace enforcement fails closed for ambiguous or unowned resources. The spawn route requires operator role and invokes the strict-workspace denial path for runtime tasks before delivery. (`CHANGELOG.md`, 2.2.0 security notes; `src/lib/workspace-isolation.ts`; `src/app/api/spawn/route.ts`)

9. Runtime capability depth is a declared, complete boolean manifest, explicitly separated from host detection. Each `true` is expected to name a shipping path, and uncertain capabilities are declared false. Fields include dispatch, session resume, PTY, working-directory control, tool policy, budget cap, structured output, skills inventory, and receipt categories. (`src/lib/agent-runtimes.ts`, `RuntimeCapabilities` and `RUNTIME_CAPABILITIES`; `src/app/api/agent-runtimes/route.ts`; `src/lib/__tests__/runtime-capabilities.test.ts`)

10. A repository-wide search at the pinned commit found no runtime-operation incarnation/generation field or stale-generation fence in the task, run, spawn, or adapter contracts. Occurrences of “generation” were unrelated UI/content generation or dependency names. This is a coverage statement about the fetched tree, not proof about uninspected private deployments.
