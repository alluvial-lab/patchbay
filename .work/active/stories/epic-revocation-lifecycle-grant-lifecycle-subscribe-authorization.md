---
id: epic-revocation-lifecycle-grant-lifecycle-subscribe-authorization
kind: story
stage: done
tags: [security, protocol]
parent: epic-revocation-lifecycle-grant-lifecycle
depends_on: [epic-revocation-lifecycle-grant-lifecycle-clock-expiry]
release_binding: v0.2.0
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Grant-check and audit Subscribe establishment

## Checkpoint

Authorize every `Subscribe` RPC establishment as `OperationKind::Query` against the authority-domain target scope, using the verified compound issuer and injected core clock. Audit allow/deny without creating Operation state. A reconnect/resume with a cursor is a fresh establishment and must re-check the grant; an already-established stream is not continuously reauthorized in v0.1.0.

## Acceptance evidence

- A live authority-domain Query grant establishes and audits a cursor-zero subscription without an Operation record.
- Missing, revoked, expired, target-mismatched, and kind-mismatched grants deny before replay; expired denial uses the existing expiry vocabulary.
- A cursor resume after grant revocation is denied, while resume under a live grant still returns only authorized events with `LSN > cursor`.
- Audit records identify the actor, endpoint, grant, authority-domain scope, outcome, and bounded reason.

## Ordering

Consumes the clock/expiry checkpoint; independent of the revocation RPC implementation after that common boundary exists.

## Implementation notes

- `Subscribe` now checks `OperationKind::Query` against an authority-domain `TargetScope` using the verified compound issuer and one injected clock sample.
- Establishment success and denial are durably audited before cursor replay; expired matching grants use `GrantExpired` with `subscription_grant_expired` and grant correlation.
- Cursor resume follows the same establishment path, so a nonzero cursor is never treated as prior authorization. The established stream remains finite and is not continuously reauthorized.

## Verification

`cargo test -p patchbay-core-server --test grpc_smoke --test trust_boundary` passed; existing operator-facing allowlist and cursor-gap assertions remain green.
