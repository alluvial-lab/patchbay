---
id: feature-persistence-snapshot-model
kind: feature
stage: done
tags: [prose, protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-command-state-ssot]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Feature: Define persistence, event ordering, and snapshot convergence

Durable acceptance and snapshot recovery are core Patchbay promises, but the docs do not yet define the persistence backend, event ordering, snapshot revisions, or core crash behavior.

## Scope

- v0 persistence model and default backend.
- Event log / inbox / command state ownership.
- Core restart and crash recovery semantics.
- Snapshot revision/cursor model.
- Event stream vs snapshot atomicity.
- Older snapshot rejection and reconciliation rules.
- Adapter snapshot capability tiers and degraded behavior.

## Acceptance criteria

- `docs/ARCHITECTURE.md` states v0 persistence/topology assumptions.
- `docs/PROTOCOL.md` defines revision/cursor semantics for events and snapshots.
- `docs/UX.md` can describe reconnect behavior without relying on wall-clock freshness alone.
- `docs/VERIFICATION.md` has enough state variables to model snapshot convergence.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.

## Implementation notes

- Files changed: `docs/PROTOCOL.md`, `docs/ARCHITECTURE.md`, `docs/VERIFICATION.md`, `docs/UX.md`.
- Tests added: none; prose/foundation documentation change.
- Verification performed: proofread changed docs in context; confirmed `depends_on` items are done; walked each acceptance criterion (ARCHITECTURE persistence/topology, PROTOCOL revision/cursor, UX reconnect without wall-clock, VERIFICATION snapshot-convergence variables). Sanity-checked whitespace/trailing whitespace.
- Review fixes: added adapter snapshot capability tiers (authoritative / partial / none) and degraded behavior rules after inline review found the brief scope item unaddressed; added matching verification property.
- Discrepancies from design: none.
- Adjacent issues parked: none.

## Review (2026-06-28)

**Verdict**: Approve

**Blockers**: none (one foundation-doc drift blocker found by fresh-context review — ARCHITECTURE overstatement of snapshot repair — fixed in `a314515`)
**Important**: two found by fresh-context review (authoritative snapshot fail-closed rule; crash-recovery model variables) — both fixed in `a314515`.
**Nits**: glossary entries for `LSN`, `cursor`, `revision`, `core generation` added in `a314515`.

**Notes**: Inline pre-review was subagent-free (umans sessions running) and caught one scope gap (adapter snapshot tiers) but was correctly treated as insufficient for a feature. Fresh-context deep review then ran on `openai-codex/gpt-5.5` (different model class from the inline author) once Codex capacity was available; first pass returned NEEDS-REVISION (1 blocker + 2 important + nits), all fixed, second pass returned APPROVED. Review bar satisfied.

