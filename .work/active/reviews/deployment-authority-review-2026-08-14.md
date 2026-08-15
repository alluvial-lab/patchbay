---
id: deployment-authority-review-2026-08-14
kind: story
stage: done
tags: [review, spawn]
parent: deployment-authority-workspace-scoped-revocable-keys
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-14
updated: 2026-08-14
---

# Thorough review — Unit 8 adapter-local deployment authority

**Verdict: MATERIAL.** The configured resolver correctly binds a non-secret credential handle to adapter/deployment/workspace/project/shape/logical-target identity, evaluates expiry and revocation against the per-call injected time, performs a fresh lookup on every attempt, and keeps built-in denial diagnostics bounded. All four required reviewer mutations were killed. However, the launch-time wrapper bypasses all core-evidence checks whenever the payload omits the reference, the continuation recheck admits core-invalid exact-prior/two-Grant shapes, and the public resolver seam can inject raw paths, labels, handles, or key material through its exception `name`/`code` into adapter diagnostics. These are current authority and redaction gaps, not nits.

## Findings

### MATERIAL — Reference omission is treated as credential-free before either configured policy or core authority is checked

**Location:** `pi-adapter/src/deployment_authority.ts:128-136`; vacuous oracle at `pi-adapter/tests/deployment_authority.test.ts:60-72`

`authorizeDeploymentIfRequired` decodes only `target_spec.shape` and then returns `undefined` solely because the accepted payload's `deployment_authority_ref` is empty. It does so before `validatedSpawnEvidence`, before the adapter-spawn Grant/claim checks, and before any adapter-configured target identity is consulted. The API also has no configured `credential required | credential free` discriminator: `target` is merely optional, while the operator-carried reference itself decides whether the resolver runs.

A clean-tree reviewer probe passed a `SpawnClaimAccepted` with no command id, no `spawn` kind, no adapter target, no claim, and no authorizing Grant; with an empty reference the launch precondition returned `undefined`. The same branch also means omission of a required reference is indistinguishable from a legitimately credential-free target. The focused test blesses only that early return, while its `MISSING_REFERENCE` assertion calls `resolver.authorize` directly and therefore does not exercise the supervisor integration path that launches use.

This violates both acceptance rows: missing references must fail for credential-requiring targets, while credential-free targets must remain valid without using reference omission as an authority bypass. It also creates a path where the adapter launch precondition succeeds for an envelope the core contract rejects.

**Concrete fix:** make credential policy an adapter-configured, discriminated part of the resolved target request rather than inferring it from an untrusted optional reference. Always validate the accepted spawn/claim and required Grant provenance before the credential-free return. For configured credential-required targets, reject an empty reference as `MISSING_REFERENCE`; for an explicitly credential-free target, require no Workspace object and reject an unexpected reference. Add launch-path tests for (1) required target + omitted reference, (2) credential-free fresh spawn with valid core evidence, and (3) credential-free continuation missing either Grant/provenance record.

### MATERIAL — The continuation precondition is weaker than the canonical exact-prior/two-Grant contract

**Location:** `pi-adapter/src/deployment_authority.ts:143-209,234-245`; incomplete oracle at `pi-adapter/tests/deployment_authority.test.ts:144-170`

The resolver checks that both Grant-id fields are non-empty, but never checks that the replacement Grant differs from `accepted_operation.authorizing_grant_id`. It compares optional `runtime_session_id` values for equality but never requires the id to be present and non-empty. Both are canonical core rejection conditions: continuation authority requires two distinct Grants and an exact prior includes the runtime-session identity.

Two clean focused fixture probes survived:

- changing `replacement_grant_id` to the same value as the adapter-spawn Grant left `continuations require both core Grant provenance records and exact claim evidence` green;
- removing `runtime_session_id` from all three prior references left the same test green.

The second case collapses exact prior identity to adapter/deployment/logical-target/generation. The first allows one provenance record to masquerade as both required authority records. Either is a `SpawnClaimAccepted` shape the core rejects but the adapter-local launch precondition accepts.

**Concrete fix:** enforce the complete canonical continuation-carriage invariants at this boundary: distinct non-empty Grant ids, a fully populated exact runtime reference including non-empty runtime-session id, exact request/claim/provenance equality, and the already-checked canonical authority kind/generation transition. Prefer a generated/shared validator or conformance vectors over maintaining a weaker hand copy. Add focused same-Grant and missing/empty runtime-session-id cases; each must fail before resolver lookup/handle return.

### MATERIAL — Arbitrary resolver exception metadata crosses the deployment-authority redaction boundary

**Location:** `pi-adapter/src/main.ts:228-245`; `pi-adapter/src/adapter_diagnostics.ts:109-127,257-260`; incomplete oracle at `pi-adapter/tests/deployment_authority.test.ts:190-233`

