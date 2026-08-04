---
id: epic-agent-operations-resource-plane-cockpit-composition-shared-resource-rendering
kind: story
stage: implementing
tags: [ux, protocol]
parent: epic-agent-operations-resource-plane-cockpit-composition
depends_on: [epic-agent-operations-resource-plane-cockpit-composition-resource-reconciliation]
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Shared resource target and Operation rendering

## Checkpoint

Make cockpit command targets honestly polymorphic across runtime sessions and
operational resources, and extract the existing delivery/failure renderer so
resource detail can compose the same canonical Operation lifecycle instead of
copying it.

Resource target parsing requires the complete nested tuple. Session-only
Observation/Elicitation paths stay session-only. This checkpoint adds no
resource mutation buttons; it makes accepted resource Operations visible when
another feature submits them.

## Primary files

- `web-cockpit/src/domain/model.ts`
- `web-cockpit/src/ui/operation-delivery.ts` (new)
- `web-cockpit/src/ui/session-detail.ts`
- `web-cockpit/tests/model.test.ts`
- `web-cockpit/tests/shell.test.ts`

## Acceptance evidence

- Exact resource-target Operations project to only their resource identity.
- Partial, mixed, legacy audit-only, and wrong-kind scopes fail closed rather
  than becoming resource command targets.
- Existing session target, command timeline, failure vocabulary, terminal
  history, cancel, and interrupt behavior is unchanged.
- One shared renderer owns canonical Operation state labels and delivery/failure
  presentation for both target categories.

## Ordering

Depends on trustworthy resource reconciliation. The final destination/linkage
checkpoint depends on this shared renderer so the canonical wrapper is complete
before adapter-domain detail is exposed.
