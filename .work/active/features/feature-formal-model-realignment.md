---
id: feature-formal-model-realignment
kind: feature
stage: drafting
tags: [verification, protocol, foundation, prose]
parent: epic-foundation-hardening
depends_on: [feature-operator-presence-and-action-inventory, feature-formal-model-seed]
created: 2026-07-06
updated: 2026-07-05
gate_origin: null
release_binding: null
---

# Feature: Re-align seed formal models with the rolled-forward foundation

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
