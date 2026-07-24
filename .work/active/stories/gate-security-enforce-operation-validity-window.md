---
id: gate-security-enforce-operation-validity-window
kind: story
stage: done
tags: [security, protocol]
parent: null
depends_on: []
release_binding: v0.1.0
gate_origin: security
created: 2026-07-24
updated: 2026-07-24
---

# Enforce Operation validity windows before durable acceptance

## Severity
Medium

## Domain
Authentication & Authorization

## Location
`core/src/acceptance/pipeline.rs:241`

## Evidence
```rust
fn validate_operation(operation: &Operation) -> Result<ValidatedOperation<'_>, String> {
    let operation_kind = OperationKind::try_from(operation.kind)
```

The wire Operation carries `validity_window` and `submitted_at`, but this validation path never reads either field before authorization and durable append.

## Remediation direction
Add an injected clock boundary and validate a well-formed, currently active Operation validity window before grant evaluation/append. Require operator surfaces and CLI builders to populate the contract, define skew/boundary behavior in the protocol registry, and add negative RPC/conformance coverage proving expired or not-yet-valid intent cannot be accepted or delivered.

## Implementation

- Added an acceptance-domain `Clock` port with a production `SystemClock` and deterministic `submit_with_clock` test seam.
- Required valid Protobuf timestamps, a non-empty half-open `[starts_at, expires_at)` interval, `submitted_at` inside that interval and not after the sampled core clock, and a currently active window before grant evaluation, target resolution, deduplication, or durable append. Expired submissions return the canonical `expired` failure; malformed, future-dated, and not-yet-valid submissions return `validation_failed`.
- Stamped every CLI-built Operation and every authenticated web-server Submit with the protocol default: `submitted_at = starts_at = surface_now`, `expires_at = surface_now + 5 minutes`. The web server overwrites the untrusted browser clock.
- Defined half-open boundaries, five-minute defaults, zero-skew v0.1.0 behavior, trusted-ingress handling, and expired-retry precedence in `docs/PROTOCOL.md`.
- Added deterministic core boundary tests and an RPC-level negative test proving expired and not-yet-valid Operations produce no durable Operation event or delivery candidate.
- Removed ENOSPC-generated `storage_proptest.proptest-regressions` entries as crash noise; no legitimate regression seed remained.

Execution capability: direct host ownership, continuing the reviewed interrupted implementation because the core, CLI, web ingress, RPC, protocol, and E2E changes share one acceptance contract.

## Verification

- `cargo test --workspace` — all workspace unit, integration, property, RPC, and doc-test suites passed (including 19 acceptance-pipeline and 7 gRPC smoke tests).
- `cd cli && npm test` — 17/17 passed.
- `cd web-server && npm test` — 24/24 passed.
- `cd e2e && npm test` — passed: durable `accepted → completed` plus final live/idle session state.
- `git diff --check` — passed.

## Bounded review

Reviewed the standalone-story diff against the protocol, security ordering, generated Operation contract, all CLI submission builders, authenticated web ingress, and RPC durability boundary. No material blocker remains. The interrupted E2E fixture's transient `working` poll was found flaky and replaced in the separate cruft story with durable command-completion synchronization.
