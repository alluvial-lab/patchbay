---
id: feature-v0-elicitation-response-contract
kind: feature
stage: review
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

## Design decisions

Resolved during the feature-design pass; pinned so implementation does not
re-litigate them.

- **D1 — Core-validation boundary: (B) contracts + core boundary validation.**
  (Captured at scope as a strategic decision; restated here for design
  completeness.) The core's acceptance pipeline validates the typed response
  payload against the active `response_contract` at submission, rejecting
  malformed responses with `validation_failed` before the durable append (Fail
  Fast). The cockpit is a browser client and cannot be the protocol's
  enforcer. A typed wire contract the core does not validate is exactly the
  "claimed-but-not-enforced conformance surface" the prior session's
  component-layer arc existed to prevent.
- **D2 — `select-many` is reserved for v0.1.0; payload is single-answer.**
  `ElicitationResponsePayload.selected_option_id` is singular. A `select-many`
  ui_hint on a `question` contract is accepted at submission (the hint is
  non-authoritative, open, reserved-for-UI) but a response carrying multiple
  selections has no typed home — it rejects as `validation_failed` until a
  future promotion adds `repeated selected_option_ids`. This matches the
  cockpit's locked select-one-only mock and EC3's settled reasoning (grouping
  is presentation, the contract stays single-answer). Promotion is an
  additive field, non-breaking for single-answer responses. Operator chose (A)
  over (B) committing `repeated selected_option_ids` now, which would commit a
  wire shape no v0.1.0 surface produces.
- **D3 — EC3 grouping is presentation, not proto.** One
  `ElicitationResponsePayload` per response Operation, correlated to one
  `ElicitationId`. No multi-answer contract (one Elicitation carrying multiple
  questions) — that is the reserved "multi-answer accumulation" seam
  (`docs/PROTOCOL.md:312`). The grouped card is the cockpit rendering N
  independent single-answer Elicitations as one visual card.
- **D4 — EC1 free-text is a `ui_hint` within the committed `question`
  contract_kind, not a contract-kind promotion.** EC2 answer-and is a
  response-payload shape, not a new contract_kind. No new
  `ResponseContractKind` values are introduced. (Both captured at scope;
  restated for completeness.)
- **D5 — Validation is contract-driven, not kind-driven.** The validation
  rule depends on the *active contract* the response targets, not the
  OperationKind alone. A response to an `approval` contract carries an
  `ApprovalResponsePayload` (existing, unchanged); a response to a `question`
  contract carries an `ElicitationResponsePayload`. The core looks up the
  Elicitation's `response_contract` to decide which payload shape to decode
  and which validation rules to apply. OperationKind `ELICITATION_RESPONSE`
  targets `question`; `APPROVAL_RESPONSE` targets `approval`.
- **D6 — Contract lookup is a new acceptance port, projected from the log.**
  The core gains an `ElicitationContractLookup` port (mirroring
  `CommandStateLookup`). The server's `ProjectionState` holds an extended
  `ElicitationSlotLayer` (now also storing the `response_contract` and
  `expected_responder_actor`) under `submit_gate`, and `catch_up` folds events
  into it before each submit — the same pattern as the other three projections.
  This keeps validation against an in-memory projection reconciled under the
  submit gate, so a contract lookup never races a concurrent append. (See
  Unit 3 for the race-free reasoning and the fold-lag property.)
- **D7 — Validation ordering: after structural validation, before target
  resolution.** Response-content validation is inserted into `submit` between
  `validate_operation` (structural) and the grant check / target resolution,
  consistent with Fail Fast at the boundary. A structurally-invalid or
  payload-malformed response rejects as `validation_failed` with no grant-check
  or storage side effects — matching the existing
  `unknown_and_reserved_operation_kinds_reject_before_grant` invariant.
- **D8 — `schema_ref` clarification.** For committed `question`/`approval`
  contract kinds, the typed messages (`QuestionContract` / the existing
  approval payload) are authoritative and `schema_ref` is ignored by v0.1.0
  validation. `schema_ref` becomes load-bearing only for the reserved
  `structured_schema` contract kind (not validatable in v0). This is documented
  in PROTOCOL.md, not a code change.

