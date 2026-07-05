## Session bank — 2026-07-05 (action-inventory implementation plan + state)

**This note is the reboot point for implementing
`feature-operator-presence-and-action-inventory`.** A fresh context can pick up
here without re-deriving the design or the plan.

## Where we are in the operator-requested sequence

The feature's workflow note specified:
**design → adversarial review of the design → implement (edit foundation docs)
→ deep adversarial review of the result.**

- ✅ Design (committed)
- ✅ Adversarial review of the design — 8 defects found (D1–D8)
- ✅ Amendment cycle 1 (D1–D8) — committed
- ✅ Re-review — 3 new defects found (N1–N3)
- ✅ Amendment cycle 2 (N1–N3) — committed
- ✅ Final re-review — **READY for implement** (no new defects, no contradictions)
- ⬜ **Implement (edit foundation docs)** ← next
- ⬜ Deep adversarial review of the result (parallel codex/gpt-5.5, 3 axes)

## The settled frame and decisions (do not re-open)

### Frame
**Operation / Observation / Elicitation** (+ Payload as content carried, not a
primitive). Actor-neutral vocabulary; `{sender, recipient}` are fields on all
primitives. The frame was validated by two fresh-context adversarial reviews
(banked at `.work/session-notes/2026-07-04-frame-adversarial-review.md` and
`2026-07-05-ooe-frame-review.md`).

### D1 + D4 — v0 Operations are operator-originated only
Non-operator Operation senders (agent→agent, Antigravity triggers, Codex service
Operations) are a **reserved seam**, not v0 behavior. This dissolves D1 and D4:
- D1 (id assignment): unchanged — command/message ids stay client-generated in
  the operator domain (`PROTOCOL.md:41-44`).
- D4 (sender verification): `CompoundIssuer` stays operator-session-shaped
  (web-server-as-principal + operator-actor). No changes to the checked property.
  Elicitation openers verified via the adapter's authenticated channel.

The actor-neutral language must not mask operator-centric v0 behavior. State
explicitly: "v0 Operations are operator-originated; actor-neutral sender
vocabulary is a reserved seam."

### D2 — reads use the full OperationState lifecycle
No fast-path `accepted → completed` (that contradicted `PROTOCOL.md:92-94`).
Reads use the standard transitions. A "no-lifecycle reads" optimization is a
reserved seam, promotable if polling volume warrants.

### D3 — spawn fleet authority (four sub-decisions + seam)
1. **Target scope = fleet-level.** Spawn grants authorize spawn across any
   adapter the operator can reach. Adapter-level available via existing
   `target scope` flexibility (no schema change).
2. **Descendant authority = spawned-session manifest.** Spawn's completion
   includes an auto-issued grant record for the spawned session (spawner =
   subject, spawned session = target). Explicit, visible/auditable. Builds
   infrastructure for future delegation (a `parent_grant_id` field, removed
   from v0 per `feature-design-grant-shape`, can reference the auto-issued
   grant). Does NOT re-open delegation — same actor, new target.
3. **Revocation = standard discipline.** Spawn grant and descendant grant
   revocable independently. No cascade in v0. Cascade available later as a
   query over grant provenance, no schema change.
4. **Idempotency = capability-manifest mechanism.** Spawn's idempotency
   strength declared in adapter capability manifest's `idempotency strength`
   field. Spawn likely `at-Patchbay-boundary` (Patchbay dedups the Operation
   record; adapter may create duplicate process on retry). Duplicate-process
   possibility is the adapter's problem to report via failure vocabulary.

**Joint-control seam (preserved + built):** cross-operator delegation over
spawned sessions requires the `parent_grant_id` field v0 omits; the auto-issued
descendant grant is the explicit record a future delegated grant can reference.
Cross-reference `feature-design-grant-shape`'s delegation removal.

### D5 — ElicitationId adapter-assigned; core doesn't open Elicitations
- ElicitationId is **adapter-assigned** (opener is always adapter/agent in v0).
  Core assigns an LSN at durable record (same as other events).
- **Core prompts are NOT Elicitations.** Lockdown, expired/revoked sessions,
  CSRF rejection are STATES imposed by the core, enforced by Operation
  rejection. Resolution is pre-protocol: operator-session establishment
  (login/re-auth) is outside the protocol's normative scope (per `PROTOCOL.md`'s
  operator-session definition). The protocol assumes a valid operator session
  exists; how that session is established is the control surface + web
  server's problem.
- The design must NOT imply re-authentication is an Operation or an Elicitation.

### D6 — agent-send reserved OperationKind
`agent-send` (or `route-message` — pick one, use consistently) is a RESERVED
OperationKind. Declared in the registry, not validatable in v0 — submission
rejects with `validation_failed` (unknown-to-Patchbay command kind is
`validation_failed` at submission per `PROTOCOL.md:197`). Promotion is a
registry update, not a schema change. Preserves the non-foreclosure seam
(agent→agent, op→op) without mediating in v0.

