---
id: feature-v0-approval-response-contract
kind: feature
stage: done
tags: [protocol, verification, foundation]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-elicitation-response-contract]
release_binding: v0.1.0
gate_origin: null
created: 2026-07-19
updated: 2026-07-20
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
- **DENIED → `Declined`; APPROVED → `Answered` — decision-driven, kind-gated.**
  An operator denial is an *answer* (the operator responded; the answer was
  "no"), not a refusal-to-engage. The approval response Operation reaches
  `Completed` (it delivered a valid decision — the slot was satisfied, per
  PROTOCOL:310). The slot layer, on a `Completed` *approval-response*
  transition, decodes the `ApprovalResponsePayload.decision` (kind-gated:
  **only** approval responses; question responses stay payload-opaque, since
  a valid question answer is always `Answered`): `APPROVED`→`Answered`,
  `DENIED`→`Declined`. `Declined` = "the operator answered with a declining
  decision" (a satisfied slot, negative valence) — *not* "the slot was
  unsatisfied." This completes the deferred work at `elicitation.rs:301` by
  implementing the *correct* mapping (decision-driven), not the one the
  deferred comment sketched (which wrongly used `Rejected` CommandState —
  see D-disambiguation below).
- **D-disambiguation — operator decline ≠ machine rejection.** The deferred
  comment at `elicitation.rs:301` ("Mapping denial (Rejected) to Declined")
  conflated two state machines: `OperationState::Rejected` (PROTOCOL:110 —
  the *system* refusing a command: unsupported, invalid target, delivery
  refusal) vs `ElicitationState::Declined` (PROTOCOL:277 — the *responder*
  answering with a denial). An operator denying an approval is an answer,
  not a command-rejection. The `Rejected`→`Declined` mapping is **not**
  implemented; the deferred comment is removed. `Rejected` CommandState
  stays purely machine-level and never terminalizes an Elicitation.
- **PROTOCOL.md roll-forward required.** The committed spec is internally
  tense: PROTOCOL:277 says `declined` is "without satisfying it" while
  PROTOCOL:310 says `answered` "only means the response slot was satisfied."
  Under the decision-driven resolution, a DENIED approval *did* satisfy the
  slot (a valid decision was delivered). PROTOCOL:277's "without satisfying
  it" is rolled forward so `declined` reads as "the operator answered with a
  declining decision" (a satisfied slot, negative valence), reconciling it
  with 310. The disambiguation between operator `Declined` and machine
  `Rejected` is made explicit. (See Unit: foundation-doc roll-forward.)
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
- **Reserved seam — surface-reject (operator surface signals it cannot handle
  an elicitation).** Distinct from operator approve/decline (an *answer*) and
  from machine `Rejected` (a command-refusal). A surface that receives an
  elicitation it cannot render or act on (no affordance for the contract
  kind) should be able to explicitly reject it rather than silently leaving it
  `pending` (which swallows it on the operator side). v0.1.0 does NOT
  implement this: a surface with no affordance leaves the elicitation
  `pending` until timeout/withdraw, as today. Reserved as a follow-on because
  it is a capability/routing concern, not an operator-answer concern, and its
  resolution has open shape (a third response decision value terminalizing
  as `Cancelled` vs a separate capability signal vs a new `ElicitationState`).
  Named here so it is not lost; promotion is a clean reserved-seam reversal.
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

## Blocker resolution (2026-07-19) — decision-driven (A), operator decline ≠ machine rejection

Resolved by the operator. The protocol text was internally tense (PROTOCOL:277
"without satisfying it" vs PROTOCOL:310 "the slot was satisfied"); the operator
disambiguated the underlying concepts:

- **Operator decline is an answer.** The operator responded; the answer was
  "no." The slot was satisfied (a valid decision was delivered). This is
  `ElicitationState::Declined` — a satisfied slot with negative valence.
- **Operator approve is an answer.** `ElicitationState::Answered`.
- **Rejection is machine-focused.** `OperationState::Rejected` (PROTOCOL:110)
  is the *system* refusing a command (unsupported, invalid target, delivery
  refusal). It is a CommandState, not an ElicitationState, and must never be
  used to implement an operator decline. The deferred comment at
  `elicitation.rs:301` conflated these; it is removed, and the `Rejected`→
  `Declined` mapping is **not** implemented.

