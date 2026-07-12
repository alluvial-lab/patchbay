# Session Note — Storage Arc: Persistence Feature Complete

## Context

Continued the storage arc from the previous session. Started with 3/4 persistence stories done (workspace-and-port, rusqlite-impl, recovery) and the 4th — proptests — at `stage: implementing`. Ended with the entire `feature-v0-core-persistence` feature `done`, unblocking the three sibling core features (acceptance, authority, sessions).

## What happened

### 1. Proptest suite: wrote, then deep-reviewed through 4 convergence rounds

The proptest story (`story-v0-core-persistence-proptests`) is a `[verification]`-tagged story, so per `.work/CONVENTIONS.md` it routed to the **deep lane** — two-phase (completeness → adversarial), fresh-context, cross-model reviewer on `openai-codex/gpt-5.6-sol`. The convergence loop ran 4 rounds:

| Round | Posture | Verdict | Key catches |
|---|---|---|---|
| 1 | completeness + adversarial (parallel) | Incomplete / Fails the genuine-checking test | LSN-only comparisons (payload-corruption mutant passes); vacuous snapshot test (arbitrary marker, compares only tail suffixes); untestable stale/wrong-domain claim; no concurrent dedup; no cross-restart dedup; vacuous gap-free-dedup; weak conflict test; misnamed cross-domain test |
| 2 | adversarial re-review | Not converged | `dedup_appends_remain_gap_free` still vacuous (count oracle derived from observed outcomes); full-EventId check missing |
| 3 | adversarial re-review | Not converged | dedup LSN sequence not anchored at 1 (windows-only check); invalid-LSN coverage missed LSN 0 + cross-domain |
| 4 | final convergence check | **Converged (ship)** | No blockers; suite is an honest evidence floor |

**Final suite: 18 tests** (15 proptests at 100 cases each + 3 mutation-discipline integration tests). The 3 mutation tests are the genuine-checking proofs: `gap_free_catches_injected_lsn_bug` (+1 LSN fault), `crash_recovery_catches_payload_corruption` (constant-payload fault), `dedup_catches_injected_double_apply` (always-append fault). Each wraps `RusqliteStorage` in a fault-injecting adapter and asserts the property FAILS on the buggy store.

### 2. Feature-level deep review: persistence as a foundation

After all 4 stories reached `done`, advanced `feature-v0-core-persistence` to `review` and ran a feature-level deep review (the final pass over the whole feature, not just the last story). Verdict: **Request changes** — 2 blockers + 6 important.

The 2 blockers were **forward-dependencies / misframing, not persistence defects**:

1. **Event registry can't represent sibling state.** `StoredEventKind` has one variant per concrete storable message, but `OPERATION`'s payload (`Operation` message) doesn't carry `OperationState` transitions, and `SESSION_STATE` can't carry session identity/generation/tombstone. This is a **forward dependency**: `StoredEventKind` is a proto3 enum — siblings add variants during their own `feature-design`. Whether `OPERATION`'s payload should be `Operation` or a richer `CommandRecord` is the **acceptance feature's** design decision, not persistence's. The storage layer correctly treats payloads as opaque bytes. Resolved as a reserved seam, documented in the feature's Extension pressure classification.

2. **Snapshot consistent-prefix materialization not implemented.** The feature body's Q3 said snapshots are written "in the same transaction as the log prefix they materialize," but `do_write_snapshot` only validates the LSN anchor + writes arbitrary bytes. This was actually **refined during the workspace-and-port review** (the port.rs `write_snapshot` doc deliberately split the obligation: port validates anchor + writes atomically; caller materializes consistent-prefix content). The implementation is honest; the feature body was stale. Q3 reconciled.

The 6 important items were real and fixed:
- Feature Testing section reconciled with actual evidence (stopped overclaiming idempotent replay / crash-no-loss / prefix consistency / stale-wrong-domain as fully tested).
- Writer-actor comment corrected (runs on tokio worker, not "off the async runtime").
- `open_in_memory()` fails explicitly on non-UTF-8 paths (was silently falling back to `./patchbay-test.db`).
- 3 weak tests renamed/made honest.
- Added Extension pressure classification section (Q1-Q5 + reserved seams) per AGENTS.md checklist.

