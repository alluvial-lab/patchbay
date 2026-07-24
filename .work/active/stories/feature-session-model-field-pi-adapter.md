---
id: feature-session-model-field-pi-adapter
kind: story
stage: done
parent: feature-session-model-field
depends_on: [feature-session-model-field-core-registry]
release_binding: v0.1.0
gate_origin: null
created: 2026-07-24
updated: 2026-07-24
---

# Story: Report Pi session model at registration and on change

Expose an adapter-local normalized `provider/modelId` string from `PiSession`.
Listen to Pi's appended `model_change` session entry and report the latest
model through the existing per-session core report path. Registration includes
the initial `session.model` value.

## Acceptance evidence

- A configured Pi model produces `provider/modelId` in the registration report;
  no active Pi model reports the permitted empty/unknown value.
- An active-binding `entry_appended` event whose entry is `model_change`
  notifies the registered session observer once with `provider/modelId`; stale
  bindings and unrelated entries do not report.
- Model reports share the per-session report queue with activity reports, so a
  delayed report cannot roll back a later model change.
- Adapter tests build against generated contract types and assert registration
  and model-change reporting.

## Ordering

Depends on the durable core model-state checkpoint; this is the Pi-specific
producer of the adapter-neutral contract.

## Implementation notes
- Execution capability: inline single-owner implementation; Pi-specific entry handling remains confined to the adapter.
- Review weight: standard (default).
- Files changed: Pi session model-change listener, runtime-registry subscriptions, adapter report identity/queue, and adapter tests.
- Tests added/removed: registry observer/disposal coverage and e2e snapshot registration assertion; no tests removed.
- Simplification: model changes share the existing per-runtime-session report tail, and queued reports re-read `session.model` at execution time.
- Discrepancies from design: the focused test invokes the registry observer seam; the real e2e test covers registration model materialization. None.
- Adjacent issues parked: none.
- Verification: `npm test` in `pi-adapter` passed (10 tests, including core smoke/reconnect e2e).