## Architectural choice

**Extend the existing port-and-projection pattern rather than inlining a
lookup into the pipeline.** The acceptance pipeline is already generic over
`Storage`, `GrantCheck`, `TargetResolver`, and `CommandStateLookup`; each is a
thin port trait, and the server's `ProjectionState` wraps a projection behind
a `Locked*` adapter and folds events in `catch_up`. Adding response-content
validation as a fifth concern follows the same seam: a new
`ElicitationContractLookup` port in `acceptance::ports`, an extended
`ElicitationSlotLayer` that stores the contract, a `Locked*` wrapper in the
server, and a fold step in `catch_up`.

This keeps the core domain logic adapter-neutral (no direct DB access — the
port is the boundary), follows Single-Source-of-Truth (the contract comes from
the durable Elicitation event, projected), and is the lowest-surprise choice
for reviewers familiar with the existing four ports. The alternative — reading
the Elicitation out of storage inline during `submit` — would be the first
ad-hoc storage read inside the pipeline and would break the clean port
boundary the acceptance module maintains.

## Implementation Units

### Unit 1: Typed proto messages (contracts layer)

**File**: `contracts/proto/patchbay/elicitations.proto`
**Story**: `story-elicitation-response-proto-messages`

Add the typed messages that a `question` contract carries and that an
`elicitation-response` Operation carries in its `Operation.payload`.

```proto
// A selectable option within a question contract.
message ResponseOption {
  string option_id = 1;       // stable id within this contract; the response references it
  string label = 2;           // human-readable label the surface renders
}

// The typed contract body for a `question` contract_kind. Carried by
// `ResponseContract.question`. EC1: `allow_free_text` permits a free-text
// response (the "or type your own answer" option). EC3 grouping is
// presentation, not proto: each Elicitation stays single-answer, so this
// contract is single-answer. `select-many` is a reserved ui_hint with no
// typed multi-select response shape in v0.1.0 (D2).
message QuestionContract {
  repeated ResponseOption options = 1;
  bool allow_free_text = 2;
}

// The typed response payload an `ELICITATION_RESPONSE` Operation carries in
// its `Operation.payload`. Exactly one of `selected_option_id` / `free_text`
// is the primary answer; `clarification` is the optional EC2 "answer-and"
// free-text supplement (never the primary answer on its own). Single-answer
// only in v0.1.0 (D2/D3).
message ElicitationResponsePayload {
  string selected_option_id = 1;  // an option_id from the contract's options; empty if free-text
  string free_text = 2;           // the free-text answer; empty if an option was selected
  string clarification = 3;      // optional EC2 "and..." supplement
}
```

`ResponseContract` gains a `oneof` body so the contract is typed per
`contract_kind`:

```proto
message ResponseContract {
  ResponseContractKind contract_kind = 1;
  string schema_ref = 2;
  repeated string ui_hints = 3;
  TimeoutPolicy timeout_policy = 4;
  InvalidResponsePolicy invalid_response_policy = 5;
  ResponderPolicy responder_policy = 6;
  ResponseSensitivity sensitivity = 7;
  // Typed contract body. Present when contract_kind is a committed v0.1.0
  // kind with a typed shape (question). Approval carries no typed body in
  // v0.1.0 (binary; the response is the existing approval payload).
  oneof contract_body {
    QuestionContract question = 8;
  }
}
```

`Operation.payload` (`PayloadEnvelope`) is unchanged in shape — it already
carries `bytes payload` + `content_type` + `schema_ref`. For an
`ELICITATION_RESPONSE`, `content_type` is `PAYLOAD_CONTENT_TYPE_PROTOBUF` and
the bytes are a serialized `ElicitationResponsePayload`. (The cockpit's TS
bindings decode it the same way the core decodes an `Operation`.)

**Implementation Notes**:
- Field numbers: `question = 8` on `ResponseContract` is the next free tag
  (1–7 used). `ResponseOption` and `QuestionContract` and
  `ElicitationResponsePayload` are new messages — fresh field numbers from 1.
