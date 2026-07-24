---
id: gate-security-enforce-operation-validity-window
kind: story
stage: drafting
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
