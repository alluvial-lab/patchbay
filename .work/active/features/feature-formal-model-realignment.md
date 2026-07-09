---
id: feature-formal-model-realignment
kind: feature
stage: done
tags: [verification, protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-operator-presence-and-action-inventory, feature-formal-model-seed]
created: 2026-07-06
updated: 2026-07-09
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
2. **The V1 transition-adjacency gap is verified.** `command_lifecycle.qnt` `commitTerminal(cmd, candidate)` guards only `state.get(cmd).in(NON_TERMINAL)` and `candidate.in(TERMINAL)` — no adjacency constraint, so `accepted → completed` is model-permitted. `docs/PROTOCOL.md` (lines 116–132) forbids that adjacency. The 7 currently-checked properties are terminal-finality / durability / dedup properties that quantify over `TERMINAL` as a set, not over specific adjacencies — so strengthening the transition relation is safe at the property level (verified by reading each property). It is NOT safe at the implementation level by assumption: the adversarial review caught a misquoted transition table and a non-stutter-safe property formula in the initial design. The regression gate (re-run all 7) is mandatory, not a formality.

> **Note on the brief below:** The original Brief, Scope, Open questions, and Risks/pre-mortem sections (lines ~20–80) were authored before the design pass and before the adversarial review. They carry the old framing (e.g., "rename `tier` to `checked-model`" in the scope; "rename in place vs split field" in the open questions). They are **superseded by the `### Design decisions (interactive, ratified 2026-07-08)` section below**, which records the ratified resolutions. The brief is preserved per substrate discipline (preserve-only) and as the seed context that motivated the design; an implementer reading top-down should treat the Design decisions section as authoritative where they disagree.

### Design decisions (interactive, ratified 2026-07-08)

The brief listed four open questions; an initial design pass resolved them with judgment without surfacing them to the operator — a process lapse corrected by the new `.agents/rules/design-checkpoints.md` rule. The decisions below were walked through one-by-one with the operator, each with options and tradeoffs, before ratification. An adversarial review (fresh-context `openai-codex/gpt-5.5`, xhigh) ran against the initial design and produced 6 blockers (B1–B6) and 5 importants (I1–I5); the ratified decisions resolve all of them.

