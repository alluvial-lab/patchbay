---
id: feature-session-model-field-surfaces
kind: story
stage: done
parent: feature-session-model-field
depends_on: [feature-session-model-field-core-registry]
release_binding: null
gate_origin: null
created: 2026-07-24
updated: 2026-07-24
---

# Story: Present the current session model in cockpit and CLI

Project the session `model` field from snapshots and durable session events into
the cockpit model, render it in session rows and detail headers with an honest
unknown fallback, and include it in the `session-health` table and JSON
projection.

## Acceptance evidence

- Cockpit registration, model-change, generation-bump, and snapshot folds all
  present the current model without changing identity, connectivity, activity,
  or stale/tombstone handling.
- The session list is searchable by model and renders a concise model label;
  the detail header repeats the current model. Empty model renders `Model unknown`,
  not a Pi-specific guess.
- `session-health --json` includes `model` as a nullable field; table output
  includes a MODEL column and preserves existing script-facing state fields.
- Cockpit and CLI tests cover populated and unavailable model values.

## Ordering

Depends on the core projection/snapshot checkpoint. It may proceed in parallel
with the Pi producer because both consume the same committed contract.

## Implementation notes
- Execution capability: inline single-owner implementation; extends existing presentation and diagnostic projections without a new UI surface.
- Review weight: standard (default).
- Files changed: cockpit session projection/list/detail and focused tests; CLI session-health projection/table and diagnostics tests.
- Tests added/removed: populated/unknown cockpit rendering and model-delta identity coverage; populated/null JSON and unknown table CLI coverage. No tests removed.
- Simplification: no surface-local model cache; both surfaces consume the generated session contract.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Verification: `npm test` in `web-cockpit` (45 passed) and `cli` (17 passed).
