---
id: backlog-authority-durable-acceptance-metadata
kind: feature
stage: backlog
tags: [security, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Backlog: Durable acceptance metadata + descendant-grant provenance

## Source
Authority design review #2 (finding C) + review #3 (finding 2 — audit_id). `feature-v0-core-authority` revision 3 documents `spawning_grant_id: Option<GrantId>` (may be None in v0.1.0) and `audit_id: Option<EventId>` (not populated in v0.1.0).

## Finding
The descendant-grant provenance obligation (PROTOCOL.md line 175-186 + `authority.proto` `DescendantGrantProvenance { spawn_operation_id, spawning_grant_id }` + `DescendantGrant.audit_id`) is not fully met in v0.1.0:
- `CommandRecord` is rebuilt from the raw `Operation` on replay (`core/src/acceptance/index.rs:158`), so `Authorized.grant_id` retained in-memory doesn't survive replay. The spawn-tail's `spawning_grant_id` is therefore payload-derived or None.
- The `audit_id` linking the descendant grant to a spawn-completion audit event has no producer (R4 defers audit).

Revision 3 ships the descendant grant as **component-tested, not protocol-complete**: provenance fields are optional/None, documented as gaps. This is honest but not compliant.

## Direction
Durably record server-attested acceptance metadata (verified actor, verified endpoint, authority domain, authorizing `GrantId`) in a generated contract, so it survives replay and populates descendant-grant provenance. Add a spawn-completion audit producer (couples with `backlog-authority-failed-authorization-audit` — both are audit records) and carry its `EventId` into `DescendantGrant.audit_id`. Until this lands, descendant grants are component-tested only.

## Priority
Required for protocol-complete descendant grants. Not blocking for v0.1.0 component-complete authority; becomes blocking when the live spawn path is exercised (couples with `backlog-authority-live-composition`).
