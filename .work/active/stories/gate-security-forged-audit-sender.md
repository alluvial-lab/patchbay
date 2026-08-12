---
id: gate-security-forged-audit-sender
kind: story
stage: done
tags: [security]
parent: null
depends_on: []
release_binding: v0.2.1
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

## Symptom
An authenticated adapter could submit a generic Observation whose target belonged to that adapter but whose sender named the operator or another endpoint/device; production audited storage then persisted those unverified sender fields as durable audit attribution.

## Root cause
Generic Observation ingress authenticated the attachment and fenced the target adapter, but passed `observation.sender` unchanged to storage. `AuditedStorage` correctly derived its audit draft from the persisted source, so the missing canonicalization at ingress made attacker-controlled identity look verified.

## Fix approach
At generic Observation ingress, obtain the canonical adapter actor and registered endpoint from the current authenticated attachment. Reject any provided actor, endpoint, device, or endpoint-generation claim that conflicts with or exceeds those verified facts, then replace the payload sender with the canonical attachment identity before any acceptance or acknowledgement append.

## Regression test
Extend the promoted resource/source-authentication conformance runner through production `AuditedStorage`: forged actor, endpoint, and device claims must each return `PERMISSION_DENIED` without appending state, while an unclaimed sender is canonicalized in the durable Observation and the production audit-draft builder derives the registered adapter actor and endpoint from that source.

## Implementation notes

- **Execution capability:** direct inline implementation; the security boundary and its promoted conformance runner were localized and did not require delegated exploration.
- **Files changed:** `server/src/adapter_service.rs` canonicalizes generic Observation senders from the current attachment; the two promoted source-authentication vectors and `server/tests/conformance_vectors.rs` cover forged dimensions and canonical audit attribution; `server/tests/spawn_completion.rs` now omits the intentionally forged success sender because such claims correctly reject at ingress.
- **Reproduction:** the strengthened conformance runner initially failed because a forged actor claim returned a successful `ObservationResult` and appended the Observation.
- **Focused confirmation:** the Rust server conformance runner passed both affected promoted source-binding vectors, including actor/endpoint/device rejection and canonical source/audit-draft attribution.
- **Vector confirmation:** `node contracts/scripts/check-vectors.mjs` passed all 54 vectors, 17 promoted vectors, 22 implementation checks, and 38 mutation witnesses.
- **Workspace confirmation:** `cargo test --workspace` passed after updating the old spawn fixture to the new reject-conflicts contract.
- **Lint confirmation:** `cargo clippy --workspace --all-targets -- -D warnings` passed.
- **Original symptom:** conflicting actor, endpoint, device, and endpoint-generation claims now fail before any Observation append; absent or matching partial claims are replaced with the authenticated adapter actor and registered endpoint before persistence.
- **Ruled out:** no storage schema or `AuditedStorage` behavior change was needed; `server/src/main.rs` already composes the audited decorator, and canonicalizing the durable source at authenticated ingress protects every downstream audit builder.
- **Adjacent issues parked:** none.

## Review (2026-08-12)

**Verdict**: Approve with comments

**Blockers**: none
**Important**: none
**Nits**: clarified the promoted resource-observation vector description so it states that conflicting sender claims reject rather than remain evidence; the vector checker stayed green.
**Rejected**: none

**Notes**: Bounded inline standalone-story review; no independent, fresh-context, or cross-model reviewer ran. Correctness review traced authentication, current-attachment lookup, sender conflict validation, canonical replacement, persistence, and audit-draft derivation. Security review confirmed all four sender dimensions fail closed when unverified and that the shared decision gate prevents attachment replacement races. Tests exercise forged actor, endpoint, and device values and the valid no-claim path; the helper also rejects unverified endpoint generation. The change intentionally hardens adapter ingress without changing Protobuf or storage schemas. Foundation documents already require sender identity to come from verified connection context, so no assertion rolled forward. Naming/comments, test integrity, and the adjusted spawn fixture were reviewed; no material findings remain. Per operator instruction, this done release-gate story remains in `.work/active/stories/` for release-deploy rather than being archived.