- No `repeated selected_option_ids` anywhere — D2. The multi-answer reserved
  seam is named in PROTOCOL.md, not in proto.
- `oneof contract_body` is forward-compatible: adding a typed body for a
  future promoted contract kind (e.g. `structured_schema`) is a new `oneof`
  arm, non-breaking. A v0.1.0 reader ignores unknown `oneof` arms per proto3.
- `buf generate` regenerates `contracts/rust/src/gen/` and
  `contracts/ts/src/gen/`. The committed Rust bindings drift vs the
  prost generator (pre-existing `check:drift` gap) — that gap is documented and
  not this feature's concern, but the generated TS must build clean.

**Acceptance Criteria**:
- [ ] `QuestionContract`, `ResponseOption`, `ElicitationResponsePayload`
      messages exist in `elicitations.proto` with the fields above
- [ ] `ResponseContract.question` `oneof` arm exists at field 8
- [ ] `buf generate` regenerates Rust + TS bindings with no generator errors
- [ ] `contracts/ts` builds clean (`npm run build`); generated TS exports the
      new message types
- [ ] No `repeated selected_option_ids` field exists (D2 reserved-seam check)
- [ ] `check:vectors` still passes (envelope + property-registry integrity)

### Unit 2: Core boundary validation (the Fail-Fast check)

**File**: `core/src/acceptance/elicitation_response.rs` (new),
`core/src/acceptance/pipeline.rs` (validation insertion),
`core/src/acceptance/ports.rs` (new port), `core/src/acceptance/mod.rs` (re-export)
**Story**: `story-elicitation-response-core-validation`

The validation function and the new lookup port.

```rust
// core/src/acceptance/ports.rs — the new lookup port
use patchbay_contracts::patchbay::{ElicitationId, ResponseContract};

/// The elicitation-contract seam used to validate a response Operation's
/// payload against the active contract before durable acceptance.
/// Implementations perform a side-effect-free read against an in-memory
/// projection reconciled under the submit gate (never a racing storage read).
pub trait ElicitationContractLookup: Send + Sync {
    fn active_contract(
        &self,
        elicitation_id: &ElicitationId,
    ) -> impl std::future::Future<Output = Option<ActiveElicitation>> + Send;
}

/// The contract + responder context a response is validated against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveElicitation {
    pub contract: ResponseContract,
    pub is_terminal: bool, // already answered/declined/expired/... — reject retries cleanly
}
```

