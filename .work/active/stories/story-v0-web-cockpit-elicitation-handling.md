---
id: story-v0-web-cockpit-elicitation-handling
kind: story
stage: drafting
tags: [ux, protocol]
parent: feature-v0-web-cockpit
depends_on: [story-v0-web-cockpit-presentation-model-fold]
created: 2026-07-20
updated: 2026-07-20
release_binding: null
gate_origin: null
---

# Story: Cockpit Unit 4 — elicitation handling (three shapes + mobile sheet)

Implements Unit 4 of `feature-v0-web-cockpit`. Now buildable against the typed
proto (the blocker that bounced the cockpit back to `drafting` is resolved —
see the feature body's Resolution note).

## Scope

`web-cockpit/src/ui/elicitation.ts` — the three elicitation shapes (EC1–EC3)
plus approval, and the mobile bottom-sheet.

## Grounded shapes (verified against shipped proto, 2026-07-20)

The typed contracts the cockpit binds to (from `contracts/proto/patchbay/`):

- **Approval** — `OperationKind.APPROVAL_RESPONSE` + `ApprovalResponsePayload
  { ApprovalDecision decision }`. Committed: `APPROVED`/`DENIED`. The four
  richer decisions are reserved (server rejects with `validation_failed`).
  `ResponseContractKind.APPROVAL` carries no typed `contract_body` in v0.1.0
  (binary). Core maps `DENIED` → `ElicitationState.Declined` (the load-bearing
  terminal mapping, pinned by conformance vector
  `approval-response-denied.json`).
- **Question** — `OperationKind.ELICITATION_RESPONSE` +
  `ElicitationResponsePayload { selected_option_id; free_text; clarification }`.
  `ResponseContract.contract_body.question: QuestionContract { repeated
  ResponseOption options; bool allow_free_text }`. `ResponseOption { option_id;
  label }`. Core maps any valid question response → `Answered`.

### EC1 — free-text option within a question contract (committed)

`QuestionContract.allow_free_text = true` → the UI appends a free-text option
("or type your own answer"). The response Operation carries `free_text`
instead of `selected_option_id`. Control shape: radio for `select-one`
(including the free-text alternative), checkbox for `select-many`. Never mix
in one elicitation.

### EC2 — answer-and composed response (committed)

A selected option *plus* an appended free-text clarification in one Operation
(the "And..." field). `ElicitationResponsePayload` carries both
`selected_option_id` and `clarification`. The clarification is supplementary;
the structured selection is the primary answer.

### EC3 — grouped multi-question (committed as grouping; multi-answer reserved)

Claude's nested multi-question maps to **N independent single-answer
Elicitations** opened as a batch, rendered as one visual card. Each is
independently single-answer and independently terminal. The payload stays
single-answer — grouping is a presentation concern, NOT a proto concern. A
true multi-answer contract (one Elicitation carrying multiple questions) is a
reserved seam ("multi-answer accumulation", PROTOCOL:312); promotion is a
clean reserved-seam reversal, not a quiet gap. **Do not promote it silently.**

### EC4 — Attention destination deferred (committed)

Elicitations surface inline in the session detail + via the `needs-you` badge
on session rows. The cross-session Attention destination is deferred from
v0.1.0; its mock (`attention/attention.html`) is preserved on disk, not wired.

## Implementation notes

- The response Operation is built from the selected option (or free-text) +
  optional clarification, correlated to the `ElicitationId`. First-answer-wins
  is enforced core-side; the UI disables controls once the elicitation
  terminalizes (ANSWERED/DECLINED/EXPIRED/CANCELLED/WITHDRAWN/SUPERSEDED/STALE)
  and shows the terminal state.
- The mobile bottom sheet clones the tapped card's content (per the locked
  mock) and force-shows the options/actions that the inline-teaser CSS hides.
- Approval = direct Approve/Deny buttons (no select-then-submit). Build the
  `ApprovalResponsePayload` and submit as `APPROVAL_RESPONSE`.
- All state-binding uses the presentation-component layer primitives
  (`.elicitation-card` + `--answered`/`--declined`/etc modifiers). Do not
  re-bind `ElicitationState` to bespoke CSS — that is a conformance-floor
  violation the component layer exists to prevent.

## Acceptance criteria

- [ ] Binary approval submits Deny/Approve directly (no select-then-submit);
  payload is `ApprovalResponsePayload { decision }`
- [ ] Question with free-text option (`allow_free_text`) submits either a
  selected `option_id` or a `free_text` string
- [ ] Answer-and submits a selected option + `clarification` in one Operation
- [ ] Grouped multi-question renders N questions as one card; each answers
  independently (N independent single-answer Elicitations — no multi-answer
  payload)
- [ ] Once terminal, the elicitation controls disable and show the terminal
  state (control shape matches `ui_hint`/contract_kind throughout)
- [ ] No silent promotion of the reserved multi-answer seam

## Verification evidence

- Unit tests: elicitation control-shape matching (radio vs checkbox by
  contract_kind/select-one vs select-many); needs-you derivation.
- Regression tests: the three submission shapes (EC1 free-text, EC2
  answer-and, EC3 grouped) produce the correct `Operation` payload.
- Conformance note: the cockpit's elicitation handling is a *consumer* of the
  machine-checked contract, not a re-definition of it. If the cockpit needs a
  shape the proto doesn't carry, surface it — do not invent an ad-hoc browser
  payload convention (that was the failure mode the approval-response arc
  existed to prevent).

## Implementation discovery (2026-07-20)

Implementation stopped before Unit 4 code because the story's `select-many`
control requirement contradicts the shipped, operator-chosen v0.1.0 contract.
This is a reserved-seam disposition, not a mechanical browser choice.

- `feature-v0-elicitation-response-contract` D2 explicitly settles
  **`select-many` as reserved for v0.1.0** and says the cockpit's locked mock is
  select-one-only. A `select-many` `ui_hint` may be wire-present because hints
  are non-authoritative/open, but multiple selections have no typed response
  home and reject until future promotion.
- The generated `ElicitationResponsePayload` carries singular
  `selected_option_id`, and core validation requires exactly one of that
  singular field or `free_text`. There is no `repeated selected_option_ids`.
- This story nevertheless requires "checkbox for select-many" and says control
  shape must match that hint, while simultaneously forbidding silent promotion
  of multi-answer. A checkbox group semantically permits multiple selections;
  serializing only one would be misleading, and serializing several is
  impossible without an ad-hoc payload or protocol promotion.

Two defensible resolutions produce materially different protocol/surface
behavior:

1. **Align the cockpit to shipped v0.1.0 (matches prior operator decision):**
   render committed question contracts as select-one/free-text only; treat a
   reserved `select-many` hint as unsupported/unrenderable and leave the
   Elicitation pending under the existing reserved surface-reject posture.
   Remove the checkbox requirement from this story and the parent feature.
2. **Promote multi-answer:** add a typed repeated-selection payload, core
   validation, vectors, adapter handling, and UI checkbox semantics through a
   protocol-change ceremony. This crosses the cockpit's forbidden write scope
   and reverses the reserved v0.1.0 decision.

Rendering checkboxes while allowing only one checked value is not a third
resolution: it presents radio semantics with the wrong accessible control and
would claim capability the boundary cannot accept. Per the design-flaw escape
hatch, this item returns to `drafting`; Units 4 and 5 are not implemented until
the operator/design lane resolves the contradiction.