**Decision: (A) decision-driven, kind-gated.** A DENIED approval response
reaches `Completed` (it delivered a valid decision); the slot layer decodes
`ApprovalResponsePayload.decision` on a `Completed` *approval-response*
transition only (questions stay payload-opaque): `APPROVED`→`Answered`,
`DENIED`→`Declined`. PROTOCOL:277 is rolled forward so "without satisfying it"
no longer contradicts a DENIED approval being a satisfied slot.

**Surface-reject (operator surface signals it cannot handle an elicitation)**
was considered for v0.1.0 and **deferred as a reserved seam** — see Extension
pressure classification. It is a capability/routing concern distinct from
operator approve/decline, and its resolution has open shape. v0.1.0 leaves an
unrenderable elicitation `pending` until timeout/withdraw, as today.

## Architectural choice

**Mirror the elicitation-response-contract feature's structure** (the question
side, `done`): typed proto message + core boundary validation (Fail Fast) +
slot-layer terminal mapping + pi-adapter delivery + conformance vectors. The
approval side is structurally simpler than the question side in one way (no
`QuestionContract` body — the decision lives in the *response payload*, not the
contract) and more complex in one way (the slot layer gains a kind-gated
decision decode to select `Answered` vs `Declined`).

The decision-driven terminal mapping is the one architectural shift from the
question side: the question side's slot layer stays fully payload-opaque (a
valid question answer is always `Answered`), but the approval side's slot layer
must decode the decision on a `Completed` approval-response transition. This is
bounded and kind-gated — only `OperationKind::ApprovalResponse` + `Completed`
triggers the decode; all other paths stay opaque. The alternative (keeping the
slot fully opaque and driving the terminal off a non-`Completed` refusal state)
was rejected because it would require borrowing `Rejected` CommandState for an
Elicitation outcome, re-creating the conflation this feature exists to fix.

## Implementation Units

### Unit 1: Typed proto message (contracts layer)

**File**: `contracts/proto/patchbay/elicitations.proto`
**Story**: `story-approval-response-proto-message`

Add the typed approval-decision payload. The decision lives in the *response
Operation's payload* (`Operation.payload`), NOT in the `response_contract`
body — the contract only needs to know it's an `approval` kind (which it
already does). No `oneof contract_body` arm for approval.

```proto
// The operator's binary decision on an approval Elicitation. v0.1.0 commits
// APPROVED and DENIED. The richer decisions (ALLOW_ONCE, ALWAYS, POLICY_AMEND,
// MODIFIED_INPUT) are reserved: named in the enum, not validatable in v0.1.0;
// a response carrying one rejects with validation_failed until promotion.
enum ApprovalDecision {
  APPROVAL_DECISION_UNSPECIFIED = 0;
  APPROVAL_DECISION_APPROVED = 1;
  APPROVAL_DECISION_DENIED = 2;
  // reserved, not validatable in v0; submissions reject with validation_failed.
  APPROVAL_DECISION_RESERVED_ALLOW_ONCE = 100;
  // reserved, not validatable in v0; submissions reject with validation_failed.
  APPROVAL_DECISION_RESERVED_ALWAYS = 101;
  // reserved, not validatable in v0; submissions reject with validation_failed.
  APPROVAL_DECISION_RESERVED_POLICY_AMEND = 102;
  // reserved, not validatable in v0; submissions reject with validation_failed.
  APPROVAL_DECISION_RESERVED_MODIFIED_INPUT = 103;
}

// The typed response payload an APPROVAL_RESPONSE Operation carries in its
// Operation.payload (PayloadEnvelope, content_type PROTOBUF). A DENIED
// decision is an *answer* (the operator responded "no"); the slot was
// satisfied. The slot layer selects Answered (APPROVED) vs Declined (DENIED).
message ApprovalResponsePayload {
  ApprovalDecision decision = 1;
}
```

