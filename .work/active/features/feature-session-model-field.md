---
id: feature-session-model-field
kind: feature
stage: drafting
tags: [protocol, ux, fast-follower]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-23
updated: 2026-07-24
research_origin: null
---

# Feature: surface the agent model in session reports

**Promoted 2026-07-24** into the pre-release fix wave.

Surfaced in live use (2026-07-23): the operator asked "do we have a way to
return what model the pi agent is running with?" The answer today: it's
recorded (the session transcript's `model_change` event — e.g.
`provider: kimi-coding, modelId: k3`) and the adapter knows it
(`session.model`), but nothing surfaces it to the cockpit or CLI.

Operator decision (2026-07-23): park as a **proper small feature** (option b),
not the quick observation-channel hack.

## Shape

A `model` field on the session report contract:

- **Proto:** add `model` (string, e.g. `kimi-coding/k3`) to
  `SessionRegistered` (and `Session`, so it materializes in snapshots). This
  is a contract change → `contracts/` regen (buf generate TS; `git checkout --
  contracts/rust/src/gen` + `cargo build` for Rust).
- **Core:** ingest it through the session registry (the
  `SessionRegistered`/`SessionRelabeled` path or a `SessionModelChanged`
  mutation if the model can change mid-session — likely yes, so model the
  mutation, not just the registration field).
- **Adapter:** report the session's current model (`session.model`) at
  registration and on change (subscribe to `model_change`).
- **Surfaces:** cockpit session row/detail header shows it;
  `cli session-health` prints it.

## Considerations

- Models CAN change mid-session (`model_change` event), so treat it as
  mutable session state with its own mutation, not a registration-time
  constant.
- The quick alternative (emit a session-model observation, fold it into
  `SessionView.model`) requires no contract change but was rejected by the
  operator in favor of the proper field.

## Simplification opportunity

None identified — additive contract field plus plumbing.
