---
id: feature-lease-scope-decision
kind: feature
stage: implementing
tags: [protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-security-threat-model]
created: 2026-06-28
updated: 2026-07-07
gate_origin: null
release_binding: null
---

# Feature: Decide lease scope for v0

Leases appear as core concepts in the current docs, but review questioned whether they are premature without a first concrete use case or fencing model.

## Retag note (2026-06-28)

Retagged from `[prose]` to a design feature. The `prose` tag was removed because the scope is explicitly a decision (leases in v0 vs. deferred) with real design work if included: lessor authority, lease epochs/fencing tokens, partition behavior, adapter obligations. This is a design decision with alternatives, not prose authoring.

## Scope

- Decide whether leases are included in v0 or explicitly deferred.
- If included, name the first lease use case.
- Define authority domain, lessor authority, lease epochs/fencing tokens, partition behavior, and adapter obligations.
- If deferred, revise docs so leases are future coordination concepts rather than v0 implementation obligations.

## Acceptance criteria

- `docs/SPEC.md` states whether leases are v0 or post-v0.
- `docs/PROTOCOL.md` no longer presents underspecified lease safety as an immediate guarantee.
- `docs/GLOSSARY.md` defines `authority domain` if the term remains.
- `docs/VERIFICATION.md` models only lease properties that are in scope.

## Related parked ideas

- `idea-multi-human-coordination` — v0 remains single-operator unless this feature decides otherwise, but the foundation should not foreclose future multi-human authority domains, grants, audit, handoffs, or third-party coordination surfaces.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.

## Design decisions (2026-07-07, operator-confirmed)

- **Q1 — leases in or out of v0?** → **Out (deferred).** No v0 use case: single-operator v0 has no contention to mediate; spawn authority is single-operator single-core (no contention); two-browser-tab races are an optimistic-UI-reconciliation concern, not a lease. The classic lease use case (two actors claiming the same exclusive resource) is multi-actor, which `idea-multi-human-coordination` parks as post-v0. All foundation docs already say leases are out of the v0 executable skeleton; this feature ratifies and crisps that rather than reversing it.
- **Q2 — what lease modeling stays in VERIFICATION.md?** → **(a) stated-normative precondition model.** Keep the lease-safety properties listed (two actors can't hold the same exclusive live lease in one authority domain; expired leases don't authorize new exclusive action; renewal respects holder identity and scope) but explicitly marked as *not* part of the v0 normative baseline, not checked, and required before any future lease-backed behavior ships. Same precondition-for-future-behavior pattern VERIFICATION already uses for delegation. Costs: a draft model nothing exercises. Benefit: gives the future lease-promoting feature a target rather than starting cold.
- **Q3 — fencing model?** → **Not designed now.** The current PROTOCOL Leases section states the safety property as an immediate guarantee with no fencing mechanism behind it — that is the "underspecified immediate guarantee" acceptance criterion 2 wants gone. Soften it to a *modeled precondition* ("holds once a fencing model exists; required before lease-backed behavior ships") without designing the fencing mechanism (epochs vs fencing tokens) now. Designing fencing for behavior that isn't shipping and has no use case is premature; it belongs to the future feature that promotes leases into v0.

## Architectural choice

**Defer leases from the v0 executable skeleton; keep them as a stated-normative precondition in the verification vocabulary.**

Chosen over:
1. **Include leases in v0** — rejected: no concrete use case in single-operator single-core v0; the fencing model (epochs/tokens) is load-bearing safety that would have to be designed from scratch against a hypothetical; premature formalism-tail-wagging-the-product-dog (the exact failure the epic review warned against).
2. **Remove lease modeling entirely** (clean-slate) — rejected: loses the forward pointer to the safety properties a future lease-promoting feature must satisfy; the stated-normative precondition pattern is already established for delegation and is cheap; a cold-start future feature is higher-risk than a draft-model-to-uplift.

