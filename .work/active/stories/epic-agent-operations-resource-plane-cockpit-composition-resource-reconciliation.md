---
id: epic-agent-operations-resource-plane-cockpit-composition-resource-reconciliation
kind: story
stage: done
tags: [ux, protocol]
parent: epic-agent-operations-resource-plane-cockpit-composition
depends_on: [epic-agent-operations-resource-plane-cockpit-composition-resource-projection-domain]
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Resource event and snapshot reconciliation

## Checkpoint

Replace the cockpit's temporary `RESOURCE_STATE` decode-and-ignore branch with
real normalized resource-event folding, and reconcile generated session and
resource snapshots atomically.

The two snapshot RPCs may observe different global LSNs. Rebuild through the
larger horizon while skipping session/resource events only through their own
snapshot LSN, then install the complete model and advance the cursor once.
Cached resource values become stale/unknown during a stream gap; no half-read
may appear reconciled.

## Primary files

- `web-cockpit/src/domain/model.ts`
- `web-cockpit/src/domain/reconcile.ts`
- `web-cockpit/tests/model.test.ts`
- `web-cockpit/tests/reconcile.test.ts`

## Acceptance evidence

- Snapshot and live folding agree for upsert, freshness, unknown, tombstone,
  replacement, and per-kind collection revision updates.
- Property/table-driven traces independently vary adapter/kind/id, revision,
  freshness, payload presence, and snapshot LSN ordering.
- Unequal session/resource snapshot LSNs rebuild the same visible prefix once;
  a failed second read or invalid replay leaves the old model unreconciled and
  does not advance the cursor.
- Decoder-local invalid data does not create a reconnect loop, while malformed
  generated events/cross-domain/impossible-revision input fails closed.
- Existing session, command, Elicitation, security, diagnostics, and filtered
  LSN-hole behavior stays green.

## Ordering

Depends on the resource presentation/decoder contract. Shared resource
rendering starts only after this checkpoint supplies trustworthy current,
stale, unknown, and tombstone views.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`; caller-selected highest tier for normalized-event validation and atomic dual-snapshot reconciliation.
- Review weight: `thorough`, explicitly supplied by the autopilot caller; feature review is deferred to the orchestrator.
- Files changed: `web-cockpit/src/domain/model.ts`, `web-cockpit/src/domain/reconcile.ts`, `web-cockpit/tests/model.test.ts`, and `web-cockpit/tests/reconcile.test.ts`.
- Tests added: exact upsert/freshness/unknown/tombstone/replacement folding, local decoder failure versus generated-event failure, stale gap dominance, unequal snapshot horizons, invalid replay atomicity, and failed second-snapshot non-installation.
- Simplification: removed the temporary `RESOURCE_STATE` decode-and-ignore branch and replaced the session-only snapshot replacement seam with one dual-baseline projection contract.
- Discrepancies from design: none. The two RPC reads remain sequential so the test can prove a failed resource half-read leaves the cached projection untouched; correctness depends on max-horizon replay rather than simultaneous reads.
- Adjacent issues parked: none.
- Verification: `cd web-cockpit && npm test` passed 88/88; contracts generated drift, presentation conformance (4 registries), and model-promotion checks passed.
