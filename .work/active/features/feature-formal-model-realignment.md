---
id: feature-formal-model-realignment
kind: feature
stage: drafting
tags: [verification, protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-operator-presence-and-action-inventory, feature-formal-model-seed]
created: 2026-07-06
updated: 2026-07-08
gate_origin: null
release_binding: null
---

# Feature: Re-align seed formal models with the rolled-forward foundation

## Misroute note (2026-07-07)

Stripped `[prose]` — this is a design feature, not prose authoring. The brief itself designs the realignment plan: VR2 metadata schema decision (rename-in-place vs split field — a schema choice with downstream traceability/CI consequences), V1 model approach decision (strengthen `command_lifecycle.qnt` in place with regression risk to 7 checked properties vs author a new `operation_lifecycle.qnt` with refinement-equivalence debt), model-authoring order + per-model bounds/tool invocations, and four new stated-normative model arcs (Elicitation, TypedCorrelation, Subscription authority, Spawn authority) each requiring bounds/properties/promotion-criteria design. The brief lists explicit "Open questions (for the design pass)" and a "Risks / pre-mortem" section — feature-design language. The prose-author lane skips the design gate, pre-mortem, and alternatives evaluation; this item needs them. Routed through `feature-design`; `prose` tag removed. Same misroute pattern documented in the epic's lane-routing discipline and the 2026-07-06 codification of the prose black-box test.

## Brief

The O/O/E roll-forward (`feature-operator-presence-and-action-inventory`) and its deep adversarial review made the verification tier scheme explicit — splitting the old `checked-normative` into **checked-model** (model promoted, no conformance vector yet) vs **checked-normative** (model + ≥1 promoted vector), with **stated-normative** reserved for draft/no-model/reserved obligations. That honesty pass exposed three classes of misalignment between the seed formal models (`specs/seed/*.qnt`, `*.als`) and the now-authoritative foundation docs:

1. **`@promotion` metadata drift (VR2).** All 16 promoted seed properties across `command_lifecycle.qnt`, `session_generation.qnt`, `reply_correlation.qnt`, `csrf_browser.qnt`, and `patchbay-relational.als` still carry `tier: checked-normative, status: promoted` in their machine-readable promotion blocks. The docs now classify these as `checked-model` (no conformance vectors exist yet, so none are `checked-normative`). A future traceability/CI reader consuming model metadata would disagree with the docs. The model metadata must be updated to `tier: checked-model` (or an equivalent split field) so the model files and `docs/VERIFICATION.md` agree.

2. **Transition-adjacency modeling gap (V1 follow-on).** `command_lifecycle.qnt`'s `commitTerminal` action allows any non-terminal state to commit any terminal candidate — so `accepted → completed` is model-permitted. The protocol's transition registry forbids that adjacency, and the no-direct-to-completed fast-path reads rule (D2) is part of the v0 contract. The docs now mark both rules **stated-normative** rather than overclaim them as checked. Promoting them requires either strengthening `command_lifecycle.qnt` to model the exact transition relation (including no `accepted → completed`) or authoring a new `OperationState`-specific model that verifies the adjacency rules the checked `CommandState` model does not.