```rust
// core/src/acceptance/elicitation_response.rs — the Fail-Fast check
use patchbay_contracts::patchbay::{
    ElicitationResponsePayload, Operation, OperationKind, ResponseContractKind,
    ResponseOption, TypedCorrelation, typed_correlation,
};
use prost::Message;

/// Validate an elicitation-response (or approval-response) Operation's payload
/// against the active contract. Returns `Ok(())` if the payload is
/// well-formed and contract-satisfying, or `Err(diagnostic)` for a
/// `validation_failed` rejection. A `None` active elicitation (unknown id,
/// or wrong domain) is a validation failure, not an authorization one.
pub fn validate_response_payload(
    operation: &Operation,
    active: Option<&ActiveElicitation>,
) -> Result<(), String> {
    // 1. The Operation must carry an ElicitationId typed correlation.
    let elicitation_id = operation
        .correlations
        .iter()
        .find_map(|c| match &c.r#ref {
            Some(typed_correlation::Ref::ElicitationId(id)) => Some(id),
            _ => None,
        })
        .ok_or_else(|| "elicitation-response Operation has no ElicitationId correlation".to_owned())?;

    let active = active.ok_or_else(|| {
        format!("no active elicitation for {elicitation_id:?} (unknown or wrong domain)")
    })?;

    if active.is_terminal {
        return Err(format!("elicitation {elicitation_id:?} is already terminal"));
    }

    // 2. Kind/contract-kind correspondence (D5).
    let contract = &active.contract;
    match (operation_kind(operation), contract.contract_kind()) {
        (OperationKind::ElicitationResponse, ResponseContractKind::Question) => {}
        (OperationKind::ApprovalResponse, ResponseContractKind::Approval) => {
            // Approval is binary; no typed body to validate in v0.1.0.
            return Ok(());
        }
        (kind, ck) => {
            return Err(format!(
                "response kind {kind:?} does not match contract kind {ck:?}"
            ));
        }
    }

    // 3. Decode the typed question payload and validate against QuestionContract.
    let question = contract.question.as_ref().ok_or_else(|| {
        "question contract is missing its typed QuestionContract body".to_owned()
    })?;
    let payload = decode_response_payload(operation)?;

    // Exactly one primary answer: selected_option_id XOR free_text (EC1).
    let has_option = !payload.selected_option_id.is_empty();
    let has_free_text = !payload.free_text.is_empty();
    match (has_option, has_free_text) {
        (true, true) => {
            return Err("response carries both a selected_option_id and free_text; exactly one primary answer is allowed".to_owned());
        }
        (false, false) => {
            return Err("response carries neither a selected_option_id nor free_text; exactly one primary answer is required".to_owned());
        }
        _ => {}
    }

    // EC1: a selected option must be one the contract declared.
    if has_option {
        let valid = question.options.iter().any(|o| o.option_id == payload.selected_option_id);
        if !valid {
            return Err(format!(
                "selected_option_id {:?} is not one of the contract's options",
                payload.selected_option_id
            ));
        }
    }

    // EC1: free-text requires allow_free_text on the contract.
    if has_free_text && !question.allow_free_text {
        return Err("response carries free_text but the contract does not allow_free_text".to_owned());
    }

    // EC2: clarification is always optional and supplementary; no further check.
    Ok(())
}
```

The pipeline insertion (D7) — in `submit`, after `validate_operation` returns
`Ok(validated)` and before the grant check:

```rust
// core/src/acceptance/pipeline.rs — inside submit, after validate_operation
if validated.operation_kind == OperationKind::ElicitationResponse
    || validated.operation_kind == OperationKind::ApprovalResponse
{
    let active = contract_lookup.active_contract(&elicitation_id_from(&operation)?).await;
    if let Err(diagnostic) = validate_response_payload(&operation, active.as_ref()) {
        return Ok(rejected_result(
            Some(validated.command_id.clone()),
            FailureCode::ValidationFailed,
            diagnostic,
        ));
    }
}
```

`submit` gains a `C: ElicitationContractLookup` generic parameter (between
`R` and `L` in the existing `<S, G, R, L>` to preserve the storage→grant→target→
state ordering, or appended; the parameter order is conventional, not
load-bearing). The call site in `server/src/service.rs` passes the new
`LockedElicitationContractLookup`.

**Implementation Notes**:
- `operation_kind` and `elicitation_id_from` are small local helpers
  (decode `Operation.kind` to the enum; scan `correlations` for the
  `ElicitationId` ref). The latter is reused from the validation function.
- The check runs for both `ElicitationResponse` and `ApprovalResponse` kinds
  so the kind/contract-kind correspondence (D5) is enforced for approval too
  (an approval response against a question contract rejects). Approval's typed
  body is a no-op pass today (binary; the existing approval payload is
  unchanged).
- A terminal elicitation rejects as `validation_failed` with a clear diagnostic
  rather than relying on the idempotent-retry path — a *different* response op
  id targeting an already-answered elicitation is not a retry (different
  idempotency key / command id) and must not be treated as one. The
  first-answer-wins structural property is unchanged (the slot layer still
  owns terminalization); this is an early rejection, not a terminalization
  rule change.
- `ResponseContractKind` is the generated enum; `.contract_kind()` is the
  prost accessor (field 1).

**Acceptance Criteria**:
- [ ] `ElicitationContractLookup` port trait + `ActiveElicitation` struct
      exist in `acceptance::ports` and are re-exported from
      `acceptance::mod`
