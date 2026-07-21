# Session note — 2026-07-20 (cockpit design refreshed; unblocked for implementation)

A durable handoff note for the next session. Read this before continuing.

## What happened this session

The cockpit feature (`feature-v0-web-cockpit`) was the last blocker on the
v0.1.0 phone-usable critical path. The prior session shipped
`feature-v0-approval-response-contract` (commit `b326067`), which resolved the
blocker that had bounced the cockpit back to `drafting` on 2026-07-19 (the
typed `ApprovalResponsePayload` did not exist then; Unit 4's binary approval
could not be built as a boundary-valid Operation).

This session ran the `feature-design` pass to refresh the existing design
body against the now-shipped typed contracts and advance to implementation.

### Design refresh — grounded against shipped reality, not re-litigated

The design body (Q1–Q4, EC1–EC4, 5 units) was written *before* the typed
contracts shipped. Rather than re-design from scratch, this pass verified the
design's assumptions against the actual shipped proto and made surgical
corrections where the shapes had crystallized:

- **Blocker status:** verified RESOLVED. All four dependencies `done`
  (`feature-v0-web-server`, `feature-v0-presentation-component-layer`,
  `feature-v0-elicitation-response-contract`, `feature-v0-approval-response-contract`).
  The typed contracts exist exactly as specified: `QuestionContract` /
  `ResponseOption` / `ElicitationResponsePayload` / `ApprovalResponsePayload` /
  `ApprovalDecision` in `contracts/proto/patchbay/elicitations.proto`.
- **Unit 1 tightened:** `LoadSnapshotResponse.snapshot_payload` is opaque
  `bytes`, not a typed message — the cockpit must `fromBinary(SessionSnapshotSchema, …)`
  and *replace* the model from it (snapshot is authority; never merge). This
  was a glossed-over seam in the original pseudocode. Also confirmed the
  web-server's `rpc.ts` already proxies all three RPCs (Submit/Subscribe/
  LoadSnapshot) with operator-session auth (CSRF on Submit, relaxed on the
  read-only two), and overwrites `operation.sender` with the session-verified
  actor — the cockpit must not trust its own sender claim.
- **Unit 2 tightened:** the fold switches on `StoredEventPayload.kind` and
  deserializes each variant via its `*Schema`. 5 cockpit-relevant kinds
  (OPERATION/OBSERVATION/ELICITATION/COMMAND_TRANSITION/SESSION_STATE); 3
  authority-family kinds (GRANT/DESCENDANT_GRANT/REVOCATION) ignored in v0.1.0
  (no operator-facing surface). `ResponseContract.contract_body` is a oneof
  (`question: QuestionContract`); approval carries no typed body in v0.1.0
  (binary).
- **Risks updated:** the EC1–EC3 + approval risk (originally "new shapes not
  yet in proto") is RESOLVED — the cockpit binds generated TS from
  `@patchbay/contracts`; no proto extension needed.
- **Resolution note appended** to the Implementation discovery section,
  recording the blocker is cleared and the design is verified accurate
  against the shipped proto.

No new design decisions were needed — Q1–Q4 and EC1–EC4 were settled
interactively in the original pass and remain valid. No semantic 50/50 remains
open. The refresh was verification + seam-tightening, not re-design.

### Stage advanced + 5 child stories spawned

`feature-v0-web-cockpit`: `drafting → implementing`. 5 child stories with a
declared `depends_on` chain matching the feature's Implementation Order:

```
story-v0-web-cockpit-protocol-client-reconcile       (U1, no deps)
  → story-v0-web-cockpit-presentation-model-fold     (U2, depends on U1)
    → story-v0-web-cockpit-markdown-rendering        (U3, ∥ with U4, depends on U2)
    → story-v0-web-cockpit-elicitation-handling      (U4, ∥ with U3, depends on U2)
      → story-v0-web-cockpit-shell-session-list-detail (U5, depends on U3 + U4)
```

Each story carries grounded shapes verified against the shipped proto and
verification evidence keyed to the conformance-floor properties
(identity-before-submission, stale-never-live, first-answer-wins,
snapshot-correctness, retry-safety).

### Commit

```
e03c7c6 feature-design: feature-v0-web-cockpit (blocker cleared; design refreshed; 5 child stories)
```

## Where we are now

`epic-v0-1-0-implementation` — **6/7 features done** + the cockpit now at
`implementing` with a real dependency graph. The cockpit is the last
substantial implementation layer before v0.1.0 is shippable. After the cockpit
lands (and the CLI, `feature-v0-cli` at `drafting`), the epic is ready for
review.

## What's next: implement the cockpit

The cockpit is ready for `implement-orchestrator` or inline `implement`. The
implementation order is the story chain above. Two load-bearing things to
watch (carried forward from prior session notes):

- **Reconnect correctness (Unit 1) is load-bearing** — the snapshot-correctness
  rule means an unreconciled snapshot must never render as live. Property-test
  the reconcile path; mutate the stale-marking and confirm the test fails.
- **The markdown renderer choice (Unit 3) should be spiked first** — must be
  small + safe + streaming-friendly. `marked` + `DOMPurify` is the suggested
  baseline; evaluate bundle size before committing. If no satisfactory option
  exists, surface as a blocker.
- The cockpit is a *consumer* of the machine-checked component layer
  (`tokens.css` + `components.css` + `check-presentation.mjs`). It must not
  re-bind protocol states to bespoke CSS — that is a conformance-floor
  violation the component layer exists to prevent. If it needs a
  presentation the locked primitives don't cover, extend the layer; do not
  bypass it.

### Conformance standard (the forward lesson)

Hold the cockpit to the same standard as the component layer: if it claims a
property (identity-before-submission, retry-safety, stale-never-live,
first-answer-wins), that property should be checkable, not just asserted in
prose. The component-layer arc's lesson applies: a claimed-but-not-enforced
conformance surface is a liability.

## Notes for the next session

- The cockpit's elicitation handling (Unit 4) is now buildable against the
  typed proto — the failure mode that bounced it (ad-hoc browser payload
  convention for approval) is structurally prevented by the typed
  `ApprovalResponsePayload`. Do not reintroduce an untyped payload path.
- EC3 (grouped multi-question) is N independent single-answer Elicitations
  as one visual card — the payload stays single-answer. A true multi-answer
  contract is a reserved seam; do not promote it silently.
- `.work/bin/work-view` hangs on `board`/`--blocking` invocations in this
  harness (pre-existing uncommitted binary modification noted across session
  notes). Verify state via `grep ^stage:` on the item files instead.
- `check:drift` is the pre-existing broken repo gap (needs `protoc-gen-prost`);
  not run in CI, not the cockpit's concern. TS build + `check:vectors` +
  `check:presentation` are the real checks.
- After the cockpit: `feature-v0-cli` (`stage: drafting`, depends on
  `feature-v0-protocol-seam` which is done). Then the epic is ready for review.
