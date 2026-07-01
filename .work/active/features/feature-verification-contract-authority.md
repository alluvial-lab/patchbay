---
id: feature-verification-contract-authority
kind: feature
stage: done
tags: [verification, protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-command-state-ssot, feature-persistence-snapshot-model, feature-security-threat-model, feature-research-contract-tooling]
created: 2026-06-28
updated: 2026-06-30
gate_origin: null
release_binding: null
---

# Design: Verification, contract, and authority order

The docs currently say prose, formal models, generated contracts, and conformance vectors all matter, but they do not define which artifact is authoritative when they disagree. `docs/PROTOCOL.md` and `docs/ARCHITECTURE.md` both state PROTOCOL is the canonical source of truth "until generated schemas or IDL exist," but neither states the authority order *after* that transition. This feature defines it.

## Retag note (2026-06-28)

Retagged from `[prose]` to a design feature. The `prose` tag was removed because the scope includes architectural choices: the authority order among prose docs, formal models, IDL/schema, conformance vectors, and implementation (which artifact wins when they disagree) is a design decision. Generation targets and traceability rules are build-pipeline design. The v0 contract source is partially grounded by `feature-research-contract-tooling` but the authority-order question is not. The prose-author black-box test should have caught this originally.

## Scope

- Authority order among prose docs, TLA+/Quint models, Alloy models, IDL/schema, conformance vectors, and implementation.
- v0 contract source: Protobuf+Buf, JSON Schema, or explicit spike decision.
- Generation targets for Rust and TypeScript.
- Traceability from model properties to contract fields and conformance vectors.
- Model promotion criteria for v0.

## Acceptance criteria

- `docs/VERIFICATION.md` states artifact authority order and traceability rules.
- `docs/SPEC.md` states the v0 contract source or explicitly blocks durable protocol implementation until the spike resolves.
- `docs/PROTOCOL.md` distinguishes semantic authority from wire encoding authority.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.

## Relationship to committed docs and research

- `docs/PROTOCOL.md` opens with "this document is the canonical source of truth … until generated schemas or IDL exist" — a transition is committed but the post-transition order is not.
- `docs/ARCHITECTURE.md` boundary rules state PROTOCOL is the "prose source of truth for state registries until generated contracts exist" and that "generated contracts … derive … from the canonical protocol registry" — derivation direction is committed, authority order is not.
- `docs/SPEC.md` names a verification floor of "at least seed formal/property checks for command acceptance, idempotent retry, session identity, snapshots, and authority" (5 areas), and names Protobuf+Buf as the "default candidate" contract source.
- `docs/VERIFICATION.md` lists ~10 required model areas with properties, and already pins the model-promotion rule (property + bounds + tool invocation + pass/fail + product-semantics note).
- `feature-research-contract-tooling` (done, source-grounded) settled the contract source: **Protobuf + Buf**, Rust via prost/prost-build, TypeScript via Protobuf-ES, `buf.gen.yaml` checked in, `buf lint` + `buf breaking` in CI. This design ratifies that as the v0 contract source rather than re-asking.

SPEC's 5-area seed and VERIFICATION's ~10 required areas disagree on v0 modeling scope; this design reconciles them via a property-graded tier (Q2 below).

## Design questions to resolve

- **Q1 — Authority model**: linear precedence list, or question-type-layered (each artifact type owns a class of question)?
- **Q2 — v0 normative baseline**: which model areas must clear the normative bar before v0 treats their semantics as product behavior?
- **Q3 — Conformance-vector authority status**: are vectors normative, derived, or promotion-graded?
- **Q4 — Traceability mechanism**: prose mapping table, machine-readable per-vector metadata, or central registry?

## Design decisions

- **Q1 — Authority model: question-type-layered.** Authority is partitioned by the *type of question*, not by a single ranked list. Each artifact owns a class of question and is authority only for that class:
  - **Formal models (TLA+/Quint/Alloy)** — authority for *invariants and dynamic/relational properties*. A model is right about whether a behavior is allowed; prose is wrong if it contradicts a checked model's invariant.
  - **`.proto` (Protobuf+Buf)** — authority for *wire shape, field identity, enum vocabulary, and payload envelopes*. `.proto` owns what a command/event/reply *looks like on the wire*; models and prose do not redefine wire shape.
  - **Prose (`docs/PROTOCOL.md`, `docs/SPEC.md`, `docs/SECURITY.md`, `docs/ARCHITECTURE.md`)** — authority for *product intent and vocabulary naming*. Prose owns what a state/term *means as product behavior* and what the canonical registry names are; models and `.proto` derive names from prose registries.
  - **Conformance vectors** — authority for *expected executable examples*. A vector owns the expected outcome for a specific input/state scenario.
  - **Implementation** — never authority. If running code disagrees with a normative artifact, the code is the bug-fix target.
  A disagreement is routed to the artifact that owns that question type; "the higher artifact wins" is not a global rule.

