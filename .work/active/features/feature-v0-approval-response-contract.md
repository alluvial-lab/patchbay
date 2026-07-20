---
id: feature-v0-approval-response-contract
kind: feature
stage: drafting
tags: [protocol, verification, foundation]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-elicitation-response-contract]
release_binding: null
gate_origin: null
created: 2026-07-19
updated: 2026-07-19
---

# Feature: Typed approval-response contract (binary Approve/Deny)

## Brief

Add the typed proto message for approval-contract elicitation responses and
make the core validate the decision + map denial to `Declined` at the system
boundary. This is the approval-side sibling of
`feature-v0-elicitation-response-contract` (which shipped the question side).

Today the `approval` `response_contract` carries only `contract_kind` +
`ui_hints` + a free-form `schema_ref`, and the core's
`ElicitationSlotLayer` (`core/src/acceptance/elicitation.rs`) maps every
`Completed` approval response to `ElicitationState::Answered` — its own comment
(line 301) defers "Mapping denial (Rejected) to Declined... to v0.x." The
validation function (`core/src/acceptance/elicitation_response.rs`) accepts any
`APPROVAL_RESPONSE` matched to an approval contract as a no-op pass — it never
decodes a decision payload. So Approve and Deny are indistinguishable to the
core: there is no typed `ApprovalResponsePayload`, no decision validation, and
no denial-to-`Declined` terminal mapping. The cockpit (`feature-v0-web-cockpit`)
cannot build its binary approval UI (Unit 4) against a contract that can't
represent the decision authoritatively — this is the blocker that returned the
cockpit to `drafting`.

