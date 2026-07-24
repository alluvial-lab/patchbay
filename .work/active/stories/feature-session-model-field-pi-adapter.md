---
id: feature-session-model-field-pi-adapter
kind: story
stage: implementing
parent: feature-session-model-field
depends_on: [feature-session-model-field-core-registry]
release_binding: null
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
