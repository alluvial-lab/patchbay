---
id: epic-revocation-lifecycle-session-principal-revocation-core-state
kind: story
stage: implementing
tags: [security, protocol]
parent: epic-revocation-lifecycle-session-principal-revocation
depends_on: [epic-revocation-lifecycle-session-principal-revocation-contract-model]
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Replayable core session and principal revocation

## Checkpoint

Implement core-assigned operator-session generations, complete process-local
session records, compound-issuer endpoint/device/generation binding, replayed
principal/endpoint/device fences, and the three principal-gated revocation RPCs.
Every durable scope mutation appends its source event and typed audit record in
one writer transaction before warming projections.

This checkpoint owns Unit 2 in the parent feature. The parent design is
authoritative for exact Rust types/signatures, idempotency, gRPC status mapping,
secret exclusion, accepted-Operation `continue` semantics, and exhaustive
`StoredEventKind` consumers.

## Acceptance evidence

- Restart replay preserves control-surface revocation and the revoke-all
  generation floor while raw operator-session ids remain process-local and
  invalid across restart.
- Session verification requires the actor, endpoint, device, and endpoint
  generation from the verified principal and updates `last_used_at`.
- Revoking a principal, endpoint, or device invalidates matching credentials and
  process-local sessions without affecting unrelated identities; repeated
  revocation returns the existing result without another source event.
- Revoke-all invalidates every existing generation, including the caller, and a
  later login receives a strictly newer generation.
- Trust-boundary tests prove revoked evidence cannot submit or subscribe, while
  an Operation accepted before revocation can continue to a valid terminal
  state without history rewrite.

## Ordering constraints

Consumes the generated contract/model checkpoint. Hold the existing submit
guard across target lookup, atomic append, catch-up, and result construction so
login/enrollment cannot race the revocation decision.
