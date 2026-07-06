---
id: feature-foundation-doc-completeness-gaps
kind: feature
stage: implementing
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

## Design decisions

Captured from the design-pass Q&A (2026-07-06). The O/O/E frame, registries, lifecycles, and settled decisions (D1–D8, N1–N3) are NOT re-opened; these three decisions only make explicit what the roll-forward left implicit.

- **Q1 (P1) — registry lifecycle column:** Add a `Lifecycle notes` column to the `docs/PROTOCOL.md` `OperationKind` registry table, porting the per-kind transition content that already exists in the design body's §4 table (which the roll-forward dropped when applying the registry to PROTOCOL). Do NOT add a separate display-label/category column in this pass — display derivation is a UX concern that can come later. Rationale: the lifecycle content is already settled in the design body; the gap is purely that the PROTOCOL registry (the SSOT) lost the column during application. Adding it back makes the registry the single source of truth and unblocks downstream protocol-IDL derivation. Reframed during the design pass when the operator pointed out the design body §4 already enumerates per-kind transitions — this is a port, not new design.
- **Q2 (P2) — first-answer-wins notification mechanism:** Terminal `ElicitationState` transitions emit on the same authorized Elicitation subscription stream (consistent with the D8 subscription model); missed terminal updates reconcile through cursor replay and/or snapshots on reconnect. Additionally, a late second answer from another surface hits the already-terminal `answered` state and is rejected as a stale late terminal candidate — that rejection (plus the terminal transition on the stream) is what forces the lagging surface to resync. Rationale: reuses the existing subscription/cursor model; no new push mechanism. The §13 open question on foreground/attention notification routing is NOT resolved here (it remains about attention push, not delivery).
- **Q3 (P3) — descendant grant field shape:** The descendant grant is a normal grant instance (per `docs/SECURITY.md` grant shape + `feature-design-grant-shape`), with `allowed OperationKinds` = the **full set of committed kinds applicable to an existing session, enumerated explicitly** (not a wildcard `all`, not a default policy — an explicit list). Fields: subject = spawner operator actor (+ optional endpoint per the general shape); target = spawned session/generation; `allowed OperationKinds` = explicit enumerated set; provenance `{spawn_operation_id, spawning_grant_id}`; standard revocation metadata + audit id. Rationale: explicit enumeration is auditable and a future narrowing is a field edit, not a semantics change. The (c) alternative — inherit allowed kinds from the spawning grant — is noted as the RESERVED future direction for delegation-aware authority (links descendant authority to the spawning grant's scope), preserved as a seam, not v0 behavior.

## Architectural choice

**Single-pass, single-agent doc edit across `docs/PROTOCOL.md` + `docs/SECURITY.md`.**

The three gaps are tightly coupled (P1 and P3 both touch the spawn descendant-grant prose; P2 and P3 both touch Elicitation terminal semantics) and the two docs cross-reference each other heavily. A single pass holding both docs in context avoids cross-reference drift. This mirrors the action-inventory roll-forward's single-agent rationale, but at much smaller scale (~3 localized edits, not a whole-frame roll-forward).

Alternatives considered:
- Per-finding sequential edits (P1 then P2 then P3): rejected — risks drift between edits as each shifts prose the others reference.
- Three parallel agents (one per finding): rejected — the edits are too small and too interdependent to parallelize safely.

## Implementation Units

### Unit 1: OperationKind registry lifecycle column (P1)
**File**: `docs/PROTOCOL.md` (the `### OperationKind registry` section, ~line 144-167)
**Story**: none (single-stride, tight cohesion — see Phase 7)

Add a `Lifecycle notes` column to the registry table, porting the per-kind transition content from the design body §4 table (`feature-operator-presence-and-action-inventory.md` lines 188-200). The content already exists and is settled; this is a mechanical port with SSOT alignment. Per-kind content:

- `spawn` — Full `CommandState` lifecycle by refinement: initial `accepted`; then `delivered`, optional `running`, or terminal. `running` is allowed for long provisioning.
- `attach` — Full lifecycle by refinement; may skip `running`, but not durable lifecycle.
- `instruct` — Full lifecycle allowed: `accepted → delivered → running → terminal`; in-flight steering may skip `running` if adapter reports immediate acceptance.
- `cancel` — Full lifecycle by refinement; the target Operation's terminal race is governed by first durable terminal commit, and cancellation completion does not rewrite an already-terminal target.
- `interrupt` — Same as `cancel`; reserved distinction for adapters that expose softer cancel vs harder interrupt.
- `query` — Full lifecycle by refinement. Reads may skip `running`, but no v0 read uses a no-delivery direct-to-completed shortcut. A no-lifecycle read variant is reserved if polling volume warrants it later.
- `approval-response` — Full lifecycle by refinement. Completion updates the Elicitation terminal (`answered` or `declined`) only if response validation succeeds and first-terminal rules allow.
- `elicitation-response` — Full lifecycle by refinement. Invalid response Operation is rejected unless explicit Elicitation policy terminalizes the slot.
- `reconfigure` — Full lifecycle by refinement; `running` only for adapters with long reconfiguration.
- `session-management` — Full lifecycle by refinement because compaction/archive/delete can be long-running; quick local actions may skip `running`.
- `agent-send` — Not validatable in v0. If submitted in v0, rejected before acceptance.
- `adapter-utility-exec` — Not validatable in v0. If submitted in v0, rejected before acceptance; full lifecycle/idempotency modeling deferred.

The column header is `Lifecycle notes` (matching the design body's `Allowed CommandState / transition notes` intent, in the docs' voice). No new section; the table just gains a column. The boundary rules and spawn-payload prose below the table are unchanged.

**Acceptance Criteria**:
- [ ] The PROTOCOL.md `OperationKind` registry table has 4 columns: `OperationKind`, `Meaning`, `Lifecycle notes`, `V0 disposition`.
- [ ] Every committed kind has lifecycle notes consistent with the design body §4 and with the `OperationState ⇿ CommandState` refinement equivalence (transition adjacency is stated-normative; checked-model properties apply by equivalence).
- [ ] Reserved kinds (`agent-send`, `adapter-utility-exec`) note "not validatable in v0."
- [ ] No prose elsewhere in PROTOCOL.md contradicts the per-kind lifecycle notes.

### Unit 2: Elicitation terminal notification/reconciliation mechanism (P2)
**File**: `docs/PROTOCOL.md` (the `### ElicitationState lifecycle` section, ~line 257-298)
**Story**: none

Add an explicit paragraph after the first-answer-wins rule (~line 247) stating the notification/reconciliation mechanism:

> Terminal `ElicitationState` transitions are delivered on the same authorized Elicitation subscription stream as `opened`/`pending` (consistent with the Presence/Subscription model in `§ Presence and Subscription`). A surface that misses the terminal transition (e.g., it was offline when another surface answered) reconciles through cursor replay and/or snapshot repair on reconnect — the terminal state is part of the durable Elicitation record. A late second answer from a lagging surface arrives after the Elicitation is already terminal; it is rejected as a stale late terminal candidate (audited, does not rewrite state), and that rejection plus the terminal transition on the stream is what forces the lagging surface to resync to the `answered` (or other terminal) state.

This is 1-2 sentences added to the existing Elicitation section, cross-referencing the Presence/Subscription section. No new mechanism is introduced — it makes the existing D8 subscription model's application to Elicitation terminals explicit.

**Acceptance Criteria**:
- [ ] The Elicitation section explicitly states terminal transitions ride the authorized Elicitation subscription stream.
- [ ] It states missed terminals reconcile via cursor replay and/or snapshot on reconnect.
- [ ] It states late second answers are rejected as stale late terminal candidates and that rejection forces resync.
- [ ] The §13 open question on foreground/attention notification routing is NOT resolved by this edit (remains about attention push, not delivery).

### Unit 3: Spawn descendant grant field shape (P3)
**Files**: `docs/PROTOCOL.md` (the `#### Spawn payload and authority commitments` descendant-grant bullet, ~line 469) AND `docs/SECURITY.md` (the `### Spawn authority` section, ~line 162-168)
**Story**: none

Make the descendant grant record concrete. Replace the current "spawner/operator subject as subject, spawned session as target" prose with an explicit field list, consistent with the general v0 grant shape in `docs/SECURITY.md` (`A v0 grant has at least:` block, ~line 147-160) and `feature-design-grant-shape`:

The descendant grant record (auto-issued at spawn completion):
- `grant id` — standard grant id (core-assigned).
- `authority domain id` — same domain as the spawning grant.
- `subject actor id` — the spawner (operator actor in v0).
- `optional subject endpoint id or endpoint class` — the spawning endpoint, if applicable.
- `target scope` — the spawned session/generation (an existing-session scope, now that the session exists).
- `allowed OperationKinds` — the full set of committed kinds applicable to an existing session, **enumerated explicitly** (not a wildcard `all`): `instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, `session-management`. (`spawn` is excluded — recursive spawning requires a separate fleet-level spawn grant; `attach` is excluded — the spawned session is already attached to its spawner's control plane.)
- `creation time and provenance` — `provenance = { spawn_operation_id, spawning_grant_id }` (explicit link to the spawn Operation and the grant that authorized it).
- `optional expiration` — none by default (descendant grant lives until revoked or the session is retired).
- `revocation generation or revoked time` — standard; revocable independently of the spawn grant (two-lever rule, no cascade).
- `revocation policy for already accepted commands` — standard.
- `audit id` — links to the spawn-completion audit event.

Both PROTOCOL.md and SECURITY.md get the field list (PROTOCOL as the authoritative registry/shape; SECURITY as the security-posture reference). The joint-control seam (delegation reserved) is preserved: the `parent_grant_id` field remains intentionally absent in v0; the `(c)` inherit-from-spawning-grant alternative is noted as the reserved future direction for delegation-aware authority.

**Acceptance Criteria**:
- [ ] `docs/PROTOCOL.md` descendant-grant bullet carries the explicit field list including `allowed OperationKinds` enumerated explicitly (not `all`).
- [ ] `docs/SECURITY.md` `### Spawn authority` section carries the same field list or a cross-reference to the PROTOCOL shape.
- [ ] The excluded kinds (`spawn`, `attach`) and the rationale are stated.
- [ ] The `parent_grant_id` field remains absent (delegation seam preserved as reserved, not v0).
- [ ] The (c) inherit-from-spawning-grant alternative is noted as a reserved future direction.
- [ ] The two-lever revocation rule (no cascade) is preserved.

## Implementation Order

All three units in a single pass (see Architectural choice). No story-level `depends_on` chains — the units are edited together in one working-tree state and reviewed together.

## Testing

This is a docs-only deliverable. Verification is by adversarial review, not tests. The review checks:
- Cross-reference integrity between PROTOCOL.md and SECURITY.md after the edits.
- Consistency of the lifecycle column with the design body §4 and the `OperationState ⇿ CommandState` refinement equivalence.
- No silent foreclosure of reserved seams (delegation, recursive spawn, tighter responder binding).
- The descendant grant field list matches the general grant shape in SECURITY.md (no field drift).

## Risks / pre-mortem

- **Risk: P1 lifecycle column drifts from the design body §4.** Mitigation: the implementer ports the design body §4 content verbatim (it's already settled), adapting only the column-header voice. Verify with a side-by-side diff after the edit.
- **Risk: P3 descendant grant field list accidentally reopens `feature-design-grant-shape` decisions.** Mitigation: the field list is the *general* grant shape applied to the descendant case — no new fields, no reopened questions. The implementer must not introduce `parent_grant_id` or any delegation field. The (c) alternative is noted as reserved, not adopted.
- **Risk: P3 `allowed OperationKinds` enumeration becomes stale if the registry changes.** Mitigation: state that the enumerated set tracks the committed kinds applicable to an existing session; a future registry addition requires revisiting the descendant grant's enumerated set. This is the cost of explicit enumeration (chosen over `all` per Q3).
- **Risk: P2 edit accidentally resolves the §13 open question on attention routing.** Mitigation: the P2 edit is strictly about delivery/reconciliation (subscription stream, cursor, snapshot), not attention push. The implementer must not add foreground/badge/push semantics — those remain the §13 open question.

## Provenance

Filed by the substrate review of `feature-operator-presence-and-action-inventory` (Phase 1 completeness pass, 2026-07-06). Findings P1–P3 recorded in that feature's review record.