`Operation.payload` (`PayloadEnvelope`) is unchanged in shape — for an
`APPROVAL_RESPONSE`, `content_type` is `PAYLOAD_CONTENT_TYPE_PROTOBUF` and the
bytes are a serialized `ApprovalResponsePayload`.

**Implementation Notes**:
- Enum values follow the `RESPONSE_CONTRACT_KIND_RESERVED_*` pattern (committed
  values low, reserved values at 100+ with comments).
- No `oneof contract_body` arm for approval — the decision is response-side.
- `buf generate` regenerates TS (canonical) + Rust (via `cargo build`/build.rs,
  NOT buf — the pre-existing generator divergence; see the elicitation-response-
  contract feature's notes). Commit TS gen (buf) + Rust gen (prost-build).
- `check:drift` is the pre-existing broken gap; do not run it. TS build +
  `check:vectors` are the real checks.

**Acceptance Criteria**:
- [ ] `ApprovalDecision` enum + `ApprovalResponsePayload` message exist with
      the fields/values above.
- [ ] No `oneof contract_body` arm for approval (decision is response-side).
- [ ] Reserved decision values (100-103) have "reserved, not validatable"
      comments.
- [ ] `buf generate` + `cargo build` regen both targets; TS builds clean.
- [ ] `check:vectors` still passes.

### Unit 2: Core boundary validation + terminal mapping (the Fail-Fast check + DENIED→Declined)

**File**: `core/src/acceptance/elicitation_response.rs` (extend
`validate_response_payload`), `core/src/acceptance/elicitation.rs` (extend
`terminalize_slot`), `core/src/acceptance/ports.rs` (no change — reuses
`ElicitationContractLookup`/`ActiveElicitation` from the question side)
**Story**: `story-approval-response-core-validation`

Two changes:

**(a) Validation** — replace the approval no-op-pass arm in
`validate_response_payload`:
```rust
// was: (ApprovalResponse, Approval) => return Ok(())  // no-op pass
// now:
(OperationKind::ApprovalResponse, ResponseContractKind::Approval) => {
    let payload = decode_approval_payload(operation)?;  // content_type PROTOBUF check + decode
    match payload.decision {
        ApprovalDecision::Approved | ApprovalDecision::Denied => {} // committed v0.1.0
        ApprovalDecision::Unspecified => {
            return Err("approval response has an unspecified decision".to_owned());
        }
        _ => {
            return Err(format!(
                "approval decision {:?} is reserved and not validatable in v0.1.0",
                payload.decision
            ));
        }
    }
    return Ok(());
}
```
The `content_type` check mirrors the question side (Blocker 2 fix): reject if
not `PAYLOAD_CONTENT_TYPE_PROTOBUF` before decoding.

**(b) Terminal mapping** — extend `terminalize_slot` in `elicitation.rs`:
```rust
// On a Completed APPROVAL_RESPONSE transition, decode the decision (kind-gated).
if response_state == OperationState::Completed
    && kind == OperationKind::ApprovalResponse
{
    let decision = decode_approval_decision(response_operation)?;
    slot.state = match decision {
        ApprovalDecision::Approved => ElicitationState::Answered,
        ApprovalDecision::Denied => ElicitationState::Declined,
        _ => return Err(AcceptanceError::CorruptRecord(
            format!("approval response terminal transition carried non-committed decision {decision:?}")
        )),
    };
    slot.terminal_lsn = Some(event_lsn);
    slot.winning_response = Some(response_operation.clone());
}
// Question responses (Completed) stay Answered (existing, payload-opaque).
// Non-Completed terminals leave the slot pending (unchanged).
```
Remove the deferred comment at line 301. The `Rejected`→`Declined` mapping is
**not** added (that was the conflation).

**Implementation Notes**:
- The slot layer's `command_operations` map (added in the elicitation-response-
  contract fix) already holds the response `Operation` — `winning_response` is
  populated. The decision decode reads `response_operation.payload`.
- Kind-gating: the decode runs **only** for `ApprovalResponse` + `Completed`.
  Question responses stay payload-opaque (a valid question answer is always
  `Answered`). This is the bounded architectural shift noted in Architectural
  choice.