- [ ] `validate_response_payload` rejects: missing ElicitationId correlation;
      unknown/wrong-domain elicitation (None active); already-terminal
      elicitation; kind/contract-kind mismatch (e.g. approval-response against
      a question contract); question contract missing its `QuestionContract`
      body; both selected_option_id and free_text set; neither set;
      selected_option_id not in the contract's options; free_text when
      `allow_free_text` is false
- [ ] `validate_response_payload` accepts: a valid selected_option_id; a
      valid free_text when `allow_free_text`; a selected option + EC2
      clarification; an approval response against an approval contract
- [ ] A malformed response Operation rejects as `validation_failed` with no
      grant-check call, no target resolution, and no durable append (Fail
      Fast) — verified by the existing
      `*_reject_before_grant_without_durable_state` test pattern
- [ ] A well-formed response Operation accepts normally (the validation is
      a pure precondition; it does not change the accepted path)
- [ ] `cargo test --workspace` and `cargo clippy --workspace -- -D warnings`
      pass

### Unit 3: ElicitationSlotLayer extension + server projection wiring

**File**: `core/src/acceptance/elicitation.rs` (extend `ElicitationRecord` +
`observe_elicitation`), `server/src/state.rs` (new projection + `Locked*`
wrapper + `catch_up` fold), `server/src/service.rs` (pass the new port into
`submit`)
**Story**: `story-elicitation-response-projection-wiring`

Extend the slot layer to store the contract so the new port can serve it, and
wire it as a fifth projection in the server.

```rust
// core/src/acceptance/elicitation.rs — extended ElicitationRecord
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElicitationRecord {
    pub elicitation_id: ElicitationId,
    pub state: ElicitationState,
    pub terminal_lsn: Option<u64>,
    /// The response_contract the Elicitation opened with. Served to the
    /// acceptance pipeline via ElicitationContractLookup so a response can be
    /// validated against the active contract. None only if the opening event
    /// was malformed in a way observe rejects (so it never reaches the slot).
    pub contract: Option<ResponseContract>,
    pub expected_responder_actor: Option<ActorId>,
}
```

`observe_elicitation` already decodes the full `Elicitation` message; it now
also stores `elicitation.response_contract` and `elicitation.expected_responder_actor`
into the record. `rebuild_slots_from_log` is unchanged (it feeds `observe`).

```rust
// server/src/state.rs — the new projection + wrapper (mirrors LockedCommandStateLookup)
pub struct LockedElicitationContractLookup {
    inner: Mutex<ElicitationSlotLayer>,
}

impl LockedElicitationContractLookup {
    pub fn new() -> Self { Self { inner: Mutex::new(ElicitationSlotLayer::new()) } }
    async fn observe(&self, event: &RecordedEvent) -> Result<(), AcceptanceError> {
        self.inner.lock().await.observe(event)
    }
}

impl ElicitationContractLookup for LockedElicitationContractLookup {
    async fn active_contract(&self, elicitation_id: &ElicitationId) -> Option<ActiveElicitation> {
        let layer = self.inner.lock().await;
        layer.get_slot(elicitation_id).map(|rec| ActiveElicitation {
            contract: rec.contract.clone()?,
            is_terminal: is_terminal_state(rec.state),
        })
    }
}
```

`ProjectionState` gains an `elicitation_slots: LockedElicitationContractLookup`
field; `rebuild` folds every event through it (like `commands.apply`);
`catch_up` folds each new event through it (like `state_lookup.apply`).
`service.rs::submit` passes `self.state.elicitation_contract_lookup()` as the
new `submit` argument.

**Implementation Notes**:
- **Fold-lag race-free reasoning.** `submit` holds `submit_guard` for the
  whole validate→append→catch-up sequence (it already does). `catch_up` folds
  all events with `LSN > last_applied` into every projection *before* the
  validation read, so the contract lookup sees every event durably committed
  before this submit — including the Elicitation opening event the response
  targets. The contract lookup therefore never reads a stale projection
  relative to the durable log. This is the same invariant the existing
  `state_lookup` relies on for deduplicated-retry state lookup.
- The `ElicitationSlotLayer` is already event-log-driven and owns no storage;
  extending it to hold the contract keeps that property. The server-side
  projection is a cache of the durable Elicitation event, never authority.
