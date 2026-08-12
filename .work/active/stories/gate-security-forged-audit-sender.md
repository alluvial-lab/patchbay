---
id: gate-security-forged-audit-sender
kind: story
stage: drafting
tags: [security]
parent: null
depends_on: []
release_binding: null
gate_origin: security
created: 2026-08-12
updated: 2026-08-12
---

# Adapter-controlled sender fields forge durable audit attribution

> Surfaced by the retroactive deep security scan of `v0.2.0`. Release-relevant: `v0.2.0` adds the production `AuditedStorage` decorator and durable queryable audit projection that trusts these fields.

## Severity
Medium

## Domain
Error Handling & Logging (audit integrity)

## Location
- `server/src/adapter_service.rs:1044`
- `server/src/adapter_service.rs:1050`
- `server/src/adapter_service.rs:1076`
- `core/src/storage/audited.rs:108`
- `server/src/main.rs:61`

## Evidence
Generic adapter observations validate the domain and target adapter but do not replace or validate `observation.sender` before persistence:
```rust
require_same_adapter(
    observation.target_scope.as_ref().and_then(target_adapter_id),
    &authenticated_adapter,
)?;
```
The unchanged observation enters the acceptance path:
```rust
acceptance::ingest_observation(&self.storage, &commands.index, observation)
```
The production audit decorator then treats its sender as authoritative:
```rust
draft.actor_id = observation.sender.as_ref().and_then(|sender| sender.actor_id.clone());
draft.endpoint_id = observation.sender.as_ref().and_then(|sender| sender.endpoint_id.clone());
draft.device_id = observation.sender.as_ref().and_then(|sender| sender.device_id.clone());
```
Thus an authenticated adapter can submit an otherwise valid event, acknowledgement, status, or result while naming the operator or another endpoint/device as its sender. The forged identity becomes durable audit evidence, undermining forensic attribution.

## Remediation direction
Derive observation audit attribution from the authenticated attachment context. Canonicalize the sender to the adapter actor and registered endpoint before persistence, or pass verified identity separately to the audit builder. Reject conflicting payload sender claims and add conformance coverage for forged actor, endpoint, and device values.
