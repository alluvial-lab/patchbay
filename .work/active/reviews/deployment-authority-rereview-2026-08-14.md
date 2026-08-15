---
id: deployment-authority-rereview-2026-08-14
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

# Thorough re-review — Unit 8 adapter-local deployment authority

**Verdict: CLEAN.** Pass 2 found no material findings or nits. All three pass-1 findings are closed, all eight requested/new and regression mutations were killed, and the full clean-tree verification matrix passed.

## Pass-1 finding closure

| Finding | Result | Evidence |
|---|---|---|
| Credential-free early return bypassed configured policy and core provenance | **CLOSED** | `DeploymentAuthorityRequest.target` now has the required `credential-required | credential-free` discriminator. `authorizeDeploymentIfRequired` calls the shared request validator before policy dispatch, so both branches validate configured adapter/deployment/logical-target identity and accepted Grant/claim provenance. Only credential-handle lookup is skipped. Credential-free malformed provenance rejects; credential-required omission rejects as `MISSING_REFERENCE` before resolver lookup. |
| Continuation admitted reused Grant ids and absent runtime-session identity | **CLOSED** | Continuation validation requires distinct non-empty spawn/replacement Grant ids, non-empty runtime-session identity, positive prior generation, canonical `session-management` replacement kind, and exact prior equality across payload request, claim, and durable provenance. Invalid evidence rejects before resolver lookup. |
| Resolver exception metadata leaked through diagnostics | **CLOSED** | The deployment-authority catch boundary uses a closed normalizer: only a real `DeploymentAuthorityError` with a code in the runtime-frozen registry preserves a hard-coded name/code; every other thrown value maps to `{ name: "DeploymentAuthorityResolverError", code: "RESOLVER_FAILURE" }`. Hostile metadata is absent from captured diagnostic input and the actual JSONL file. |

## Findings

None.

### Core-forwarder caveat disposition

`deployment.authority.denied` is intentionally absent from `PI_FORWARDED_DIAGNOSTIC_CODES`, so the production `CoreDiagnosticsForwarder` rejects/drops this event before report construction. The empty forwarded collection in the hostile-error test is therefore the real current production behavior, not an unexercised leak path. The two surfaces that do receive the event—the local diagnostic input and JSONL sink—are exercised with hostile metadata and kill the generic-normalizer mutation. The forwarder itself maps only registry-owned codes into a generated structural payload and has no arbitrary error-metadata field, so this is not a current gap.

## Mutation matrix

All mutations were applied on the main tree, run only against the named focused test, and reverted with `git restore`. The clean full suite subsequently rebuilt and retested the restored sources.

| Mutation | Result | Focused oracle |
|---|---|---|
| Reintroduce the reference-empty credential-free return before shared validation | **KILLED** | `credential-free launch still validates fresh and continuation Grant/claim provenance` failed with `Missing expected rejection`. |
| Remove the distinct-Grant comparison | **KILLED** | `continuation rejects reused Grants and missing runtime identity before resolver lookup` failed with `Missing expected rejection`. |
| Remove non-empty runtime-session validation | **KILLED** | The same continuation test failed with `Missing expected rejection`. |
| Restore generic `diagnosticError` for deployment-authority failures | **KILLED** | `hostile resolver exception metadata is closed before every diagnostics surface` detected `resolver-secret-name` in the real JSONL file. |
| Remove the revocation check | **KILLED** | `each continuation attempt rechecks current revocation state instead of caching success` failed with `Missing expected rejection`. |
| Cache the first successful authorization by reference | **KILLED** | The same continuation recheck test failed after revocation. |
| Remove workspace/project binding comparisons | **KILLED** | `paths and labels in opaque adapter payload cannot widen project or workspace scope` failed with `Missing expected rejection`. |
| Add the configured project id to the denial diagnostic | **KILLED** | `supervisor integration records only bounded denial metadata on every redaction surface` detected the forbidden label. |

## Full clean verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, and 38 killed mutation witnesses.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 23/23 tests.
4. `cd pi-adapter && npm test` — **PASS**, 38/38 tests.

## Recommendation

**Advance `deployment-authority-workspace-scoped-revocable-keys` to `done`.** The configured policy, continuation provenance, revocation/scope, and redaction boundaries now have non-vacuous regression oracles. The downstream Pi supervisor policy wiring remains the intentional separate-feature seam and is not a finding.