- `decode_approval_decision` is a small helper (decode `ApprovalResponsePayload`
  from the Operation's `PayloadEnvelope.payload`, return the `decision`).
- The idempotent-retry path (exact-retry of a terminal approval) still works:
  `winning_response == Some(operation)` exempts the retry (added in the
  elicitation-response-contract fix); the decision decode is not re-run on a
  dedup hit.

**Acceptance Criteria**:
- [ ] `validate_response_payload` rejects: approval with unspecified decision;
      approval with a reserved decision (100-103); approval with wrong
      content_type; approval against a non-approval contract (kind mismatch,
      existing).
- [ ] `validate_response_payload` accepts: APPROVED; DENIED.
- [ ] `terminalize_slot`: Completed APPROVED → `Answered`; Completed DENIED →
      `Declined`; Completed question response → `Answered` (unchanged);
      non-Completed terminal → slot stays pending (unchanged).
- [ ] The deferred comment at `elicitation.rs:301` is removed; no
      `Rejected`→`Declined` mapping exists.
- [ ] Idempotent retry of a terminal approval response returns the existing
      command (not `validation_failed`) — regression test.
- [ ] `cargo test --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` pass (with `CARGO_HOME=/home/agent/projects/patchbay/.cargo-cache`).

### Unit 3: Pi-adapter delivery of the approval decision

**File**: `pi-adapter/src/delivery.ts` (the `DeliveryTranslator` arm),
`pi-adapter/src/pi_session.ts` (the `ApprovalHandler` resolution path)
**Story**: `story-approval-response-adapter-delivery`

Wire the `APPROVAL_RESPONSE` arm in `DeliveryTranslator.deliver` (currently
`unsupported_command`). On delivery, decode the `ApprovalResponsePayload`
decision and resolve the pending approval:

```typescript
case OperationKind.APPROVAL_RESPONSE: {
  const payload = decodeApprovalPayload(operation);  // content_type PROTOBUF + decode
  const session = entry.session;
  if (payload.decision === ApprovalDecision.APPROVED) {
    await session.resolveApproval(operation, /*approved*/ true);
  } else if (payload.decision === ApprovalDecision.DENIED) {
    await session.resolveApproval(operation, /*approved*/ false);
  } else {
    throw new UnsupportedCommandError(`approval decision ${payload.decision} not deliverable in v0.1.0`);
  }
  return {};
}
```

The `ApprovalHandler` in `pi_session.ts` (today auto-approves via `() => true`)
gains a real resolution path: when an approval Elicitation is pending and a
DENIED response arrives, the handler returns the denial (the tool call is
blocked with "denied by operator"). `ELICITATION_RESPONSE` (question side) stays
`unsupported_command` — the question-side producer is a separate follow-on.

**Implementation Notes**:
- The adapter does NOT open approval Elicitations today (no `OpenElicitation`
  RPC in `adapter_control.proto`). This unit wires only the *response delivery*
  (the adapter receives an APPROVAL_RESPONSE Operation and resolves the pending
  approval). The producer side (adapter opening an approval Elicitation when a
  tool call needs a gate) is a separate follow-on — the cockpit tests against
  vectors + a fake transport, same as the question side.
- The `ApprovalHandler` auto-approve default stays as a fallback for tool calls
  arriving with no open approval gate (unchanged behavior); this unit adds the
  Elicitation-driven resolution path.
- The adapter reports the delivery outcome: APPROVED → the tool runs →
  `Completed`; DENIED → the tool is blocked → the response Operation itself
  still reaches `Completed` (it delivered a valid decision) — the *tool* is
  blocked, not the response. (The response Operation's terminal state is
  `Completed` either way; the Elicitation terminal is what differs.)

**Acceptance Criteria**:
- [ ] `DeliveryTranslator.deliver` handles `APPROVAL_RESPONSE` (no longer
      `unsupported_command`): decodes the decision, resolves the approval.
- [ ] `ELICITATION_RESPONSE` stays `unsupported_command` (question-side producer
      deferred).
- [ ] DENIED resolution blocks the pending tool call; APPROVED allows it.
- [ ] Reserved decisions (100-103) reject as `unsupported_command` at delivery.
- [ ] `pi-adapter` builds + its tests pass (`npm run build && npm test`).

### Unit 4: Conformance vectors

**File**: `contracts/vectors/approval-response-approved.json`,
`contracts/vectors/approval-response-denied.json`,
`contracts/vectors/approval-response-invalid-unspecified-decision.json`,
`contracts/vectors/approval-response-invalid-reserved-decision.json`,
`contracts/vectors/approval-response-invalid-wrong-content-type.json`
**Story**: `story-approval-response-conformance-vectors`

Five conformance vectors pinning the two committed decisions + three rejection
classes. Acceptance vectors reference `ElicitationInvalidResponseRejected`
(exercise the valid side); rejection vectors reference `boundary-validation`.

- `approval-response-approved.json` — APPROVED → `Answered`.
- `approval-response-denied.json` — DENIED → `Declined` (the load-bearing
  vector: pins the decision-driven terminal mapping).
- `approval-response-invalid-unspecified-decision.json` — UNSPECIFIED →
  `validation_failed`.
- `approval-response-invalid-reserved-decision.json` — ALLOW_ONCE (reserved) →
  `validation_failed`.
- `approval-response-invalid-wrong-content-type.json` — JSON content_type →
  `validation_failed`.

**Implementation Notes**:
- `proto_fields_constrained` lists: `patchbay.ApprovalDecision`,
  `patchbay.ApprovalResponsePayload.decision`,
  `patchbay.Operation.payload`, `patchbay.PayloadEnvelope.content_type`.
- Vectors carry structurally-complete `response_operation` (sender, target_scope,
  idempotency_key) + canonical `SubmissionResult` expectations (per the
  Important-1 fix from the elicitation-response-contract review).
- Same limitation as the question side: `check-vectors` validates envelope +
  property-id but does not execute the core's validation against the input (no
  invariant checker registered). The Rust unit tests (Unit 2) are the
  executable check.