Feature advanced to `done`.

## Key lessons

1. **A 4-round convergence loop is the cost of honest verification evidence.** Round 1 caught the obvious issues (LSN-only comparisons, vacuous snapshot test). But round 2 found the round-1 fix for `dedup_appends_remain_gap_free` was *still* vacuous — the readback count oracle was derived from the observed outcomes, so an always-Duplicate mutant produced `0 == 0` and passed. Round 3 found the round-2 fix didn't anchor the sequence at LSN 1. Each round the reviewer found a mutant the previous fix missed. The convergence loop is not bureaucratic overhead — it's the mechanism that makes the evidence floor honest. A single-pass review would have shipped a vacuous suite.

2. **Mutation tests must assert the property FAILS on the buggy store, not just that two values differ.** The original `gap_free_property_distinguishes_off_by_one` compared two computed arrays — proving Rust ranges differ, not that the property catches the bug. The real fix (`gap_free_catches_injected_lsn_bug`) wraps the store in a `+1`-LSN adapter and asserts `run_gap_free_check(...).is_err()`. The distinction: a mutation test that doesn't run the property against a buggy implementation is not a mutation test.

3. **"Forward dependency" is not "blocker."** The feature reviewer flagged the event registry as a blocker, but the persistence layer's job is to store opaque bytes + a kind discriminator, not to define the sibling features' message schemas. The siblings add their own `StoredEventKind` variants during their `feature-design`. Calling this a persistence blocker would have blocked the foundation on work that properly belongs downstream. The honest framing: reserved seam, documented in the extension classification.

4. **Stale feature bodies lie.** The feature body's Q3 said "same transaction as the log prefix they materialize," but the actual port.rs design (refined during an earlier review) split the obligation. The implementation was honest; the doc was stale. The reviewer correctly flagged the mismatch. Lesson: when a review refines a design decision, update the feature body in the same stride, or it becomes a lying artifact.

5. **"Reopen" is not "crash."** The crash-recovery tests drop the handle and reopen the file — they prove committed-event visibility across handle reuse, not `synchronous=FULL` durability against power loss. Process-level durability is a config assertion, not a property a proptest can prove without a fault-injection harness that kills the process mid-transaction. This is an honest scope boundary, documented in the story body and test doc-comments, not a test defect.

## Current state

```
epic-v0-1-0-implementation  (drafting)
├── epic-v0-core  (implementing)
│   ├── feature-v0-core-persistence   ✅ DONE (4/4 stories)
│   ├── feature-v0-core-acceptance    (drafting — UNBLOCKED, depends_on persistence)
│   ├── feature-v0-core-authority     (drafting — UNBLOCKED, depends_on persistence)
│   └── feature-v0-core-sessions      (drafting — UNBLOCKED, depends_on persistence)
├── feature-v0-protocol-seam  (drafting, blocked on core)
├── feature-v0-pi-adapter      (drafting, blocked on core)
├── feature-v0-web-server      (drafting, blocked on seam)
├── feature-v0-web-cockpit     (drafting, blocked on web-server)
└── feature-v0-cli             (drafting, blocked on seam)
```

52 tests green across `patchbay-core` (18 proptests + 18 rusqlite + 9 recovery + 7 port smoke). Clippy + rustfmt clean.

## Next

The persistence foundation is complete. The three sibling core features — **acceptance**, **authority**, **sessions** — are all unblocked and can proceed in parallel (each depends only on the now-done `feature-v0-core-persistence`). Each needs its own `feature-design` pass to define its port and how it consumes the `Storage` trait.

The critical path to the phone-usable walking skeleton is: core (acceptance/authority/sessions) → protocol-seam → web-server → web-cockpit, with the Pi adapter parallel.
