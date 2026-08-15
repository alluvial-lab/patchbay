---
id: fleet-target-resolution-rereview-2026-08-14
kind: story
stage: done
tags: [review, spawn]
parent: fleet-spawn-target-resolution
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-14
updated: 2026-08-14
---

# Thorough re-review — Unit 1 target resolution and compound authority, pass 2

**Verdict: CLEAN.** Commit `61f9c13` closes the pass-1 BLOCKER without weakening the operation-aware resolver from `f217140`. Valid continuations now hit one explicit temporary guard after generated payload validation and before Grant lookup, target resolution, or the ordinary durable acceptance append; fresh spawn still traverses both authority phases and persists the selected single Grant. The guard is explicitly assigned to Unit 2 for removal when its atomic `SpawnClaimAccepted` writer lands.

## Findings

No material findings or nits.

- **Guard and zero-write boundary:** `core/src/acceptance/pipeline.rs` has one `SpawnRequest.intent = continuation` guard. It returns rejected / `unsupported_command` / `unsupported_command` before either Grant port, the resolver port, or `append_dedup_with_payload`. The focused oracle uses a compound-ready continuation, proves all three port call counts remain zero, and observes an empty authority-domain log. No continuation `AcceptedOperation` remains for `SpawnDescendantTail` or command/claim projections.
- **Fresh spawn:** the same oracle proves fresh spawn remains accepted, calls the initial and resolved authority phases once each, resolves once, writes one `Operation`, and preserves the selected adapter-spawn Grant id.
- **Compound decision and replay:** `continuation_resolution_round_trips_both_grant_ids_and_exact_prior` compares the production decision result with an independent expected broad Grant id, replacement Grant id, canonical `session-management` kind, and exact prior. The same oracle runs against both the live registry and a registry rebuilt from the durable authority log.
- **Both halves remain load-bearing:** the broad-spawn-only witness rejects when replacement authority is absent; the replacement-only witness rejects fabricated broad-spawn provenance. Expired, revoked, wrong-subject, wrong-endpoint, and wrong-generation replacement Grants remain covered.
- **One-shot sequencing:** repository search found no second valid-continuation submit guard. Structural continuation validation and post-resolution compound-carriage validation remain ordinary safety checks rather than hidden feature barriers. The pipeline comment and the story's fix-round notes identify atomic claim/provenance persistence as the removal condition and name Unit 2 as owner.
- **Pass-1 vacuity removed:** the accepted-continuation durable-envelope test and its broad-Grant-only assertion are gone. The only decoded spawn envelope in the replacement boundary oracle is the accepted fresh-spawn envelope.

The normal server policy may still persist a protocol-distinct rejected-attempt audit record at the RPC wrapper. That record creates no command or claim state, has no spawn-completion reason, and is inert to `SpawnDescendantTail`; it is not the unsafe continuation acceptance event identified in pass 1.

## Mutation matrix

All mutations were applied to the clean `61f9c13` tree, killed by the named focused oracle, restored with `git restore`, and followed by a green restored run.

| Mutation | Result | Oracle |
|---|---|---|
| Remove the sole continuation guard from `core/src/acceptance/pipeline.rs` | **KILLED** (`exit 101`) | `guarded_continuation_writes_no_event_while_fresh_spawn_uses_resolved_grant_path` failed with actual `Accepted` versus expected `Rejected`. |
| Strip `replacement_grant_id` from the production compound resolution result in `core/src/authority/check.rs` | **KILLED** (`exit 101`) | `continuation_resolution_round_trips_both_grant_ids_and_exact_prior` failed against the independent expected `replacement-a` Grant id. |
| Restore clean source | **PASS** | Both focused tests passed after their respective restores; `git diff --check` and the clean-tree check passed before the full suite. |

## Full clean verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, and 38 killed mutation witnesses.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 23/23 tests.
4. `cd pi-adapter && npm test` — **PASS**, 29/29 tests.

The worktree was clean after mutation restoration and after the full suite, before this review file was written.

## Recommendation

**Advance `fleet-spawn-target-resolution` to `done`.** The intentional sequencing dependency remains: Unit 2 removes this single guard only when atomic accepted claim, complete compound provenance, fence, prior-work effects, and deduplication land together.