- `is_terminal_state` already exists in `core/src/acceptance/state.rs`.
- The `Locked*` wrappers each hold a `Mutex` and release before the next port
  is called (the existing "no nested projection locks" discipline). The new
  wrapper follows the same rule.

**Acceptance Criteria**:
- [ ] `ElicitationRecord` stores `contract` + `expected_responder_actor`;
      `observe_elicitation` populates them from the opening event
- [ ] `LockedElicitationContractLookup` implements `ElicitationContractLookup`
- [ ] `ProjectionState` holds the new projection; `rebuild` + `catch_up` fold
      events into it alongside the existing three projections
- [ ] `service.rs::submit` passes the new lookup into `acceptance::submit`
- [ ] A response submitted after the Elicitation's opening event was appended
      (and caught up) validates against the real contract; a response
      submitted referencing an Elicitation whose opening event is not yet in
      the projection rejects as `validation_failed` (unknown elicitation) —
      this is correct Fail-Fast behavior, not a bug
- [ ] Existing acceptance/elicitation tests still pass after the
      `ElicitationRecord` field additions

### Unit 4: Conformance vectors (the earned-not-asserted check)

**File**: `contracts/vectors/elicitation-response-question-select-one.json`,
`contracts/vectors/elicitation-response-question-free-text.json`,
`contracts/vectors/elicitation-response-question-answer-and.json`,
`contracts/vectors/elicitation-response-invalid-mismatched-option.json`,
`contracts/vectors/elicitation-response-invalid-free-text-disallowed.json`,
`contracts/vectors/elicitation-response-invalid-both-primary-answers.json`,
`contracts/vectors/elicitation-response-invalid-terminal-elicitation.json`
**Story**: `story-elicitation-response-conformance-vectors`

Conformance vectors pinning the three committed response shapes and the four
classes of rejection. These make the validation contract executable, not
prose — the lesson from the prior session: a conformance surface is a
liability until it is machine-checkable.

Each vector uses the existing envelope (`vector_id`, `property_id`,
`promotion_status`, `proto_fields_constrained`, `description`, `input`,
`expected_outcome`, `invariant_check`). The three acceptance vectors reference
the stated-normative `ElicitationInvalidResponseRejected` property (already in
the `check-vectors` registry); the four rejection vectors reference the
descriptive `boundary-validation` id (draft-only, per the registry) since the
fine-grained rejection rules are boundary validation, not a named formal
property.

`property_id` choices:
- Acceptance vectors (select-one, free-text, answer-and):
  `ElicitationInvalidResponseRejected` — these exercise the *valid* side of
  that property (a valid response is accepted, not rejected).
- Rejection vectors (mismatched option, free-text-disallowed,
  both-primary-answers, terminal): `boundary-validation` (descriptive draft).

**Implementation Notes**:
- `proto_fields_constrained` lists the new fields:
  `patchbay.ResponseContract.question`, `patchbay.QuestionContract.options`,
  `patchbay.QuestionContract.allow_free_text`, `patchbay.ResponseOption.option_id`,
  `patchbay.ElicitationResponsePayload.selected_option_id`,
  `patchbay.ElicitationResponsePayload.free_text`,
  `patchbay.ElicitationResponsePayload.clarification`.
- `input` carries a full `Elicitation` (the opening, with its
  `response_contract.question`) + the response `Operation` with a serialized
  `ElicitationResponsePayload`. `expected_outcome` states
  `accepted`/`rejected` + the `failure_code` (`validation_failed` for
  rejections).
- The `check-vectors` script validates envelope + property-id registry but
  does **not** execute the core's validation against the vector input (no
  invariant-expectation checker is registered for these properties yet — see
  `INVARIANT_EXPECTATION_CHECKS` in `check-vectors.mjs`). The vectors are
  therefore *contract-specifying* (they pin the shape + property for review)
  but not yet *executable-against-the-core*. Registering an invariant checker
  that runs `validate_response_payload` against each vector's input and
  asserts the expected outcome is the natural promotion path; it is called
  out as a follow-on in Risks, not required for v0.1.0 acceptance of this
  feature (the Rust unit tests in Unit 2 are the executable check today).

