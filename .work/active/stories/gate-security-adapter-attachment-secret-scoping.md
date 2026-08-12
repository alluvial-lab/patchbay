---
id: gate-security-adapter-attachment-secret-scoping
kind: story
stage: review
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

## Root cause
`AdapterControlServiceImpl::attach` called `AdapterEvidenceVerifier::verify_attach` against one process-global byte string before reading the registration, so authentication established only “knows the shared adapter secret,” not an adapter principal. The path then trusted the registration's caller-supplied `adapter_id` and `adapter_generation`; `AdapterRegistry::preflight` rejected only generations lower than the selected identity's durable generation. A shared-secret holder could therefore select another adapter, submit an equal or maximal generation, commit its registration, and replace that identity's process-local attachment token and delivery-stream epoch. Post-attach subscription and observation checks were token-scoped correctly, but the forged Attach first issued the attacker the victim identity's current token.

## Fix approach
The core now provisions a fail-closed map of independently scoped adapter credentials through `PATCHBAY_ADAPTER_ATTACHMENT_CREDENTIALS`. Startup rejects empty maps, empty/non-ASCII credentials, and—critically—the same credential assigned to multiple adapter ids. Attach treats `registration.adapter_id` as an untrusted claim and verifies that id's configured credential before inspecting or accepting its generation, durable registration, token replacement, or stream epoch. Post-attach request authentication uses the same adapter-id-scoped credential lookup before checking the attachment token.

This is the minimal realization of the existing foundation requirement that adapter enrollment use configured adapter material or an adapter-specific trust root. It changes no Protobuf shape, generation rule, registration projection, or adapter process contract: each adapter still receives only its own `PATCHBAY_ADAPTER_ATTACHMENT_SECRET`; only the core's former global secret changes to a per-id JSON map. A target-naming wrapper or credential derived from the old shared secret was ruled out because every shared-secret holder could still forge evidence for every target. A global compatibility fallback was ruled out because it would preserve the vulnerability.

## Regression test
`server/src/adapter_service/tests.rs::adapter_attachment_evidence_cannot_cross_adapter_identity` first reproduced the bug: a Pi credential successfully replaced the token-commune registration at `u64::MAX` generation and received a new attachment token. It now proves:

- the core refuses duplicate credential material across adapter identities;
- Pi's credential cannot attach or replace token-commune at a maximal generation;
- Pi's attachment cannot establish token-commune delivery subscription or observation ingestion;
- the rejected attempt leaves token-commune's legitimate ingestion and subscription current; and
- durable replay retains token-commune generation 1, so the forged maximal generation cannot fence legitimate reattachment.

## Implementation notes

**Execution capability**: direct inline implementation. The defect was high-severity and authentication-adjacent, but bounded to the adapter service's credential verifier, Attach ordering, process configuration, and matching harness configuration. The standalone fix lane and recursion guard explicitly prohibited a subagent; no broader architectural decision was needed because the foundation already requires adapter-specific trust roots.

**Files changed**:
- `server/src/adapter_service.rs` — per-adapter credential registry; identity-bound Attach and post-attach verification.
- `server/src/main.rs` — fail-closed JSON credential-map configuration.
- `server/src/adapter_service/tests.rs` — cross-adapter attach/replace/subscribe/ingest regression.
- `server/src/checkpoint.rs`, `server/tests/conformance_vectors.rs`, `server/tests/grpc_smoke.rs`, `server/tests/spawn_completion.rs` — identity-scoped Rust harness credentials.
- `pi-adapter/tests/e2e.test.ts`, `token-commune-adapter/tests/e2e.test.ts`, `cli/tests/core-smoke.mjs`, `cli/tests/real-core-resource-projection.mjs`, `web-server/tests/core-smoke.mjs`, `e2e/walking-skeleton.mjs` — core process provisioning updated; the token-commune cross-owner fixture now has an independent credential.
- `docs/RUNBOOK.md` — operator configuration contract for core credential maps and adapter-local secrets.

**Confirmation evidence**:
1. New regression: `cargo test -p patchbay-core-server adapter_attachment_evidence_cannot_cross_adapter_identity -- --nocapture` — pass.
2. Full Rust suite: `cargo test --workspace` — pass.
3. Original reproduction: maximal-generation cross-adapter Attach now returns `Unauthenticated`; victim generation/token, ingestion, and subscription remain current — covered by the regression.
4. Report match: a holder of one adapter credential cannot obtain another adapter's token, receive its delivery stream, or ingest under its identity; duplicate shared provisioning fails startup construction.
5. Lint/type checks: `cargo clippy --workspace --all-targets -- -D warnings`, `npm --prefix pi-adapter run build`, `npm --prefix token-commune-adapter run build`, and `node --check` for changed `.mjs` harnesses — pass.

**Adjacent issues parked**: none.
