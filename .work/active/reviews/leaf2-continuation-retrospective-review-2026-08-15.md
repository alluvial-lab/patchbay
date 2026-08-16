---
id: leaf2-continuation-retrospective-review-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-continuation-payload-authority-contract
created: 2026-08-15
updated: 2026-08-15
---

# Retrospective deep review — Leaf 2 continuation payload and authority provenance

## Verdict

**APPROVE — CLEAN.**

The landed Leaf 2 contract is complete at its intended structural boundary and all four requested adversarial mutants are killed by focused tests. Fresh and continuation are disjoint variants of one generated `SpawnRequest`; continuation requires an exact prior logical/runtime generation; accepted and descendant records preserve the adapter-spawn Grant id plus the distinct exact-prior replacement Grant id and canonical `session-management` authority kind; fresh carriage rejects the continuation half.

The leaf correctly performs no Grant selection or target resolution. Those decisions remain outside `core/src/acceptance/spawn.rs` and `core/src/contract_validation.rs`; downstream authority/target tests corroborate integration but were not treated as proof of the leaf's own validator. Review target: landed implementation `46f896c49be508474fa54d5b54dc3dd2e01c0403`, review repair `a184e4458222118ad72ceda57339dd60114523ed`, and completion transition `dba571e0b0cc297c450f0de93be5baec686e8903`, inspected against the current landed tree.

## Findings

### Blockers

None.

### Material

None.

### Nits

None.

## Completeness disposition

| Requirement | Verdict | Direct evidence |
|---|---|---|
| One generated fresh/continuation payload | **PASS** | `contracts/proto/patchbay/operations.proto` defines one `SpawnRequest.intent` oneof; committed Rust exposes `spawn_request::Intent::{Fresh, Continuation}` and TypeScript exposes the corresponding discriminated union. `generated_spawn_intents_round_trip_as_disjoint_variants` and both-order raw-wire rejection exercise the shape. |
| Continuation names an exact prior | **PASS** | `SpawnContinuation.prior` is a generated `RuntimeGenerationRef`; `validate_spawn_request` rejects omission, empty logical/runtime identity, zero generation, and overflow. `exact_prior_mutations_reject_before_acceptance` exercises each rejection independently. |
| Durable two-Grant provenance | **PASS** | The adapter-spawn id remains `AcceptedOperation.authorizing_grant_id`; `ContinuationAuthorityProvenance` carries `exact_prior`, `replacement_grant_id`, and `replacement_authority_kind`; current `SpawnClaimAccepted.compound_authority` durably composes it with the accepted operation. `DescendantGrantProvenance` preserves `spawning_grant_id` plus the same continuation provenance. Direct round-trip and acceptance/replay tests assert concrete, distinct ids and the exact prior. |
| Canonical replacement authority | **PASS** | Neutral contract validation admits only generated `OperationKind::SessionManagement`, rejects missing/reused replacement ids, and is reused by acceptance and descendant replay. The direct test enumerates every generated non-session-management kind plus unknown integer values. |
| Fresh has no continuation provenance | **PASS** | `validate_spawn_authority_carriage` rejects continuation authority on the fresh branch; `fresh_authority_carries_only_the_spawning_grant` is mutation-sensitive. |
| No target resolution in the leaf | **PASS** | Leaf validation consumes only decoded generated structures and selected-id carriage. Its comments and imports expose no registry, resolver, storage, clock, or Grant lookup. Same-subject/endpoint/domain and exact Grant containment remain the downstream decision owner's obligations. |
| Acceptance rows have executable evidence | **PASS** | The dedicated Leaf 2 suite covers intent disjointness, exact prior, bounded target envelope, both Grant links, exact authority kind, fresh-only carriage, malformed/unknown wire fields, generation overflow, and pre-stateful-work boundary rejection. Current downstream tests separately cover same issuer/endpoint/domain, exact prior scope, Grant liveness, accepted durable replay, and descendant replay. |

## Mutation matrix

Each mutant was applied alone on the main tree, exercised with one focused test, and reverted with `git restore` before the next mutant. No mutant was committed.

| Mutant | Focused oracle | Result |
|---|---|---|
| Accept `SpawnContinuation { prior: None }` | `exact_prior_mutations_reject_before_acceptance` | **KILLED**, exit 101: observed `Ok(())`, expected `MissingExactPrior`. |
| Admit continuation provenance without the replacement Grant / distinct-id check | `compound_authority_requires_both_distinct_grants_and_exact_prior` | **KILLED**, exit 101: missing replacement Grant was admitted instead of returning `MissingReplacementGrant`. |
| Admit a non-`session-management` replacement-authority kind | `compound_authority_requires_both_distinct_grants_and_exact_prior` | **KILLED**, exit 101: `Unspecified` was admitted instead of returning `WrongReplacementAuthorityKind`. |
| Let fresh intent carry exact-prior continuation provenance | `fresh_authority_carries_only_the_spawning_grant` | **KILLED**, exit 101: mutant returned `Ok(())` instead of `UnexpectedContinuationAuthority`. |

The unmodified dedicated test binary then passed all 11 Leaf 2 tests.

## Clean-tree verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — generated paths clean, 57 vectors, 17 promoted vectors, 26 implementation checks, 38 mutation witnesses, and 54 model-promotion blocks.
3. `cd operator-domain && npm run build && npm test`: **PASS**, 27/27.
4. `cd pi-adapter && npm test`: **PASS**, 38/38 including the real core/adapter generation-bump, reconnect, and core-restart loop.

`git diff --check` passed and the tracked tree was clean after mutation restoration and verification. No temporary worktree was created; `/` retained 55G free.

## Retrospective note

The original 2026-08-13 independent artifact cannot be recovered from repository history, so this report does not invent its reviewer claims. It is a fresh retrospective review of the landed implementation and repair commits. The Leaf 2 validator and dedicated test files have no code diff from completion commit `dba571e0` to the reviewed current tree; later protobuf additions append downstream claim/promotion contracts without changing the reviewed `SpawnRequest`, `SpawnContinuation`, or `ContinuationAuthorityProvenance` shapes.

One initial workspace-suite attempt was discarded because another concurrent lane temporarily mutated unrelated spawn files while it ran. The four verification groups reported above were rerun from a clean prechecked tree and passed; only those clean reruns count as review evidence.
