---
id: feature-v0-core-sessions
kind: feature
stage: drafting
tags: [protocol, verification, foundation]
parent: epic-v0-core
depends_on: [feature-v0-core-persistence]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Feature: Session registry and generation

## Brief

Build the session registry: session state (connectivity × activity axes), session generation, session replacement, and stale/offline/unknown presentation. The core tracks which sessions exist, which machine/project/adapter/runtime each belongs to, and each session's authoritative connectivity and activity status. Session identity is stable enough that late replies cannot affect the wrong session.

Session state composes two axes: `SessionConnectivityState` (`live`, `stale`, `offline`, `unknown`, `failed`) and `SessionActivityState` (idle, working, etc.). A stale/unknown connectivity value dominates presentation. Session generation never decreases; a session replacement (e.g. Pi's `session_new`) bumps the generation and tombstones the prior generation, so late events binding to the pre-replacement context become `stale_event` audit records rather than polluting the new conversation.

This feature implements the session-registry port that the acceptance pipeline calls for target-identity binding. It can proceed in parallel with acceptance and authority after persistence lands.

## Epic context

- Parent epic: `epic-v0-core`
- Position in epic: depends on persistence (session state is durable). Implements the target-identity port that acceptance calls; acceptance and sessions can proceed in parallel after persistence lands because the port interface decouples them.

## Formal-model backing

- `GenerationMonotonic` (promoted, `session_generation.qnt`) — the live session generation never decreases. Strict-supersession (equal/lower reports are no-ops) is additionally enforced by the action guard but is NOT a checked temporal property (exceeded Apalache's experimental temporal support; see `idea-tlc-temporal-workaround`).
- Session-identity tuple stability, late-generation inertness, labels-cannot-override-identity — stated-normative obligations (v1 formal gate owns the real properties).

## Foundation references

- `docs/PROTOCOL.md` — Sessions; Session state axes; Id spaces; Snapshots and streams (session snapshots)
- `docs/ARCHITECTURE.md` — Runtime/session plane; State and snapshot plane
- `docs/VERIFICATION.md` — `GenerationMonotonic` promoted property; stated-normative session-identity obligations
- `docs/ADAPTER-PI.md` — `session_new` = generation bump + tombstone; `session_compact` does not bump; snapshot tier = partial
- `contracts/proto/patchbay/sessions.proto` — `Session`, `SessionState`, `SessionConnectivityState`, `SessionActivityState`, `SessionSnapshot`, `ViewRevision`
- `contracts/proto/patchbay/common.proto` — `RuntimeSessionId`, `Generation`, `AdapterId`
- `specs/seed/session_generation.qnt` — generation state and `GenerationMonotonic` property