**Acceptance Criteria**:
- [ ] Three acceptance vectors exist and pass `check:vectors` (valid
      select-one, valid free-text-with-allow_free_text, valid answer-and
      with clarification)
- [ ] Four rejection vectors exist and pass `check:vectors` (mismatched
      option_id; free_text when allow_free_text=false; both selected_option_id
      and free_text; response against a terminal elicitation)
- [ ] Each vector's `proto_fields_constrained` references real fields added
      in Unit 1
- [ ] The generated conformance-traceability table in `docs/VERIFICATION.md`
      (regenerated by `check:vectors`) lists the new vectors under
      `ElicitationInvalidResponseRejected` and `boundary-validation`
- [ ] CI's "Protocol conformance vectors" step passes with the new vectors

## Implementation Order

1. **Unit 1** (proto messages) — the types everything else depends on. Must
   land + `buf generate` before Units 2/3 compile against the new messages.
2. **Unit 2** (core validation) — depends on Unit 1's types. The Fail-Fast
   check + port trait. Compiles standalone against a fake
   `ElicitationContractLookup` in tests.
3. **Unit 3** (projection wiring) — depends on Units 1 + 2. Extends the
   slot layer, wires the server. The end-to-end validation path is live after
   this unit.
4. **Unit 4** (conformance vectors) — depends on Unit 1's field names. Can
   be authored in parallel with Unit 3 once Unit 1 lands; pinned last so the
   field names are stable.

Units 1 → (2 ∥ 4) → 3. The orchestrator baseline is one implementation agent
carrying all four stories as checkpoints; Unit 1 is the critical-path head.

## Simplification

- No existing code is deleted. The core's opaque-bytes handling in the slot
  layer's *terminalization* path is unchanged — validation is an additive
  boundary check that runs before the durable append; the slot layer still
  owns first-answer-wins terminalization exactly as the formal model specifies.
- `schema_ref` is clarified as non-authoritative for committed contract kinds
  (D8) — a documentation simplification, not a code removal.
- No `[refactor]`/`[cleanup]` child stories — the touched area is extended,
  not restructured. `validate_operation` stays as-is; the new check is a
  sibling function called from `submit`.

## Testing

- **Interface tests (Unit 2, the load-bearing check)** — `validate_response_payload`
  is a pure function over `(Operation, Option<ActiveElicitation>) -> Result`.
  Table-driven tests cover every accept/reject branch in the acceptance
  criteria. This is where the conformance claim is *earned*: each rejection
  rule is exercised by a failing-otherwise input. (Value: pins the
  contract; a mutation that loosens a rule fails a test.)
- **Regression tests (Unit 2/3)** — the existing
  `acceptance_elicitation.rs` first-answer-wins / terminal-race tests must
  still pass unchanged (the validation check does not alter terminalization).
  A new test asserts a malformed response rejects as `validation_failed`
  *before* grant check / target resolution / durable append, using the
  existing `*_reject_before_grant_without_durable_state` pattern. (Value:
  protects the Fail-Fast ordering invariant.)
- **Property test (Unit 3)** — the fold-lag invariant: after `catch_up` folds
  the opening event, `active_contract` returns the real contract; before it,
  it returns `None`. (Value: protects the race-free reasoning in D6.)
- **Conformance vectors (Unit 4)** — the seven vectors pin the wire shape for
  review and the `check:vectors` CI gate. (Value: makes the contract
  specifiable, not just implementable; promotion path to an executable
  invariant checker is documented in Risks.)
- **Test removal**: none. The existing
  `unknown_and_reserved_operation_kinds_reject_before_grant` test is extended
  (not removed) if it needs a non-response default operation — the new port
  is only consulted for response kinds.

## Risks