This is consistent with every foundation doc's current stance (SPEC/ARCHITECTURE/PROTOCOL/SECURITY/VERIFICATION all already place leases outside v0). The design work here is *crisping the deferral* — removing the one underspecified immediate-guarantee assertion (PROTOCOL Leases section), ratifying the stated-normative precondition framing, and recording the forward reference — not inventing new v0 behavior.

## Implementation Units

This is a docs-only foundation feature. Three small, tightly-coupled edits across four docs; single-stride inline implementation (no child stories — the chunks all touch the same lease vocabulary and are not independently testable).

### Unit 1: `docs/PROTOCOL.md` — soften the Leases section's safety assertion

**File**: `docs/PROTOCOL.md` (## Leases section, ~line 502)

Current state: the section defines the lease shape (resource id, holder actor, scope, expiration, renewal rules, release rules), states the safety property ("two live leases cannot grant exclusive ownership of the same resource and scope at the same time"), then says "V0 reserves leases as an extension seam."

Problem: the safety property is stated as an immediate guarantee with no fencing model behind it. This is the "underspecified lease safety as an immediate guarantee" the acceptance criterion targets.

**Edit**: rewrite the safety sentence from a bare guarantee into a *modeled precondition*. Keep the lease field list as-is (it's the forward vocabulary). Change:

> Within one modeled Patchbay authority domain, two live leases cannot grant exclusive ownership of the same resource and scope at the same time.

into:

> **Lease exclusivity is a modeled precondition for future lease-backed behavior, not a v0 guarantee.** The following properties hold once a fencing model (lease epochs or fencing tokens) exists and lease-backed behavior is promoted into v0 by a future feature; they are required before any such promotion and are not checked in v0:
> - Two actors cannot simultaneously hold the same exclusive live lease within one authority domain.
> - Expired leases do not authorize new exclusive action.
> - Lease renewal respects holder identity and scope.
>
> A future feature promoting leases into v0 must define the fencing mechanism, the lessor authority, lease lifecycle registry, partition behavior, and adapter obligations before shipping lease-backed behavior. That feature must not overload `CommandState` or session state.

Keep the existing "V0 reserves leases as an extension seam. Lease-backed behavior must define its own lifecycle registry before shipping; it must not overload `CommandState` or session state." sentence (it's consistent) but fold the lifecycle-registry point into the precondition block above so it isn't duplicated.

**Acceptance Criteria**:
- [ ] PROTOCOL Leases section no longer states lease exclusivity as an immediate guarantee.
- [ ] The three safety properties are listed as a precondition gated on a future fencing model.
- [ ] No fencing mechanism (epochs vs tokens) is committed; the choice is explicitly deferred to the promoting feature.
- [ ] The "must not overload CommandState or session state" constraint is preserved.

### Unit 2: `docs/VERIFICATION.md` — mark lease safety as out-of-v0 stated-normative

**File**: `docs/VERIFICATION.md` (## Lease safety section, ~line 242; and the precondition list at ~line 145)

Current state already says lease safety is "not part of the v0 executable walking skeleton unless later foundation work explicitly promotes a specific lease-backed workflow." Confirm this is crisp and consistent with the PROTOCOL softening, and that the three properties match Unit 1's list verbatim (they currently do). The line 145 "lease exclusivity" precondition entry should cross-reference the PROTOCOL precondition framing.

**Edit**: minimal — confirm the wording already reads as a precondition (it does); add a one-line cross-reference to the PROTOCOL precondition block so the two docs stay in lockstep. If the VERIFICATION properties list and the PROTOCOL precondition list have any wording drift, reconcile to the PROTOCOL list (PROTOCOL is the canonical home for the safety properties; VERIFICATION points at it).

**Acceptance Criteria**:
- [ ] VERIFICATION lease-safety section clearly marks the properties as not-checked-in-v0, required-before-future-lease-behavior.
- [ ] The three properties match the PROTOCOL precondition list verbatim.
- [ ] No lease property is claimed checked-normative in v0.

### Unit 3: `docs/SPEC.md` and `docs/GLOSSARY.md` — ratify deferral + confirm term

**File**: `docs/SPEC.md` (v0 walking skeleton line ~32 and non-goals line ~42)

Current state already states leases are outside the v0 executable skeleton unless promoted, and excludes lease-backed exclusive coordination from v0 non-goals. This is correct; no semantic change needed.

**Edit**: minimal — confirm the wording is consistent with the "deferred, stated-normative precondition" framing (it already is). If SPEC's lease sentence would read more crisply as "v0 does not implement leases; lease-safety properties are a stated-normative precondition for future lease-backed behavior (see PROTOCOL § Leases)," apply that tightening; otherwise leave as-is.

**File**: `docs/GLOSSARY.md` (## Lease ~line 97; authority domain ~line 17)

Acceptance criterion 3 ("GLOSSARY defines `authority domain` if the term remains") is **already satisfied** — `authority domain` is defined at GLOSSARY line 17. The Lease definition at line 97 is correct and stays. No edit needed unless the Lease glossary entry should note it's a future-only concept; a one-line append ("v0 does not implement leases; see `docs/PROTOCOL.md` § Leases") is optional polish.

**Acceptance Criteria**:
- [ ] SPEC states leases are post-v0 (already satisfied; confirm).
- [ ] GLOSSARY defines `authority domain` (already satisfied; confirm).
- [ ] No semantic change to the v0 walking skeleton or non-goals.

### Unit 4: extension-seams registry row update

**File**: `docs/PROTOCOL.md` (Extension seams registry, leases row ~line 607)

The leases row currently reads "out of v0 executable skeleton; modeled/reserved as a future seam. Tracks `feature-lease-scope-decision` (still drafting) — if that feature promotes leases into v0, this row flips to `C` after its fencing-model / lease-lifecycle-registry design lands."

**Edit**: this feature *is* `feature-lease-scope-decision` and it has now resolved: leases are deferred, not promoted. Update the row to reflect the resolved state — drop the "still drafting" tracker note and state the settled classification plainly: lease-backed coordination is deferred from v0 and reserved as a future seam; promotion requires the fencing model + lease lifecycle registry a future feature would design. The classification stays `X (v0); R (future)`.

**Acceptance Criteria**:
- [ ] Leases registry row reflects the resolved (deferred) state, not a pending tracker.
- [ ] Classification remains `X (v0); R (future)`.
- [ ] The forward reference to a future fencing-model/lifecycle-registry design is preserved.

## Implementation Order

Single-stride inline. All four units are small, tightly-coupled doc edits in the lease vocabulary with no independent test surface. Order: Unit 1 (PROTOCOL softening) → Unit 2 (VERIFICATION cross-reference, depends on Unit 1's exact property wording) → Unit 4 (registry row, depends on the resolved classification) → Unit 3 (SPEC/GLOSSARY confirm, independent). No child stories.

## Testing

No code tests — docs-only foundation feature. Verification is by:
- Grepping for any remaining bare/immediate lease-safety guarantee in PROTOCOL/VERIFICATION (should be zero after Unit 1).
- Confirming the three safety properties appear verbatim in both PROTOCOL and VERIFICATION.
- Walking each acceptance criterion against the edited docs.
- Cross-checking that no doc now claims leases are checked-normative or in the v0 executable skeleton.

## Risks

- **Risk: a future use case needs leases and this framing is too soft.** Mitigation: the stated-normative precondition keeps the safety properties as a target; a promoting feature uplifts rather than starts cold. The framing does not foreclose promotion — that's the point of `X (v0); R (future)`.
- **Risk: PROTOCOL and VERIFICATION drift on the property wording.** Mitigation: Unit 2 reconciles VERIFICATION to PROTOCOL's list verbatim and adds a cross-reference; PROTOCOL is canonical.
- **Risk: deferring fencing leaves a future feature with a hard design problem.** Mitigation: that's the correct outcome — fencing is a real design problem that should be solved against a real use case, not speculatively. Recording it as a required precondition makes the debt visible rather than hidden.