`AdapterProcessOptions` accepts any `DeploymentAuthorityResolver`, but `authorizeDeployment` sends every thrown value through the generic `diagnosticError`. That formatter copies arbitrary exception `name` and string/number `code`; the file diagnostic sink then persists those fields. The structural sanitizer does not remove arbitrary paths, labels, credential handles, or raw key strings unless they happen to match a configured secret or one of its assignment regexes.

A clean-tree reviewer probe installed a type-valid custom resolver that threw `Object.assign(new Error(...), { code: "/raw/private/project-label" })`. The actual `AdapterProcess.authorizeDeployment` diagnostic record contained that raw value unchanged. The committed test covers only the built-in resolver's closed `DeploymentAuthorityError` and therefore cannot enforce the redaction guarantee at the public resolver boundary.

**Concrete fix:** normalize errors specifically at the deployment-authority catch boundary. Preserve a code only when the value is a `DeploymentAuthorityError` whose code belongs to `DEPLOYMENT_AUTHORITY_ERROR_CODES`; map all other resolver failures to one closed, value-free code/name. Do not route arbitrary resolver metadata through `diagnosticError`. Add an integration test with a hostile custom resolver whose name, message, code, cause, handle, path, label, reference, and key material all contain sentinels; assert the actual file diagnostic record and any forwarded/core diagnostic surface contain none of them.

## Checklist disposition

| Review requirement | Result |
|---|---|
| Exact adapter/deployment/workspace/project/shape/logical-target scope | **PASS** for `ConfiguredDeploymentAuthorityResolver`; all configured dimensions are exact and raw payload fields cannot override them. |
| Credential handle only | **PASS** for the built-in binding/result shape; no raw key field was added. |
| Credential-free target | **FAIL / MATERIAL** — payload omission, not configured policy, controls the bypass and skips core evidence. |
| Revocation/expiry with injected time | **PASS** — both are checked on every call against the supplied `now`; no constructor-time wall clock is captured. |
| Per-continuation recheck/no cache | **PASS** — fresh map lookup per call; both required mutations were killed. |
| Adapter-local authority cannot replace core Grants | **FAIL / MATERIAL** — the credential-free branch checks neither Grant, and the credential-bearing branch admits reused Grant ids/incomplete exact prior identity. |
| Redaction on actual diagnostic path | **FAIL / MATERIAL** — built-in error is safe, but a valid resolver implementation can inject arbitrary metadata into the adapter diagnostic record. |
| Supervisor seam / Pi separation | **PASS with the above material findings** — `AdapterProcess.authorizeDeployment` is a launch precondition and adds no Pi subprocess/session-path behavior. |

## Mutation matrix

All source mutations were temporary, restored with `git restore`, and the repository was clean before the full suite and before this review file was written.

| Mutation / probe | Result | Focused oracle / observation |
|---|---|---|
| Remove the revocation check | **KILLED** | `each continuation attempt rechecks current revocation state instead of caching success` failed with “Missing expected rejection”. |
| Cache the first successful authorization by reference before later lookups | **KILLED** | The same continuation recheck test failed after revocation. |
| Remove workspace/project comparisons from the scope match | **KILLED** | `paths and labels in opaque adapter payload cannot widen project or workspace scope` failed with “Missing expected rejection”. |
| Add the request project id to the supervisor denial diagnostic | **KILLED** | `supervisor integration records only bounded denial metadata on every redaction surface` found the forbidden label. |
| Pass a malformed, Grant-free accepted envelope with an empty reference through the clean launch wrapper | **SURVIVED / GAP** | `authorizeDeploymentIfRequired` returned `undefined`. |
| Reuse the adapter-spawn Grant id as `replacement_grant_id` in the continuation fixture | **SURVIVED / GAP** | The focused continuation authority test still passed. |
| Remove `runtime_session_id` from all exact-prior fixture copies | **SURVIVED / GAP** | The focused continuation authority test still passed. |
| Throw a raw path in a custom resolver's exception code | **SURVIVED / LEAK** | Actual AdapterProcess diagnostic input contained `code: "/raw/private/project-label"`. |

## Full clean verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, and 38 killed mutation witnesses.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 23/23 tests.
4. `cd pi-adapter && npm test` — **PASS**, 35/35 tests.

## Final recommendation

**Return `deployment-authority-workspace-scoped-revocable-keys` to `implementing`.** Make credential-free status adapter-configured while retaining core-evidence validation, close the canonical continuation-carriage gaps, and normalize hostile resolver failures at the redaction boundary. Re-run the thorough review after the new launch-path and hostile-resolver tests are green.