- **`submit` signature widening.** Adding a `C: ElicitationContractLookup`
  parameter touches every caller and every test that constructs `submit`. The
  existing tests use a fake `CommandStateLookup`; they need a fake
  `ElicitationContractLookup` (a `None`-returning stub for non-response
  tests). This is mechanical but broad — expect a wide diff in
  `core/tests/acceptance_pipeline.rs`. Mitigation: the stub is trivial
  (`async fn active_contract(&_, _) -> Option<ActiveElicitation> { None }`),
  and non-response tests are unaffected because the validation branch only
  runs for response kinds.
- **Invariant-expectation checker not registered.** The conformance vectors
  (Unit 4) specify the contract but `check-vectors` does not yet execute the
  core's validation against them — no checker is registered in
  `INVARIANT_EXPECTATION_CHECKS`. The Rust unit tests (Unit 2) are the
  executable check today. Registering a JS-side checker that calls into the
  core (or a TS port of the validation) is the promotion path to a
  fully-executable vector gate; it is a clean follow-on, not a v0.1.0 blocker.
  Flagged so the review does not mistake the vectors for an unenforced
  claim — they are spec-and-traceability, with the executable enforcement in
  Rust tests, and the promotion path documented.
- **Approval-response validation is a no-op pass today.** D5 enforces
  kind/contract-kind correspondence for approval too, but the approval typed
  body is unchanged (binary). If a future contract adds a typed approval
  body, the validation function gains an arm; the `oneof contract_body` is
  ready for it. Not a v0.1.0 risk — called out for the reviewer.
- **`check:drift` will report diffs.** A proto change regenerates the Rust
  bindings; the pre-existing `check:drift` gap means it reports drift vs the
  committed bindings regardless of correctness. This is documented and not
  run in CI. The generated TS must build clean (that *is* checked). Mitigation:
  verify `npm run build` in `contracts/ts` before committing.
- **Multi-answer reserved seam must stay reserved.** The proto must not
  introduce any field that carries multiple questions or multiple selected
  options in one Elicitation/response. D2/D3 + the Unit 1 acceptance
  criterion ("no `repeated selected_option_ids`") guard this. The
  extension-pressure classification records it; review should re-confirm.

## Implementation summary

All four implementation checkpoints are complete in dependency order:

1. `ResponseOption`, `QuestionContract`, and `ElicitationResponsePayload` were
   added to the proto, with `ResponseContract.question` as oneof field 8;
   TypeScript and prost-build Rust bindings were regenerated.
2. The core now exposes `ElicitationContractLookup`/`ActiveElicitation`, runs
   typed question and approval correspondence validation after structural
   validation and before grant checks, and has table-driven branch coverage
   plus a fail-fast no-side-effects regression.
3. Elicitation projection records retain the typed contract and expected actor;
   server rebuild/catch-up folds a locked contract lookup and passes it to
   submit. A fold-lag invariant test covers unknown-before-fold and
   contract-after-fold behavior.
4. Seven conformance vectors pin the three accepted response shapes and four
   rejection classes; `docs/VERIFICATION.md` now traces 19 vectors.

Child commits:
- `614890e` — `implement: story-elicitation-response-proto-messages`
- `17c6d5c` — `implement: story-elicitation-response-core-validation`
- `615a1fb` — `implement: story-elicitation-response-conformance-vectors`
- `ead73b6` — `implement: story-elicitation-response-projection-wiring`

## Final verification

- `CARGO_HOME=.cargo-cache cargo build --workspace --all-targets`: passed
- `CARGO_HOME=.cargo-cache cargo test --workspace`: passed (all workspace
  tests; 19 vector files are checked separately)
- `CARGO_HOME=.cargo-cache cargo clippy --workspace --all-targets -- -D warnings`:
  passed
- `cd contracts/ts && npm run build`: passed
- `cd contracts/ts && npm run check:vectors`: passed (19 vectors, 0 promoted,
  0 invariant expectation checks)

The known pre-existing `check:drift` divergence was not run, per the feature
and environment instructions. No `repeated selected_option_ids` field was
introduced. The generated protobuf `ResponseContract` is `PartialEq` rather
than `Eq` because its generated timestamp-containing fields are not `Eq`; the
projection and lookup preserve the generated type without changing semantics.

