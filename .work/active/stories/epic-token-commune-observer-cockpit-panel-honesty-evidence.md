---
id: epic-token-commune-observer-cockpit-panel-honesty-evidence
kind: story
stage: done
tags: [adapter, ux]
parent: epic-token-commune-observer-cockpit-panel
depends_on: [epic-token-commune-observer-cockpit-panel-panel-component, epic-token-commune-observer-cockpit-panel-cli-projection]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-08
---

# Cross-surface honesty and mutation evidence

## Checkpoint

Close the integrated web/CLI summary with mutation-sensitive evidence for every locked honesty invariant and run the complete shared-domain, cockpit, CLI, and contract verification surface. This is implementation evidence, not formal promotion.

## Primary files

- `operator-domain/tests/token-commune.test.ts`
- `web-cockpit/tests/token-commune-panel.test.ts`
- `web-cockpit/tests/resource-view.test.ts`
- `cli/tests/resource-projection.test.ts`

## Acceptance evidence

- Focused tests fail when production averages/sums/inverts capacity, treats null as zero, selects a non-5h window, or creates pool remaining.
- Tests fail when stale looks current, provider labels cross adapters, divergent draw selects first, supplied health counts are trusted, model availability is ignored, or one 100% maximum exhausts a mixed pool.
- Tests fail when contributor/member/raw data appears or any required footer derivation disappears.
- Shared package build/tests, full web-cockpit tests, full CLI tests, contract drift/presentation checks, and `git diff --check` pass.
- Recorded evidence names executed/reverted mutants and makes no model-checked or checked-normative claim.

## Ordering

Depends on both rendered surfaces. It is the final child checkpoint before integrated feature review at thorough weight.

## Implementation notes

- Mutation-sensitive tests cover exact summary keys (no aggregate/remaining field), maximum real non-null 5h selection, null/non-5h rejection, draw ambiguity, supplied-count distrust, exact adapter/provider joins, model availability, mixed-pool exhaustion, stale dominance, distinct credential/telemetry axes, required Patchbay-synthesis footer text, rejected `gpt-5.6`, and contributor/member/raw-data absence.
- Self-mutation check executed and reverted three production mutants: (1) removed adapter equality from the pool/draw join — wrong-adapter test failed (`current` vs `unavailable`); (2) inverted the 5h maximum comparator — capacity test failed (`0.35` vs `1`); (3) removed freshness-first verdict handling — stale tests failed (`runnable` vs `telemetry-stale`). All source was restored before final verification.
- Final verification: `operator-domain` 7/7; explicit `web-cockpit` build clean and tests 113/113; `cli` tests 42/42; generated-contract drift check passed; presentation conformance passed (including its axe scan); panel-local axe-core check reported 0 critical violations; `git diff --check` passed.
- This is implementation/mutation evidence only. No formal, model-checked, checked-normative, or release-verified promotion is claimed.
