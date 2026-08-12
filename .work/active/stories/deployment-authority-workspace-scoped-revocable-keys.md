---
id: deployment-authority-workspace-scoped-revocable-keys
kind: story
stage: implementing
tags: [security, adapter, architecture]
parent: research-handoff-spawn
depends_on: [fleet-spawn-target-resolution]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Adapter-owned deployment-authority references for spawn

## Redesign disposition

Rewritten to consume, not compete with, compound core authority. This checkpoint remains adapter-local and may not satisfy either the adapter-scoped spawn Grant or the exact-prior session-management Grant.

## Checkpoint

Provide an optional bounded `deployment_authority_ref` for adapter target specs whose external runtime requires a local expiring/revocable launch credential. The core carries the opaque reference for delivery/audit shape only; credential bytes and handles never enter core logs, Observations, diagnostics, snapshots, or surfaces.

A continuation re-resolves and rechecks the reference against its target spec/logical target immediately before external launch. Prior-process credential presence is not continuation authority.

## Design

**Files**
- Generated `SpawnTargetSpec` contract from the continuation payload leaf.
- New `pi-adapter/src/deployment_authority.ts` — downstream adapter implementation.
- Downstream supervisor/configuration integration and redaction tests.
- `docs/SECURITY.md` canonical redaction boundary update during implementation.

```ts
export interface DeploymentAuthorityResolver {
  authorize(request: DeploymentAuthorityRequest, now: Date): Promise<{
    readonly credentialHandle: string;
  }>;
}
```

Project/cwd remains adapter-owned. The resolver binds a reference to configured adapter-local target-spec identity; raw paths/labels do not widen scope.

## Acceptance evidence

- [ ] Missing/expired/revoked/unknown/scope-mismatched references fail before external launch.
- [ ] Valid local reference cannot authorize another adapter/project/shape/logical target.
- [ ] Continuation rechecks instead of inheriting the prior credential.
- [ ] Both core Grants are already proven by accepted compound provenance; local authority cannot replace/widen either.
- [ ] Raw credential/handle material is absent from log/audit/diagnostics/snapshot/CLI/web scans.
- [ ] Target specs needing no external credential remain valid without manufacturing a Workspace entity.

## Ordering constraint

Consumes the complete generated target/continuation contract and operation-aware core authority resolution. Restart orchestration consumes it after completion semantics are ready.
