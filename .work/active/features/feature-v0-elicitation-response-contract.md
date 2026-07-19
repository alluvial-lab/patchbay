---
id: feature-v0-elicitation-response-contract
kind: feature
stage: drafting
tags: [protocol, verification, foundation]
parent: epic-v0-1-0-implementation
depends_on: [feature-protocol-idl-and-conformance]
release_binding: null
gate_origin: null
created: 2026-07-19
updated: 2026-07-19
---

# Feature: Typed elicitation-response contract (EC1–EC3)

## Brief

Add the typed proto messages for question-contract elicitation responses and
make the core validate them at the system boundary. Today the
`response_contract` for a `question` carries only `contract_kind` +
`ui_hints: repeated string` + a free-form `schema_ref` string, and the core's
`ElicitationSlotLayer` (`core/src/acceptance/elicitation.rs`) treats response
payloads as opaque bytes — it tracks command lifecycle + terminal transition,
never response content. No conformance vector pins the response shape. The
cockpit (`feature-v0-web-cockpit`) cannot build its elicitation handling
(Units 2/4/5) against typed bindings that do not exist.

This feature closes that gap with **Option A — typed proto messages**: add
`QuestionContract { options[], allow_free_text }` + `ResponseOption` +
`ElicitationResponsePayload { selected_option_id, free_text, clarification }`,
bind them into `Elicitation`/`Operation.payload`, regen Rust + TS, and add
core boundary validation that rejects malformed response payloads against the
active `response_contract` (Fail Fast). It also adds conformance vectors
pinning the three committed response shapes (EC1 free-text option, EC2
answer-and clarification, EC3 grouped = N independent single-answer).

This is the protocol-semantics-bearing prerequisite the cockpit is blocked on.
It is scoped as a sibling feature (not folded into the cockpit) so the
contracts-layer change gets its own design + review surface, and so the
cockpit's elicitation handling can be conformance-checked the same way the
presentation-component layer was — the lesson from the prior session's 7-pass
thorough review: a claimed-but-not-enforced conformance surface is a liability.

## Strategic decisions

- **Core-validation boundary: (B) contracts + core boundary validation.** The
  feature owns both the typed proto messages AND making the core validate the
  typed response payload against the active `response_contract` at the
  acceptance boundary. Rationale: a typed wire contract that the core does not
  validate is exactly the "claimed-but-not-enforced conformance surface" the
  prior session's component-layer arc existed to prevent. The cockpit is a
  browser client and cannot be the enforcer of a protocol contract. Fail Fast
  at the system boundary is the project's stated rule. (A) contracts-only was
  rejected for this reason; (C) adding pi-adapter question-Elicitation
  emission is deferred — the cockpit tests against vectors + a fake transport,
  and a live producer is a cleanly separable follow-on adapter story.

- **EC3 grouping is presentation, not proto.** The grouped multi-question
  shape (N independent single-answer Elicitations rendered as one visual card)
  is a cockpit presentation concern. The proto payload stays single-answer:
  one `ElicitationResponsePayload` per response Operation, correlated to one
  `ElicitationId`. This feature must NOT introduce a multi-answer contract
  (one Elicitation carrying multiple questions) — that is the reserved
  "multi-answer accumulation" seam at `docs/PROTOCOL.md:312`. Promotion of
  that seam is a clean reserved-seam reversal, not a quiet gap-fill.

- **EC1 free-text is a `ui_hint` within the committed `question`
  contract_kind, not a contract-kind promotion.** A `select-one`/`select-many`
  question may append a free-text option; the response carries the free-text
  string instead of a selected option id. No new `ResponseContractKind`.

- **EC2 answer-and is a response-payload shape, not a new contract_kind.** A
  question response may carry a selected option plus an appended free-text
  clarification in one Operation. The clarification is supplementary; the
  structured selection remains the primary answer.

## Extension pressure classification

- **Committed v0.1.0:** the three response shapes (EC1 free-text option, EC2
  answer-and clarification, EC3 grouped-as-presentation). These carry
  normative validation semantics (what the core rejects) and must have
  conformance-vector coverage.
- **Reserved seam:** multi-answer accumulation (one Elicitation carrying
  multiple questions) — `docs/PROTOCOL.md:312`. The proto shape must carry
  the future-relevant demarcator (single-answer payload) so promotion is a
  visible reversal, not a quiet gap. The reserved `RESPONSE_CONTRACT_KIND_*`
  enum values already exist and remain reserved.
- **Explicitly rejected for v0.1.0:** untyped `PayloadEnvelope` JSON for
  question responses (Option B from the prior session) — the "hand copies"
  anti-pattern; un-checkable. Pi-adapter question-Elicitation emission
  (Option C) — deferred to a follow-on adapter story; not this feature's
  concern.

## Simplification opportunity

- The free-form `schema_ref: string` on `ResponseContract` becomes
  load-bearing only for reserved structured-schema contracts; for committed
  `question`/`approval` kinds the typed messages replace the need to consult
  an external schema. No deletion required, but the design should clarify
  when `schema_ref` is authoritative vs. when the typed `QuestionContract` is.
- No existing code is removed — the core's opaque-bytes handling is extended,
  not replaced. The `ElicitationSlotLayer` keeps its lifecycle role; response
  validation is an additive boundary check, not a rewrite of the slot layer.

## Foundation references

- `docs/PROTOCOL.md` — `response_contract` registry (§ response_contract),
  `ui_hints` registry, reserved multi-answer seam (line 312), first-valid-
  answer-wins, invalid-response policy
- `contracts/proto/patchbay/elicitations.proto` — `ResponseContract`,
  `ResponseContractKind`, `ElicitationState`
- `contracts/proto/patchbay/operations.proto` — `Operation`,
  `OperationKind::ELICITATION_RESPONSE`, `PayloadEnvelope`
- `contracts/proto/patchbay/common.proto` — `PayloadEnvelope`,
  `PayloadContentType`, `StoredEventKind`
- `core/src/acceptance/elicitation.rs` — `ElicitationSlotLayer` (opaque
  payload handling today)
- `core/src/acceptance/pipeline.rs` — `submit()` boundary validation order
- `contracts/vectors/` — conformance vector envelope + property mapping
- `feature-v0-web-cockpit` — the consumer whose Units 2/4/5 reference the
  types this feature adds (Risks section flags EC1–EC3 as the blocker)

## Scope boundaries (what this feature does NOT do)

- Does NOT add pi-adapter question-Elicitation emission (deferred — Option C).
- Does NOT promote the reserved multi-answer seam.
- Does NOT change `ElicitationState` or the command-lifecycle state machine.
- Does NOT change the `approval` contract shape (binary approval already
  works; this feature is about the `question` contract's typed options).
- Does NOT fix the pre-existing `check:drift` repo gap (documented; not run
  in CI; not this feature's concern — but a proto change will touch the
  generated Rust bindings, so the drift check will report diffs regardless
  of correctness).

<!-- Subsequent sections (Design, Implementation Notes, etc.) accumulate as
work progresses. -->