- **Q2 — v0 normative baseline: property-graded (two tiers).** Replace the area-level all-or-nothing with a per-property risk tier, applied across and within areas:
  - **checked-normative** (must clear the model-promotion rule AND have ≥1 promoted conformance vector tracing to the property before v0 ships the behavior): the safety/security-critical properties — terminal-finality / first-durable-terminal-commit-wins, idempotent retry at the Patchbay boundary, session-generation/tombstone + wrong-session prevention, authority-grant checks, crash-recovery's "no accepted command disappears silently" + idempotent log replay, and the browser/CSRF rejection spine.
  - **stated-normative** (documented v0 obligation with a *draft* model, not yet checked-to-pass; scheduled for promotion post-v0): the liveness/cosmetic/operational properties — snapshot-convergence nuances (compaction, cursor validity), audit-integrity completeness, adapter-failure-vocabulary distinguishability refinements, reply-correlation edge cases.
  This reconciles SPEC's 5-area seed (the checked set is "seed done right" — the seed areas plus crash-recovery safety + the CSRF spine) with VERIFICATION's ~10 required areas (all are obligated at v0; only the safety-critical ones are checked-to-pass at v0). SPEC's verification-floor sentence is reconciled with VERIFICATION's required-areas list by this tier split rather than by picking one list.

- **Q3 — Conformance-vector authority status: normative-once-promoted.** Vectors are draft/derived until explicitly promoted (mirroring the existing model-promotion rule), then normative. Promotion requires the vector to declare which model property it exercises. A promoted vector is a peer authority for executable examples; a contradiction between a promoted vector and its model is a *detected, surfaced contradiction* (both normative, both cross-referenced), not a silent override by whichever artifact is "higher." The resolution is a reconciliation decision: either the model is wrong (update model → re-check every vector exercising it) or the vector is wrong (demote → fix → re-promote). This makes the layered authority (Q1) coherent on the model↔vector axis: a promoted vector genuinely owns executable examples as a peer authority, but only because it traces to a model property. Vectors are never authority for invariants (that's models) or wire shape (that's `.proto`).

- **Q4 — Traceability mechanism: machine-readable per-vector metadata + CI coverage check.** Each conformance vector file carries structured frontmatter naming: the model property it exercises, its promotion status, and the `.proto` fields/enums it constrains. A CI script reads all vectors and (a) fails if a checked-normative property lacks a promoted vector, (b) fails if a vector references a missing/misspelled property, (c) fails if a promoted vector's expected outcome contradicts its referenced model property's invariant (a surfaced contradiction, per Q3), and (d) generates the `docs/VERIFICATION.md` traceability table as a checked-in artifact so the human-readable mapping never drifts. Chosen over a central registry generator (Q4=C) because the repo is greenfield with no code yet — a separate registry is premature tooling (Late-Binding). Migration path to a central registry is explicit: if the property/vector set grows large, promote the per-vector metadata into a central source later; nothing about this choice blocks that.

## Architectural choice

Ratify a question-type-layered authority model (Q1=B) with a property-graded normative baseline (Q2=C) and promotion-graded conformance vectors (Q3=C) verified by machine-readable per-vector traceability (Q4=B). This is the only combination under which the layered authority is both coherent (Q3=C makes the model↔vector axis work) and checkable (Q4=B makes promotion status and coverage CI-verifiable) without over-building tooling before there is code (the greenfield argument against Q4=C).

The rejected alternatives were:

