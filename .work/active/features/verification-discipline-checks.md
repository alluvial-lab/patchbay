---
id: verification-discipline-checks
kind: feature
stage: drafting
tags: [verification, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-08
---

# Verification discipline checks

## Brief
Consolidate parked verification-discipline and registry-fidelity checks into executable guardrails. Absorbed findings:

- `idea-check-models-draft-discipline-enforcement`: make model checks enforce draft demotion metadata, TBD invocations, and rejection of vacuous draft stubs.
- `idea-csrf-trace-fidelity`: require safety models to verify against immutable attempted evidence rather than accepting actions' recorded trace fields.
- `idea-proto-prose-registry-consistency-check`: detect drift between canonical prose registries and corresponding proto enums.
- `idea-tlc-temporal-workaround`: resolve or explicitly bound the residual risk of temporal properties relying on an experimental checker path, with TLC/invariant alternatives evaluated.

This feature feeds `epic-public-product-contract-executable-release-assurance`.

## Simplification opportunity
Centralize registry and model-discipline checks in the existing contract verification scripts; avoid parallel metadata-only checkers that duplicate the executable assurance path.