### D7 — response_contract committed/reserved boundary
- **Committed v0 contract kinds:** `approval`, `question`, `freeform`
  (grounded: Claude `AskUserQuestion`, Codex `requestUserInput`, OpenCode
  `question.asked`, Antigravity `ASK_QUESTION`).
- **Reserved contract kinds** (named, not validatable in v0): `secret`,
  `function_result`, `file_attachment`, `structured_schema`, `service_request`.
- **UI hints** (select-one, select-many, free-text, upload, draw) are optional
  open-set sub-fields of `question`/`approval` contract kinds.
- `elicitation-response` OperationKind is committed v0. Unknown/unsupported
  contract kinds are `validation_failed` at submission.

### D8 — Presence/Subscription authorization
1. **D8a — Subscription is grant-checked at establish, transport-layer (no
   `OperationState` lifecycle), audited, not durably recorded as an Operation,
   reconciled via cursor on reconnect.** Introduces a second authority
   mechanism (grant-checked-without-lifecycle alongside
   grant-checked-with-lifecycle for Operations/Elicitations). Justified by the
   semantic mismatch between `OperationState` (designed for finite Operations
   reaching a terminal) and long-lived streams.
2. **D8b — Presence is a derived fact, not a query target.** Derived from
   endpoint observations + session connectivity state. One-shot "is session X
   present?" routes through snapshot queries (Operations per D2). No
   `query-presence` OperationKind. Matches the existing `SessionConnectivityState`
   derivation pattern.
3. **D8c — Presence-leak prevention is a reserved seam.** Single-operator v0
   has no presence-leak threat. Filter-scoping deferred to multi-operator.

### N1 — Elicitation responder binding
1. **Binding = operator-actor.** Elicitations bind to the operator actor, NOT
   a specific endpoint. Any authenticated operator endpoint may respond.
   Tighter binding (endpoint, class, fallback chain) reserved.
2. **Delivery = fan-out to all subscribed surfaces.** Elicitation delivered via
   the subscription layer (fan-out), not per-Elicitation direct addressing.
3. **Clear-on-answer = first-answer-wins clears it everywhere.** First
   response terminalizes (`answered`) for all surfaces; subsequent attempts
   rejected as already-answered (stale/late terminal candidate).
4. **Audit = responding endpoint captured in response Operation's audit.**
   Which surface answered, for debugging. Captured at response time, not
   pre-bound.
5. **Two seams preserved:**
   - Responder-binding seam (tighter binding reserved).
   - Responder-identity audit seam (v0 captures responding endpoint; multi-
     operator adds responder-actor distinction — which operator, for
     authority/audit when multiple operators share a session).

### N2 — spawn target-scope taxonomy
1. **One `spawn` OperationKind.** No per-variant kinds in v0. Per-variant
   OperationKinds reserved.
2. **`target_spec.shape` from reserved open registry.** The spawn payload
   carries `target_spec.shape` from an open reserved registry. The protocol
   names shapes for vocabulary/audit/display but does NOT validate them at
   the protocol layer in v0. The adapter capability manifest declares which
   shapes it supports; adapter accepts or rejects at delivery with
   `unsupported_command`.
3. **Per-spawn-variant authority reserved.** v0 uses one fleet-level spawn
   grant. Per-variant authority expressible via grant `target scope` or by
   promoting variants to distinct OperationKinds later.

### N3 — Subscription cursor authorization obligation (mechanical)
The implementation checklist for `docs/VERIFICATION.md` includes all three
subscription properties: `SubscriptionGrantChecked`, `SubscriptionAudited`,
AND `SubscriptionCursorReplayAuthorized`.

## The design body (the spec the implementer applies)

`.work/active/features/feature-operator-presence-and-action-inventory.md` —
596 lines, amended twice, final re-review verdict: READY. The implementer
applies this design; does not re-open any settled decision; does not invent
new design.

## Implementation plan

### Implementation pass (single agent)
- **Model:** `umans/umans-glm-5.2` (400k context window — holds the entire
  design body + all 7 foundation docs + formal models at once, enabling
  consistent cross-reference resolution across docs in a single pass).
