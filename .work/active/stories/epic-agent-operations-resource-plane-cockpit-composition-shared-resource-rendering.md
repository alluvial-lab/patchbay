---
id: epic-agent-operations-resource-plane-cockpit-composition-shared-resource-rendering
kind: story
stage: done
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

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`; caller-selected highest tier for the command-target union and shared lifecycle renderer extraction.
- Review weight: `thorough`, explicitly supplied by the autopilot caller; feature review is deferred to the orchestrator.
- Files changed: `web-cockpit/src/domain/model.ts`, `web-cockpit/src/ui/operation-delivery.ts`, `web-cockpit/src/ui/session-detail.ts`, `web-cockpit/tests/model.test.ts`, and `web-cockpit/tests/shell.test.ts`.
- Tests added: exact session/resource target projection, rejection of partial/mixed/legacy scopes, resource-target command projection, and shared lifecycle/action rendering.
- Simplification: one extracted Operation delivery module now owns state labels, kind labels, failure vocabulary, timeline transitions, and contextual cancel/interrupt controls; the session renderer contains no copy.
- Discrepancies from design: the pre-existing session action callback continues to receive its raw `SessionIdentity` through a thin session-detail adapter so the session-only submission builder remains unchanged; the durable `CommandView.target` and shared delivery API use the designed discriminated union.
- Adjacent issues parked: none.
- Verification: `cd web-cockpit && npm test` passed 91/91; contracts generated drift, presentation conformance (4 registries), and model-promotion checks passed.
