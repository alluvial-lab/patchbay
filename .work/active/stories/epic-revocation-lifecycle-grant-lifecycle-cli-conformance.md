---
id: epic-revocation-lifecycle-grant-lifecycle-cli-conformance
kind: story
stage: done
tags: [security, protocol, foundation]
parent: epic-revocation-lifecycle-grant-lifecycle
depends_on: [epic-revocation-lifecycle-grant-lifecycle-revocation-decision, epic-revocation-lifecycle-grant-lifecycle-subscribe-authorization]
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Expose grant revocation in CLI and lock executable evidence

## Checkpoint

Add the CLI `grant-revoke` administration command, expose grant ids in redacted audit query output for grant discovery, regenerate Rust/TypeScript contracts, add draft conformance vectors plus property/integration tests for expiry, policy effects, and subscription resume, and roll the foundation assertions forward without claiming formal promotion beyond the existing stated-normative authority/subscription properties.

## Acceptance evidence

- `grant-revoke <grant-id> [--reason TEXT] [--json]` calls the authenticated ControlService RPC, reports changed vs already-revoked truthfully, and has stable exit behavior for success, denial, and service/validation failure.
- `audit-query --kind grant_created,grant_revoked --json` includes safe `grantId` fields and never exposes secret-bearing payloads.
- Buf generation/drift, Rust workspace tests, CLI tests, vector checks, model metadata checks, and documentation checks pass.
- SECURITY, PROTOCOL, UX, VERIFICATION, and GLOSSARY remain consistent with the implemented self-scope, policy, expiry, and Subscribe semantics.

## Ordering

Runs after both RPC surfaces are stable so generated consumers, CLI behavior, vectors, and foundation prose bind the final contract rather than an intermediate shape.

## Implementation notes

- Added authenticated `grant-revoke` dispatch with truthful changed/idempotent JSON and table output, stable denial/service failure handling, and reason validation.
- Added `audit-query --grant-id` filtering and safe grant-id rendering for redacted grant and command audit views.
- Added generated-contract consumers, CLI coverage for changed and already-revoked outcomes, and five draft conformance vectors covering expiry, revocation policy effects, future authorization, and subscription establishment/resume rechecks.
- Updated the generated verification traceability block without promoting any new formal property.

## Verification

`npm test` in `cli` passed (28 tests); contract vector, model-promotion, presentation, and presentation meta-tests passed; Buf generation completed via `npx --yes @bufbuild/buf generate`; `cargo test --workspace` passed before this CLI-only checkpoint.