1. **Linear precedence (Q1=A)** — rejected because it misroutes. A prose typo should not override a checked model, but a model encoding a wrong product intent should defer to prose. A single ranked list cannot express "who is right depends on what kind of question it is"; routing every disagreement to the top of one list misattributes authority.
2. **Seed-only normative floor (Q2=A)** — rejected because it under-protects: it accidentally drops crash recovery and the browser/CSRF boundary from the v0 floor, yet both carry safety/security-critical claims ("no accepted command disappears silently after a crash"; "a state-changing request without an authenticated operator session is rejected before command acceptance"). SPEC's seed was a scope hint, not a safety analysis.
3. **Full required-set normative floor (Q2=B)** — rejected because it over-blocks: v0 cannot ship until ~10 model areas each clear the promotion bar AND each has a promoted vector. That is the formalism-tail-wagging-the-product-dog the epic review warned against.
4. **Vectors-as-flat-normative (Q3=A)** — rejected because of poisoning risk: a single vector encoding a wrong expected outcome becomes unchallengeable without a protocol-change ceremony, and under Q1=B that vector gets primacy on its partition. One bad promoted vector locks in a bug as "expected behavior."
5. **Vectors-as-flat-derived (Q3=B)** — rejected because it guts Q1=B on the model↔vector axis: if vectors are always downstream of models, they don't really own executable examples — it's a linear `models > vectors` order smuggled into a layered framing.
6. **Central-registry traceability (Q4=C)** — rejected as premature for a greenfield repo with no code yet; per-vector metadata (Q4=B) already eliminates hand-copy drift via the generated VERIFICATION.md table, without adding a generator before there is code to generate for. Reserved as an explicit migration path.

## Implementation Units

### Unit 1: State the layered authority order in VERIFICATION.md

**File**: `docs/VERIFICATION.md` (new section near the top, after the intro / before "TLA+ and Quint position")

```text
## Artifact authority order

Authority is question-type-layered, not a single ranked list. Each artifact type owns one class of question and is authority only for that class:

| Question type | Authority | Not authority for |
|---|---|---|
| Invariants, dynamic/relational properties | Formal models (TLA+/Quint/Alloy), once promoted | wire shape, product intent naming |
| Wire shape, field identity, enum vocabulary, payload envelopes | `.proto` (Protobuf+Buf) | invariants, product intent |
| Product intent, vocabulary naming, registry names | Prose (PROTOCOL.md, SPEC.md, SECURITY.md, ARCHITECTURE.md) | invariants, wire shape |
| Expected executable examples for a specific scenario | Conformance vectors, once promoted | invariants, wire shape, product intent |
| Anything | Implementation (never authority) | — |

Disagreements route to the artifact that owns the question type. A contradiction between two promoted artifacts that each own their question type (e.g. a promoted vector and its referenced model) is a surfaced reconciliation event, not a silent override: either the model is wrong (update model, re-check every vector exercising it) or the vector is wrong (demote, fix, re-promote). Implementation is never authority; if running code disagrees with a normative artifact, the code is the bug-fix target.
```

**Implementation Notes**:
- Place this section before the existing "TLA+ and Quint position" section so it frames everything below.
- This does not change the model-promotion rule; it sits above it as the authority framing.
- Cross-reference: PROTOCOL.md and ARCHITECTURE.md both currently say PROTOCOL is canonical "until generated schemas or IDL exist." That transition wording stays (PROTOCOL is authority for product intent + vocabulary naming both before and after `.proto` exists), but this new section defines the post-transition order.

**Acceptance Criteria**:
- [ ] `docs/VERIFICATION.md` states the question-type-layered authority order with the 5-row table.
- [ ] The surfaced-reconciliation rule for promoted-vector↔model contradictions is stated.
- [ ] "Implementation is never authority" is stated.

---

### Unit 2: State the property-graded normative baseline and tier split in VERIFICATION.md

**File**: `docs/VERIFICATION.md` (new section, after "Artifact authority order"; before "TLA+ and Quint position" or integrated with the model-promotion rule)

```text
## v0 normative baseline (property-graded)

Each required model area is obligated at v0. Properties within each area are tiered:

- **checked-normative** — must clear the model-promotion rule AND have ≥1 promoted conformance vector tracing to the property before v0 treats the behavior as product. Covers safety/security-critical properties:
  - Operator intent delivery: TerminalFinality, LsnDeterminesTerminalWinner, PreAppendTerminalChoice; accepted-command durability; timeout-neither-success-nor-denial.
  - Wrong-session prevention: session identity tuple; LateGenerationInert; GenerationMonotonic; labels cannot override verified identity.
  - Idempotent retry: boundary dedup; retry reuses id+key; terminal retry returns existing record.
  - Authority safety: no-command-without-grant rejection; CompoundIssuer; GrantAuthorityIsCommandKinds; revocation of future acceptance.
  - Crash recovery: no accepted command disappears silently after restart; idempotent log replay reconstructs identical state.
  - Browser session and CSRF boundary: no state-changing request without an authenticated operator session; no state-changing request without a valid session-bound CSRF proof; revoked/expired sessions cannot issue new commands.
- **stated-normative** — documented v0 obligation with a draft model, not yet checked-to-pass; scheduled for promotion post-v0. Covers liveness/cosmetic/operational properties:
  - Snapshot convergence: compaction/cursor validity, late-event audit handling nuances.
  - Audit integrity: completeness of audit records, correlation coverage.
  - Adapter failure visibility: failure-vocabulary distinguishability refinements.
  - Reply correlation: typed-correlation edge cases beyond the checked wrong-session/idempotency core.

This reconciles `docs/SPEC.md`'s 5-area verification-floor seed with this document's required-areas list: the checked-normative set is the seed plus crash-recovery safety and the CSRF spine; the rest are obligated but draft at v0.
```

