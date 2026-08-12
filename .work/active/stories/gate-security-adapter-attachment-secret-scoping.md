---
id: gate-security-adapter-attachment-secret-scoping
kind: story
stage: implementing
tags: [security]
parent: null
depends_on: []
release_binding: null
gate_origin: security
created: 2026-08-12
updated: 2026-08-12
---

# Shared attachment evidence lets one adapter replace another adapter's identity

> Surfaced by the retroactive deep security scan of `v0.2.0` (the inline release gate missed it — see `workflow-top-level-orchestrator-gate-trip-upward`). Release-relevant: `v0.2.0` introduced the token-commune adapter as a second independently scoped adapter, but both adapters receive the same attachment secret. Candidate for a fast `v0.2.1`.

## Severity
High

## Domain
Authentication & Authorization

## Location
- `server/src/adapter_service.rs:639`
- `server/src/adapter_service.rs:647`
- `core/src/adapter/mod.rs:76`
- `server/src/adapter_service.rs:708`

## Evidence
`Attach` authenticates only the global evidence, then accepts the adapter identity from the request:
```rust
self.evidence.verify_attach(&request.attachment_evidence)?;
```
```rust
let adapter_id = registration.adapter_id.clone()
```
Registration rejects only lower generations, so an equal or arbitrarily high caller-selected generation can replace an existing registration:
```rust
if reported_generation < current_generation {
```
The newly issued token then replaces the selected adapter's active token:
```rust
.insert(adapter_id.clone(), attachment_token_hash);
```
Any process holding `PATCHBAY_ADAPTER_ATTACHMENT_SECRET` can therefore attach as another adapter, fence its existing token and delivery stream, receive its queued Operations, and submit observations under its routing identity. An attacker can choose a maximal generation to prevent the legitimate adapter from reattaching.

## Remediation direction
Bind attachment evidence to a specific adapter identity — preferably with independently provisioned per-adapter credentials or trust roots. Verify the authenticated identity before accepting the registration's adapter ID and generation. Add negative coverage proving one adapter's credential cannot attach, replace, subscribe, or ingest as another adapter.
