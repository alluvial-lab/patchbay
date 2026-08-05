---
id: epic-token-commune-observer-conformance
kind: feature
stage: drafting
tags: [adapter, verification]
parent: epic-token-commune-observer
depends_on:
  - epic-token-commune-observer-adapter-foundation
  - epic-token-commune-observer-snapshot-mapping
  - epic-token-commune-observer-polling-ingestion
  - epic-token-commune-observer-cockpit-panel
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-05
---

# token-commune observer conformance and end-to-end evidence

## Brief

Executable conformance vectors and end-to-end tests proving the token-commune
observer is honest and trustworthy — the gate the `control-attention` epic
explicitly waits for ("after the observer adapter is trustworthy in daily use"),
and a primary input to the sibling `epic-public-product-contract-adapter-portability-proof`
feature's cross-adapter v1 boundary proof.

It delivers: conformance vectors proving the observer reconnects and reconciles
within its real limits, snapshots at the declared PARTIAL tier (never claiming
authoritative), degrades honestly on event gaps (>50-event window) and
disconnect, source-authenticates its reports against the current adapter
generation, fully redacts the gateway credential from all Observations/payloads/
diagnostics/audit, and fails safely on adapter failure. It also covers the
real-core end-to-end path (adapter attaches, reports resources, cockpit renders,
disconnect degrades) analogous to the Pi adapter's real-core E2E.

This is the per-adapter correctness evidence, distinct from the cross-adapter
boundary proof (which consumes both Pi and token-commune to show neither's
concepts entered the core ontology).

## Epic context

- Parent epic: `epic-token-commune-observer`
- Position in epic: **closing evidence** — consumes the whole arc. Its vectors
  are a primary input to `epic-public-product-contract-adapter-portability-proof`
  (cross-adapter v1 proof) and the trust gate for `epic-token-commune-control-attention`.

## Simplification opportunity

- Reuse the conformance-vector harness and property-oracle pattern proven in the
  resource-plane `conformance` feature; do not re-prove core invariants — prove
  the adapter's honest behavior against them.
- Apply the same self-validating-evidence discipline (data-driven counts/
  registries that fail-closed on drift) learned in the resource-plane deep-lane
  review.

## Foundation references

- `docs/VERIFICATION.md` — property-graded baseline; conformance-vector rigor.
- `docs/SPEC.md` — "v1 adapter proof" (Pi + token-commune must prove through
  executable conformance that the boundary supports both shapes without either
  adapter's concepts entering the core ontology).
- `docs/SECURITY.md` — credential redaction; no-log rules.
- `.agents/skills/patterns/` and the resource-plane `conformance` feature for
  the vector/property-oracle harness shape.
- `pi-adapter/tests/e2e.test.ts` for the real-core E2E shape.

## Key design decisions (inherited)

- **Honesty over coverage theater.** Vectors must be genuinely
  mutation-sensitive: each promoted claim must fail when the invariant it
  asserts is broken. A vector that passes whether or not the invariant holds is
  a defect (the recurring anti-pattern from the resource-plane deep-lane review).
- **Prove the gaps, not hide them.** The >50-event window, partial-only tier,
  composite-identity collision risk, and no-read-scope credential are real
  limitations; vectors must prove the adapter reports them honestly, not paper
  over them.

<!-- The design pass fills in the vector set, property oracles, mutation
harness, and E2E scenarios. -->