**Implementation Notes**:
- Reference SPEC's verification-floor sentence explicitly so the reconciliation is visible.
- This composes with the existing model-promotion rule: a property promotes its model AND its vectors together. "checked-normative" = model promoted + ≥1 promoted vector; "stated-normative" = draft model, no promoted vector yet.
- Delegation precondition and Lease safety already sit in their own sections as preconditions-for-future-behavior; they are NOT part of the v0 normative baseline (consistent with their existing framing).

**Acceptance Criteria**:
- [ ] `docs/VERIFICATION.md` states the two tiers and lists the checked-normative properties by area.
- [ ] The reconciliation with SPEC's 5-area seed is explicit.
- [ ] The composition with the model-promotion rule is stated.

---

### Unit 3: State the conformance-vector promotion rule and traceability metadata in VERIFICATION.md

**File**: `docs/VERIFICATION.md` (extend the existing "Conformance testing" section)

```text
### Conformance-vector promotion and traceability

Conformance vectors are draft/derived until explicitly promoted, mirroring the model-promotion rule. A promoted vector is a peer authority for expected executable examples (see Artifact authority order).

Vector promotion requires:
- a named model property the vector exercises (property id);
- the `.proto` fields/enums the vector constrains (or "none" for pure state-transition vectors);
- an expected outcome matching the referenced model property's invariant;
- a reviewed status (a vector is promoted by review, not automatically).

Each conformance vector file carries structured frontmatter:
  property: <property-id>
  status: draft | promoted
  proto_fields: [field/path, ...]   # or [none]
  expected: <outcome>

A CI script reads all vectors and:
- fails if a checked-normative property lacks a promoted vector;
- fails if a vector references a missing or misspelled property id;
- fails if a promoted vector's expected outcome contradicts its referenced model property's invariant (surfaced contradiction, per the authority order);
- generates the traceability table in this document as a checked-in artifact (so the human-readable mapping never drifts).

A promoted vector that later contradicts its model is a reconciliation event: either the model is wrong (update model, re-check every vector exercising it) or the vector is wrong (demote, fix, re-promote). It is never a silent override.
```

**Implementation Notes**:
- Keep the existing conformance-testing bullet list (golden vectors, terminal-commit race vectors, property tests, adapter conformance, replay/reconnect tests); this extends it with the promotion + traceability rule.
- The frontmatter shape is a design target for `feature-protocol-idl-and-conformance` to implement; this feature commits to the rule and the fields, not to a file format.
- "A vector is promoted by review, not automatically" matches the human-gate posture of the model-promotion rule.

**Acceptance Criteria**:
- [ ] The vector-promotion rule is stated with its 4 requirements.
- [ ] The frontmatter shape is specified (property, status, proto_fields, expected).
- [ ] The 4 CI checks are stated.
- [ ] The surfaced-reconciliation rule for promoted-vector↔model contradictions is stated.

---

### Unit 4: Ratify the v0 contract source and generation targets in SPEC.md

**File**: `docs/SPEC.md` (Protocol contracts subsection)

```text
## Protocol contracts

Patchbay uses Protobuf schemas managed by Buf as the v0 boundary-contract source for durable protocol messages, command/event payloads, and shared enum vocabularies across the Rust core and TypeScript operator domain.

- `.proto` files are the source for wire contracts and boundary DTOs, not the full internal domain model.
- Rust types are generated via prost/prost-build; TypeScript types via Protobuf-ES. Generated outputs are artifacts, never hand-edited.
- `buf.gen.yaml` is checked in; `buf lint` and `buf breaking` run locally and in CI.
- JSON Schema / TypeBox / Zod are reserved for JSON-native local validation surfaces, not as the cross-language protocol source.
- TypeSpec is a reserved future direction if Patchbay later needs OpenAPI, JSON Schema, and Protobuf emitted as peer outputs from one authoring language.

`.proto` is authority for wire shape only (see `docs/VERIFICATION.md` Artifact authority order); product intent and vocabulary naming remain prose authority, and invariants remain model authority.
```

