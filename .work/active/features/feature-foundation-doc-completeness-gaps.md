---
id: feature-foundation-doc-completeness-gaps
kind: feature
stage: drafting
tags: [foundation, protocol]
parent: epic-foundation-hardening
depends_on: [feature-operator-presence-and-action-inventory]
created: 2026-07-06
updated: 2026-07-05
gate_origin: null
release_binding: null
---

# Feature: Close foundation-doc completeness gaps from the O/O/E roll-forward review

## Brief

The Phase 1 (completeness/complementary) substrate review of
`feature-operator-presence-and-action-inventory` returned **Approve with comments**
— no blockers, but three `important` completeness gaps where the rolled-forward
foundation docs under-specify something a downstream implementer (protocol IDL,
adapter parity, grant-check derivation, UX) would need to reconstruct from
scattered prose. This feature closes those gaps in a single prose-authoring pass.

The O/O/E frame, registries, lifecycles, and settled decisions (D1–D8, N1–N3)
are NOT re-opened. This is doc tightening: making explicit what is currently
implicit, in the existing docs' voice and structure.

## Scope (from review findings P1–P3)

### P1 — OperationKind registry promises lifecycle/display authority but the table omits those fields
**Location:** `docs/PROTOCOL.md:144-167`
**Gap:** The section says "One registry owns kinds, lifecycle policy, authority matching, adapter capability mapping, display labels, and generated contract variants," but the table carries only `OperationKind`, `Meaning`, `V0 disposition`. The design body's §4 table had per-kind "Allowed `CommandState` / transition notes." Downstream consumers must reconstruct per-kind lifecycle/display behavior from scattered prose.
**Fix:** Add a registry-owned column (or adjacent subtable) for lifecycle/transition notes and display label/category per kind. Cover especially `query` (read lifecycle, may skip `running`), response Operations (`approval-response`, `elicitation-response`), `spawn` (long provisioning, `running` allowed), and the reserved-but-not-validatable kinds (`agent-send`, `adapter-utility-exec` — "not validatable in v0"). Keep it consistent with the `OperationState ⇿ CommandState` refinement equivalence (checked-model properties apply by equivalence; transition adjacency is stated-normative).

### P2 — First-answer-wins clears everywhere, but the notification/reconciliation mechanism is implicit
**Location:** `docs/PROTOCOL.md:292-294`, `:418`, `:433-434`
**Gap:** The docs specify that the first valid answer terminalizes the Elicitation and clears it everywhere (N1), but do not explicitly say HOW subscribed surfaces learn that another surface answered. The likely mechanism is an `ElicitationState` terminal update emitted on the same authorized Elicitation subscription stream, with LSN/cursor replay and snapshot repair on reconnect — but that's implicit.
**Fix:** State explicitly that terminal Elicitation transitions are delivered/replayed on the same authorized Elicitation subscription stream (consistent with D8 subscription model), and that missed terminal updates reconcile through cursor replay and/or snapshots. One or two sentences in the Elicitation section cross-referencing the Presence/Subscription section.

### P3 — Spawn descendant grant shape is not concrete enough for grant-check derivation
**Location:** `docs/PROTOCOL.md:455-477`, `docs/SECURITY.md:145-168`
**Gap:** The descendant grant prose says spawn completion records a grant with "spawner/operator subject as subject, spawned session as target," but does not explicitly specify: allowed OperationKinds (full session-control authority vs narrower default?), provenance link to the spawn Operation/grant, or the full field shape a grant-check implementation needs.
**Fix:** Define the descendant grant record as a normal grant instance with explicit fields: subject actor, target spawned session/generation, allowed OperationKinds / default policy, provenance `{spawn_operation_id, spawning_grant_id, spawner_endpoint?}`, revocation metadata, and audit id. Reference the general grant shape from `feature-design-grant-shape` / `docs/SECURITY.md`. Keep the joint-control seam (delegation reserved) intact — this is making the v0 same-actor descendant grant concrete, not introducing cross-actor delegation.

## Out of scope

- Re-opening the O/O/E frame, registries, lifecycles, or any settled decision (D1–D8, N1–N3).
- Formal-model edits (those are `feature-formal-model-realignment`).
- The review's two nits (P4 VISION success-criteria Elicitation durability bullet; P5 glossary entries for `OperationState`/`agent-send`/`adapter-utility-exec`) — these are small enough to apply inline if picked up, or fold into this pass.

## Routing

**Not `[prose]`.** Despite being a docs-only deliverable (no code surface), this feature involves genuine semantic and architectural commitments — choosing the OperationKind lifecycle/display field shape (P1), specifying the notification/reconciliation mechanism (P2), and defining the descendant grant field shape (P3) all involve choosing between approaches and pinning a model. Per the prose black-box test in `.work/CONVENTIONS.md` (and the epic's lane-routing discipline), semantic commitments route through `feature-design`, not `prose-author`. The design gate runs (alternatives, pre-mortem) before the writing pass.

## Provenance

Filed by the substrate review of `feature-operator-presence-and-action-inventory` (Phase 1 completeness pass, 2026-07-06). Findings P1–P3 recorded in that feature's review record.
