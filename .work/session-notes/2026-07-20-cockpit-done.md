# Session note — 2026-07-20 (cockpit designed, implemented, reviewed, done)

A durable handoff note for the next session. Read this before continuing.

## What happened this session

The cockpit (`feature-v0-web-cockpit`) — the v0.1.0 product center, the
phone-usable critical path's last substantial layer — went from `drafting` all
the way to `done` in one session, through a full design → implement → review →
fix → close cycle. This is the arc the prior two session notes had been
building toward.

### 1. Design refresh (commit `e03c7c6`)

Ran `feature-design` to refresh the existing design body against the
now-shipped typed contracts (the approval-response blocker that had bounced
the cockpit to `drafting` was cleared by the prior session). Grounded every
assumption against the shipped proto; made surgical corrections: tightened
Unit 1 (`LoadSnapshotResponse.snapshot_payload` → `fromBinary(SessionSnapshotSchema)`),
tightened Unit 2 (fold switches on `StoredEventPayload.kind`), updated the
EC1–EC3 risk to RESOLVED. Advanced `drafting → implementing`; spawned 5 child
stories (U1→U2→U3∥U4→U5).

### 2. Implementation — Units 1–3 (commits `9c9db28`, `b2f8dcc`, `342b8d2`)

One feature-owning worker (`gpt-5.6-sol`, high) implemented U1 (protocol
client + cursor-reconcile), U2 (presentation-model fold), U3 (markdown
rendering). 12 tests passing including the load-bearing reconnect +
generation-monotonicity properties.

### 3. Blocker on Unit 4 — surfaced honestly (commit `633c218`)

The worker hit a genuine semantic 50/50: EC1 (cockpit design) said "checkbox
for select-many," but D2 of the done `feature-v0-elicitation-response-contract`
reserves `select-many` for v0.1.0 (the payload is single-answer). The worker
correctly returned the story to `drafting` rather than forcing it through —
exactly the design-flaw escape hatch working as intended.

**Operator decision (option 1):** align the cockpit to the shipped single-
answer contract. Render all `question` contracts as select-one radio; a
`select-many` ui_hint is non-authoritative (PROTOCOL § ui_hints) and renders
as select-one. This does not reduce a committed guarantee (select-many was
never committed for v0.1.0); it removes a contradictory clause. Design body
EC1/Unit-4 corrected; story returned to `implementing` (commit `8e5f1d1`).

### 4. Implementation — Units 4–5 + feature → review (commits `64f6217`, `ceb1e4d`, `ccb0804`)

Re-dispatched the worker; it implemented U4 (elicitation handling, select-one
radio throughout) + U5 (shell + list + detail). All 5 stories `done`; feature
`review`. 25 tests passing; presentation conformance + 24 vectors green.

### 5. Standard-weight review — Request changes, 6 blockers (commit `614c013`)

Fresh-context `gpt-5.6-sol` reviewer returned **Request changes** with 6
receiver-confirmed material current-cycle blockers. All adjudicated against
repo context and confirmed material:

1. **Reconnect snapshot permanently skips non-session state** — `replaceFromSnapshot`
   built a sessions-only model and advanced the cursor, permanently skipping
   commands/observations/elicitations in the gap. Load-bearing snapshot-
   correctness violation.
2. **No runnable integrated cockpit** — package was library-only; no browser
   entry, no served assets. Epic contract is "running code is the output."
3. **Failure/retry/degraded surfaces absent** — no `failure-banner`, no
   `deduplicated` wiring, no reconnect/stale/offline banners. Conformance-floor
   gap.
4. **EC3 grouped questions only as an isolated helper** — `renderElicitationGroup`
   existed but was never wired; model had no grouping key.
5. **Two load-bearing tests were mutation-survivable (self-defining)** —
   cursor-before-fold and first-answer guards passed when removed. THE MOST
   IMPORTANT finding: exactly the failure mode this verification program
   exists to prevent.
6. **Expanded delivery trace contradicted SPEC.md:117** — Q2's "tap expands
   full state history + LSNs" vs SPEC's "no trace-timeline UI in v0.1.0."