**Acceptance Criteria**:
- [ ] Five vectors exist, pass `check:vectors`.
- [ ] The DENIED→`Declined` vector's `expected_outcome` asserts the
      Elicitation terminalizes as `declined` (the decision-driven mapping).
- [ ] VERIFICATION.md traceability table lists the new vectors.

### Unit 5: Foundation-doc roll-forward

**File**: `docs/PROTOCOL.md`
**Story**: `story-approval-response-foundation-doc` (or folded into Unit 2's
commit if small)

Roll PROTOCOL.md forward (rolling-foundation — no "previously" prose):
- **Line 277** (`declined` def): reconcile "without satisfying it" — a DENIED
  approval *did* satisfy the slot (a valid decision was delivered). Reword so
  `declined` reads as "the operator answered with a declining decision" (a
  satisfied slot, negative valence), consistent with line 310.
- **Add the disambiguation** between operator `Declined` (ElicitationState — an
  answer) and machine `Rejected` (CommandState — the system refusing a
  command). Make explicit that `Rejected` never terminalizes an Elicitation.
- **Line 156** (approval-response row): confirm "Completion updates the
  Elicitation terminal (`answered` or `declined`)" reads correctly under the
  decision-driven resolution (Completion = `Completed`; the decision picks the
  terminal). Add a clause that the decision is decoded from the typed
  `ApprovalResponsePayload`.
- **Reserved seam** (line 312 area): add "surface-reject (operator surface
  signals it cannot handle an elicitation)" to the reserved-seams list, noting
  it is distinct from operator approve/decline and from machine rejection.

**Acceptance Criteria**:
- [ ] PROTOCOL:277 no longer contradicts PROTOCOL:310 for DENIED approvals.
- [ ] The operator-`Declined` vs machine-`Rejected` disambiguation is explicit.
- [ ] The surface-reject reserved seam is named.

## Implementation Order

1. **Unit 1** (proto message) — the types everything depends on. `buf generate`
   + `cargo build` regen.
2. **Unit 2** (core validation + terminal mapping) — depends on Unit 1. The
   load-bearing unit.