3. **New stated-normative properties with no models.** The roll-forward reserved property ids for new obligations that currently have NO model at all, only stated-normative prose in `docs/VERIFICATION.md`:
   - `ElicitationState` lifecycle: `ElicitationPendingFinality`, `ElicitationFirstAnswerWins`, `ElicitationCorrelationTyped`, `ElicitationTimeoutNeitherSuccessNorDenial`, `ElicitationInvalidResponseRejected`, `ElicitationStaleTargetInert`, `ElicitationWithdrawalFinality`.
   - Response Operation → Elicitation typed correlation (the `TypedCorrelation` extension beyond `reply_correlation.qnt`'s Reply → Command/Message core).
   - Subscription authority: `SubscriptionGrantChecked`, `SubscriptionAudited`, `SubscriptionCursorReplayAuthorized` (grant-checked-without-lifecycle — a second authority mechanism distinct from Operation authority).
   - Spawn authority: `FleetAuthorityForSpawn`, `SpawnCreatesDescendantGrant`, `SpawnRevocationDoesNotCascade`, `ElicitationResponderAuthority`.
   - `browser_local_state_not_authority` (currently a non-promoted documentation invariant in `csrf_browser.qnt` — needs either promotion to checked-model with metadata or explicit stated-normative reservation; flagged as VR4).

   Each of these is a model-authoring arc that, if it passes, promotes the corresponding doc property from stated-normative toward checked-model (and eventually checked-normative once conformance vectors land).

## Design stance

This is a `[verification]` + `[prose]` feature: it designs the model-realignment plan (which models to strengthen, which to author new, in what order, with what bounds and tool invocations) and then the implementation pass edits model files' promotion metadata + authors the new models. A future design pass (feature-design or prose-author routing) will flesh out the per-model plan; this item captures the scope now so the work is tracked rather than lost.

The feature depends on `feature-operator-presence-and-action-inventory` (which established the tier scheme and the stated-normative property ids) and `feature-formal-model-seed` (which authored the original seed models now being realigned). It does NOT re-open the O/O/E frame or any settled decision (D1–D8, N1–N3).

## Scope (initial — to be refined by a design pass)

### In scope
- **VR2:** Update `@promotion` metadata in all 16 promoted seed properties to `tier: checked-model` (from `checked-normative`), preserving `status: promoted`. Decide whether to introduce a split field (e.g. `model_status: promoted` + `normative_tier: checked-model`) or rename in place. Update stale header comments in the model files to match.
- **VR4:** Resolve `browser_local_state_not_authority` — promote with full promotion metadata (bounds, tool invocation, pass/fail) to checked-model, or reserve as stated-normative. Remove its "Non-promoted documentation invariant" limbo.
- **V1 follow-on:** Strengthen `command_lifecycle.qnt` to model the exact `OperationState` transition relation (no `accepted → completed`; no-direct-to-completed fast-path for reads), OR author a new `operation_lifecycle.qnt` model that verifies the adjacency rules and establishes the refinement equivalence to `CommandState` for the checked properties. Decide which approach preserves the existing checked properties without regression.
- **New model arcs (each a potential child story):**
  - `ElicitationState` lifecycle model (7 reserved property ids).
  - Response Operation → Elicitation `TypedCorrelation` extension (extend `reply_correlation.qnt` or new model).
  - Subscription authority model (3 reserved property ids; the grant-checked-WITHOUT-lifecycle second authority mechanism).
  - Spawn authority model (4 reserved property ids; fleet-level target scope, descendant grant, no-cascade revocation, responder authority).
- **Traceability:** confirm the post-realignment model metadata and `docs/VERIFICATION.md` agree on every property's tier. A future conformance-vector feature (separate) will then promote checked-model → checked-normative.

### Out of scope
- Conformance vector authoring (that's a separate verification effort — vectors don't exist yet and are the gate from checked-model to checked-normative).
- Any change to the O/O/E frame, registries, or settled decisions.
- Editing foundation docs (the docs are already honest post-amendment; this feature brings the MODELS up to the docs, not the reverse).

## Open questions (for the design pass)

1. **Metadata schema.** Rename `tier: checked-normative` → `tier: checked-model` in place, or introduce a split field (`model_status` + `normative_tier`)? The latter is more explicit but touches every promotion block; the former is minimal but loses the "normative" signal. What does the downstream traceability/CI reader expect?
2. **V1 approach.** Strengthen `command_lifecycle.qnt` in place (risk: regressing the existing checked properties if the transition relation changes the state space) vs. author a new `operation_lifecycle.qnt` (cleaner separation, but establishes a second model that must be kept in refinement equivalence with `command_lifecycle.qnt`). Which preserves the 7 existing checked properties with least risk?
3. **Model-authoring order.** Which new stated-normative model arc is highest-priority for v0 safety? `ElicitationState` (first-answer-wins, terminal finality) and subscription authority (grant-checked-without-lifecycle) seem most safety-critical; spawn authority is v0-committed behavior that currently has no model. Sequence them.
4. **Bounds and tool invocation.** Each new model needs finite bounds and a documented Apalache/TLC/Quint invocation. Carry forward the bounds discipline from `feature-formal-model-seed`.

## Risks / pre-mortem

- **Regression risk (V1):** strengthening `command_lifecycle.qnt` could break the 7 properties it currently checks. Mitigation: run the existing model against the strengthened transition relation before claiming any new property; if a property fails, the strengthening is wrong, not the property.
- **Metadata drift recurrence.** If the model files and docs can disagree once, they can disagree again. Mitigation: this feature should consider whether a traceability check (property ids in docs ↔ `@promotion` blocks in models) belongs in the verification harness, not just in this one-time realignment.
- **Scope creep.** The new stated-normative model arcs are substantial. This feature should plan them as child stories with depends_on chains, not attempt all in one stride.

## Key files

- Foundation docs (authoritative post-amendment): `docs/VERIFICATION.md` (tier definitions, property lists), `docs/PROTOCOL.md` (registries, lifecycles).
- Seed models to realign: `specs/seed/command_lifecycle.qnt`, `session_generation.qnt`, `reply_correlation.qnt`, `csrf_browser.qnt`, `snapshot_recovery.qnt`, `authority.qnt`, `patchbay-relational.als`.
- Provenance: deep-review findings VR1–VR4 (`.work/session-notes/` 2026-07-05 implementation-plan + the re-review reports), V1 model-equivalence defect (verified against `command_lifecycle.qnt:53-56`).

---

## Design (2026-07-08)

### Grounding discoveries (load-bearing for the design)

Two facts discovered during the design grounding reshape the brief's open questions and pre-mortem. They are recorded here because they change the recommended approach, not just confirm it.

1. **The `@promotion`-reading CI script does not exist.** The seed feature's Q5 design committed to "a CI script greps the fenced `@promotion` blocks and generates the traceability table in `docs/VERIFICATION.md` as a checked-in artifact." That script was never written. The only CI script today, `contracts/scripts/check-vectors.mjs`, hardcodes the property→tier registry in three JS arrays (`CHECKED_MODEL_PROPERTIES`, `STATED_NORMATIVE_PROPERTIES`, `CHECKED_NORMATIVE_PROPERTIES = []`) and generates the traceability table from **conformance vectors**, not from model `@promotion` metadata. Consequence: the `tier` field inside every `@promotion` block is currently **write-only** — nothing reads it. The docs and the models can already silently disagree (and do: 16 blocks say `checked-normative`, docs say `checked-model`). This means VR2 is not just a one-time metadata edit; the recurrence pre-mortem is already realized. The real fix is to close the read loop, not just rename a field.
2. **The V1 transition-adjacency gap is verified and low-regression-risk.** `command_lifecycle.qnt` `commitTerminal(cmd, candidate)` guards only `state.get(cmd).in(NON_TERMINAL)` and `candidate.in(TERMINAL)` — no adjacency constraint, so `accepted → completed` is model-permitted. `docs/PROTOCOL.md` (lines 116–132) forbids that adjacency: `accepted → delivered | rejected` only. Crucially, all 7 currently-checked properties (`CommandDurability`, `TerminalFinality`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `BoundaryDedup`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting`) are terminal-finality / durability / dedup properties that are **independent of which terminal a state reaches** — they quantify over `TERMINAL` as a set, not over specific adjacencies. Strengthening the transition relation to forbid `accepted → completed` (and the other non-`delivered`/`rejected` adjacencies) therefore cannot break these properties by construction: they don't mention adjacency. This must still be proven by re-running the 7 checks after strengthening (the pre-mortem's regression mitigation), but the design can proceed on the expectation that the strengthening is safe.

### Design decisions

- **Q1 — Metadata schema: rename `tier: checked-normative` → `tier: checked-model` in place (no split field).** Rationale: the split-field option (`model_status` + `normative_tier`) was motivated by preserving a "normative" signal, but the tier vocabulary already encodes it — `checked-model` means "model promoted, normative claim exists, awaiting vectors," and `checked-normative` means "model + ≥1 promoted vector." A split field would touch every block to add a field that duplicates information the tier name already carries, and would require the (still-nonexistent) reader to reconcile two fields. The rename-in-place is minimal, lossless, and matches the vocabulary `docs/VERIFICATION.md` already uses. The downstream reader (designed below) consumes a single `tier` field. **Decision recorded against the recurrence pre-mortem:** the rename alone does not prevent drift recurrence; the traceability check (Unit TR) is what closes the loop.
- **Q2 — V1 approach: strengthen `command_lifecycle.qnt` in place (do NOT author a separate `operation_lifecycle.qnt`).** Rationale: the 7 checked properties are adjacency-independent (grounding discovery #2), so strengthening the transition relation is low-regression-risk and keeps a single source of truth for the lifecycle. Authoring a separate `operation_lifecycle.qnt` would create a second model that must be kept in refinement equivalence with `command_lifecycle.qnt` — refinement-equivalence debt for no gain, since `OperationState` is already `CommandState` by documented refinement (`docs/PROTOCOL.md:138-142`). The strengthening adds an adjacency-guarded `commitTerminal` and a new checked property `NoAcceptedToCompleted` (plus the read fast-path rule as a reserved, not-yet-checked property). The existing 7 properties are re-verified after the change as the regression gate.
- **Q3 — Model-authoring order (by v0 safety criticality):**
  1. **VR2 metadata realignment + traceability check** (Unit TR + Unit M) — first, because it establishes the closed loop that makes every subsequent promotion honest. Doing this first means the new model arcs below are born into a regime where drift is caught, not into the current write-only regime.
  2. **V1 transition-adjacency strengthening** (Unit CL) — second, because it promotes two stated-normative rules (no `accepted → completed`; no-direct-to-completed reads) toward checked, and it's the highest-regression-risk change so it should run early while the surface is small.
  3. **Elicitation lifecycle model** (Unit EL) — third; first-answer-wins and terminal finality are the most safety-critical new obligations (an unanswered approval gate or a double-answer is a real safety failure). 7 reserved property ids.
  4. **Spawn authority model** (Unit SA) — fourth; spawn is committed v0 behavior with no model, and descendant-grant / no-cascade-revocation are authority-critical. 4 reserved property ids.
  5. **Subscription authority model** (Unit SUB) — fifth; the grant-checked-without-lifecycle second authority mechanism. 3 reserved property ids. Lower than Elicitation/Spawn because a subscription mis-grant is a read-leak, not a control-plane forgery, but still v0-committed.
  6. **TypedCorrelation extension** (Unit TC) — sixth; extends `reply_correlation.qnt` to response Operation → Elicitation. Last because it depends on the Elicitation model existing (Unit EL) to define the ElicitationId space it correlates against.
  7. **VR4 `browser_local_state_not_authority`** (folded into Unit M) — resolve the limbo: promote with full metadata to `checked-model` (the invariant already exists and is genuine per the seed review's mutation-test proof), or explicitly reserve as stated-normative. Decision: **promote to `checked-model`** — the invariant is already authored, already genuine (independent oracle over raw attempted evidence, mutation-tested in the seed review), and already runs. Its only defect is missing `@promotion` metadata. Adding the block and setting `tier: checked-model, status: promoted` is a metadata completion, not a new model.
- **Q4 — Bounds and tool invocation:** carry forward the bounds discipline from `feature-formal-model-seed`. Each new model uses the smallest finite bounds that exercise the relevant race (≥2 competing candidates where first-wins matters; ≥2 id spaces where forgery matters), documents the exact Apalache/TLC/Quint invocation in its `@promotion` block, and records `expected: pass`. Temporal properties use the Apalache default backend with `echo y |` (the `--backend tlc` path does not work for `next()`-in-`always()` per the seed's Implementation discovery; all temporal properties here are `always(...)` safety). Alloy relational checks use `--type text` with skolem-witness inspection (the reliable UNSAT method per the seed review).

### Architectural choice

The realignment is structured as **one closed-loop metadata layer + one strengthening + four new model arcs**, sequenced so the traceability loop is established before any new model is authored. The metadata layer (Unit TR) is the keystone: it makes the `@promotion` blocks machine-readable and CI-enforced, so that VR2's one-time rename and every subsequent promotion are checked for drift rather than trusted. The V1 strengthening (Unit CL) is done in-place on `command_lifecycle.qnt` rather than via a new model, because the checked properties are adjacency-independent and a second model would only add refinement-equivalence debt. The four new arcs (Elicitation, Spawn, Subscription, TypedCorrelation extension) are each independent Quint models (or extensions) authored with the genuine-checking discipline proven in the seed arc: permissive actions + independent-oracle invariants + mutation-test proof at promotion time.

This composes with the existing verification architecture: the property-id vocabulary stays the SSOT (established by the seed feature), the `@promotion` block format is unchanged (only the `tier` values are corrected), and the new traceability script extends the `contracts/scripts/` family alongside `check-vectors.mjs` and `check-generated-drift.mjs`.

### Implementation Units

#### Unit TR: Traceability check script (keystone — designed first)

**File**: `contracts/scripts/check-models.mjs`
**Story**: spawned as `story-formal-model-realignment-traceability`

Closes the read loop that the seed feature's Q5 committed to but never built. Parses every `@promotion { ... }` block in `specs/seed/*.qnt` and `specs/seed/*.als`, extracts the structured fields (`property`, `tier`, `status`, `model`, `backend`, `invocation`, `bounds`, `expected`, `proto_fields`, `semantics`), and cross-checks against `docs/VERIFICATION.md` and `contracts/scripts/check-vectors.mjs`'s registry arrays.

**Checks it enforces (exit 1 on any failure):**
1. **Coverage:** every property-id in the VERIFICATION.md tier tables has exactly one `@promotion` block, and every `@promotion` block's `property` field names a property-id that appears in VERIFICATION.md. Catches omission and misspelling.
2. **Tier agreement:** for each promoted block (`status: promoted`), the `tier` field agrees with VERIFICATION.md's classification of that property. A block saying `tier: checked-normative` for a property VERIFICATION.md lists as `checked-model` fails. **This is the check that would have caught VR2 automatically.**
3. **Status-vs-vector consistency:** a block with `tier: checked-normative, status: promoted` requires ≥1 promoted conformance vector tracing to that property (delegates to the vector registry in `check-vectors.mjs`). A `checked-model, status: promoted` block requires zero vectors (informational only, not failing) but is consistent with the "no promoted vector yet" state.
4. **Invocation well-formedness:** every promoted block's `invocation` field is non-empty, names the tool (`quint` or `java`), and (for Quint) includes the model file path. Draft blocks may have `<TBD>` invocations.
5. **Drift detection vs the hardcoded registry:** the property→tier map derived from `@promotion` blocks must agree with the `CHECKED_MODEL_PROPERTIES` / `STATED_NORMATIVE_PROPERTIES` / `CHECKED_NORMATIVE_PROPERTIES` arrays in `check-vectors.mjs`. If they disagree, the script reports which side is stale and exits 1. **This makes the hardcoded arrays a checked-in cache of the model metadata, not an independent source of truth** — the models become the SSOT, the arrays become derived.

**Design notes:**
- The parser is a fenced-block regex extractor (`/\/\/ @promotion \{([\s\S]*?)\n\/\/ \}/` for Quint `//` comments; `/\/\/ @promotion \{([\s\S]*?)\n\/\/ \}/` also covers Alloy `//` comments). It does not depend on a YAML parser — the fields are `key: value` lines, parsed with a simple split.
- The script writes a generated `## Generated model-promotion traceability` section into `docs/VERIFICATION.md` (parallel to the existing generated conformance-vector table), listing each property, its model file, tier, status, and backend. This makes the model↔doc agreement a checked-in artifact.
- Wired into `contracts/ts/package.json` as `check:models` and into a root `check:all` aggregate (alongside `check:vectors` and `check:drift`).

**Acceptance Criteria:**
- [ ] `node contracts/scripts/check-models.mjs` exits 0 on the current (post-VR2-rename) model set.
- [ ] Deliberately changing one `@promotion` block's `tier` to disagree with VERIFICATION.md causes exit 1 with a message naming the property and the disagreement.
- [ ] Deliberately removing a `@promotion` block for a VERIFICATION.md-listed property causes exit 1 (coverage failure).
- [ ] The hardcoded arrays in `check-vectors.mjs` are confirmed in-agreement with the model-derived map (or, if not, the disagreement is reported — expected to be in-agreement after Unit M lands).
- [ ] Generated traceability section appears in `docs/VERIFICATION.md`.

---

#### Unit M: VR2 metadata realignment + VR4 resolution

**File**: edits to all 7 seed model files (`command_lifecycle.qnt`, `session_generation.qnt`, `reply_correlation.qnt`, `csrf_browser.qnt`, `patchbay-relational.als`, `snapshot_recovery.qnt`, `authority.qnt`)
**Story**: folded into `story-formal-model-realignment-traceability` (same stride — the metadata edit and the check that validates it belong together)

**VR2 — rename in place:** change `tier: checked-normative` → `tier: checked-model` in the 16 promoted `@promotion` blocks across the 5 checked models (`command_lifecycle.qnt` ×7, `session_generation.qnt` ×4, `reply_correlation.qnt` ×1, `csrf_browser.qnt` ×3, `patchbay-relational.als` ×1). The 10 `stated-normative` blocks (in `snapshot_recovery.qnt`, `authority.qnt`, and the 2 demoted Alloy asserts) are unchanged — `stated-normative` is already correct. `status: promoted` is preserved on all 16. Update stale header comments in model files that say "checked-normative" to say "checked-model" where they describe the model's tier.

**VR4 — promote `browser_local_state_not_authority`:** the invariant already exists in `csrf_browser.qnt` (lines ~150-160) as a non-promoted documentation invariant with a genuine independent oracle (mutation-tested in the seed review). Add a full `@promotion` block: `property: browser_local_state_not_authority, tier: checked-model, status: promoted, model: specs/seed/csrf_browser.qnt, language: quint, backend: apalache, invocation: quint verify csrf_browser.qnt --invariant browser_local_state_not_authority --max-steps 12, bounds: { sessions: 4, proofs: 5, max_steps: 12 }, expected: pass, proto_fields: [none], semantics: browser-local UI claims cannot grant authority or override server-side session/CSRF grant checks`. This promotes the property from stated-normative to checked-model in both the model and (via Unit TR's generated table) VERIFICATION.md.

**Acceptance Criteria:**
- [ ] All 16 previously-`checked-normative` blocks now read `tier: checked-model`.
- [ ] `browser_local_state_not_authority` has a complete `@promotion` block with `status: promoted`.
- [ ] `node contracts/scripts/check-models.mjs` exits 0 (tier agreement with VERIFICATION.md).
- [ ] `node contracts/scripts/check-vectors.mjs` still exits 0 (the hardcoded arrays must be updated to move `browser_local_state_not_authority` from `STATED_NORMATIVE_PROPERTIES` to `CHECKED_MODEL_PROPERTIES` — this is part of this unit since the arrays must track the model metadata).
- [ ] All 4 checked Quint models still `quint parse` + `quint compile` exit 0 (metadata-only change, but verified).
- [ ] The 17th checked-model property (`browser_local_state_not_authority`) is reflected in VERIFICATION.md's checked-model list and the generated traceability table.

---

#### Unit CL: V1 transition-adjacency strengthening (trickiest unit — highest regression risk)

**File**: `specs/seed/command_lifecycle.qnt`
**Story**: spawned as `story-formal-model-realignment-adjacency`

Strengthens `commitTerminal` to enforce the `docs/PROTOCOL.md` transition adjacency and adds a new checked property verifying the no-`accepted → completed` rule. The read/query no-direct-to-completed fast-path rule is added as a **reserved, not-yet-checked** property (it requires modeling OperationKind-specific skip-`running` behavior, which is a larger surface — reserved for a follow-on).

**Strengthened transition relation:**

```quint
// The allowed adjacency from docs/PROTOCOL.md:116-132.
// accepted  -> delivered | rejected
// delivered -> running | completed | rejected | failed | expired | cancelled | superseded
// running   -> completed | failed | expired | cancelled | superseded
pure def allowedTransition(from, to) =
  if (from == "accepted")
    to.in(Set("delivered", "rejected"))
  else if (from == "delivered")
    to.in(Set("running", "completed", "rejected", "failed", "expired", "cancelled", "superseded"))
  else if (from == "running")
    to.in(Set("completed", "failed", "expired", "cancelled", "superseded"))
  else
    false
```

The `commitTerminal(cmd, candidate)` action gains the guard `allowedTransition(state.get(cmd), candidate)`. The `lateTerminalCandidate` action is unchanged (it's a no-op by design; adjacency doesn't apply to late candidates that don't mutate state). A new non-terminal→non-terminal `advance(cmd, candidate)` action models `accepted → delivered` and `delivered → running` (the non-terminal transitions the current model omits — without it, `accepted` can only go terminal, never to `delivered`, which under-strengthens rather than over-strengthens but should be modeled for completeness).

**New checked property:**

```quint
// @promotion { property: NoAcceptedToCompleted, tier: checked-model, status: promoted,
//   model: specs/seed/command_lifecycle.qnt, language: quint, backend: apalache,
//   invocation: quint verify command_lifecycle.qnt --invariant no_accepted_to_completed --max-steps 12,
//   bounds: { cmd_ids: 3, idempotency_keys: 3, max_steps: 12 },
//   expected: pass, proto_fields: [none],
//   semantics: a command in 'accepted' state cannot transition directly to 'completed';
//              it must pass through 'delivered' (docs/PROTOCOL.md transition adjacency) }
// INDEPENDENT oracle: checks the raw state-transition fact (no cmd is ever both accepted-then-completed
// without an intervening delivered/running), not the allowedTransition helper. Mutation test: breaking
// allowedTransition to allow accepted->completed must fail this invariant.
val no_accepted_to_completed =
  CMD_IDS.forall(cmd =>
    not(state.get(cmd) == "completed") or
      /* the cmd must have passed through delivered/running to reach completed */
      true)  // see Implementation Notes — the two-state form is below
```

**Implementation Notes:**
- The one-state invariant above is too weak (it can't see the transition). The genuine check is a **temporal** property: `always(forall cmd => next(state.get(cmd)) == "completed" implies state.get(cmd).in(Set("delivered", "running")))`. This makes `NoAcceptedToCompleted` an `apalache-temporal` property, not an `apalache` invariant. The `@promotion` block's `backend` field is `apalache-temporal` and the invocation is `echo y | quint verify command_lifecycle.qnt --temporal no_accepted_to_completed --max-steps 10`.
- **Genuine-checking discipline (load-bearing):** the invariant must not re-use `allowedTransition`. It checks the raw state fact (`next state is completed` implies `current state was delivered or running`). A mutation that breaks `allowedTransition` to allow `accepted → completed` must fail this invariant — verified by mutation test at promotion time.
- **Regression gate (the pre-mortem's mitigation):** after strengthening, re-run all 7 existing checked properties. They are expected to pass unchanged because they quantify over `TERMINAL` as a set and don't mention adjacency (grounding discovery #2). If any fails, the strengthening is wrong, not the property — investigate before adjusting.
- The `advance` action (non-terminal→non-terminal) is added so the state space includes the `accepted → delivered → running → completed` path; without it the strengthened `commitTerminal` would make `completed` unreachable from `accepted`, which would make `NoAcceptedToCompleted` vacuously true (no `completed` state ever arises). The `advance` action is the non-vacuity guarantee.
- `command_lifecycle.emitted.tla` is regenerated (`quint compile --target tlaplus`) and re-committed.

**Acceptance Criteria:**
- [ ] `quint parse` + `quint compile` exit 0.
- [ ] All 7 existing checked properties still pass (`echo y | quint verify --temporal <p>` for the 5 temporal; `quint verify --invariant <v>` for the 2 invariants).
- [ ] `NoAcceptedToCompleted` passes (`echo y | quint verify --temporal no_accepted_to_completed --max-steps 10`, exit 0).
- [ ] Mutation test: breaking `allowedTransition` to permit `accepted → completed` causes `NoAcceptedToCompleted` to fail (exit 1, counterexample). This is the genuine-checking proof.
- [ ] `command_lifecycle.emitted.tla` regenerated and committed.
- [ ] `@promotion` block for `NoAcceptedToCompleted` present; `node contracts/scripts/check-models.mjs` exits 0 (new property reflected in VERIFICATION.md tier tables).
- [ ] VERIFICATION.md updated: `NoAcceptedToCompleted` added to checked-model list; the "OperationState transition adjacency" stated-normative bullet narrowed to note that no-`accepted → completed` is now checked-model, while the full adjacency graph and the read fast-path rule remain stated-normative.

---

#### Unit EL: Elicitation lifecycle model

**File**: `specs/seed/elicitation_lifecycle.qnt` (new)
**Story**: spawned as `story-formal-model-realignment-elicitation`

Models the `ElicitationState` lifecycle from `docs/PROTOCOL.md:270-305`: `opened → pending → (answered | declined | expired | cancelled | withdrawn | superseded | stale)`, with first-durable-terminal-commit-wins finality. Carries 7 reserved property ids toward checked-model.

**State variables** (trace to PROTOCOL `ElicitationState`, `ElicitationId`, `response_contract`):

```quint
module elicitation_lifecycle {
  pure val TERMINAL = Set("answered", "declined", "expired", "cancelled", "withdrawn", "superseded", "stale")
  pure val NON_TERMINAL = Set("opened", "pending")
  pure val ELICITATION_IDS = Set("e1", "e2")           // bound: 2 elicitations
  pure val ENDPOINTS = Set("ep-a", "ep-b")              // bound: 2 subscribed surfaces
  pure val CONTRACT_KINDS = Set("approval", "question") // committed v0 contract kinds

  var state: str -> str                  // ElicitationId -> ElicitationState
  var terminalLsn: str -> int            // ElicitationId -> LSN at terminal commit (0 = not terminal)
  var lsn: int                           // monotonic gap-free log sequence number
  var responderActor: str -> str         // ElicitationId -> expected responder actor (operator in v0)
  var answeredBy: str -> str             // ElicitationId -> endpoint that answered (audit; "none" if not answered)
  var contractKind: str -> str           // ElicitationId -> response_contract.contract_kind
}
```

**Actions** (permissive — first-answer-wins is checked *against* the race, not baked into a guard):
- `openElicitation(eid, actor, contract)` — `opened` initial state.
- `makePending(eid)` — `opened → pending` (visible to subscribed surfaces).
- `answer(eid, endpoint, lsn)` — a valid response from any authenticated endpoint for the expected responder actor; first durable terminal commit wins.
- `lateAnswer(eid, endpoint)` — a second answer after terminal: no-op (the TerminalFinality guarantee).
- `decline(eid)`, `expire(eid)`, `cancel(eid)`, `withdraw(eid)`, `supersede(eid)`, `goStale(eid)` — terminal candidates competing for first-commit.

**Checked properties** (7, each with `@promotion` block, all `tier: checked-model, status: promoted` after passing):
- `ElicitationPendingFinality` (temporal) — once terminal, later candidates don't mutate.
- `ElicitationFirstAnswerWins` (temporal) — for single-answer contracts, the first durably committed valid answer wins; later answers are no-ops.
- `ElicitationCorrelationTyped` (invariant) — response Operations reference a known ElicitationId in the same authority/responder context; cannot forge across id spaces. (Projects the correlation check; the full TypedCorrelation extension is Unit TC.)
- `ElicitationTimeoutNeitherSuccessNorDenial` (invariant) — `expired` terminal does not imply `answered` or `declined`.
- `ElicitationInvalidResponseRejected` (invariant) — invalid response leaves the Elicitation `pending` (default reject-and-leave-pending policy).
- `ElicitationStaleTargetInert` (temporal) — responses to stale/superseded targets don't mutate live state.
- `ElicitationWithdrawalFinality` (temporal) — opener withdrawal terminalizes without later response mutation.

**Implementation Notes:**
- Bounds: 2 elicitations × 2 endpoints × 2 contract kinds. Small enough for Apalache; large enough to exercise the first-answer race (2 endpoints answering the same elicitation).
- Genuine-checking: the `answer` action is permissive (any endpoint may attempt; the guard checks only that the endpoint is authenticated for the expected responder actor). The invariant proves first-answer-wins, not the guard.
- `ElicitationCorrelationTyped` here checks the id-space disjointness and known-id resolution within this model; the cross-model response-Operation→Elicitation correlation is Unit TC.
- `elicitation_lifecycle.emitted.tla` generated and committed.

**Acceptance Criteria:**
- [ ] `quint parse` + `quint compile` exit 0.
- [ ] All 7 properties pass with documented invocations.
- [ ] Mutation test for `ElicitationFirstAnswerWins`: allowing a second answer to mutate state fails the property.
- [ ] `@promotion` blocks present; `check-models.mjs` exits 0; VERIFICATION.md updated (7 properties move from stated-normative to checked-model).

---

#### Unit SA: Spawn authority model

**File**: `specs/seed/spawn_authority.qnt` (new)
**Story**: spawned as `story-formal-model-realignment-spawn`

Models spawn fleet authority, descendant-grant creation, two-lever revocation, and Elicitation responder authority. 4 reserved property ids.

**State variables** (trace to PROTOCOL spawn authority, `Grant`, descendant grant):

```quint
module spawn_authority {
  pure val ACTORS = Set("alice")                  // v0: operator-only
  pure val SESSIONS = Set("s1", "s2", "s3")        // s3 does not exist initially (spawn target)
  pure val GRANTS = Set("g-fleet", "g-descendant")
  pure val ENDPOINTS = Set("ep-a")

  var spawnGrantLive: bool                         // fleet-level spawn grant is live
  var descendantGrant: str -> str                   // spawned session -> descendant grant id ("none" if not spawned)
  var descendantGrantLive: str -> bool             // descendant grant id -> live
  var sessionExists: Set[str]                      // sessions that have been spawned
  var lsn: int
  var terminalLsn: str -> int                     // spawn-op id -> terminal LSN
}
```

**Actions:**
- `spawn(targetSession)` — requires `spawnGrantLive`; on completion, creates the session and issues a descendant grant (`descendantGrant[target] = "g-descendant"`, `descendantGrantLive["g-descendant"] = true`).
- `revokeSpawnGrant()` — sets `spawnGrantLive = false`; does NOT touch `descendantGrantLive` (no-cascade).
- `revokeDescendantGrant(session)` — sets the descendant grant for that session to not-live (separate lever).
- `spawnAfterRevoke(targetSession)` — permissive: attempts spawn after spawn grant revoked; must be rejected (the property proves it).

**Checked properties** (4, `tier: checked-model, status: promoted`):
- `FleetAuthorityForSpawn` (invariant) — spawn accepted only when `spawnGrantLive`; a per-session grant alone (modeled as absence of fleet grant) does not authorize spawn of a not-yet-existing session.
- `SpawnCreatesDescendantGrant` (invariant) — successful spawn completion records an explicit descendant grant for the spawned session.
- `SpawnRevocationDoesNotCascade` (temporal) — revoking the spawn grant prevents future spawns but does not revoke already-created descendant grants.
- `ElicitationResponderAuthority` (invariant) — a response Operation is accepted only from an authenticated endpoint for the expected responder actor. (Modeled here rather than in Unit EL because it's an authority property; projects the responder-actor binding from the Elicitation model.)

**Acceptance Criteria:**
- [ ] `quint parse` + `quint compile` exit 0.
- [ ] All 4 properties pass.
- [ ] Mutation test for `SpawnRevocationDoesNotCascade`: cascading revocation (revokeSpawnGrant also revokes descendant) must fail the property.
- [ ] `@promotion` blocks present; `check-models.mjs` exits 0; VERIFICATION.md updated.

---

#### Unit SUB: Subscription authority model

**File**: `specs/seed/subscription_authority.qnt` (new)
**Story**: spawned as `story-formal-model-realignment-subscription`

Models the grant-checked-without-lifecycle second authority mechanism: subscription establishment is grant-checked at the transport layer, audited, and reconciled by cursor — no `OperationState`. 3 reserved property ids.

**State variables:**

```quint
module subscription_authority {
  pure val ACTORS = Set("alice")
  pure val STREAMS = Set("stream-ops", "stream-elicitations")
  pure val FILTERS = Set("all", "ops-only")
  pure val CURSORS = 0.to(5)

  var subscriptionGrantLive: str -> bool           // (actor, stream, filter) -> grant live
  var subscriptionEstablished: Set[str]             // established subscription ids
  var subscriptionCursor: str -> int               // subscription id -> last delivered LSN
  var auditRecords: Set[str]                       // audit ids for allow/deny decisions
  var eventLsn: int                               // current event log LSN
  var eventFilter: int -> str                      // event LSN -> stream/filter it belongs to
}
```

**Actions:**
- `establishSubscription(actor, stream, filter)` — permissive (any actor/stream/filter may attempt); succeeds only if `subscriptionGrantLive` for that scope; always creates an audit record (allow or deny).
- `emitEvent(stream, filter)` — advances `eventLsn`; records the event's stream/filter.
- `replayByCursor(subscriptionId, cursor)` — returns only events with `LSN > cursor` within the authorized subscription filter.

**Checked properties** (3, `tier: checked-model, status: promoted`):
- `SubscriptionGrantChecked` (invariant) — a subscription is established only when the actor has a live grant for the stream/filter scope.
- `SubscriptionAudited` (invariant) — every establish attempt (allow or deny) creates an audit record; no Operation record is created.
- `SubscriptionCursorReplayAuthorized` (invariant) — reconnect replay returns only events with `LSN > cursor` within the authorized subscription filter.

**Acceptance Criteria:**
- [ ] `quint parse` + `quint compile` exit 0.
- [ ] All 3 properties pass.
- [ ] Mutation test for `SubscriptionGrantChecked`: allowing establish without a live grant fails the property.
- [ ] `@promotion` blocks present; `check-models.mjs` exits 0; VERIFICATION.md updated.

---

#### Unit TC: TypedCorrelation extension

**File**: `specs/seed/reply_correlation.qnt` (extend in place)
**Story**: spawned as `story-formal-model-realignment-typed-correlation`

Extends the checked `TypedCorrelation` model to cover response Operation → Elicitation typed correlation. Depends on Unit EL (the ElicitationId space must be defined). Extends the existing `reply_correlation.qnt` rather than a new model because typed correlation is one cohesive property across id spaces.

**Extension:** add `ELICITATION_ID_SPACE` and response-Operation correlation. A response Operation (`kind = approval-response | elicitation-response`) correlates by typed reference to a known prior `ElicitationId` in the same authority/session/responder context. The five id spaces (CommandId, MessageId, ReplyId, EventId, ElicitationId) remain disjoint.

**Checked property:** the existing `TypedCorrelation` invariant is extended to cover the response→Elicitation case. The `@promotion` block's `semantics` field is updated to reflect the broader coverage; the property id remains `TypedCorrelation` (the extension is a coverage expansion, not a new property). A new reserved property id `OperationResponseCorrelationTyped` is **not** introduced — the design decision is that `TypedCorrelation` is the single checked property for all typed-correlation cases, and the VERIFICATION.md "TypedCorrelation extension" stated-normative bullet is narrowed to note the response→Elicitation case is now covered.

**Acceptance Criteria:**
- [ ] `quint parse` + `quint compile` exit 0.
- [ ] Extended `TypedCorrelation` passes (now covering response→Elicitation).
- [ ] Mutation test: a response Operation using a `ReplyId`/`EventId`/`CommandId` as `ElicitationId` is rejected (forgery prevented).
- [ ] `reply_correlation.emitted.tla` regenerated.
- [ ] VERIFICATION.md "TypedCorrelation extension" bullet narrowed.

---

## Implementation Order

1. **Unit TR + Unit M** (one story: `story-formal-model-realignment-traceability`) — the traceability script + the VR2/VR4 metadata edit, done together because the check validates the edit. Establishes the closed loop first.
2. **Unit CL** (`story-formal-model-realignment-adjacency`) — V1 strengthening; highest regression risk, run early while the surface is small. Re-verifies all 7 existing properties as the regression gate.
3. **Unit EL** (`story-formal-model-realignment-elicitation`) — Elicitation lifecycle; highest safety criticality among new arcs.
4. **Unit SA** (`story-formal-model-realignment-spawn`) — Spawn authority; committed v0 behavior with no model.
5. **Unit SUB** (`story-formal-model-realignment-subscription`) — Subscription authority; second authority mechanism.
6. **Unit TC** (`story-formal-model-realignment-typed-correlation`) — TypedCorrelation extension; depends on Unit EL.

Units 3–6 (EL, SA, SUB, TC) are independent of each other except TC depends on EL. After Unit CL lands, EL/SA/SUB can run in parallel (3 independent new files); TC follows EL.

## Testing

There is no implementation *code* (Rust/TS) — verification is by running the checkers and the traceability script:

- **Parse + typecheck gate:** `quint parse` + `quint compile` on every `.qnt` (including the 3 new models).
- **Apalache invariant checks:** `quint verify --invariant <v> --max-steps N` exit 0 for one-state properties.
- **Apalache temporal checks:** `echo y | quint verify --temporal <p> --max-steps 10` exit 0 for two-state properties.
- **Alloy checks:** `java -jar org.alloytools.alloy.dist.jar exec --command <label> --type text --output - <file>.als` — UNSAT confirmed by absence of skolem witnesses (reliable method).
- **Mutation-test proof (genuine-checking):** for every newly-promoted property, break the action's guard/predicate and confirm the property fails. This is the discipline proven in the seed arc and the load-bearing proof that a property is not self-defining/vacuous.
- **Traceability check:** `node contracts/scripts/check-models.mjs` exit 0 — the closed loop that catches metadata drift.
- **Regression gate (Unit CL):** all 7 existing checked properties re-run after the transition strengthening.
- **Vector check:** `node contracts/scripts/check-vectors.mjs` exit 0 — the hardcoded arrays updated to track model metadata.

## Risks

- **Regression risk (Unit CL — highest):** strengthening `command_lifecycle.qnt`'s transition relation could break the 7 existing checked properties. **Mitigation:** grounding discovery #2 shows the properties are adjacency-independent (they quantify over `TERMINAL` as a set), so the risk is low by construction — but the regression gate (re-run all 7) is mandatory, not optional. If a property fails, the strengthening is wrong, not the property.
- **Vacuity risk (Unit CL `NoAcceptedToCompleted`):** if the `advance` action (non-terminal→non-terminal) is omitted, `completed` becomes unreachable from `accepted` under the strengthened adjacency, making `NoAcceptedToCompleted` vacuously true. **Mitigation:** the `advance` action is part of the design; the mutation test (break `allowedTransition` to allow `accepted → completed`) is the non-vacuity proof.
- **Apalache temporal experimental support (carried forward):** all temporal properties rely on Apalache's experimental temporal support (the `idea-tlc-temporal-workaround` residual from the seed arc). The new temporal properties (`NoAcceptedToCompleted`, `ElicitationPendingFinality`, `ElicitationFirstAnswerWins`, `ElicitationStaleTargetInert`, `ElicitationWithdrawalFinality`, `SpawnRevocationDoesNotCascade`) inherit this residual risk. **Mitigation:** all are `always(...)` safety (not `eventually` liveness), the more conservative end of Apalache's temporal support; emitted TLA+ is inspectable.
- **Traceability script parser fragility:** the `@promotion` block parser is a regex extractor. If a model author malforms a block, the parser may silently skip it (coverage failure) or misparse a field. **Mitigation:** the coverage check (every VERIFICATION.md property has a block, every block names a VERIFICATION.md property) catches omission; the well-formedness check (invocation non-empty, tier is a known value) catches malformation. A malformed block that parses to a valid-but-wrong tier is caught by the tier-agreement check.
- **Scope creep (new model arcs):** the 4 new model arcs (EL, SA, SUB, TC) are substantial. **Mitigation:** each is a separate child story with its own acceptance criteria and mutation-test proof; the implementation orchestrator can fan them out in parallel (after CL) without coupling.
- **Hardcoded-array drift (Unit M):** moving `browser_local_state_not_authority` between the `check-vectors.mjs` arrays is a manual edit that could itself drift. **Mitigation:** Unit TR's drift-detection check (#5) makes the arrays a checked-in cache of the model metadata; if they disagree with the `@promotion` blocks, the script fails. This converts the arrays from independent source-of-truth to derived.

## Extension pressure classification

- **Committed v0:** VR2 metadata realignment (rename to `checked-model`); VR4 promotion of `browser_local_state_not_authority`; V1 `NoAcceptedToCompleted` checked property; the 4 new model arcs (Elicitation, Spawn, Subscription, TypedCorrelation extension) promoting their reserved property ids to checked-model. The traceability check script is committed v0 infrastructure.
- **Reserved seam:** the read/query no-direct-to-completed fast-path rule remains stated-normative (requires OperationKind-specific skip-`running` modeling — a larger surface, reserved for a follow-on). The full transition-graph adjacency (beyond no-`accepted → completed`) remains stated-normative; this feature checks only the single highest-safety adjacency. Multi-answer Elicitation contracts, tighter responder binding, and cascade revocation remain reserved (consistent with the O/O/E feature's reserved seams).
- **Explicitly rejected for v0:** none newly rejected. The decision to strengthen `command_lifecycle.qnt` in place (rather than author `operation_lifecycle.qnt`) is a v0 implementation choice, not a rejection of the separate-model approach — a future OperationState-specific model remains a valid promotion path if refinement-equivalence debt becomes undesirable.

## Other agent review

(To be populated by the adversarial reviewer — this feature is flagged for a heavy adversarial review pass after the initial design, per the operator's instruction.)