This feature closes that gap with the same Option-A discipline as the question
side: add a typed `ApprovalResponsePayload { decision }` proto message, bind it
into the approval `response_contract`, regen Rust + TS, add core boundary
validation that decodes + validates the decision (Fail Fast), and complete the
deferred denial-to-`Declined` terminal mapping. The pi-adapter's
`DeliveryTranslator` currently rejects `APPROVAL_RESPONSE` as
`unsupported_command` ("approval Elicitation delivery is an explicit minimal-
slice follow-on") — this feature also wires adapter delivery of the response
decision so the decision reaches the agent that opened the approval gate.

The protocol semantics are **already committed** in `docs/PROTOCOL.md` (line 156:
"Completion updates the Elicitation terminal (`answered` or `declined`) only if
response validation succeeds"; line 277: `declined` "Covers... approval denial
when the response contract treats denial as terminal"; line 330: `approval` =
"Allow/deny/allow-once/always/policy-amend/modified-input permission response").
This feature implements already-committed semantics; it is not a new semantic
decision. The formal model (`specs/seed/elicitation_lifecycle.qnt`) already
includes `declined` as a terminal state and `approval` as a contract kind.

## Strategic decisions

- **Core-validation boundary: (B) contracts + core boundary validation +
  terminal mapping.** (Inherited from the question-side feature's D1; restated
  for this feature.) The core decodes + validates the typed decision payload at
  the acceptance boundary (Fail Fast) AND completes the deferred
  denial-to-`Declined` terminal mapping. A typed wire contract the core does not
  validate is the "claimed-but-not-enforced conformance surface" the prior
  session's component-layer arc existed to prevent. The cockpit is a browser
  client and cannot be the protocol's enforcer.
- **Approval-decision vocabulary: binary Approve/Deny committed v0.1.0; the
  richer 4 decisions reserved.** `ApprovalResponsePayload.decision` is an enum
  with exactly two committed values (`APPROVED`, `DENIED`) plus reserved values
  for `ALLOW_ONCE`/`ALWAYS`/`POLICY_AMEND`/`MODIFIED_INPUT` (mirroring the
  `RESPONSE_CONTRACT_KIND_RESERVED_*` pattern). The cockpit's locked mock is
  binary Approve/Deny only; the richer decisions have no v0.1.0 producer (the
  pi-adapter's `ApprovalHandler` returns a plain `boolean`) or consumer.
  Committing 4 unused wire values would be speculative (extension-pressure
  checklist). Promotion of the richer decisions is additive (new enum values),
  non-breaking. Operator chose (B) over (A) full 6-value vocabulary.
- **DENIED → `Declined`; APPROVED → `Answered`.** A `Completed` approval
  response with `decision = DENIED` terminalizes the Elicitation as `Declined`
  (completing the deferred work at `elicitation.rs:301`). `decision = APPROVED`
  terminalizes as `Answered` (existing behavior). This is the
  already-committed PROTOCOL:156/277 semantics. A non-`Completed` terminal
  response (Rejected/Failed/Expired/...) still leaves the slot pending (the
  response itself failed; another surface may answer) — unchanged.
- **Adapter delivery of the decision.** The pi-adapter's `DeliveryTranslator`
  gains an `APPROVAL_RESPONSE` arm that decodes the decision and resolves the
  pending `ApprovalHandler` (today it auto-approves; this wires the real
  operator decision through). `ELICITATION_RESPONSE` (question side) stays
  `unsupported_command` for now — the question-side producer is a separate
  follow-on; this feature wires only the approval delivery the cockpit's Unit 4
  exercises. (The cockpit tests against vectors + a fake transport, so it does
  not require a live producer — same posture as the question side.)

## Extension pressure classification

- **Committed v0.1.0:** the binary approval decision (`APPROVED`/`DENIED`) +
  the DENIED→`Declined` terminal mapping. These carry normative validation +
  lifecycle semantics and must have conformance-vector coverage.
- **Reserved seam:** the richer approval decisions (`ALLOW_ONCE`/`ALWAYS`/
  `POLICY_AMEND`/`MODIFIED_INPUT`) — named in the proto enum, not validatable in
  v0.1.0; a contract carrying one rejects as `validation_failed` until
  promotion. Promotion is a clean reserved-seam reversal (new committed enum
  value), not a quiet gap.
- **Explicitly rejected for v0.1.0:** an ad-hoc browser-only approval payload
  convention (untyped, un-validated) — the "hand copies" anti-pattern; the exact
  un-checked surface the question-side feature existed to prevent. Pi-adapter
  question-Elicitation emission — deferred (separate follow-on; the cockpit
  tests against vectors + a fake transport).

## Simplification opportunity

- The deferred comment at `core/src/acceptance/elicitation.rs:301` ("Mapping
  denial (Rejected) to Declined is a response-contract validation concern,
  deferred to v0.x") is **resolved** by this feature — the comment is removed
  and the mapping implemented. No dead code left behind.
- The `ApprovalHandler` auto-approve default (`() => true`) in
  `pi-adapter/src/pi_session.ts` becomes the real operator-decision path; the
  default stays as a fallback only if no Elicitation is pending (a tool call
  arriving with no open approval gate still auto-approves per current behavior —
  this feature does not change that fallback, it adds the Elicitation-driven
  path).
- No existing code is deleted beyond the deferred comment. The slot layer's
  terminalization gains a DENIED branch; the validation function gains an
  approval arm.

## Foundation references

- `docs/PROTOCOL.md` — `response_contract` registry (line 330: `approval` =
  "Allow/deny/..."); line 156 (approval-response terminalizes `answered` or
  `declined`); line 277 (`declined` covers approval denial); line 308
  (invalid-response policy reserved); `ElicitationState` lifecycle (line 287-291)
- `contracts/proto/patchbay/elicitations.proto` — `ResponseContract`,
  `ResponseContractKind`, the `oneof contract_body` (question arm exists; this
  feature does NOT add an approval arm — approval carries the decision in the
  *response payload*, not the contract body)
- `contracts/proto/patchbay/operations.proto` — `Operation`,
  `OperationKind::APPROVAL_RESPONSE`, `PayloadEnvelope`
- `core/src/acceptance/elicitation_response.rs` — `validate_response_payload`
  (the approval no-op-pass arm at the `ApprovalResponse + Approval => return
  Ok(())` line is what this feature replaces with real decision validation)
- `core/src/acceptance/elicitation.rs` — `ElicitationSlotLayer` terminalization
  (line 298-306: the deferred DENIED→Declined mapping)
- `core/src/acceptance/pipeline.rs` — `submit` (validation already runs for
  `ApprovalResponse`; this feature makes it decode the decision)
- `pi-adapter/src/delivery.ts` — `DeliveryTranslator` (the
  `APPROVAL_RESPONSE`/`ELICITATION_RESPONSE` `unsupported_command` arm)
- `pi-adapter/src/pi_session.ts` — `ApprovalHandler` / `ApprovalRequest`
- `feature-v0-elicitation-response-contract` — the structural template (same
  proto + core-validation + projection + vectors shape, question side)
- `feature-v0-web-cockpit` — the consumer whose Unit 4 is blocked on this

## Scope boundaries (what this feature does NOT do)

- Does NOT add a typed body to the `approval` `response_contract` (the decision
  lives in the *response payload* `ApprovalResponsePayload`, not the contract —
  the contract only needs to know it's an `approval` kind, which it already does).
- Does NOT commit the richer 4 approval decisions (reserved).
- Does NOT wire pi-adapter question-Elicitation emission (deferred — separate
  follow-on).
- Does NOT change `ElicitationState` or the command-lifecycle state machine
  beyond the DENIED→Declined mapping.
- Does NOT fix the pre-existing `check:drift` repo gap (documented; not run in
  CI; a proto change will touch the generated Rust bindings).

<!-- Subsequent sections (Design, Implementation Notes, etc.) accumulate as
work progresses. -->

## Blocker (2026-07-19) — protocol semantics: is a DENIED approval `Answered` or `Declined`?

Surfaced during feature-design. This is a semantic 50/50 in the *committed
protocol spec*, not an implementation choice — the protocol text is internally
tense on whether an operator denial is a satisfied response or a refusal.

### The tension in `docs/PROTOCOL.md`

- **Line 156** (approval-response row): "Completion updates the Elicitation
  terminal (`answered` or `declined`) only if response validation succeeds."
  Reads decision-driven: the Completion (OperationState) updates the terminal,
  and *which* terminal depends on the decision.
- **Line 276** (`answered` def): "A valid response Operation **satisfied the
  contract** and first durable terminal commit selected it as the answer."
- **Line 310**: "`answered` does not imply the underlying tool/action succeeded;
  it only means the response slot was satisfied."
- **Line 277** (`declined` def): "The expected responder explicitly
  refused/rejected/denied the Elicitation **without satisfying it**. Covers
  question rejection and approval denial when the response contract treats
  denial as terminal."

### The two defensible readings

- **(A) Decision-driven — DENIED is a satisfied slot that terminalizes `Declined`.**
  A DENIED approval is a valid response that satisfied the contract by stating
  a decision; the response Operation reaches `Completed`; the slot reads the
  decision and selects `Declined`. Supported by 156 + 276 + 310 ("the slot was
  satisfied"). Under this reading, `Declined` is a *subtype of answer* — the
  slot was satisfied, the valence of the decision picks the terminal label.
  Cost: the slot layer (payload-opaque today) gains a kind-gated approval-
  decision decode; `declined`'s "without satisfying it" wording (277) must be
  reconciled (a DENIED approval *did* satisfy the contract — it gave a valid
  decision).
- **(B) Refusal-driven — DENIED is a refusal that terminalizes `Declined`.**
  A DENIED approval is the responder refusing to satisfy the contract (per 277's
  "without satisfying it"); the response Operation does NOT reach `Completed`
  (it's not a satisfied response); the slot maps the refusal to `Declined`.
  Supported by 277. Under this reading, `Answered` = "operator said yes (or
  gave a real answer)"; `Declined` = "operator refused." Cost: a denial is not
  a `Completed` response, so what OperationState does the denial response
  reach? `Rejected` conflates with command-rejection (PROTOCOL:110 — "system
  refused the command"); a new state or a repurposed one is needed. This is
  where the deferred comment at `elicitation.rs:301` ("denial (Rejected) to
  Declined") went wrong — it borrowed `Rejected` CommandState for an
  Elicitation refusal, conflating the two state machines.

### Why this is a blocker, not an implementation call

The choice changes:
- the proto shape (does the decision live in a `Completed` payload, or is
  denial a distinct non-`Completed` outcome?);
- the core terminal mapping (slot decodes the decision, vs. slot maps a
  refusal-state);
- what the cockpit renders (is "Denied" an answer or a refusal?);
- and the `answered`/`declined` definitions in PROTOCOL.md, which must be
  rolled forward to disambiguate regardless of which is chosen.

It is a protocol-semantics decision affecting a safety-claiming state machine
and the `answered`/`declined` terminal semantics. Per the harness rule
(semantic 50/50 ⇒ surface, do not resolve with judgment), this must go to the
operator. The feature stays at `stage: drafting` until resolved.

### What I need from the operator

Pick (A) or (B). Either is defensible; they produce materially different
protocol behavior. My lean is **(A)** — it keeps `Completed` as "the response
delivered a valid decision" (consistent with 310's "slot was satisfied"), and
`Declined` becomes the terminal for a satisfied-but-negative decision, which
matches how an operator thinks about a denial (they *did* answer; the answer was
no). (B) is more literal to 277's "without satisfying it" but forces a new
non-`Completed` outcome for denials and risks the `Rejected`-conflation the
deferred comment fell into. But this is your call.

If (A): I'll roll PROTOCOL:277 forward so "without satisfying it" no longer
contradicts a DENIED approval being a satisfied slot, and the slot layer gains
the kind-gated decision decode.
If (B): I'll roll PROTOCOL forward to define the denial response's
OperationState (not `Rejected`), and the slot layer stays payload-opaque.
