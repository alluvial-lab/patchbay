---
id: leaf6-runtime-evidence-rereview6-2026-08-14
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-runtime-evidence-promotion-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-14
updated: 2026-08-14
---

# Deep re-review pass 7 — runtime-evidence-promotion-contract (Leaf 6), 2026-08-14

**Verdict: CLEAN.** Independent fresh-context `openai-codex/gpt-5.6-sol` thorough pass 7 over the cumulative implementation through `9ae4488`, in completeness → adversarial order. The round-6 BLOCKER is genuinely closed: successful spawn `Result → Completed` is exclusive to the deferred evidence route, the ordinary atomic writer rejects it without writes, exact retries remain idempotent, and the promotion owner remains the only path to public spawn completion. No material finding or nit survived this pass.

## Round-6 closure verdict

**PASS.** `ObservationWriteRoute` and `classify_observation_write_route` are the single classifier used by the SQLite writer helper for prefix retry validation and by both dedicated Observation writers. The classifier receives the `OperationKind` from the command rebuilt from the durable prefix inside the writer transaction; no Observation- or caller-supplied operation kind participates. Both writers call `recorded_events_in_transaction` and rebuild `CommandIndex` before route admission or insertion.

The required cross-dedicated probe passed for both `Delivered` and `Running` spawn prefixes:

- `append_observation_transition_audited(successful Result, → Completed, CommandCompleted audit)` returned `UnsupportedOperation` and left the complete prefix byte-for-byte unchanged;
- `append_spawn_result_deferred_audited` committed only Observation + `spawn_completion_deferred` audit, returned the original ids on exact retry, and replay retained the non-terminal pre-promotion command state;
- spawn Status, `unsupported_command`/rejected Result, and `execution_failed` Result committed through the ordinary Observation → transition → audit transaction and zero-write rejected through the deferred writer.

## Adversarial classifier assessment

The classifier's effective input is constrained by `derive_transition` plus pair validation before route admission:

- a successful spawn Result framed with any non-`Completed` transition rejects before durability;
- a Status framed as completion rejects; a canonical Status remains the ordinary `→ Running` transition;
- `Expired`, `Cancelled`, and `Superseded` are policy transition states, not Status/Result-derived outcomes, and forged Observation pairs selecting them reject without writes;
- a successful non-spawn Result remains ordinary completion. A reviewer-only durable Query prefix completed and replayed through the atomic writer while the deferred writer rejected it without writes;
- identical duplicate command correlations retain the protocol's one-logical-correlation semantics, while conflicting command correlations reach neither dedicated writer;
- a failed spawn Result cannot reach the deferred writer and remains compatible with the conflict-suppression machinery in both failure→success and success→failure order.

Boundary confusion was not found. The decisive kind is `record.operation.kind` from the transaction-local durable rebuild. The hot path and restart path converge because exact retries are reclassified at their original replay position using the same helper before canonical Observation/transition/audit validation.

## Findings

None.

## Probe and mutant matrix

Reviewer-only probes and mutants ran in detached temporary worktrees and were removed before the final clean-tree suite.

| Probe / mutant | Oracle | Result |
|---|---|---|
| Real durable spawn in `Delivered` and `Running`; successful Result offered to ordinary writer | `successful_spawn_result_is_exclusive_to_deferred_writer` | **PASS** — `UnsupportedOperation`, zero writes. |
| Same Result through deferred writer, exact retry, command replay | same cross-dedicated oracle | **PASS** — one source/audit pair, exact ids reused, command remains non-terminal. |
| Spawn Status, rejected Result, failed Result across both writers | `non_success_spawn_observations_stay_on_atomic_writer` | **PASS** — ordinary transaction admitted; deferred route zero-write rejected. |
| Durable Query successful Result across both writers and replay | reviewer boundary matrix | **PASS** — ordinary `Delivered → Completed`; deferred zero-write rejected. |
| Successful spawn Result framed as `Expired`, `Cancelled`, or `Superseded`; Status framed as `Completed`; conflicting command correlations | reviewer boundary matrix | **PASS** — every malformed/mixed pair rejected without writes. |
| Managed-origin omission/bypass and exact staged retry | authenticated adapter managed-report oracle | **PASS** — candidate never registers current; exact staged id reused. |
| Delayed and old-producer SessionReports | authenticated stale-report oracle | **PASS** — outer quarantine + linked audit only; hot/replay state unchanged. |
| Result ordering, both conflict orders, exact failed retry, staged idempotency | runtime-evidence + spawn-completion oracles | **PASS**. |
| Quarantine context forgery and nested-candidate redispatch mutant (b) | forged-context and all-family outer-only oracles | **PASS**. |
| Generic-route exclusivity, source/transition rollback, ordered authority → session → claim → command fold, restart/catch-up | storage, aggregate, server restart oracles | **PASS**. |
| **Fresh mutant:** classify successful Query Result as deferred | classifier table + reviewer durable-Query boundary | **KILLED** — both failed with exit 101. |
| **Fresh mutant:** return `AtomicTransition` for successful spawn Result | classifier table + cross-dedicated storage probe | **KILLED** — both failed with exit 101; ordinary completion bypass became observable. |

The other three dedicated writers remain structurally disjoint: each accepts a distinct generated Rust envelope, writes one fixed distinct `StoredEventKind`, rejects its kind on generic routes, and validates the complete typed envelope before insertion. Quarantine has no bytes/`Any` candidate arm. No typed-envelope overlap or replay reinterpretation was found.

## Prior-oracle regression sweep

All prior material oracles remained green:

- managed SessionReport bypass and omitted-origin quarantine;
- stale-report outer quarantine and unchanged hot/replay snapshot;
- lifecycle-before-Result ordering;
- conflicting Result suppression in both orders;
- quarantine context forgery;
- nested quarantine redispatch mutant (b);
- authority-before-session phase typing / ordered four-view publication mutant (d);
- six generic Observation-route exclusions plus backend guard;
- transition fault rollback and historical stranded-Result fail-closure;
- exact successful and failed Result retries;
- staged-successor exact retry/conflict/restart;
- aggregate restart/catch-up, promotion audit/grant linkage, and real descendant-authorized submit after restart.

The parked adapter-honesty trust boundary, unreleased-prefix compatibility posture, and downstream unguarded continuation-submit path were not treated as Leaf-6 findings, as directed.

## Full verification suite

Final commands ran on the restored clean target tree:

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**, including 25 runtime-evidence tests, server integration/restart tests, doctests, and warnings-denied clippy.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS**; 54 vectors, 17 promoted vectors, 22 implementation checks, 38 mutation witnesses, generated bindings clean.
3. `cd operator-domain && npm run build && npm test` — **PASS**, 9/9.
4. `cd pi-adapter && npm test` — **PASS**, 29/29 including the real core/adapter restart e2e.

`git diff --check` passed and the tree was clean before this review file was written.

## Final recommendation

**Advance `research-handoff-spawn-runtime-evidence-promotion-contract` to `done`.** Pass 7 has no material current-cycle blocker and no nit; the thorough convergence loop is complete for Leaf 6.
