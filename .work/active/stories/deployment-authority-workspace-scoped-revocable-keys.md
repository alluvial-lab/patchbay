---
id: deployment-authority-workspace-scoped-revocable-keys
kind: story
stage: review
tags: [security, adapter, architecture]
parent: research-handoff-spawn
depends_on: [fleet-spawn-target-resolution]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-14
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

- [x] Missing/expired/revoked/unknown/scope-mismatched references fail before external launch.
- [x] Valid local reference cannot authorize another adapter/project/shape/logical target.
- [x] Continuation rechecks instead of inheriting the prior credential.
- [x] Both core Grants are already proven by accepted compound provenance; local authority cannot replace/widen either.
- [x] Raw credential/handle material is absent from log/audit/diagnostics/snapshot/CLI/web scans.
- [x] Target specs needing no external credential remain valid without manufacturing a Workspace entity.

## Ordering constraint

Consumes the complete generated target/continuation contract and operation-aware core authority resolution. Restart orchestration consumes it after completion semantics are ready.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`; selected by the caller for this security-sensitive generated-contract and adapter-boundary implementation.
- Review weight: `thorough` from the caller; implementation stops at `stage: review` for the independent review loop.
- Files changed: `pi-adapter/src/deployment_authority.ts` adds the consumer-owned resolver, exact configured binding, bounded safe errors, mutable revocation, expiry, fresh/continuation evidence validation, and the credential-free bypass; `pi-adapter/src/main.ts` supplies the downstream supervisor/composition-root call; `pi-adapter/src/adapter_diagnostics.ts` registers its bounded denial event; `pi-adapter/tests/deployment_authority.test.ts` supplies boundary and mutation-sensitive coverage; `docs/SECURITY.md` extends the canonical redaction list.
- Mechanism: the resolver decodes the landed generated `SpawnRequest` from `SpawnClaimAccepted`, requires the accepted adapter-scoped spawn Grant id, exact claim/command/domain/logical-target evidence, and—for continuation—the exact requested prior plus the core-selected `session-management` replacement-Grant provenance. It then performs a fresh lookup on every call and binds the opaque reference to exact adapter, deployment, workspace, project, shape, and logical-target ids. This lookup is an adapter-local launch precondition only; it neither evaluates nor substitutes for either core Grant.
- Tests added: six focused tests cover valid/credential-free targets; missing, unknown, expired, revoked, and scope-mismatched references; adapter/deployment/workspace/project/shape/logical-target isolation; hostile raw path/label payloads; both core Grant provenance records and exact continuation claim evidence; per-attempt revocation recheck; and bounded diagnostic redaction with no credential handle, raw key material, path, label, or reference leakage.
- Mutation kills (all reverted with `git restore`, restored focused tests green): removing the revocation check failed `each continuation attempt rechecks current revocation state instead of caching success`; caching the first successful authorization across attempts failed the same test after revocation; omitting workspace/project comparisons failed `paths and labels in opaque adapter payload cannot widen project or workspace scope`; recording the project/label in the denial diagnostic failed `supervisor integration records only bounded denial metadata on every redaction surface`.
- Simplification: no new credential store, Workspace protocol entity, raw-key field, target-payload parser, or local Grant model was introduced. Credential-free specs bypass the resolver before requiring adapter-local workspace/project identity, and diagnostics carry only a closed error name/code.
- Discrepancies from design: the concrete RPC spawn supervisor is a downstream Pi story and does not exist yet, so this unit integrates through `AdapterProcessOptions.deploymentAuthorityResolver` plus `AdapterProcess.authorizeDeployment`; the downstream supervisor can call that precondition immediately before launch without Pi-specific behavior landing here. No proto/generated artifact changed and `npm run gen` was not run manually.
- Adjacent issues parked: none.
- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, and 38 killed mutation witnesses.
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS**, 23/23 tests.
- Verification group 4 — `cd pi-adapter && npm test`: **PASS**, 35/35 tests including the real core/adapter restart E2E and six new deployment-authority tests.

### Fix round — configured credential policy, canonical continuation provenance, and resolver redaction

- Review disposition: all three MATERIAL findings in `.work/active/reviews/deployment-authority-review-2026-08-14.md` were accepted. Direct reading was sufficient because the review named the complete write set and exact probes; no sub-agent or design change was required.
- Credential-policy fix: `DeploymentAuthorityRequest.target` now carries an adapter-configured `credential-required | credential-free` discriminator. Both branches validate the configured target, accepted spawn envelope, adapter-scoped spawn Grant, claim, and continuation provenance before policy dispatch. A required target with an omitted reference fails `MISSING_REFERENCE`; a credential-free target has no workspace/project fields, rejects an unexpected reference, and skips only credential-handle lookup.
- Continuation fix: the adapter launch boundary now requires two distinct non-empty Grant ids and a complete exact prior runtime reference, including a non-empty runtime-session id, positive generation, and exact request/claim/provenance equality. Reused Grant ids and missing/empty runtime-session identity fail `INVALID_CORE_EVIDENCE` before any resolver lookup or handle return.
- Redaction fix: `AdapterProcess.authorizeDeployment` no longer sends resolver failures through generic `diagnosticError`. It preserves only a member of the runtime-frozen error-code registry from a real `DeploymentAuthorityError`; every other thrown value becomes the closed `{ name: "DeploymentAuthorityResolverError", code: "RESOLVER_FAILURE" }` diagnostic with no exception metadata.
- Tests added/strengthened: launch-path policy cases cover required-reference omission, credential-free fresh/continuation success with valid core evidence, unexpected credential-free references, and missing either Grant/provenance record; continuation cases cover same-Grant and missing/empty runtime-session identity before resolver lookup; the hostile-resolver integration drives name/message/code/cause/handle/path/label/reference/key sentinels through the captured diagnostic input, real JSONL file sink, and core-forwarder surface and finds none.
- Mutation kills, all reverted with `git restore` and restored focused tests green: moving the credential-free return before validation failed `credential-free launch still validates fresh and continuation Grant/claim provenance` with “Missing expected rejection”; removing the distinct-Grant comparison failed `continuation rejects reused Grants and missing runtime identity before resolver lookup`; removing the non-empty runtime-session check failed that same focused oracle; replacing the deployment-authority normalizer with generic `diagnosticError` failed `hostile resolver exception metadata is closed before every diagnostics surface` on a leaked hostile error name in the file diagnostic.
- Files changed: `pi-adapter/src/deployment_authority.ts`, `pi-adapter/src/main.ts`, `pi-adapter/tests/deployment_authority.test.ts`, and this story. No proto, generated contract, core, server, or consumer-supervisor wiring changed.
- Simplification/discrepancies: the former payload-reference-driven early-return helper was removed; one shared deployment-request validator now owns configured policy plus core-evidence validation for both the wrapper and built-in resolver. The downstream Pi supervisor remains the owner of generated `Delivery.accepted_spawn` consumption, as scoped. Adjacent issues parked: none.
- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, and 38 killed mutation witnesses.
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS**, 23/23 tests.
- Verification group 4 — `cd pi-adapter && npm test`: **PASS**, 38/38 tests including the real core/adapter restart E2E and nine deployment-authority tests.
