---
id: cockpit-mockup-conformance
kind: feature
stage: drafting
tags: [ux, foundation]
parent: epic-public-product-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-17
updated: 2026-08-17
---

# Cockpit mockup authority refresh + machine-checked conformance

## Origin (live UAT finding, 2026-08-17)

Operator UAT of the spawn stride surfaced a two-part UX failure mode:

1. **Unmocked new surfaces.** The spawn stride's UI-fallback decision added
   spawn/restart/claim-warning/target-picker controls into existing cockpit
   chrome with no mockup pass (per-feature "reuse canonical presentation").
2. **Mocked surfaces silently drifted.** The signed-off shell authority
   (`docs/UX.md` → `.mockups/screens/epic-revocation-lifecycle-lockdown/option-hybrid.html`)
   no longer matches the rendered cockpit, and nothing in the pipeline notices:
   `check:presentation` verifies state bindings/contrast/a11y and the shell
   test asserts aria-labels — neither compares layout/field composition to the
   mockup. Measured drift at UAT time:

   | | Signed-off mock | Live |
   |---|---|---|
   | Destinations | 6 (no Resources) | 7 (Resources second) |
   | Mobile tabs | Sessions/Security/More | Sessions/Resources/More |
   | Session-row context slot | cwd path | model name |
   | Session-row label | human name (first line) | absent |

   Operator-visible consequence: "I can give that session commands but don't
   know what cwd it's attached to" — the mock put cwd in the row; the live
   surface dropped it.

## Direction

Two coupled deliverables; treat as release-blocking alongside
`fix-fresh-spawn-one-shot-managed-target`:

### A. Mockup authority refresh (design-bearing → mockup pass first)

Per the project's mockup-first convention, re-mock the surfaces that now exist
(`ux-ui-design:palette/screens` pattern, operator picks an option before
production changes):

1. **Sessions screen** — session-list rows (restore or deliberately replace
   the label + cwd/project-context slot; decide context-source = cwd vs opaque
   project label vs model, considering the adapter-redaction trust boundary),
   spawn action + managed-target picker, claim-poison/retry-risk presentation.
2. **Session detail** — header composition (identity, restart, adapter
   status), staged/promoted lifecycle presentation, composer.
3. **Destination set** — decide Resources' place (rail item when a
   resource-capable adapter is attached vs. always-present), mobile tab order.
4. Diagnostics/Files/Git planned placeholders stay or change with intention.

Sign-off: operator picks options; the chosen mocks become the recorded
authority (updated `docs/UX.md` pointer, superseding option-hybrid.html for
the surfaces re-mocked).

### B. Machine-checked conformance (the structural fix)

Drift must fail CI the way state-binding drift already does:

1. Derive a **machine-readable expectation artifact** from the signed-off
   mocks (destinations + order, mobile tab set, per-surface field contracts:
   session-row slots and their content source, detail-header composition).
   Single source of truth; mocks are the input, the artifact is generated and
   committed.
2. Add a **layout/field-conformance suite** (jsdom rendering the real cockpit
   components, same pattern as the existing shell tests) asserting against the
   artifact. Renaming/reordering/removing a signed-off slot fails CI.
3. Wire into the existing presentation gate family (`check:presentation`
   neighbor or a sibling `check:mockups`), so surface PRs cannot pass while
   diverging from the recorded authority.
4. Reconcile current drift: after (A) sign-off, update the artifact and fix
   the cockpit to match in one reviewable stride.

## Acceptance evidence

- [ ] Fresh signed-off mocks exist for sessions/detail/destination-set, with
      recorded operator sign-off and updated UX.md authority pointer.
- [ ] Expectation artifact generated from the signed-off mocks; committed.
- [ ] Conformance suite green against artifact + cockpit; a seeded drift
      (destination removed / row slot renamed) fails it (mutation-tested).
- [ ] Live cockpit matches the new authority (operator UAT confirms).
- [ ] `check:*` wiring documented in docs/UX.md (what the gate covers).

## Non-goals

- Pixel-perfect screenshot diffing (semantic field/layout contract only).
- Mocking every future micro-change; the artifact changes only with a
  re-signed-off mock.

## Ordering constraint

This feature blocks release sign-off of the spawn stride (with
`fix-fresh-spawn-one-shot-managed-target`) but does not block further
functional UAT.
