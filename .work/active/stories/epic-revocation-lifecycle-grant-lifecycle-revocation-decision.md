---
id: epic-revocation-lifecycle-grant-lifecycle-revocation-decision
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

# Make grant revocation a durable policy decision

## Checkpoint

Add the self-scoped `RevokeGrant` control RPC and make its core-owned Revocation event the single durable policy boundary. The event records exact accepted-command effects using the authorizing-grant provenance captured at acceptance. Apply `continue`, `cancel`, and `require_reauthorization` deterministically in command and diagnostics replay, couple the Revocation plus grant/command audit records in one storage transaction, and serialize production command decisions through one shared server decision gate.

## Acceptance evidence

- Only the verified subject actor (and the grant's endpoint when narrowed) can revoke; missing, cross-subject, endpoint-mismatched, and expired grants fail closed without a Revocation event.
- Revocation is non-cascading and repeat calls by the same subject are idempotent.
- `continue` preserves non-terminal work; `cancel` terminalizes accepted/delivered/running work as `Cancelled`; `require_reauthorization` rejects only not-yet-delivered `Accepted` work and requires a new Operation under a fresh grant.
- Replay, diagnostics, live state, and late-event handling agree on the Revocation LSN as the policy terminal boundary.
- Injected storage failure leaves neither source nor audit records; concurrent revoke/adapter-transition tests leave a replayable log.

## Ordering

Consumes the clock/expiry checkpoint so revocation timestamps and self-authorization use the same injected time boundary.

## Implementation notes

- Added generated `GrantRevocationEffect` entries and replay folding through the canonical acceptance transition validator. `cancel` terminalizes accepted/delivered/running work; `require_reauthorization` rejects only accepted work; `continue` emits no effects.
- Added self-scoped `RevokeGrant` with verified compound issuer checks, endpoint narrowing, expiry denial, non-cascade behavior, and same-subject idempotent repeats.
- Added atomic source-plus-many-audits storage support. Revocation source, grant audit, and per-command policy audits commit as one transaction; the audit projection carries the grant id.
- Accepted-operation grant provenance is checked by command and diagnostics replay; late transitions are terminal-final no-ops.

## Verification

`cargo test --workspace` passed, including replay, authority, storage atomicity, adapter, gRPC, and trust-boundary suites.