- **Why single, not parallel:** the 7 foundation docs heavily cross-reference
  each other (PROTOCOL defines primitives/registries/terms that VERIFICATION
  references and GLOSSARY defines; ARCHITECTURE points at PROTOCOL; UX/SECURITY
  derive from PROTOCOL's state machines). Parallel edits risk cross-reference
  drift (section numbers shift, anchor text changes, renames don't propagate).
  The 400k window lets one agent hold all docs in context and edit consistently.
- **Dispatch:** subagent tool, `model: "umans/umans-glm-5.2"`, `thinking: high`.
- **Brief:** "Apply the design body to the foundation docs per its implementation
  scope. Do NOT re-open any settled decision. Do NOT invent new design. The
  design body is the spec; you are applying it." Then list the doc edits (below).
- **Built-in self-review:** after editing, the implementer runs
  `rg` for stale references (Command/Message/Reply where the design renamed to
  Operation/Observation/Elicitation; old `expected_responder_endpoint` if any;
  stale path references), re-reads the edited docs against the design body, and
  reports any mechanical drift before handing off to the deep review.

### Foundation doc edits (the implementer's scope, from the design body)

The design body's "Implementation scope" section specifies exactly what changes
where. Summary:

- **`docs/VISION.md`** — elevate "machine-independent durable operator presence"
  + "harness-agnostic control" + "core as reachable fixed point; operators and
  agents both reconnecting clients" from implied to the central thesis. Replace
  the "may colocate for simplicity" hedge with the explicit reachability
  principle (colocate-on-one-host is deployment convenience, not architecture).
- **`docs/ARCHITECTURE.md`** — same reachability principle; update the planes
  (operator intent plane → operation plane; message/command plane →
  operation/observation/elicitation plane); update the component view and v0
  topology to reflect Operation/Observation/Elicitation; update the data flow
  for Operations, Observations, Elicitations, and subscriptions.
- **`docs/PROTOCOL.md`** — the largest edit:
  - Add the Operation/Observation/Elicitation primitive definitions (actor-
    neutral, with v0 = operator-originated Operations stated explicitly).
  - Add the `OperationKind` registry (spawn/attach/drive/cancel/interrupt/query/
    approval-response/elicitation-response/reconfigure/session-management +
    reserved `agent-send`).
  - Reframe `CommandState` as `OperationState` by documented equivalence
    (refinement mapping; the checked model is unchanged, renamed).
  - Add `ElicitationState` lifecycle (opened/pending → answered | declined |
    expired | cancelled | withdrawn | superseded | stale; first-durable-
    terminal-commit finality).
  - Add `response_contract` registry (committed: approval/question/freeform;
    reserved: secret/function_result/file_attachment/structured_schema/
    service_request; UI hints as optional open-set sub-fields).
  - Add the fifth id space: ElicitationId (adapter-assigned; core assigns LSN
    at durable record).
  - Add the Presence/Subscription section (six axes; subscription is grant-
    checked-without-lifecycle; presence is derived; leak-prevention reserved).
  - Apply vocabulary: glossary-carve Command (distinguish from harness
    slash-commands); prompt-as-payload; Message-drop (operator-originated
    no-grant Message drops for v0; agent-originated question/elicitation is the
    Elicitation primitive).
  - Add spawn authority: fleet-level target scope; spawned-session manifest
    (auto-issued descendant grant); standard revocation; capability-manifest
    idempotency; joint-control seam (delegation reserved, references
    `feature-design-grant-shape`).
  - Add N1: Elicitation responder binding (operator-actor; fan-out to all
    subscribed surfaces; first-answer-wins; responding endpoint in audit;
    tighter binding + responder-actor distinction reserved).
  - Add N2: one `spawn` OperationKind; `target_spec.shape` from reserved open
    registry; adapter accepts/rejects at delivery; per-variant authority
    reserved.
- **`docs/VERIFICATION.md`** — honest checked-vs-draft classification:
  - `OperationState` ⇿ `CommandState` refinement mapping (reuse, not new model;
    the checked properties `CommandDurability`, `TerminalFinality`,
    `LsnDeterminesTerminalWinner`, etc. apply to OperationState by equivalence).
  - New `ElicitationState` model obligations (stated-normative until promoted):
    reserve property-ids (e.g., `ElicitationPendingFinality`,
    `ElicitationFirstAnswerWins`, `ElicitationCorrelationTyped`).
  - `TypedCorrelation` extension (response Operation → Elicitation is a new
    obligation; `reply_correlation.qnt` doesn't cover it today).
  - `authority.qnt` promotion requirements (fleet-authority for spawn;
    actor-neutral grant subjects — but v0 stays operator-only, so this is
    reserved; spawn's descendant grant is a new stated-normative property).
  - Subscription authority obligations (stated-normative):
    `SubscriptionGrantChecked`, `SubscriptionAudited`,
    `SubscriptionCursorReplayAuthorized` (N3 — all three in the checklist).
  - Confirm which existing checked properties are unaffected (no regression):
    `CommandDurability`, `TerminalFinality`, `BoundaryDedup`,
    `LsnDeterminesTerminalWinner`, `PreAppendTerminalChoice`,
    `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting`,
    `SessionIdentityTuple`, `LabelsCannotOverrideIdentity`,
    `GenerationMonotonic`, `LateGenerationInert`, `TypedCorrelation` (for
    replies), `CsrfRejectsUnauthenticated`, `CsrfRejectsMissingProof`,
    `RevokedSessionCannotCommand`, `ActorIdsUnique`.
- **`docs/GLOSSARY.md`** — add/update entries: Operation, Observation,
  Elicitation, OperationKind, ElicitationState, response_contract,
  ElicitationId, Subscription, Presence (derived), spawn manifest. Glossary-
  carve Command (distinguish patchbay Command from harness slash-commands).
- **`docs/UX.md`** — update the composer / send-intent section to reflect the
  Operation/Observation/Elicitation frame (the "send intent" bullet that
  hand-waved the enumeration now references the action inventory).
- **`docs/SECURITY.md`** — align grant/authority section with spawn fleet-
  authority + descendant grant; align with the actor-neutral vocabulary (v0
  operator-only); note the joint-control seam (delegation reserved).

### Deep adversarial review (parallel codex/gpt-5.5, 3 axes)
- **Model:** `openai-codex/gpt-5.5`, thinking high. **Cross-model from the
  orchestrator** (umans) — this is the point of the outside-reviewer slot per
  the global routing rule. Do NOT use glm-5.2 here (same-model-as-orchestrator
  sacrifices the cross-model property the rule exists for; the operator
  chose (a) codex review over (b) glm-5.2 review on 2026-07-05).
- **Three independent axes, dispatched in parallel:**
  1. **Verification-posture axis** — does the rolled-forward foundation honestly
     classify checked vs draft? Does `OperationState` ⇿ `CommandState`
     refinement hold? Are new properties (ElicitationState, subscription
     authority, spawn descendant grant) properly stated-normative? Are existing
     checked properties preserved (no regression)?
  2. **Grounding-coverage axis** — does the rolled-forward foundation's action
     inventory stay grounded in the research corpus? Are citation handles
     accurate? Does the frame cover all 7 surveyed harnesses?
  3. **Coherence-seams axis** — do the 7 docs cross-reference consistently
     after the roll-forward? Are the reserved seams (agent-send, tighter
     responder binding, delegation, no-lifecycle reads, per-variant spawn
     authority, presence-leak prevention) explicitly preserved? Any silent
     foreclosure?
- **Brief each reviewer:** fresh context, read the rolled-forward foundation +
  the design body + (for grounding axis) the research corpus. Attack, don't
  ratify. Output a structured review with defects numbered.
- **Orchestrator harvests:** collect the three reviews, dedupe overlapping
  defects, triage by severity (blocker / serious / nit), present to the
  operator for fix-or-accept decisions before advancing the feature to
  stage:review.

### Stage discipline
- The implement step does NOT advance `stage: drafting` → `stage: implementing`
  automatically in this workflow. The operator-requested sequence has the deep
  adversarial review as the final gate. After the deep review:
  - If no blockers: advance to `stage: review` (the feature's terminal review
    state) and present to the operator.
  - If blockers: amend the foundation docs (not the design body — the design is
    settled; the implementer made mechanical errors), re-review the affected
    axis only, then advance.

## Key files (reboot reference)

- Design body (the spec): `.work/active/features/feature-operator-presence-and-action-inventory.md`
- Frame reviews: `.work/session-notes/2026-07-04-frame-adversarial-review.md`,
  `.work/session-notes/2026-07-05-ooe-frame-review.md`
- This plan: `.work/session-notes/2026-07-05-implementation-plan.md`
- Research corpus: `.research/analysis/campaigns/harness-action-surfaces/`
  (parent.md, verification-checklist.md, specialists/)
- Formal models: `specs/seed/*.qnt`, `specs/seed/patchbay-relational.als`
- Foundation docs to edit: `docs/{VISION,ARCHITECTURE,PROTOCOL,VERIFICATION,
  GLOSSARY,UX,SECURITY}.md`

## Provenance note (for the reboot context)

During this session, an uncommitted "Update — fresh-context adversarial frame
review banked (2026-07-05)" section appeared appended to
`.work/session-notes/2026-07-04-action-inventory-reframe.md`. The orchestrator
did not write it; the operator confirmed this was the only session working in
patchbay. A peer agent (`/home/agent/projects/SNC@SNC`) was online on the mesh
with filesystem access; the orchestrator sent a provenance query but no reply
was received before the session moved on. The anomaly was reverted (reframe
note restored to committed state + SNC relink); the working tree is clean. If
the reboot context sees similar unattributed appends, treat as a provenance
concern and surface to the operator rather than preserving or acting on the
content.
