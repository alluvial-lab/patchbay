---
id: authority-descendant-grant-completion
kind: feature
stage: drafting
tags: [security, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Descendant-grant live completion (audit producer + composition root)

## Brief
Close the descendant-grant provenance obligation for the live path. Absorbs two coupled findings split out of `authority-provenance-hardening` (currency verified 2026-08-09):

- `backlog-authority-durable-acceptance-metadata` — **PARTIAL**: acceptance now overwrites sender with the verified issuer + durably stores the grant (`core/src/acceptance/pipeline.rs:316-322`), replay reconstructs the grant-bearing command (`index.rs:139-175`), and the spawn-tail consumes it (`spawn_tail.rs:148-152`); **but `audit_id` is still `None`** (`spawn_tail.rs:292-295`). *Src:* authority review #2(C)+#3(2).
- `backlog-authority-live-composition` — **OPEN**: `SpawnDescendantTail` "does not write grants or own a live consumer loop" (`spawn_tail.rs:1-5`); no production composition root feeds committed events to it. *Src:* authority review #3(E).

## Direction
Add the spawn-completion audit producer and carry its `EventId` into `DescendantGrant.audit_id`; wire a live composition root (startup rebuild → bootstrap → cursor catch-up → continuous committed-event delivery) that drives `SpawnDescendantTail`'s `Issuance` into `ingest_descendant_grant` durably. Do not expose a spawn as complete until registration/bump, descendant grant, and audit record are durably committed through one decision (or equivalently crash-safe protocol). Couples with the ingress features (verified-issuer supply) and with the spawn redesign's descendant-authority requirement.

## Foundation references
- `docs/PROTOCOL.md` — descendant-grant provenance (`DescendantGrantProvenance { spawn_operation_id, spawning_grant_id }` + `DescendantGrant.audit_id`)
- `docs/SECURITY.md` — grant-lifecycle provenance + audit
- Code: `core/src/authority/spawn_tail.rs`, `core/src/acceptance/pipeline.rs`, `core/src/acceptance/index.rs`