3. **Unit 5** (foundation-doc roll-forward) — depends on Unit 2's semantics
   being settled; can parallelize with Units 3/4.
4. **Unit 3** (pi-adapter delivery) — depends on Units 1 + 2.
5. **Unit 4** (conformance vectors) — depends on Unit 1's field names.

Units 1 → 2 → (3 ∥ 4 ∥ 5). One implementation agent carries all five as
checkpoints.

## Simplification

- The deferred comment at `elicitation.rs:301` is **removed** (resolved, not
  deferred). No dead "TODO" left.
- The `ApprovalHandler` auto-approve default is retained as a fallback (not
  deleted) — tool calls with no open approval gate still auto-approve.
- No `[refactor]`/`[cleanup]` children — the slot layer is extended with a
  kind-gated decode, not restructured.

## Testing

- **Interface tests (Unit 2, load-bearing)** — `validate_response_payload` +
  `terminalize_slot` table-driven tests: every accept/reject branch (APPROVED,
  DENIED, unspecified, reserved, wrong content_type, kind mismatch) + every
  terminal mapping (Completed APPROVED→Answered, Completed DENIED→Declined,
  Completed question→Answered, non-Completed→pending). The DENIED→Declined
  test is the load-bearing assertion of the decision-driven design.
- **Regression test** — idempotent retry of a terminal approval response
  returns the existing command (not `validation_failed`); the decision decode
  is not re-run on a dedup hit.
- **Adapter tests (Unit 3)** — APPROVED allows the tool; DENIED blocks it;
  reserved decisions reject as `unsupported_command`.
- **Conformance vectors (Unit 4)** — five vectors pin the wire shapes.
- **Test removal**: none.

## Risks

- **Slot layer gains payload awareness (kind-gated).** The question side's slot
  layer is fully payload-opaque; the approval side decodes the decision on a
  `Completed` approval-response transition. This is bounded (one kind, one
  state, one field) but it is a real architectural shift. Mitigation: the
  decode is kind-gated and fails-closed (a corrupt/undecodable payload is a
  `CorruptRecord` error, not a silent wrong terminal). If a future contract
  needs decision-driven terminals for question responses too, the decode
  generalizes — but that's not v0.1.0.
- **`submit` signature unchanged.** Unlike the question side (which added a new
  port), this feature reuses `ElicitationContractLookup`/`ActiveElicitation`
  (already wired). No signature widening — lower test-diff risk than the
  question side.
- **Adapter producer side deferred.** The pi-adapter does not open approval
  Elicitations today (no `OpenElicitation` RPC). This feature wires only
  response delivery. The cockpit tests against vectors + a fake transport; a
  live producer is a follow-on (same posture as the question side).
- **Surface-reject deferred.** An unrenderable elicitation stays `pending`
  until timeout/withdraw in v0.1.0. Recorded as a reserved seam; not a
  regression (status quo), but a known v0.1.0 limitation.
- **`check:drift`** reports the pre-existing adapter-proto divergence
  regardless of correctness; not run in CI. TS build is the real check.

## Implementation summary

All five child-story checkpoints completed in dependency order:

1. `ApprovalDecision` and `ApprovalResponsePayload` landed in the proto with regenerated canonical Rust and TypeScript bindings.
2. Core boundary validation now accepts only APPROVED/DENIED, rejects unspecified/reserved/wrong-content/kind-mismatch inputs, and the kind-gated slot projection maps completed APPROVED→`Answered` and completed DENIED→`Declined` while machine rejection leaves the slot open.
3. The Pi adapter decodes approval decisions and resolves a pending approval gate; APPROVED allows and DENIED blocks the tool, reserved decisions remain `unsupported_command`, and question-response delivery remains deferred.
4. Five approval-response vectors bring the suite to 24 and explicitly pin the load-bearing DENIED→`Declined` outcome; `docs/VERIFICATION.md` traceability was regenerated.
5. `docs/PROTOCOL.md` now describes decision-driven completion, distinguishes operator decline from machine rejection, and names the surface-reject reserved seam.

