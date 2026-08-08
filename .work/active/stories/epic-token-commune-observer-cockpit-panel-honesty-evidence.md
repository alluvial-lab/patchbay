---
id: epic-token-commune-observer-cockpit-panel-honesty-evidence
kind: story
stage: implementing
tags: [adapter, ux]
parent: epic-token-commune-observer-cockpit-panel
depends_on: [epic-token-commune-observer-cockpit-panel-panel-component, epic-token-commune-observer-cockpit-panel-cli-projection]
release_binding: null
gate_origin: null
created: 2026-08-07
updated: 2026-08-07
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