**Operator decision (Blocker 6):** reduce the UX decision to a reserved seam;
keep v0.1.0 as SPEC described. v0.1.0 shows current `CommandState` + last
transition only; full expandable trace is deferred to post-v0.1.0. SPEC.md is
the source of truth and is unchanged. Q2 revised; promotion is additive, not
a quiet widening.

### 6. Fix stride + receiver closure → done (commits `08c3f36`, `bbd1bc9`)

A focused fix worker addressed all 6 blockers. The receiver (orchestrator)
independently re-verified the Blocker 5 mutation tests rather than trusting
the worker's claim:
- Cursor-before-fold mutation → "cursor does not advance when projection
  folding throws" FAILED. Earned.
- `applyCompletedResponse` terminal-guard mutation → "second completed response
  cannot overwrite first answer" FAILED. Earned.
- Worker additionally recorded the `foldElicitation` guard mutation →
  "late Elicitation event cannot rewrite first terminal state" FAILED. Earned.

All three mutation tests genuinely fail on mutations — the "earned not
asserted" standard is met. Final integrated verification green: 36 cockpit
tests, presentation conformance, 24 vectors, 19 web-server tests. Feature
advanced `review → done`.

## Where we are now

`epic-v0-1-0-implementation` — **7/8 features done** + `feature-v0-cli` at
`drafting`. The cockpit (the phone-usable critical path) is complete. The
epic stays at `implementing` because `feature-v0-cli` (the last v0.1.0 layer)
is not done.

## What's next: the CLI, then the epic review

`feature-v0-cli` (`stage: drafting`, depends on `feature-v0-protocol-seam`
which is done). Per its deps it's ready to design. After it lands, all epic
children are done and the epic is ready for its deeper aggregate review.

## Commits this session

```
bbd1bc9 review: feature-v0-web-cockpit (Approve — 6 blockers fixed + verified; -> done)
08c3f36 review-fix: feature-v0-web-cockpit (6 blockers addressed; -> review)
614c013 review: feature-v0-web-cockpit (Request changes -> implementing; 6 blockers + Q2 revision)
ccb0804 implement: feature-v0-web-cockpit
ceb1e4d implement: story-v0-web-cockpit-shell-session-list-detail
64f6217 implement: story-v0-web-cockpit-elicitation-handling
8e5f1d1 design: resolve cockpit U4 select-many blocker (option 1)
633c218 implementation-discovery: story-v0-web-cockpit-elicitation-handling
342b8d2 implement: story-v0-web-cockpit-markdown-rendering
b2f8dcc implement: story-v0-web-cockpit-presentation-model-fold
9c9db28 implement: story-v0-web-cockpit-protocol-client-reconcile
1b0e820 session-note: 2026-07-20 cockpit design refreshed; unblocked
e03c7c6 feature-design: feature-v0-web-cockpit (blocker cleared; design refreshed; 5 child stories)
```

## Notes for the next session

- The cockpit is a *consumer* of the machine-checked component layer + typed
  contracts. It must not re-bind protocol states to bespoke CSS or invent
  ad-hoc payload conventions. Both constraints held through review.
- The cockpit's claimed conformance properties (identity-before-submission,
  stale-never-live, first-answer-wins, snapshot-correctness, cursor-after-fold)
  are now genuinely earned — each has a mutation test that fails on the
  mutated implementation. The component-layer arc's lesson was applied forward
  successfully.
- Two operator decisions were recorded this cycle and should NOT be
  re-litigated:
  - **select-many:** renders as select-one in v0.1.0 (D2 reserves select-many;
    ui_hints are non-authoritative).
  - **delivery trace:** v0.1.0 shows current `CommandState` + last transition
    only; the full expandable trace is a reserved seam (SPEC.md is source of
    truth). Q2 was revised.
- `web-server/src/main.ts` gained a cockpit asset-serving route (Blocker 2
  fix). It serves the built cockpit bundle at `/` without touching
  auth/CSRF/RPC/login.
- `.work/bin/work-view` still hangs on `board`/`--blocking` (pre-existing
  uncommitted binary mod). Verify state via `grep ^stage:`.
- `check:drift` is the known broken repo gap (needs `protoc-gen-prost`);
  not in CI, not the cockpit's concern.
- After `feature-v0-cli` lands, the epic is ready for its deeper aggregate
  review (broader lenses, not a repeat of line-level child review).