**Implementation Notes**:
- Replace the existing "Protobuf + Buf is the default candidate" wording (which hedged the decision) with the ratified commitment. The hedge was appropriate before `feature-research-contract-tooling` concluded; that research is done and source-grounded, so the commitment is now earned.
- This closes the SPEC acceptance criterion ("states the v0 contract source").
- Cross-reference VERIFICATION.md's authority order so the `.proto`-is-wire-shape-authority-only boundary is explicit.

**Acceptance Criteria**:
- [ ] `docs/SPEC.md` states Protobuf+Buf as the v0 contract source (no longer "default candidate").
- [ ] Generation targets (prost for Rust, Protobuf-ES for TypeScript) and CI checks (buf lint, buf breaking) are stated.
- [ ] The `.proto`-is-wire-shape-authority-only boundary is stated.

---

### Unit 5: Distinguish semantic authority from wire-encoding authority in PROTOCOL.md

**File**: `docs/PROTOCOL.md` (update the opening paragraph)

```text
This document defines concepts and required behavior, not a final wire encoding. It is the canonical source of truth for command state, session state, failure vocabulary, and transition semantics — the product intent and vocabulary naming authority (see `docs/VERIFICATION.md` Artifact authority order). Wire shape, field identity, and enum encoding are authority of the generated `.proto` contract once it exists; until then this document is the provisional wire reference. Future TypeScript/Rust enums, TLA+/Quint variables, conformance vectors, and UI presentation labels derive from these registries rather than redefining them.
```

**Implementation Notes**:
- The existing sentence "Until generated schemas or IDL exist, this document is the canonical source of truth" is preserved in spirit but split: PROTOCOL remains product-intent + vocabulary authority permanently; it is the *provisional* wire authority only until `.proto` exists.
- This closes the PROTOCOL acceptance criterion ("distinguishes semantic authority from wire encoding authority").
- No change to ARCHITECTURE.md's "prose source of truth until generated contracts exist" boundary rule — it stays, and is consistent with this split.

**Acceptance Criteria**:
- [ ] `docs/PROTOCOL.md` opening paragraph distinguishes product-intent/vocabulary authority (permanent, prose) from wire-shape authority (provisional, passes to `.proto`).
- [ ] Cross-reference to VERIFICATION.md's authority order is present.

## Implementation Order

1. Update `docs/VERIFICATION.md` — authority order (Unit 1), normative baseline (Unit 2), vector promotion + traceability (Unit 3). These are one cohesive edit to one doc.
2. Update `docs/SPEC.md` — ratify contract source (Unit 4).
3. Update `docs/PROTOCOL.md` — semantic-vs-wire authority split (Unit 5).

No child stories are spawned. This is a single-stride documentation/verification design with tight cohesion across three foundation docs; stories would add overhead rather than useful parallelism. (Matches the `feature-design-grant-shape` precedent: cross-doc foundation-doc design = single feature stride.)

## Testing

There is no implementation code yet. Verification for this design is by document consistency:
- confirm `docs/VERIFICATION.md` states the layered authority order, the property-graded tiers with the checked-normative property list, and the vector promotion + traceability rule;
- confirm `docs/SPEC.md` ratifies Protobuf+Buf (no longer "default candidate") and names generation targets;
- confirm `docs/PROTOCOL.md` opening paragraph distinguishes product-intent/vocabulary authority from wire-shape authority;
- confirm the three docs cross-reference `docs/VERIFICATION.md`'s authority order consistently;
- confirm SPEC's 5-area seed and VERIFICATION's required-areas list are reconciled by the tier split, not by one overwriting the other.

## Risks

