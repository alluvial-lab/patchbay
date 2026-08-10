---
id: adapter-report-source-ordering
kind: feature
stage: drafting
tags: [adapter, protocol]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Adapter report source-ordering (stale-report rollback prevention)

## Brief
Close the adapter-source-ordering gap split out of `sessions-soundness-coverage`. Absorbs:

- `backlog-session-report-source-ordering` — **OPEN**: `SessionReport` (`contracts/proto/patchbay/adapter_control.proto:35-47`) carries no adapter-side revision, so the core treats arrival order as source order. A delayed-but-sequential stale report carrying an older value derives a valid backward mutation (`B → A`) and rolls a mutable field backward. LSN/`from` checks protect durable replay order but cannot identify a stale *source* report. *Src:* `feature-session-model-field` review (2026-07-24).

## Direction
Add a monotonic, generation-scoped adapter-side report revision to `SessionReport`; core ingest rejects (or marks stale) reports whose revision is not greater than the last applied for that session generation. This is a wire **contract change** → requires a conformance vector + extension-seams-registry classification; consider whether it generalizes beyond `model` to other report-carried mutable fields. Distinct from `session-registry-replay-domain-soundness`: LSN ordering (core arrival) ≠ source ordering (adapter). The Pi adapter's promise-tail serialization mitigates this for v0.1.0's only adapter; this closes it at the contract.

## Foundation references
- `docs/PROTOCOL.md` — session reports; mutable non-identity metadata (current model)
- `contracts/proto/patchbay/adapter_control.proto` — `SessionReport`
- Code: `core/src/session/ingest.rs`, `core/src/session/registry.rs`
