---
id: fix-cockpit-empty-state-capability-fetch
kind: story
stage: review
tags: [bug, ux]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-17
updated: 2026-08-17
---

# Fix: empty-session cockpit does not reliably fetch adapter capability

## Symptom

With the Pi adapter attached and zero runtime sessions, the Sessions sidebar
renders the `+` action disabled because `model.adapters["pi"].status.capability`
is absent. The declared managed target (`uat-logical-target`) therefore never
reaches `declaredManagedSpawnTarget`, even though the adapter status projection
can supply it.

## Root cause

Ground-truth probing separated the response merge from the trigger:

- A real-shaped completed `QueryDiagnosticsResponse` with an adapters result
  merges the Pi status and capability into `model.adapters`; adapter-id lookup
  and the generated oneof are not the defect.
- The production default has subscriptions enabled, so the shell's one-shot
  initial no-selection callback can issue one unscoped query. However,
  `Reconciler.onReconciliationComplete` fires only after stream-loss snapshot
  recovery, not initial login/subscription establishment. There is no
  composition-owned first-login capability refresh.
- Capability discovery is therefore coupled to an incidental shell selection
  render. In a headless composition with subscription startup disabled,
  `queryAdapterStatus(undefined, ...)` explicitly returns before querying; the
  regression probe observed zero calls. In the live empty state, a missed or
  failed one-shot render query leaves no selected session transition to retry
  it.

The empty state needs a bounded, explicit lifecycle-owned refresh rather than
using `undefined` selection as the capability-discovery trigger.

## Fix approach

- Add one composition-owned adapter-status refresh at startup/login.
- On reconciliation completion, refresh every adapter already known in
  `model.adapters`, include the selected session adapter when present, and use
  one unscoped query only when no adapter id is known.
- Keep selection/connectivity refreshes scoped to an actual selected session;
  an empty selection itself no longer drives diagnostics.
- Dedupe concurrent requests by adapter scope independent of trigger reason so
  a reconciliation render and its selection microtask cannot double-query the
  same adapter.
- Keep capability advisory: the UI derives spawn availability from the returned
  generated declaration and does not create authority or delivery behavior.

## Regression test

`web-cockpit/tests/main.test.ts` composes the real cockpit shell/projection/
reconciler around a protocol stub with zero sessions. It asserts a bounded
startup-plus-reconcile query set, exact capability merge, the enabled
`Spawn uat-logical-target on pi` action, and the canonical disabled reason for
an adapter status with no capability. Removing the explicit empty-state
startup/reconciliation refresh makes the focused test fail with zero queries.

## Design decisions

- Direct composition-root ownership was chosen over changing protocol or
  snapshot semantics: adapter status is already an authoritative diagnostics
  projection, and the bug is when the existing read is scheduled.
- Known adapters are refreshed individually for precise cache clearing; an
  unscoped page is used only when the model has no adapter identity yet. This
  is adapter-neutral and bounded by the current adapter registry.
- No proto edits are required.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` (caller-selected for the
  composition/reconciliation trigger and generated capability boundary).
  Direct implementation was used because this delegated worker cannot fan out
  under the harness recursion guard and the write surface was confined to the
  cockpit composition root, shell, and one integration test.
- Ground truth: the generated response oneof and `mergeAdapterStatusResult`
  correctly install a real-shaped adapter capability. The production default
  does enable subscriptions, so the post-`22c5aee` shell initial callback can
  make one request; the defect was that this incidental render callback was the
  only first-login empty-state owner, while the reconciler callback is
  stream-recovery-only. The pre-fix compose probe with subscription startup
  disabled observed **0** diagnostics calls and both new empty-state assertions
  failed. Capability discovery now has explicit startup and reconciliation
  lifecycle owners.
- Implementation: startup refreshes the known adapter registry once (one
  unscoped query while it is empty); reconciliation refreshes sorted known
  adapter ids plus the selected adapter, or uses the same unscoped fallback.
  Empty shell selection no longer issues diagnostics. Concurrent trigger
  reasons dedupe by adapter scope, preventing a reconciliation render's
  selection microtask from duplicating the same read.
- UX UAT nit: the Resources rail and mobile destination are disabled and
  annotated only when every known adapter has an authoritative capability and
  none declares a resource capability. Unknown capability stays honest rather
  than being treated as absence; existing resource records/collections keep the
  destination available.
- Files changed: `web-cockpit/src/main.ts`,
  `web-cockpit/src/ui/shell.ts`, `web-cockpit/tests/main.test.ts`, and this
  story. No Protobuf, generated contract, operator-domain, core, server, or
  adapter implementation changed.
- Regression evidence: the jsdom/protocol-stub composition starts with zero
  sessions, observes exactly one startup query and exactly one further query
  after explicit reconciliation (**2 total**), merges the Pi capability,
  enables `Spawn uat-logical-target on pi`, and keeps a capability-less adapter
  disabled with `Adapter capability is unavailable.` The same test verifies the
  Pi-only capability annotates both Resources destinations as unavailable.
- Mutation evidence: removing the explicit startup
  `refreshAdapterStatuses("startup")` call caused both focused empty-session
  tests to fail with zero diagnostics calls (**2/2 killed**). `git restore`
  reinstated the committed source, and the focused tests then passed **2/2**.
- Full verification (2026-08-17):
  1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**.
  2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS** (60 vectors, 19 promoted, 33 implementation checks, 38 mutation witnesses).
  3. `cd operator-domain && npm run build && npm test` — **PASS** (34/34).
  4. `cd pi-adapter && npm test && npm run test:mutations` — **PASS** (131/131; 33/33 mutations killed).
  Consumer suite: `cd web-cockpit && npm test` — **PASS** (153/153).
- Four-step confirmation: the focused regression passes; the full cockpit and
  four verification groups pass; the production composition probe now exposes
  the declared spawn action from an empty snapshot; the live UAT stack was not
  restarted or touched, so browser confirmation remains the operator's review
  checkpoint.
- Review weight: `standard` (project default). This standalone story remains at
  `stage: review` for the orchestrator's bounded review and operator UAT.
- Adjacent issues parked: none.

## Follow-up fixes (same UAT session, 2026-08-17)

- **Layout stacking (bug #9):** `.session-detail`/`.resources-view` set their
  own `display`, overriding the `hidden` attribute — sessions screen rendered
  session chat + resources + planned views stacked in one pane. Fixed with the
  global `.cockpit [hidden] { display: none !important; }` invariant in
  shell.css. Live-verified: sessions shows only the detail; Git shows only the
  planned view.
- **Invisible restart (bug #8):** the detail header (restart's only home) was
  `hidden = !mobile` — unreachable on desktop. Header now stays visible on
  both layouts; only the back affordance hides outside mobile. Live-verified:
  restart renders 34x34, enabled.
