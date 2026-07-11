---
id: feature-v0-pi-adapter
kind: feature
stage: drafting
tags: [adapter, protocol]
parent: epic-v0-1-0-implementation
depends_on: [feature-v0-core]
release_binding: null
gate_origin: null
created: 2026-07-11
updated: 2026-07-11
---

# Feature: Pi adapter

## Brief

Build the Pi adapter — the first and only required runtime adapter for v0.1.0. The adapter translates between Patchbay's adapter-neutral protocol and Pi's session model: session discovery and status, prompt/instruction delivery, cancel/interrupt where supported, and replies/events/snapshots streamed back to the core.

The adapter is a principal with an explicit registration lifecycle (attach with capability manifest, detach, failure, capability redeclaration). It declares a snapshot tier of `partial` (per `docs/ADAPTER-PI.md`), meaning it can provide recent/current state via transcript event log replay but not arbitrary historical reconstruction. The core's degraded-behavior rules handle the rest honestly.

Pi know-how harvests from remote_pi's `pi-extension/` (Node + TypeScript), which already implements session gating, turn state, transcript event log, transcript projection, and SDK session projection. The harvest is re-housed behind Patchbay's adapter port — harvesting the Pi know-how, not the extension shape. This is real adapter-implementation work, not a copy. See `.work/backlog/idea-harvest-remote-pi-extension-as-adapter.md` for the harvest mapping (what harvests, what does not, re-housing caveat).

## Epic context

- Parent epic: `epic-v0-1-0-implementation`
- Position in epic: parallel with the web chain (protocol-seam → web-server → web-cockpit) after the core lands. The agent-control path (core → pi-adapter) is independent of the phone-usable path.
- The Pi adapter and the web chain can proceed in parallel once the core is up.

## Key design decisions (already settled in `docs/ADAPTER-PI.md`)

- **`session_new` = session replacement, generation bump + tombstone.** Pi's `session_new` tears down the old SDK context and marks it stale; it maps to a `session_generation` bump, not a same-generation clear. Late events binding to the pre-`new` context become `stale_event` audit records.
- **`spawn` not implemented in v0.1.0.** Provisioning is out-of-band sysadmin (pi-supervisord). The adapter declares `spawn` unsupported at delivery (`unsupported_command`); the operator provisions runtimes out-of-band and Patchbay `attach`es.
- **Snapshot tier = `partial`.** remote_pi provides a transcript event log replayed via `session_sync`, not authoritative historical reconstruction.
- **`session_compact` does not bump generation.** Compaction is in-place.

## Foundation references

- `docs/ADAPTER-PI.md` — Pi parity checklist, capability mapping, session_new classification, snapshot tier
- `docs/PROTOCOL.md` — Adapter capabilities, adapter registration and lifecycle, adapter snapshot capability tiers
- `docs/ARCHITECTURE.md` — Adapter plane, adapter registration and lifecycle
- `contracts/proto/patchbay/adapter.proto` — adapter capability, registration, attachment method
- `.work/backlog/idea-harvest-remote-pi-extension-as-adapter.md` — harvest mapping (what to harvest, what to replace, re-housing caveat)
- remote_pi source: `/home/agent/projects/remote_pi/pi-extension/`
