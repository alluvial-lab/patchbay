---
id: story-v0-web-cockpit-presentation-model-fold
kind: story
stage: implementing
tags: [ux, protocol]
parent: feature-v0-web-cockpit
depends_on: [story-v0-web-cockpit-protocol-client-reconcile]
created: 2026-07-20
updated: 2026-07-20
release_binding: null
gate_origin: null
---

# Story: Cockpit Unit 2 — presentation model fold

Implements Unit 2 of `feature-v0-web-cockpit`. The pure projection the UI
renders.

## Scope

`web-cockpit/src/domain/model.ts` — the in-browser presentation model: a pure
fold over `StoredEventPayload` events producing the view state (sessions with
connectivity×activity axes, commands with `CommandState`, pending
elicitations). Browser-side analog of the core's `SessionRegistry` /
`CommandIndex` — read-only, never authoritative. Binds to the
presentation-component layer primitives for rendering.

## Grounded shapes (verified against shipped proto, 2026-07-20)

- `StoredEventPayload { StoredEventKind kind; bytes payload }` — kind-
  discriminated bytes. The fold switches on `kind` and deserializes each
  variant via its `*Schema`:
  - `OPERATION` → `Operation` (registers a command; may carry an
    ELICITATION_RESPONSE/APPROVAL_RESPONSE payload that opens/closes an
    elicitation)
  - `OBSERVATION` → `Observation` (agent messages, tool calls, lifecycle
    facts)
  - `ELICITATION` → `Elicitation` (state + `ResponseContract` +
    `contract_body.question: QuestionContract` when kind is `question`)
  - `COMMAND_TRANSITION` → `CommandTransition` (advances `CommandView.state`
    / `failure_code`; `to_state` is `OperationState`, the refined
    `CommandState`)
  - `SESSION_STATE` → `SessionStateEvent` (oneof mutation:
    `registered`/`generation_bumped`/`connectivity_changed`/
    `activity_changed`/`relabeled`)
  - `GRANT`/`DESCENDANT_GRANT`/`REVOCATION` — authority-family events the
    cockpit ignores in v0.1.0 (no operator-facing surface yet).
- `ResponseContract.contract_body` is a `oneof { QuestionContract question }`
  (approval carries no typed body in v0.1.0 — it is binary). `QuestionContract
  { repeated ResponseOption options; bool allow_free_text }`. `ResponseOption
  { option_id; label }`.
- `ElicitationResponsePayload { selected_option_id; free_text; clarification }`
  (EC1 free-text, EC2 answer-and) — the cockpit reads these off the
  ELICITATION_RESPONSE `Operation.payload` for already-answered display, and
  builds them on submission (Unit 4).
- `ElicitationState`: OPENED/PENDING (active) → ANSWERED/DECLINED/EXPIRED/
  CANCELLED/WITHDRAWN/SUPERSEDED/STALE (terminal). 9 members — the component
  layer binds all 9.

## Implementation notes

- `fold(model, ev)` is a pure function `(PresentationModel, StoredEventPayload)
  → PresentationModel`. Read-only: it never writes back. Reconnect
  reconciliation (Unit 1) replaces the model from a snapshot.
- `activityDetail` (Option C) is composed from the Observation stream
  (`tool_call`, `tool_execution_start/end`, `message_update`, `agent_end`,
  `turn_start/end`) — an ephemeral presentation hint. The durable `activity`
  stays `idle`/`working`/`unknown`.
- `needsYou` is derived: a session is needs-you if its last command is
  terminal-and-awaiting-input OR it has a pending (`OPENED`/`PENDING`)
  elicitation.
- `SessionGenerationBumped` tombstones the prior generation; the fold must
  not render a superseded generation as live (identity-before-intent +
  snapshot-correctness).

## Acceptance criteria

- [ ] `fold` is a pure function over (model, event) → model
- [ ] Stale/unknown connectivity never renders as live (dominance rule enforced
  in the view binding, not just present in data)
- [ ] `activityDetail` composes from Observations but does not mutate durable
  `activity` state
- [ ] Reconnect replaces the model from a snapshot; the old model is never
  rendered as live during reconciliation
- [ ] A superseded session generation does not render as live

## Verification evidence

- Property test: fold against event sequences — generation monotonicity,
  stale-never-live, reconnect reconciliation, terminal-first-durable-commit-wins
  (a late terminal candidate is audit-only, never overwrites the committed
  terminal).
- The fold is the highest-leverage unit to property-test; it carries the
  conformance-floor properties (identity-before-submission, stale-never-live,
  first-answer-wins derives from the core, but the fold must preserve it in
  projection).
