---
id: deployment-authority-workspace-scoped-revocable-keys
kind: story
stage: implementing
tags: [security, adapter, architecture]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-restart-continuation-orchestration]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Adapter-owned deployment-authority references for spawn

## Ownership decision

Keep this checkpoint under the spawn feature, but narrow it to the adapter boundary. The v1 project/cwd decision rules out a core `Workspace` or universal project-key authority layer. Canonical Patchbay grants remain the only core Operation authority. This story adds a fail-closed **adapter-local deployment-authority reference** for target specs whose external workspace/runtime needs an expiring or revocable launch credential.

Mission Control's useful lesson is strict workspace denial; the lesson does not justify importing its workspace ontology or role derivation. The core carries a bounded opaque reference for audit and delivery but never credential bytes and never interprets cwd/project labels as authority.

## Design

**Files**
- `contracts/proto/patchbay/operations.proto` — optional bounded `deployment_authority_ref` on `SpawnTargetSpec`; it is an identifier, not a bearer secret or Grant.
- `pi-adapter/src/deployment_authority.ts` — adapter-owned resolver/check interface and fail-closed configured implementation.
- `pi-adapter/src/spawn_supervisor.ts` — resolve the reference immediately before external create/continue and bind it to the adapter-local target-spec identity.
- `pi-adapter/src/main.ts` — load references from protected local configuration; never from browser-supplied raw secret material.
- `docs/SECURITY.md` — add the reference/credential bytes to the canonical boundary and redaction statement without introducing a second grant system.
- `pi-adapter/tests/spawn.test.ts` and `server/tests/trust_boundary.rs` — expiration/revocation/scope mismatch and redaction evidence.

```ts
export interface DeploymentAuthorityRequest {
  readonly reference: string;
  readonly targetSpecShape: string;
  readonly projectRef?: string;
  readonly logicalTargetId: string;
}

export interface DeploymentAuthorityResolver {
  authorize(request: DeploymentAuthorityRequest, now: Date): Promise<{
    readonly credentialHandle: string;
  }>;
}
```

The returned handle is consumed only inside the adapter/supervisor. Neither the handle's secret value nor resolved credential material enters an Operation payload, Observation, diagnostic, audit record, or snapshot. A continuation re-evaluates expiry/revocation and scope; credential presence from the prior generation is not authority continuity.

## Acceptance evidence

- [ ] Missing, expired, revoked, unknown, project-mismatched, or shape-mismatched references fail at adapter delivery with a canonical refusal and no process creation.
- [ ] A valid reference cannot authorize another adapter-local project/target-spec scope.
- [ ] Restart-as-continuation re-checks the reference instead of inheriting the prior process credential.
- [ ] Core grant authorization still runs before acceptance; adapter deployment authority cannot widen a Grant.
- [ ] Raw credentials/handles are absent from durable log, audit/diagnostics, snapshots, and CLI/web output under byte/encoded-secret scans.
- [ ] Omitting `deployment_authority_ref` remains valid for target specs that need no separate external credential; this story does not manufacture a workspace requirement.

## Ordering constraint

Runs after restart orchestration has an explicit logical target and target-spec continuation path. It must not shape core target identity.
