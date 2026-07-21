---
id: story-v0-web-cockpit-elicitation-handling
kind: story
stage: done
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
instead of `selected_option_id`. Control shape: **select-one radio** for all
`question` contracts (including the free-text alternative when
`allow_free_text`). `select-many` is a reserved, non-authoritative ui_hint
(D2 of `feature-v0-elicitation-response-contract`; `docs/PROTOCOL.md` § ui_hints):
the `question` contract is single-answer in v0.1.0
(`selected_option_id` is singular), so a `select-many` hint renders as
select-one, never as a multi-select checkbox group.

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

- [x] Binary approval submits Deny/Approve directly (no select-then-submit);
  payload is `ApprovalResponsePayload { decision }`
- [x] Question with free-text option (`allow_free_text`) submits either a
  selected `option_id` or a `free_text` string
- [x] Answer-and submits a selected option + `clarification` in one Operation
- [x] Grouped multi-question renders N questions as one card; each answers
  independently (N independent single-answer Elicitations — no multi-answer
  payload)
- [x] Once terminal, the elicitation controls disable and show the terminal
  state (control shape is select-one radio throughout)
- [x] No silent promotion of the reserved multi-answer seam; a `select-many`
  ui_hint renders as select-one (non-authoritative hint; contract is authoritative)

## Verification evidence

- Unit tests: elicitation control-shape is select-one radio for all
  `question` contracts (a `select-many` hint renders as select-one, not
  checkbox); needs-you derivation.
- Regression tests: the three submission shapes (EC1 free-text, EC2
  answer-and, EC3 grouped) produce the correct `Operation` payload.
- Conformance note: the cockpit's elicitation handling is a *consumer* of the
  machine-checked contract, not a re-definition of it. If the cockpit needs a
  shape the proto doesn't carry, surface it — do not invent an ad-hoc browser
  payload convention (that was the failure mode the approval-response arc
  existed to prevent).

## Implementation discovery (2026-07-20) — RESOLVED by operator

Implementation stopped before Unit 4 code because the story's `select-many`
control requirement contradicted the shipped, operator-chosen v0.1.0 contract.
This was a reserved-seam disposition, not a mechanical browser choice.

- `feature-v0-elicitation-response-contract` D2 explicitly settles
  **`select-many` as reserved for v0.1.0** and the cockpit's locked mock is
  select-one-only. A `select-many` `ui_hint` may be wire-present because hints
  are non-authoritative/open, but multiple selections have no typed response
  home and reject until future promotion.
- The generated `ElicitationResponsePayload` carries singular
  `selected_option_id`, and core validation requires exactly one of that
  singular field or `free_text`. There is no `repeated selected_option_ids`.
- The story's original "checkbox for select-many" clause contradicted D2.

### Resolution (operator, 2026-07-20): option 1 — align to shipped v0.1.0

The operator confirmed **option 1**: the cockpit renders all committed
`question` contracts as select-one (radio + optional free-text). A
`select-many` ui_hint is non-authoritative (`docs/PROTOCOL.md` § ui_hints:
"changing a prompt from select-one to free-text does not change the protocol
contract kind") and renders as select-one — the contract is authoritative, not
the hint. This does not reduce a committed guarantee (`select-many` was never
committed for v0.1.0); it removes a contradictory clause from the design body.

**Proto promotion (option 2) is out of scope** — it would be a reserved-seam
reversal (protocol-change ceremony) crossing the cockpit's forbidden write
scope. Rendering checkboxes while allowing only one checked value was rejected:
it presents radio semantics with the wrong accessible control and claims
capability the boundary cannot accept.

The feature body's EC1 and Unit 4 clauses are corrected (this commit). Story
returned to `stage: implementing`; Units 4 and 5 may now proceed.

## Implementation completion notes (2026-07-20)

- Execution capability: inline feature-owning worker; one typed boundary module
  plus DOM-interface tests was cohesive and did not warrant delegation.
- Review weight: standard (project/default); child story closes on green
  verification and receives no independent review.
- Files changed: `web-cockpit/src/ui/elicitation.ts`,
  `web-cockpit/tests/elicitation.test.ts`.
- Tests added: 7 interface/regression tests covering direct approval decisions,
  select-one/free-text/answer-and payloads, non-authoritative `select-many`
  hints, terminal disabling/state modifiers, independent grouped responses, and
  the live-control mobile clone.
- Simplification: one response-Operation builder path owns correlation,
  protobuf envelope, schema reference, and command identity for both response
  kinds; grouping reuses the same independent question renderer rather than
  inventing a multi-answer browser payload.
- Discrepancies from design: none. Mobile sheet classes are layout-only; every
  protocol-state binding uses the locked `.elicitation-card` modifiers.
- Adjacent issues parked: none.

## Verification result (2026-07-20)

- `cd web-cockpit && npm test` — PASS (19 tests).
- Acceptance walk: direct approval, EC1, EC2, EC3, terminal first-answer UI,
  select-one-only control shape, and typed proto consumption all pass.