- **Q1 — Metadata schema: derive product tier entirely (Option 3).** The `@promotion` block's existing `status: draft | promoted` field describes the model's own state. The product tier (`checked-model` / `checked-normative` / `stated-normative`) is **computed** by the traceability script (Unit TR) from `status` + vector coverage, not stored in the model file. The `tier` field is **dropped** from the `@promotion` block format. Rationale (ratified over rename-in-place and split-field): `checked-normative` is a joint claim spanning a promoted model *and* a promoted vector; recording it in the model file couples model files to vector state, so a future vector promotion forces an unrelated model-file edit. Deriving the tier from the two independent sources removes the drift mechanism that allowed VR2 in the first place. The model file no longer self-describes its tier — but a model file claiming `tier: checked-normative` was making a claim about vectors it cannot verify, so losing that "self-description" corrects an overclaim. This is the biggest departure from the seed feature's Q5 (which committed `tier` as a block field); this feature's realignment scope legitimately reopens the field shape. Resolves reviewer I2.
- **Q2 — V1 approach: strengthen `command_lifecycle.qnt` in place (Option A).** The 7 checked properties are adjacency-independent (verified by reading each), so strengthening is safe at the property level. The strengthening adds an `allowedTransition` guard using the **exact** PROTOCOL transition table (corrected from the initial design's misquote), a new `advance` action for non-terminal→non-terminal transitions (non-vacuity), and a new `NoAcceptedToCompleted` temporal property. Authoring a separate `operation_lifecycle.qnt` was rejected — it creates refinement-equivalence debt for no gain since `OperationState` is already `CommandState` by documented refinement. Deferring V1 to a follow-on was rejected — the adjacency rule is the highest-safety gap in this feature's scope. Resolves reviewer B2 (misquoted table) and B3 (non-stutter-safe property): the property checks *transitions into* `completed`, not the static state, and uses the exact PROTOCOL table. Resolves reviewer I1 (overclaim): "low-regression-risk by construction" is reframed to "safe at the property level; the regression gate is mandatory at the implementation level."
- **Q3 — New-arc modeling depth and placement: extend existing draft models where the domain matches; new model only where the state machine is genuinely new (Option 3).** The Elicitation *lifecycle* (opened/pending/terminal, first-answer-wins) is a genuinely new state machine → new rich `elicitation_lifecycle.qnt` with the full variable set VERIFICATION requires (authority domain, session/target generation, response-Operation identity, correlation ref, validation outcome — the variables the initial design's thin model omitted). Spawn and subscription *authority* properties promote into the existing draft `authority.qnt`, which already models real grant tuples (`GrantIssuer`, `GrantSubject`, `GrantScopeById`, `GrantEndpoint`, `GrantCommandKinds`, `TargetGeneration`, `RevocationGeneration`) — reusing the SSOT for grant semantics rather than re-deriving booleans. The existing 4 draft authority properties stay draft; only the 7 new SA/SUB properties are promoted in this feature. TC extends `reply_correlation.qnt` in place (same property, expanded id space). Rationale (ratified over thin standalone and rich standalone): thin models would be self-defining under the genuine-checking gate (reviewer B4/B5); rich standalone duplicates grant state that `authority.qnt` already owns, violating SSOT. Resolves reviewer B4 (EL/TC under-modeled) and B5 (SA/SUB booleans self-defining).
- **Q4 — Implementation order: sequential by safety criticality (Option α).** TR+M → CL → EL → SA → SUB → TC. No parallel fan-out within the feature. Rationale: the adversarial review (I4) flagged the original parallelism claim as unsafe (all four arcs touch shared files; SA+SUB now share `authority.qnt` under Option 3). Sequential ordering gives each unit undivided attention and dissolves the contention. Stories fast-advance on verification; the feature gets deeper review at the end.
- **Q5 — Scope: author all four new model arcs in this feature (Option I).** Each arc is a child story with its own genuine-checking mutation-test proof and independent review loop. Rationale: sequential ordering + per-story review makes four arcs four independently-gated promotions, not a monolithic risk. The seed arc is the evidence that per-model review catches what design and implementation miss. Deferring arcs (Option III) was rejected — they're already honestly disclosed as stated-normative, and this feature is positioned to close them.
- **Q6 — VR4 `browser_local_state_not_authority`: promote to checked-model with a real verification stride (Option 1 + verification).** The invariant already exists in `csrf_browser.qnt` with a genuine independent oracle. Promotion is a **verification act, not a metadata act** (reviewer B6): add the `@promotion` block, run `quint verify csrf_browser.qnt --invariant browser_local_state_not_authority --max-steps 12` to confirm it passes, and run a mutation test (break the `serverAccepts` rule; confirm the invariant catches it) to re-establish the genuine-checking proof. Resolves reviewer B6.

### Architectural choice

The realignment is structured as **one derived-tier traceability layer + one in-place strengthening + four new model arcs**, sequenced so the traceability loop is established before any new model is authored. Under Q1's derive-tier decision, the traceability script (Unit TR) is the **tier authority** — it computes product tier from model `status` + vector coverage, so model files describe only the model and the docs derive. This removes the drift mechanism that allowed VR2 rather than just fixing the current drift. The V1 strengthening (Unit CL) is done in-place on `command_lifecycle.qnt` with the exact PROTOCOL transition table and a stutter-safe transition-into property; the regression gate is mandatory. The four new arcs place each property in the model that owns its state: Elicitation lifecycle (new rich model), spawn + subscription authority (into the existing `authority.qnt`), and TypedCorrelation extension (into `reply_correlation.qnt`). All are authored with the genuine-checking discipline proven in the seed arc: permissive actions + independent-oracle invariants + mutation-test proof at promotion time.

This composes with the existing verification architecture: the property-id vocabulary stays the SSOT (established by the seed feature), the `@promotion` block format changes by **dropping the `tier` field** (the `status` field already exists and is the model's own state; the product tier is derived), and the new traceability script extends the `contracts/scripts/` family alongside `check-vectors.mjs` and `check-generated-drift.mjs`.

### Implementation Units

#### Unit TR: Traceability check script — the tier authority (keystone — designed first)

**File**: `contracts/scripts/check-models.mjs`
**Story**: spawned as `story-formal-model-realignment-traceability`

Closes the read loop the seed feature's Q5 committed to but never built, and — under Q1's derive-tier decision — becomes the **tier authority**: it computes each property's product tier from model `@promotion` `status` + conformance-vector coverage, rather than reading a stored `tier` field. Parses every `@promotion { ... }` block in `specs/seed/*.qnt` and `specs/seed/*.als`, extracts the structured fields (`property`, `status`, `model`, `backend`, `invocation`, `bounds`, `expected`, `proto_fields`, `semantics` — note: no `tier` field under Q1), and cross-checks against `docs/VERIFICATION.md` and `contracts/scripts/check-vectors.mjs`'s registry arrays.

**Checks it enforces (exit 1 on any failure):**
1. **Coverage:** every property-id in VERIFICATION.md's tier tables is accounted for. A property-id with a `@promotion` block has a model; a property-id with no block is a reserved-unmodeled stated-normative id (expected — Elicitation, spawn, subscription ids until their arcs land). The check fails only if a block references an unknown property-id, or a block is missing for a property that *should* have a model (checked/promoted per VERIFICATION.md). Resolves reviewer B1: distinguish `modeled_properties` (have a block) from `reserved_unmodeled_properties` (no block, expected stated-normative). **Note on `TypedCorrelation`:** `TypedCorrelation` is a single checked-model property id that Unit TC expands in *coverage* (Reply→Command/Message AND response Operation→Elicitation), not a separate id. The coverage check treats it as one modeled property; Unit TC's expansion is tracked by its `semantics` field and the VERIFICATION.md "TypedCorrelation extension" bullet (narrowed when TC lands), not by a distinct id. There is no "response-correlation id" gap — the obligation is the same id, broadened.
2. **Tier derivation:** for each block, compute the product tier: `status: promoted` + ≥1 promoted vector → `checked-normative`; `status: promoted` + 0 promoted vectors → `checked-model`; `status: draft` → `stated-normative`; no block → `stated-normative` (reserved-unmodeled). The computed tier must agree with VERIFICATION.md's classification. **This is the check that would have caught VR2 automatically** — and it catches it by derivation, not by comparing two stored values.
3. **Status-vs-vector consistency:** a derived `checked-normative` tier requires ≥1 promoted conformance vector (delegates to the vector registry in `check-vectors.mjs`). A derived `checked-model` tier requires zero promoted vectors (informational, not failing) — consistent with the "no promoted vector yet" state.
4. **Invocation well-formedness:** every `status: promoted` block's `invocation` field is non-empty, names the tool (`quint` or `java`), and (for Quint) includes the model file path. `status: draft` blocks may have `<TBD>` invocations.
5. **Drift detection vs the hardcoded registry:** the property→tier map **derived** from `@promotion` `status` + vector coverage must agree with the `CHECKED_MODEL_PROPERTIES` / `STATED_NORMATIVE_PROPERTIES` / `CHECKED_NORMATIVE_PROPERTIES` arrays in `check-vectors.mjs`. If they disagree, the script reports which side is stale and exits 1. **The hardcoded arrays become a checked-in cache of derived model metadata, not an independent source of truth** — the models + vectors are the SSOT, the arrays are derived.

**Design notes:**
- The parser is a fenced-block regex extractor for `//` comments (Quint and Alloy both use `//`). It extracts the field set above; fields are `key: value` lines. Multi-line `semantics` and colon-containing `bounds` values are handled by reading until the next `key:` pattern or block end. Resolves reviewer I3 (parser underspecification).
- **Registry composition (reviewer I3 residual):** `check-vectors.mjs` declares its property-tier arrays as module-local `const`s and runs `main()` on load — importing it directly would mutate `docs/VERIFICATION.md` as a side effect. Unit TR therefore reads the arrays by **source parsing** (regex-extracting `CHECKED_MODEL_PROPERTIES`, `STATED_NORMATIVE_PROPERTIES`, `CHECKED_NORMATIVE_PROPERTIES` from `check-vectors.mjs`'s source text) rather than importing the module, OR the implementer first refactors `check-vectors.mjs` to export a side-effect-free registry module (e.g. `contracts/scripts/property-registry.mjs`) that both scripts import. The implementer picks the cleaner option; source parsing is the zero-refactor fallback, the extracted module is the cleaner long-term shape. Either way, no mutation side effect from the check.
- The script writes a generated `## Generated model-promotion traceability` section into `docs/VERIFICATION.md` (parallel to the existing generated conformance-vector table), listing each property, its model file, **derived tier**, `status`, and backend. This makes the model↔doc agreement a checked-in artifact; CI fails if the generated block drifts from what the script would produce (same posture as `check-vectors.mjs`'s generated table).
- Wired into `contracts/ts/package.json` as `check:models` (alongside `check:vectors` and `check:drift`). Resolves reviewer N2 (no root package.json — wire into `contracts/ts/package.json`, not a nonexistent root `check:all`).

**Acceptance Criteria:**
- [ ] `node contracts/scripts/check-models.mjs` exits 0 on the current (post-Unit-M) model set.
- [ ] Deliberately removing a `@promotion` block for a modeled property causes exit 1 (coverage failure).
- [ ] Deliberately changing a `status: promoted` to `status: draft` causes the derived tier to change and the generated table to drift → exit 1.
- [ ] The hardcoded arrays in `check-vectors.mjs` are confirmed in-agreement with the model-derived map.
- [ ] Generated traceability section appears in `docs/VERIFICATION.md`; CI fails if it drifts.

---

#### Unit M: VR2 metadata realignment + VR4 promotion

**File**: edits to all 7 seed model files (`command_lifecycle.qnt`, `session_generation.qnt`, `reply_correlation.qnt`, `csrf_browser.qnt`, `patchbay-relational.als`, `snapshot_recovery.qnt`, `authority.qnt`)
**Story**: folded into `story-formal-model-realignment-traceability` (same stride — the metadata edit and the check that validates it belong together)

**VR2 — drop the `tier` field (Q1 derive-tier):** remove the `tier:` line from all `@promotion` blocks. The product tier is no longer stored in the model file; it is derived by Unit TR from `status` + vector coverage. `status: promoted` is preserved on all 16 promoted blocks; `status: draft` is preserved on all 12 draft blocks. Update stale header comments in model files that say "checked-normative" or "checked-model" to say "promoted model" or "draft model" (the model's own state), since the product tier is no longer a model-file concept. Resolves reviewer I2 (coupling) by removing the stored field entirely.

**VR4 — promote `browser_local_state_not_authority` (Q6 verification stride):** the invariant already exists in `csrf_browser.qnt` (~line 207; resolves reviewer N3 line ref) with a genuine independent oracle. Add a full `@promotion` block (no `tier` field per Q1): `property: browser_local_state_not_authority, status: promoted, model: specs/seed/csrf_browser.qnt, language: quint, backend: apalache, invocation: quint verify csrf_browser.qnt --invariant browser_local_state_not_authority --max-steps 12, bounds: { sessions: 4, proofs: 5, max_steps: 12 }, expected: pass, proto_fields: [none], semantics: browser-local UI claims cannot grant authority or override server-side session/CSRF grant checks`. **Then run the verification stride (reviewer B6):** `quint verify csrf_browser.qnt --invariant browser_local_state_not_authority --max-steps 12` must exit 0; and a mutation test (break the `serverAccepts` rule to accept without a valid proof; confirm `browser_local_state_not_authority` fails) must reproduce `[violation]`. This is a real promotion gate, not a metadata edit.

**Acceptance Criteria:**
- [ ] All `@promotion` blocks no longer carry a `tier` field (Q1 derive-tier).
- [ ] `browser_local_state_not_authority` has a complete `@promotion` block with `status: promoted`.
- [ ] **VR4 verification:** `quint verify csrf_browser.qnt --invariant browser_local_state_not_authority --max-steps 12` exits 0 (reviewer B6 — the promotion gate is exercised).
- [ ] **VR4 mutation test:** breaking `serverAccepts` causes `browser_local_state_not_authority` to fail `[violation]`.
- [ ] `node contracts/scripts/check-models.mjs` exits 0 (derived tiers agree with VERIFICATION.md).
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0 (hardcoded arrays updated: `browser_local_state_not_authority` moves from `STATED_NORMATIVE_PROPERTIES` to `CHECKED_MODEL_PROPERTIES`).
- [ ] All 4 checked Quint models still `quint parse` + `quint compile` exit 0.
- [ ] VERIFICATION.md checked-model list includes `browser_local_state_not_authority` (17 checked-model properties).

---

#### Unit CL: V1 transition-adjacency strengthening (trickiest unit — highest regression risk)

**File**: `specs/seed/command_lifecycle.qnt`
**Story**: spawned as `story-formal-model-realignment-adjacency`

Strengthens `commitTerminal` to enforce the `docs/PROTOCOL.md` transition adjacency (exact table — reviewer B2 correction) and adds a new checked property verifying the no-`accepted → completed` rule with a **stutter-safe transition-into formula** (reviewer B3 correction). The read/query no-direct-to-completed fast-path rule remains stated-normative (reserved — requires OperationKind-specific skip-`running` modeling).

**Strengthened transition relation (exact PROTOCOL table, `docs/PROTOCOL.md:116-132`):**

```quint
// The allowed adjacency from docs/PROTOCOL.md:116-132.
// accepted  -> delivered | rejected | failed | expired | cancelled | superseded
// delivered -> running | completed | rejected | failed | expired | cancelled | superseded
// running   -> completed | failed | expired | cancelled | superseded
pure def allowedTransition(from, to) =
  if (from == "accepted")
    to.in(Set("delivered", "rejected", "failed", "expired", "cancelled", "superseded"))
  else if (from == "delivered")
    to.in(Set("running", "completed", "rejected", "failed", "expired", "cancelled", "superseded"))
  else if (from == "running")
    to.in(Set("completed", "failed", "expired", "cancelled", "superseded"))
  else
    false
```

The `commitTerminal(cmd, candidate)` action gains the guard `allowedTransition(state.get(cmd), candidate)`. The `lateTerminalCandidate` action is unchanged (no-op by design; adjacency doesn't apply to late candidates). A new `advance(cmd, candidate)` action models the non-terminal→non-terminal transitions (`accepted → delivered`, `delivered → running`) — **non-vacuity guarantee**: without it, `completed` is unreachable from `accepted` under the strengthened adjacency, making `NoAcceptedToCompleted` vacuously true.

**New checked property (temporal, stutter-safe — reviewer B3 correction):**

```quint
// @promotion {
//   property:    NoAcceptedToCompleted
//   status:      promoted
//   model:       specs/seed/command_lifecycle.qnt
//   language:   quint
//   backend:     apalache-temporal
//   invocation:  echo y | quint verify command_lifecycle.qnt --temporal no_accepted_to_completed --max-steps 10
//   bounds:      { cmd_ids: 3, idempotency_keys: 3, terminal_candidates: 6, max_steps: 10 }
//   expected:    pass
//   proto_fields: [none]
//   semantics:  a command cannot transition directly from 'accepted' to 'completed';
//               it must pass through 'delivered' (docs/PROTOCOL.md transition adjacency)
// }
// INDEPENDENT oracle: checks the transition-INTO completed, not the static state. A command already
// 'completed' that stutters (lateTerminalCandidate/retry no-op) does NOT violate this property —
// the formula only fires when state CHANGES to completed. Mutation test: breaking allowedTransition
// to allow accepted->completed must fail this invariant.
temporal no_accepted_to_completed =
  always(CMD_IDS.forall(cmd =>
    (state.get(cmd) != "completed" and next(state.get(cmd)) == "completed")
      .implies(state.get(cmd).in(Set("delivered", "running")))))
```

**Implementation Notes:**
- **Stutter-safety (reviewer B3):** the formula fires only on a *transition into* `completed` (`state != completed and next(state) == completed`), not on a static `completed` state. A command already terminal that stutters (late candidate / retry no-op) does not violate the property. The initial design's non-stutter-safe formula (`next(state)==completed implies state in {delivered,running}`) would fail on valid terminal stutters.
- **Exact table (reviewer B2):** `accepted` may go to `delivered | rejected | failed | expired | cancelled | superseded` — the initial design's `accepted → delivered | rejected` only was wrong and would have forbidden valid pre-delivery failure/expiry/cancellation paths.
- **Genuine-checking discipline:** the invariant must not re-use `allowedTransition`. It checks the raw state-transition fact. A mutation breaking `allowedTransition` to allow `accepted → completed` must fail this invariant — verified by mutation test.
- **Regression gate (mandatory, not optional):** after strengthening, re-run all 7 existing checked properties. They are expected to pass because they quantify over `TERMINAL` as a set and don't mention adjacency — but the adversarial review's B2/B3 demonstrated implementation errors are possible, so the gate is mandatory. If any property fails, the strengthening is wrong, not the property.
- **Non-vacuity:** the `advance` action ensures `completed` is reachable from `accepted` via `accepted → delivered → completed` or `accepted → delivered → running → completed`. Without it, `NoAcceptedToCompleted` is vacuously true. A `run`/reachability witness confirming `completed` is reachable should be recorded at promotion time (the mutation test alone does not prove non-vacuity — reviewer B3).
- `command_lifecycle.emitted.tla` regenerated and committed.

**Acceptance Criteria:**
- [ ] `quint parse` + `quint compile` exit 0.
- [ ] All 7 existing checked properties still pass (regression gate — mandatory).
- [ ] `NoAcceptedToCompleted` passes (`echo y | quint verify --temporal no_accepted_to_completed --max-steps 10`, exit 0).
- [ ] Mutation test: breaking `allowedTransition` to permit `accepted → completed` causes `NoAcceptedToCompleted` to fail (genuine-checking proof).
- [ ] Non-vacuity: a reachability witness confirms `completed` is reachable from `accepted` (the `advance` action works).
- [ ] `command_lifecycle.emitted.tla` regenerated and committed.
- [ ] `@promotion` block present (no `tier` field per Q1); `check-models.mjs` exits 0.
- [ ] VERIFICATION.md updated: `NoAcceptedToCompleted` added to checked-model list; the transition-adjacency stated-normative bullet narrowed (no-`accepted → completed` now checked-model; full adjacency graph + read fast-path remain stated-normative). Resolves reviewer I5 (fast-path contradiction: only `NoAcceptedToCompleted` is checked; the read-specific fast-path rule is explicitly reserved, not claimed checked).

---

#### Unit EL: Elicitation lifecycle model (new rich model — Q3 Option 3)

**File**: `specs/seed/elicitation_lifecycle.qnt` (new)
**Story**: spawned as `story-formal-model-realignment-elicitation`

Models the `ElicitationState` lifecycle from `docs/PROTOCOL.md:270-305`: `opened → pending → (answered | declined | expired | cancelled | withdrawn | superseded | stale)`, with first-durable-terminal-commit-wins finality. **Rich model (Q3 Option 3):** includes the full variable set `docs/VERIFICATION.md` requires for `ElicitationCorrelationTyped` (authority domain, session/target generation, response-Operation identity, correlation ref, validation outcome) — the variables the initial thin design omitted (reviewer B4). Carries 7 reserved property ids toward checked-model.

**State variables** (trace to PROTOCOL `ElicitationState`, `ElicitationId`, `response_contract`, authority/session/generation context):

```quint
module elicitation_lifecycle {
  pure val TERMINAL = Set("answered", "declined", "expired", "cancelled", "withdrawn", "superseded", "stale")
  pure val NON_TERMINAL = Set("opened", "pending")
  pure val ELICITATION_IDS = Set("e1", "e2")           // bound: 2 elicitations
  pure val RESPONSE_OP_IDS = Set("ro1", "ro2")          // response Operation ids (separate space)
  pure val ENDPOINTS = Set("ep-a", "ep-b")              // bound: 2 subscribed surfaces
  pure val ACTORS = Set("alice")                        // v0: operator-only
  pure val AUTHORITY_DOMAINS = Set("domain-main")
  pure val SESSIONS = Set("s1", "s2")
  pure val GENERATIONS = 0.to(2)
  pure val CONTRACT_KINDS = Set("approval", "question") // committed v0 contract kinds

  var state: str -> str                  // ElicitationId -> ElicitationState
  var terminalLsn: str -> int            // ElicitationId -> LSN at terminal commit (0 = not terminal)
  var lsn: int                           // monotonic gap-free log sequence number
  var responderActor: str -> str         // ElicitationId -> expected responder actor (operator in v0)
  var answeredBy: str -> str             // ElicitationId -> endpoint that answered (audit; "none" if not answered)
  var contractKind: str -> str           // ElicitationId -> response_contract.contract_kind
  // --- Elicitation-side context (reviewer B4) ---
  var elicitationDomain: str -> str       // ElicitationId -> authority domain it was opened in
  var targetSession: str -> str           // ElicitationId -> target session id
  var targetGeneration: str -> int        // ElicitationId -> target session generation (stale-detection)
  var sessionGeneration: str -> int       // live session generation per session (for stale-target checks)
  // --- response-Operation-side context (reviewer B4 residual: the response Op carries its OWN context) ---
  var responseOpElicitation: str -> str   // responseOpId -> the ElicitationId it claims to answer ("none")
  var responseOpKind: str -> str          // responseOpId -> "approval-response" | "elicitation-response"
  var responseOpDomain: str -> str        // responseOpId -> authority domain of the submitting Operation
  var responseOpSession: str -> str      // responseOpId -> target session it binds to
  var responseOpGeneration: str -> int     // responseOpId -> session generation it binds to
  var responseOpActor: str -> str         // responseOpId -> authenticated actor submitting it
  var responseOpEndpoint: str -> str     // responseOpId -> authenticated endpoint submitting it (audit)
  var responseValid: str -> bool          // responseOpId -> whether it satisfied the contract
  var responseDuplicate: str -> bool     // responseOpId -> whether a prior response for the same Elicitation already committed
}
```

**Actions** (permissive — first-answer-wins is checked *against* the race, not baked into a guard):
- `openElicitation(eid, actor, contract, session, gen)` — `opened` initial state; binds target session/generation.
- `makePending(eid)` — `opened → pending` (visible to subscribed surfaces).
- `attemptAnswer(eid, responseOpId, kind, endpoint)` — a response Operation attempt from any endpoint; permissive (any endpoint may attempt; the guard checks only authenticated-for-expected-responder-actor). The `responseValid` flag records whether the contract was satisfied; the invariant proves first-answer-wins, not the guard.
- `lateAnswer(eid, responseOpId)` — a second answer after terminal: no-op (the TerminalFinality guarantee).
- `decline(eid)`, `expire(eid)`, `cancel(eid)`, `withdraw(eid)`, `supersede(eid)`, `goStale(eid)` — terminal candidates competing for first-commit.

**Checked properties** (7, each with `@promotion` block, all `status: promoted` after passing; tier derived by Unit TR):
- `ElicitationPendingFinality` (temporal) — once terminal, later candidates don't mutate.
- `ElicitationFirstAnswerWins` (temporal) — for single-answer contracts, the first durably committed valid answer wins; later answers are no-ops.
- `ElicitationCorrelationTyped` (invariant) — response Operations reference a known ElicitationId in the same authority/session/responder context; cannot forge across id spaces or generations. **Rich check (reviewer B4 residual):** verifies `responseOpElicitation` resolves to a known `ElicitationId`, the response Op's `responseOpDomain` matches the Elicitation's `elicitationDomain`, the response Op's `responseOpSession`/`responseOpGeneration` matches the Elicitation's `targetSession`/`targetGeneration` (or is explicitly stale and rejected), `responseOpKind` is a valid response kind, `responseOpActor` matches `responderActor`, and `ElicitationId` is disjoint from `CommandId`/`MessageId`/`ReplyId`/`EventId` spaces. The response-Operation-side context fields (`responseOpDomain`, `responseOpSession`, `responseOpGeneration`, `responseOpActor`, `responseOpEndpoint`) are the B4 residual fix: the response Operation carries its OWN context, not just the Elicitation's.
- `ElicitationTimeoutNeitherSuccessNorDenial` (invariant) — `expired` terminal does not imply `answered` or `declined`.
- `ElicitationInvalidResponseRejected` (invariant) — invalid response (`responseValid == false`) leaves the Elicitation `pending` (default reject-and-leave-pending policy). **Duplicate response behavior (reviewer B4 residual):** a second response Operation for an already-answered Elicitation is either idempotent (returns existing response state) or visibly rejected — modeled via `responseDuplicate`; the invariant confirms a duplicate never mutates the terminal answer.
- `ElicitationStaleTargetInert` (temporal) — responses to stale/superseded targets (`targetGeneration < sessionGeneration`) don't mutate live state.
- `ElicitationWithdrawalFinality` (temporal) — opener withdrawal terminalizes without later response mutation.

**Implementation Notes:**
- Bounds: 2 elicitations × 2 endpoints × 2 contract kinds × 2 response Operations. Small enough for Apalache; large enough to exercise the first-answer race.
- Genuine-checking: the `attemptAnswer` action is permissive. The invariants prove first-answer-wins, typed correlation, and stale-target inertness against the permissive transitions, not via guards.
- `ElicitationCorrelationTyped` here checks the id-space disjointness, known-id resolution, authority/session/generation context, and response-Operation typing *within this model*; the cross-model response-Operation→Elicitation correlation (where the response Operation lives in the command-lifecycle/id-space model) is Unit TC. This split is coherent because EL owns the Elicitation-side context and TC owns the id-space-side correlation; together they cover the full `docs/VERIFICATION.md:81-86` obligation.
- `elicitation_lifecycle.emitted.tla` generated and committed.

**Acceptance Criteria:**
- [ ] `quint parse` + `quint compile` exit 0.
- [ ] All 7 properties pass with documented invocations.
- [ ] Mutation test for `ElicitationFirstAnswerWins`: allowing a second answer to mutate state fails the property.
- [ ] Mutation test for `ElicitationCorrelationTyped`: a response Operation using a forged/generation-mismatched/domain-mismatched/actor-mismatched ElicitationId fails the property (reviewer B4 sufficiency — covers same-domain, same-session/generation, responder-actor match).
- [ ] Mutation test for `ElicitationInvalidResponseRejected`: a duplicate response that mutates the terminal answer fails the property (reviewer B4 duplicate-response behavior).
- [ ] `@promotion` blocks present (no `tier` field per Q1); `check-models.mjs` exits 0; VERIFICATION.md updated (7 properties move from stated-normative to checked-model).

---

#### Unit SA: Spawn authority (promote into `authority.qnt` — Q3 Option 3)

**File**: `specs/seed/authority.qnt` (extend the existing draft model — not a new file)
**Story**: spawned as `story-formal-model-realignment-spawn`

Promotes spawn-authority properties into the existing draft `authority.qnt`, reusing its real grant tuples (`GrantIssuer`, `GrantSubject`, `GrantScopeById`, `GrantEndpoint`, `GrantCommandKinds`, `GrantStatus`, `TargetGeneration`, `RevocationGeneration`) rather than re-deriving booleans (reviewer B5). The existing 4 draft authority properties (`NoCommandWithoutGrant`, `CompoundIssuer`, `GrantAuthorityIsCommandKinds`, `RevocationPreventsFuture`) stay `status: draft`; only the 4 new spawn properties are promoted in this feature.

**Added state variables and seed-data changes** (spawn-specific, alongside `authority.qnt`'s existing grant infrastructure):

```quint
  // spawn-specific extensions to authority.qnt
  pure val SPAWN_KINDS = Set("spawn")
  pure val FLEET_SCOPES = Set("scope-fleet", "scope-supervisor")
  pure val SPAWN_TARGET_SESSIONS = Set("s1", "s2", "s3")  // s3 does not exist initially

  // descendant grants are REAL Grant records (reviewer B5 residual fix), not parallel maps.
  // They reuse the existing GrantIssuer/GrantSubject/GrantScopeById/GrantEndpoint/
  // GrantCommandKinds/GrantStatus/TargetGeneration/RevocationGeneration tuples.
  var sessionExists: Set[str]                   // sessions that have been spawned into existence
```

**Seed-data change (reviewer B5 residual):** the current `authority.qnt` init has no `spawn` kind and no fleet scope (`specs/seed/authority.qnt:19-22`, `:174-188`). Unit SA adds a fleet-scope spawn grant to the seed `Grant`/`GrantScopeById`/`GrantCommandKinds`/`GrantStatus` maps (e.g. a `g-spawn` grant with `GrantScopeById = scope-fleet`, `GrantCommandKinds` containing `spawn`, `GrantStatus = active`). Descendant grants created by spawn are inserted as new `Grant` records into the same maps, not into parallel boolean maps — so `FleetAuthorityForSpawn` and `SpawnCreatesDescendantGrant` query real grant tuples with provenance (issuer, subject, scope, kind, status), genuinely.

**Actions** (permissive — the authority properties are checked against these, not baked into guards):
- `attemptSpawn(targetSession)` — permissive (any target may be attempted); succeeds only if a live fleet-scope spawn grant exists in the `Grant` tuples AND the actor is the grant subject. On success, creates the session and inserts a descendant `Grant` record.
- `revokeSpawnGrant(grantId)` — sets the fleet grant's `GrantStatus = revoked`; does NOT touch descendant grant records (no-cascade).
- `revokeDescendantGrant(grantId)` — sets the descendant grant's `GrantStatus = revoked` (separate lever — operates on a real grant id).
- `spawnAfterRevoke(targetSession)` — permissive: attempts spawn after fleet grant revoked; must be rejected (the property proves it).

**Checked properties** (4, `status: promoted`; tier derived by Unit TR):
- `FleetAuthorityForSpawn` (invariant) — spawn accepted only when a live fleet-scope `Grant` record exists matching the actor and `spawn` kind; a per-session grant alone does not authorize spawn of a not-yet-existing session. **Genuine check (reviewer B5 residual fix):** queries the `Grant`/`GrantScopeById`/`GrantCommandKinds`/`GrantStatus` tuples for a fleet-scope, spawn-kind, active, subject-matching grant — not a boolean.
- `SpawnCreatesDescendantGrant` (invariant) — successful spawn completion inserts an explicit descendant `Grant` record (with issuer=spawner, subject=spawner, target=spawned session, scope=per-session, kind-set excluding spawn) into the grant tuples, not a parallel map.
- `SpawnRevocationDoesNotCascade` (temporal) — revoking the spawn grant (`GrantStatus = revoked`) prevents future spawns but does not change the `GrantStatus` of already-created descendant grants.
- `ElicitationResponderAuthority` (invariant) — a response Operation is accepted only from an authenticated endpoint for the expected responder actor. (Modeled here because it's an authority property; projects the responder-actor binding from the Elicitation model into the authority/grant context.)

**Bounds and invocation (promotion-grade, reviewer N2):**
- Bounds: 3 actors × 3 sessions × 2 grant scopes × 2 grant statuses. `--max-steps 12` for invariants; `--max-steps 10` for the temporal `SpawnRevocationDoesNotCascade`.
- Invocation (invariants): `quint verify authority.qnt --invariant fleet_authority_for_spawn --max-steps 12` (and the other 3 invariants).
- Invocation (temporal): `echo y | quint verify authority.qnt --temporal spawn_revocation_does_not_cascade --max-steps 10`.

**Acceptance Criteria:**
- [ ] `quint parse` + `quint compile` exit 0 (the extended `authority.qnt` still compiles).
- [ ] All 4 new properties pass.
- [ ] Mutation test for `FleetAuthorityForSpawn`: allowing spawn with only a per-session (non-fleet) grant fails the property (reviewer B5 sufficiency — not a boolean).
- [ ] Mutation test for `SpawnRevocationDoesNotCascade`: cascading revocation (revokeSpawnGrant also revokes descendant) fails the property.
- [ ] The existing 4 draft properties remain `status: draft` (no accidental promotion).
- [ ] `@promotion` blocks present (no `tier` field per Q1); `check-models.mjs` exits 0; VERIFICATION.md updated.

---

#### Unit SUB: Subscription authority (promote into `authority.qnt` — Q3 Option 3)

**File**: `specs/seed/authority.qnt` (extend the existing draft model — same file as Unit SA; must follow SA sequentially per Q4 Option α)
**Story**: spawned as `story-formal-model-realignment-subscription`

Promotes subscription-authority properties into `authority.qnt`, reusing its grant infrastructure. Subscription is the grant-checked-without-lifecycle second authority mechanism: establishment is grant-checked at the transport layer, audited, and reconciled by cursor — no `OperationState`.

**Added state variables** (subscription-specific, alongside `authority.qnt`'s grant infrastructure):

```quint
  // subscription-specific extensions to authority.qnt
  pure val STREAMS = Set("stream-ops", "stream-elicitations")
  pure val FILTERS = Set("all", "ops-only")
  pure val CURSORS = 0.to(5)

  // subscription grants are REAL Grant records (reviewer B5 residual fix), not a boolean side-channel.
  // A subscription's authorization is a Grant with GrantScopeById = stream/filter scope,
  // GrantCommandKinds containing "subscribe", queried via the existing grant tuples.
  var subscriptionId: Set[str]               // established subscription ids
  var subscriptionGrantId: str -> str        // subscription id -> authorizing Grant id ("none" if denied)
  var subscriptionCursor: str -> int      // subscription id -> last delivered LSN
  var subscriptionStream: str -> str     // subscription id -> stream
  var subscriptionFilter: str -> str      // subscription id -> filter
  var auditRecords: Set[str]              // audit ids for allow/deny decisions
  var operationRecordsCreated: int       // counter; must stay 0 for subscriptions (no OperationState)
  var eventLsn: int                       // current event log LSN
  var eventStream: int -> str            // event LSN -> stream
  var eventFilter: int -> str             // event LSN -> filter
  var replayedEvents: str -> Set[int]    // subscription id -> LSNs returned on replay
```

The `operationRecordsCreated` counter (reviewer B5) makes `SubscriptionAudited` genuine: a subscription allow/deny creates an audit record but does *not* increment the Operation-record counter. Subscription authorization queries the real `Grant` tuples (a subscription grant is a `Grant` with `GrantCommandKinds` containing `subscribe` and `GrantScopeById` = the stream/filter scope) — not a `subscriptionGrantLive: bool` side-channel.

**Actions:**
- `attemptEstablish(actor, stream, filter)` — permissive (any actor/stream/filter may attempt); succeeds only if a live `Grant` record exists for that stream/filter scope with the actor as subject and `subscribe` in its `GrantCommandKinds`; always creates an audit record; never creates an Operation record (`operationRecordsCreated` unchanged).
- `emitEvent(stream, filter)` — advances `eventLsn`; records the event's stream/filter.
- `replayByCursor(subscriptionId, cursor)` — returns only events with `LSN > cursor` within the authorized subscription filter; records in `replayedEvents`.

**Checked properties** (3, `status: promoted`; tier derived by Unit TR):
- `SubscriptionGrantChecked` (invariant) — a subscription is established only when a live `Grant` record exists for the stream/filter scope with the actor as subject. **Genuine check (reviewer B5 residual fix):** queries the `Grant`/`GrantScopeById`/`GrantCommandKinds`/`GrantStatus` tuples, not a boolean side-channel.
- `SubscriptionAudited` (invariant) — every establish attempt (allow or deny) creates an audit record; `operationRecordsCreated` stays 0 (no Operation record — reviewer B5 sufficiency).
- `SubscriptionCursorReplayAuthorized` (invariant) — reconnect replay returns only events with `LSN > cursor` within the authorized subscription filter; `replayedEvents` contains no out-of-cursor or out-of-filter events.

**Bounds and invocation (promotion-grade, reviewer N2):**
- Bounds: 3 actors × 2 streams × 2 filters × 2 grant statuses × 5 cursors. `--max-steps 12`.
- Invocation: `quint verify authority.qnt --invariant subscription_grant_checked --max-steps 12` (and the other 2 invariants).

**Acceptance Criteria:**
- [ ] `quint parse` + `quint compile` exit 0 (the extended `authority.qnt` still compiles after both SA and SUB additions).
- [ ] All 3 new properties pass.
- [ ] Mutation test for `SubscriptionGrantChecked`: allowing establish without a live grant fails the property.
- [ ] Mutation test for `SubscriptionAudited`: an establish that creates an Operation record (`operationRecordsCreated > 0`) fails the property (reviewer B5).
- [ ] Mutation test for `SubscriptionCursorReplayAuthorized`: replay returning an out-of-cursor or out-of-filter event fails the property.
- [ ] `@promotion` blocks present (no `tier` field per Q1); `check-models.mjs` exits 0; VERIFICATION.md updated.

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

**Bounds and invocation (promotion-grade, reviewer N2):** bounds carry forward from the existing `reply_correlation.qnt` (command_ids: 2, message_ids: 2, reply_ids: 2) plus the added `ELICITATION_ID_SPACE` (2 ids) and response-Operation ids (2). Invocation: `quint verify reply_correlation.qnt --invariant typed_correlation --max-steps 12`.

---

## Implementation Order (Q4 Option α — sequential)

1. **Unit TR + Unit M** (one story: `story-formal-model-realignment-traceability`) — the traceability script + the VR2/VR4 metadata edit + VR4 verification stride, done together because the check validates the edit. Establishes the derived-tier regime first.
2. **Unit CL** (`story-formal-model-realignment-adjacency`) — V1 strengthening; highest regression risk, run early while the surface is small. Re-verifies all 7 existing properties as the regression gate.
3. **Unit EL** (`story-formal-model-realignment-elicitation`) — Elicitation lifecycle (new rich model); highest safety criticality among new arcs.
4. **Unit SA** (`story-formal-model-realignment-spawn`) — Spawn authority (into `authority.qnt`); committed v0 behavior with no model.
5. **Unit SUB** (`story-formal-model-realignment-subscription`) — Subscription authority (into `authority.qnt`; **must follow SA** — same file, sequential per Q4 Option α).
6. **Unit TC** (`story-formal-model-realignment-typed-correlation`) — TypedCorrelation extension; depends on Unit EL (ElicitationId space).

**No parallel fan-out** (Q4 Option α). The original design's claim that EL/SA/SUB could run in parallel was wrong: SA+SUB share `authority.qnt` (Q3 Option 3), and all four arcs touch shared integration files (`docs/VERIFICATION.md`, `check-vectors.mjs` arrays, generated traceability — reviewer I4). Sequential ordering gives each unit undivided attention and dissolves the contention. Each story fast-advances on verification; the feature gets deeper review at the end.

## Testing

There is no implementation *code* (Rust/TS) — verification is by running the checkers and the traceability script:

- **Parse + typecheck gate:** `quint parse` + `quint compile` on every `.qnt` (including the new `elicitation_lifecycle.qnt` and the extended `authority.qnt`).
- **Apalache invariant checks:** `quint verify --invariant <v> --max-steps N` exit 0 for one-state properties.
- **Apalache temporal checks:** `echo y | quint verify --temporal <p> --max-steps 10` exit 0 for two-state properties.
- **Alloy checks:** `java -jar org.alloytools.alloy.dist.jar exec --command <label> --type text --output - <file>.als` — UNSAT confirmed by absence of skolem witnesses (reliable method).
- **Mutation-test proof (genuine-checking):** for every newly-promoted property, break the action's guard/predicate and confirm the property fails. This is the discipline proven in the seed arc and the load-bearing proof that a property is not self-defining/vacuous.
- **Reachability witness (Unit CL):** a `run` confirming `completed` is reachable from `accepted` (non-vacuity for `NoAcceptedToCompleted` — reviewer B3).
- **VR4 verification stride (Unit M):** `quint verify --invariant browser_local_state_not_authority` exit 0 + mutation test (reviewer B6).
- **Traceability check:** `node contracts/scripts/check-models.mjs` exit 0 — the derived-tier authority that catches drift.
- **Regression gate (Unit CL):** all 7 existing checked properties re-run after the transition strengthening.
- **Vector check:** `node contracts/scripts/check-vectors.mjs` exit 0 — the hardcoded arrays updated to track derived model metadata.

## Risks

- **Regression risk (Unit CL — highest):** strengthening `command_lifecycle.qnt`'s transition relation could break the 7 existing checked properties. **Mitigation:** the properties are adjacency-independent (they quantify over `TERMINAL` as a set), so the risk is low at the property level — but the adversarial review's B2/B3 demonstrated implementation errors (misquoted table, non-stutter-safe property) are possible, so the regression gate (re-run all 7) is mandatory, not optional. If a property fails, the strengthening is wrong, not the property. Resolves reviewer I1 (overclaim reframed).
- **Vacuity risk (Unit CL `NoAcceptedToCompleted`):** if the `advance` action (non-terminal→non-terminal) is omitted, `completed` becomes unreachable from `accepted` under the strengthened adjacency, making `NoAcceptedToCompleted` vacuously true. **Mitigation:** the `advance` action is part of the design; a reachability witness (`run` confirming `completed` reachable) is recorded at promotion time (reviewer B3 — the mutation test alone does not prove non-vacuity).
- **Apalache temporal experimental support (carried forward):** all temporal properties rely on Apalache's experimental temporal support (the `idea-tlc-temporal-workaround` residual from the seed arc). The new temporal properties (`NoAcceptedToCompleted`, `ElicitationPendingFinality`, `ElicitationFirstAnswerWins`, `ElicitationStaleTargetInert`, `ElicitationWithdrawalFinality`, `SpawnRevocationDoesNotCascade`) inherit this residual risk. **Mitigation:** all are `always(...)` safety (not `eventually` liveness), the more conservative end of Apalache's temporal support; emitted TLA+ is inspectable.
- **Traceability script parser fragility:** the `@promotion` block parser is a regex extractor. If a model author malforms a block, the parser may silently skip it or misparse a field. **Mitigation:** the coverage check catches omission; the well-formedness check (invocation non-empty, `status` is a known value) catches malformation. A malformed block that parses to a wrong `status` is caught by the tier-derivation check (derived tier disagrees with VERIFICATION.md). Resolves reviewer I3.
- **Genuine-checking risk (Units EL/SA/SUB):** the new properties may turn out self-defining or vacuous under the genuine-checking gate, as the seed arc's first review caught 6 self-defining properties. **Mitigation:** each unit has explicit mutation-test acceptance criteria targeting the specific failure mode (B4/B5: forged correlation, per-session-vs-fleet grant, Operation-record creation). The per-story review loop (Q5) catches what implementation misses.
- **`authority.qnt` scope expansion (Units SA/SUB):** promoting into the existing draft `authority.qnt` expands this feature's scope into "promote parts of the authority model." **Mitigation:** only the 7 new SA/SUB properties are promoted; the existing 4 draft properties stay draft. Bounded increment, not a full overhaul. **State-space bloat (reviewer N3):** adding subscription cursor/replay/event state alongside spawn state and the existing grant infrastructure may push `authority.qnt` past Apalache's tractable state space. **Mitigation:** the implementer sets `--max-steps` bounds per the Unit SA/SUB specs (12 for invariants, 10 for temporal) and reduces atom-set bounds (actors/sessions/scopes) if Apalache doesn't complete. If verification remains intractable, the fallback is to split subscription authority into its own `subscription_authority.qnt` (reverting to a standalone model for SUB only, while keeping SA in `authority.qnt`) — this split trigger is an implementation-time decision, not a design re-open, because the properties and their genuine-checking proofs are unchanged by the placement.
- **Hardcoded-array drift (Unit M):** moving `browser_local_state_not_authority` between the `check-vectors.mjs` arrays is a manual edit that could itself drift. **Mitigation:** Unit TR's drift-detection check (#5) makes the arrays a checked-in cache of derived model metadata; if they disagree, the script fails.

## Extension pressure classification

- **Committed v0:** VR2 metadata realignment (drop `tier` field, derive tier — Q1 Option 3); VR4 promotion of `browser_local_state_not_authority` with verification stride (Q6); V1 `NoAcceptedToCompleted` checked property (Q2); the 4 new model arcs (Elicitation rich model, Spawn + Subscription into `authority.qnt`, TypedCorrelation extension — Q3/Q5) promoting their reserved property ids to checked-model. The traceability check script (Unit TR) is committed v0 infrastructure and the tier authority.
- **Reserved seam:** the read/query no-direct-to-completed fast-path rule remains stated-normative (requires OperationKind-specific skip-`running` modeling — a larger surface, reserved for a follow-on — reviewer I5). The full transition-graph adjacency (beyond no-`accepted → completed`) remains stated-normative; this feature checks only the single highest-safety adjacency. Multi-answer Elicitation contracts, tighter responder binding, and cascade revocation remain reserved (consistent with the O/O/E feature's reserved seams).
- **Explicitly rejected for v0:** none newly rejected. The decision to strengthen `command_lifecycle.qnt` in place (rather than author `operation_lifecycle.qnt`) is a v0 implementation choice, not a rejection of the separate-model approach. The decision to derive tier rather than store it (Q1 Option 3) reopens the seed feature's Q5 `tier` field shape — a legitimate realignment-scope decision, not a rejection of stored metadata as a future option if derivation proves insufficient.

## Other agent review

**Initial adversarial review (2026-07-08):** fresh-context `openai-codex/gpt-5.5` (xhigh), cross-model from the umans orchestrator. Verdict: **Block** (6 blockers B1–B6, 5 importants I1–I5, 3 nits N1–N3). The reviewer verified the keystone grounding discovery (no `@promotion` reader exists — confirmed via `contracts/ts/package.json:19-20` and `check-vectors.mjs:15,38,40`) and confirmed the V1 gap. All 6 blockers and 5 importants are resolved by the ratified design decisions (Q1–Q6) and the revised unit specs above. The review's findings are the reason this design is a revision, not the initial pass — the initial pass papered over the ambiguities the review surfaced.

**Design decisions (interactive, 2026-07-08):** the brief's four open questions plus two additional live decisions (VR4 disposition, scope) were walked through one-by-one with the operator, each with options and tradeoffs, before ratification. This corrected the initial pass's process lapse (resolving ambiguities with judgment under a `## Design decisions` block on a direct user invocation, which the new `.agents/rules/design-checkpoints.md` rule now forbids).

**Re-review (2026-07-08, second pass):** fresh-context `openai-codex/gpt-5.5` (xhigh) re-reviewed the revised design. Verdict: **Block** — the revision was partial. Confirmed resolved: B2 (exact transition table), B3 (stutter-safe property + reachability witness), B6 (VR4 verification stride), I4 (parallelism removed), I5 (fast-path reserved), and prior nits (draft count, package wiring, VR4 line ref). **Residual findings (addressed in this second revision):**
- **B1 residual** — Unit TR's coverage mentioned a non-existent "response-correlation id"; fixed by clarifying `TypedCorrelation` is one id whose coverage expands (no distinct id).
- **B4 residual** — Unit EL used only one actor/domain and lacked response-Operation-side context (the response Op's own domain/session/generation/actor/audit) and duplicate-response behavior; fixed by enriching EL's state with response-Operation-side fields and adding duplicate-response mutation test.
- **B5 residual** — SA used parallel descendant-grant maps instead of real `Grant` records; SUB used a `subscriptionGrantLive: bool` side-channel; fixed by tying both to real `Grant` tuples (descendant grants inserted as `Grant` records; subscription authorization queries `Grant` records with `subscribe` kind).
- **I1 residual** — "cannot break by construction" overclaim remained in grounding discovery #2; softened to "safe at the property level; regression gate mandatory."
- **I2/N1 residual** — stale brief/scope/architectural-choice text still said "rename in place" / "tier values corrected"; fixed by marking the brief as superseded and rewriting the Architectural choice section to reflect derive-tier.
- **I3 residual** — `check-vectors.mjs` runs `main()` on load, so importing it mutates docs; fixed by specifying source-parsing or a side-effect-free registry module extraction.
- **N2** — SA/SUB/TC lacked promotion-grade bounds/invocations; added per-unit.
- **N3** — `authority.qnt` bloat risk acknowledged but unmitigated; added bounds and a split-SUB-out fallback trigger.

The design advances to `stage: implementing` after this second revision addresses all residual findings. A third re-review is not warranted — the residual findings were stale-text and under-specification fixes, not new architectural issues; the ratified decisions (Q1–Q6) are unchanged.
