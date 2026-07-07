---
id: feature-audit-persistence-snapshot-model
kind: feature
stage: drafting
tags: [foundation]
parent: epic-retroactive-design-gate-audit
depends_on: [feature-persistence-snapshot-model]
created: 2026-07-07
updated: 2026-07-07
gate_origin: null
release_binding: null
---

# Feature: Retroactive design-gate audit — persistence, ordering, and snapshot model

## Brief

`feature-persistence-snapshot-model` slipped through to `done` tagged `[prose]`, structurally skipping the design gate. Its scope defines the persistence backend abstraction, event log/inbox/command-state ownership, core restart and crash recovery, snapshot revision/cursor model, event-stream vs snapshot atomicity, older-snapshot rejection, and adapter snapshot capability tiers (the authoritative/partial/none tiers were added during review — a design decision made in the prose lane). The implementation notes record "added adapter snapshot capability tiers and degraded behavior rules after inline review found the brief scope item unaddressed" — exactly the kind of design decision the skipped gate should have evaluated up-front.

2 downstream dependents (`feature-verification-contract-authority`, `feature-observability-operator-admin`).

## What to read

- The target: `.work/active/features/feature-persistence-snapshot-model.md` (read FULLY — "Scope," "Implementation notes" recording the snapshot-tier addition as a review fix).
- The docs it produced: `docs/ARCHITECTURE.md` (persistence/topology assumptions), `docs/PROTOCOL.md` (revision/cursor semantics, snapshot atomicity, adapter snapshot tiers), `docs/VERIFICATION.md` (snapshot-convergence variables), `docs/UX.md` (reconnect behavior).
- The checked model: `specs/seed/snapshot_recovery.qnt` (if it exists — verify its properties match the protocol's snapshot rules).
- The 2 downstream dependents listed above.
- Foundation context: `docs/ARCHITECTURE.md`, `docs/PROTOCOL.md`, `docs/VERIFICATION.md`, `AGENTS.md`, `.agents/rules/` (Ports & Adapters is directly load-bearing here — did the backend abstraction achieve it?).

## Scope

1. **Alternatives evaluation** for each load-bearing decision:
   - Persistence backend abstraction (Ports & Adapters): local durable store behind ports (vs mandated engine / vs remote-from-start / vs embedded-only). Verify the port boundary is real, not a doc claim that leaks a storage engine into domain logic.
   - Event log / inbox / command-state ownership and the single totally-ordered durable event log per authority domain (vs per-session logs / vs outbox pattern).
   - The `(authority_domain_id, LSN)` tuple key shape as federation forward-compat (vs bare LSN / vs HLC) — note this was *also* touched by `feature-extension-seams-non-foreclosure`; cross-reference to avoid duplicate findings.
   - Core restart / crash recovery ("no accepted command disappears silently"; idempotent log replay) — alternatives for the recovery contract.
   - Snapshot revision/cursor model and older-snapshot rejection (vs wall-clock freshness / vs snapshot versioning).
   - Adapter snapshot capability tiers: authoritative/partial/none (vs boolean / vs tiering-all-capabilities) — added during review, so likely has no alternatives record.
   - Event-stream vs snapshot atomicity.
2. **Faulty-assumption hunt.** Re-derive each from current first principles. Flag any accident-of-prose. Pay special attention to: whether the Ports & Adapters claim is actually honored in the docs (or whether a storage assumption leaked); whether "no accepted command disappears silently" is a checked guarantee or an asserted one (cross-ref `command_lifecycle.qnt`'s `CommandDurability`); whether the snapshot-tier addition left a gap between what the protocol claims and what the model checks.
3. **Propagation check** across the 2 dependents. Did `feature-verification-contract-authority` assume a snapshot-recovery model posture that the skipped gate would have surfaced? Did `feature-observability-operator-admin` (still drafting) inherit an assumption?
4. **Verdict.** `holds` / `holds-with-caveats` / `faulty-assumption-found`.

## Acceptance criteria

- [ ] Every load-bearing persistence/snapshot decision has a recorded alternatives evaluation.
- [ ] The adapter snapshot-tier addition (a review fix) has an alternatives record.
- [ ] Ports & Adapters verification: confirm the backend abstraction does not leak a storage assumption into domain logic (or flag the leak).
- [ ] Propagation check across the 2 dependents recorded.
- [ ] Verdict recorded; any `faulty-assumption-found` produced a filed corrective item with re-opening `depends_on`.

## Notes

Routes through `feature-design`. No pre-mortem per operator direction. Coordinate with `feature-extension-seams-non-foreclosure` (done) on the `(authority_domain_id, LSN)` federation seam — that feature already classified it; don't re-decide, just cross-reference.