- **Promoted-vector poisoning despite the promotion gate.** Q3=C requires promotion by review, but a bad vector can still be promoted. Mitigation: the CI surfaced-contradiction check (Unit 3) catches a promoted vector whose expected outcome contradicts its referenced model's invariant; the reconciliation rule makes contradiction a visible event, not a silent override. Residual risk: a vector wrong in a way that does *not* contradict its model (e.g. an under-specified scenario that passes the invariant check but encodes a wrong expected outcome) is still not auto-caught. Accepted: review is the gate, same as for model promotion.
- **Property-set stability.** The checked-normative property list (Unit 2) is a v0 commitment. If implementation reveals a safety-critical property that was classified stated-normative, it must be promoted before its behavior ships. Mitigation: the tier split is designed so promotion is a per-property operation, not a re-open of the baseline.
- **Traceability-metadata schema drift.** The vector frontmatter shape (Unit 3) is a design target; `feature-protocol-idl-and-conformance` implements it. If that feature finds the shape insufficient, this design's Q4=B commitment (machine-readable per-vector metadata + CI coverage check) still holds; only the field set evolves. Mitigation: the 4 CI checks are the load-bearing part, not the exact field names.
- **`.proto` not yet existing.** The authority order and traceability rules reference `.proto` and vector frontmatter that do not exist yet (greenfield). This is intentional Late-Binding: the rules are committed so that when `feature-protocol-idl-and-conformance` and `feature-formal-model-seed` implement, they conform to the authority model rather than inventing one. The rules hold vacuously until artifacts exist; they become enforced when CI is wired.

## Implementation notes

- Files changed: `docs/VERIFICATION.md`, `docs/SPEC.md`, `docs/PROTOCOL.md`.
- Tests added: none; this is foundation-doc implementation (greenfield, no code yet).
- Discrepancies from design: none. All five units landed as specified. Verified each named checked-normative property (`TerminalFinality`, `LsnDeterminesTerminalWinner`, `PreAppendTerminalChoice`, `LateGenerationInert`, `GenerationMonotonic`, `CompoundIssuer`, `GrantAuthorityIsCommandKinds`, plus the crash-recovery and CSRF spine claims) resolves to an existing property in the VERIFICATION model-areas list — no invented property ids.
- Adjacent issues parked: none.
- Verification: `rg` confirmed the 5-row authority table is present; "default candidate" is fully removed from SPEC; PROTOCOL's opening paragraph distinguishes product-intent/vocabulary authority (permanent) from wire-shape authority (provisional, passes to `.proto`); cross-references are bidirectional (VERIFICATION↔PROTOCOL↔SPEC); ARCHITECTURE's unchanged "prose source of truth until generated contracts exist" boundary rule composes with the split. The "Required model areas" header is retained because the tiers are *within* areas, not a replacement for the areas list.

## Review (2026-06-30)

**Verdict**: Approve with comments (after fixes)

**Review lane**: deep, substrate mode, fresh-context cross-model (different class than the GLM-5.2 implementor). One convergence loop on `openai-codex/gpt-5.5` (high thinking); ran 3 passes, findings stabilized → no second adversarial pass needed.

**Blocker** (resolved in review stride):
- Snapshot semantics demoted despite SPEC's verification floor. SPEC line 32 names "snapshots" in the v0 verification floor, but the implementation had put *all* of "Snapshot convergence" in stated-normative, leaving the safety-critical snapshot properties (reject stale/cross-authority snapshots, consistent log-prefix read) outside the checked floor. This was foundation-doc drift (SPEC↔VERIFICATION) and contradicted the design's own "seed done right" rationale. **Fixed**: split the snapshot area — core snapshot safety (stale/cross-domain rejection, consistent log prefix, late-event audit-not-rewrite) promoted to checked-normative; compaction/cursor/operational nuances stay stated-normative. The checked property text mirrors the existing Snapshot convergence model-area properties verbatim.

**Important** (resolved in review stride):
- `TypedCorrelation` left out of checked-normative. v0 ships correlated replies/events and `TypedCorrelation` is anti-forgery (a reply cannot masquerade as a command or cross session/authority contexts) — safety semantics, not an edge-case refinement. **Fixed**: promoted the core `TypedCorrelation` property to checked-normative; only duplicate-reply and reference-resolution refinements remain stated.
- `.proto` "enum vocabulary" wording blurred the wire-shape-only boundary. If `.proto` owns "enum vocabulary" while prose owns "vocabulary naming," enum *names* are ambiguously placed, risking the canonical prose registry losing authority over variant names. **Fixed**: narrowed VERIFICATION's authority table to "enum wire encoding" (not authority for "enum variant naming") and SPEC's wording to "wire encoding of enum vocabularies" + "not the canonical registry of protocol variant names." Product variant naming stays prose authority.

**Notes**: Reviewer's bash tool hit the same `.claude/commands` sandbox issue seen earlier this session, so it read the full current artifact files directly rather than `git show` — did not affect the review. All three findings were genuine drift/coherence gaps, not padding; the review bar earned its keep again (grant-shape found blocker+important; this feature found blocker+2-important). Nits: none.