Child commits:
- `3301432` — `implement: story-approval-response-proto-message`
- `6464c59` — `implement: story-approval-response-core-validation`
- `761f200` — `implement: story-approval-response-adapter-delivery`
- `f2c6ac5` — `implement: story-approval-response-conformance-vectors`
- `bd0431f` — `implement: story-approval-response-foundation-doc`

Integrated verification passed:
- `CARGO_HOME=/home/agent/projects/patchbay/.cargo-cache cargo build --workspace --all-targets`
- `CARGO_HOME=/home/agent/projects/patchbay/.cargo-cache cargo test --workspace`
- `CARGO_HOME=/home/agent/projects/patchbay/.cargo-cache cargo clippy --workspace --all-targets -- -D warnings`
- `cd contracts/ts && npm run build && npm run check:vectors` (24 vectors)
- `cd pi-adapter && npm run build && npm test` (6 tests)

Execution capability was direct inline feature ownership: shared context across the five ordered checkpoints reduced handoff risk. Effective review weight is standard (project default); the feature is now at `stage: review` for the caller's review lane. The known pre-existing `check:drift` generator divergence was not run, per the feature contract. No unrelated files were changed.

## Review record

**Effective weight:** standard (project default). **Pass count:** 1. **Reviewer:** fresh-context `openai-codex/gpt-5.6-sol` (different model class from the umans orchestrator — satisfies cross-model advisory review). **Closure:** receiver-confirmed blockers fixed + verified; advanced `review → done` without a second pass (standard policy).

**Verdict:** Approve (after fixes).

The reviewer verified the load-bearing behavior is genuinely earned, not asserted: the DENIED→`Declined` test uses serialized generated `DENIED` input with a hard-coded `Declined` oracle — changing the mapping, swapping decisions, or mapping machine rejection to decline would fail it. Core correctness confirmed: content_type checked before decode; missing/corrupt/unspecified/reserved/unknown/kind-mismatched payloads fail closed; only `ApprovalResponse + Completed` decodes the decision; question responses stay payload-opaque; `Rejected` does not terminalize; corrupt terminal payloads return `CorruptRecord` without mutating the slot; exact terminal approval retry clears validation and composes with the existing pipeline dedup regression.

**Findings adjudicated:**

- **Blocker (fixed): PROTOCOL.md approval row + extension-seams registry drift.** The `approval` contract-kind row (`docs/PROTOCOL.md:332`) presented all six decisions (allow/deny/allow-once/always/policy-amend/modified-input) as committed v0.1.0, contradicting the implemented binary-only contract (APPROVED/DENIED committed; the four richer decisions reserved). The consolidated extension-seams registry also omitted the four reserved `ApprovalDecision` values and the surface-reject seam despite claiming to be the single view of what v0.1.0 leaves open. **Fix:** rewrote the `approval` row to distinguish committed binary decisions from the four reserved; added the reserved approval decisions + surface-reject to both the "Reserved extension seams" prose summary and the extension-seams-registry table. Receiver-confirmed — a genuine foundation-doc contradiction (assertion drift, not omission).

- **Important (fixed): stale test comment retained the rejected conflation.** `core/tests/acceptance_elicitation.rs:311` said "Mapping denial/Rejected to Declined is a v0.x response-contract concern" — now false and directly contradicting the feature's safety decision. **Fix:** replaced with the settled rule (operator denial is a completed typed `DENIED` decision → `Declined`; `Rejected`/`Failed` response Operations never terminalize the slot).

- **Rejected proposals:** (a) require a promoted vector executor in this feature — the vector checker is intentionally envelope/traceability-only and that limitation is documented; the Rust mapping/validation tests are the executable evidence and are not self-defining; (b) require the Pi adapter to open approval Elicitations — producer-side opening is explicitly out of scope; (c) require `Rejected`→`Declined` — would recreate the state-machine conflation the feature correctly removes.

**Post-fix verification:** `cargo test --workspace` (all pass), `npm run check:vectors` (24 vectors) — both green. The fixes are doc + comment only; no behavior change.

**Review-fix commit:** `review-fix: feature-v0-approval-response-contract` (this commit).
