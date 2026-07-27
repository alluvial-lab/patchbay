---
id: epic-revocation-lifecycle-grant-lifecycle
kind: feature
stage: drafting
tags: [security, foundation, protocol]
parent: epic-revocation-lifecycle
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Grant lifecycle: revocation, expiry enforcement, Subscribe check

## Brief

Grants are durable and near-permanent in v0.1.0: there is no public
grant-admin path (a grant cannot be revoked), `Subscribe` authenticates the
compound issuer but performs no grant check, and expiry has a correctness
debt — `GrantRecord.is_expired()` evaluates `expires_at` against
`SystemTime::now()` directly (no clock port, and the field comment at
`core/src/authority/state.rs:45` still falsely claims expiry is "intentionally
not evaluated").

This feature delivers the grant lifecycle contract: a **public
grant-revocation path** (control-service RPC with its own grant
authorization; revocation is durable and audited, per-command
`GrantRevocationPolicy` for already-accepted work), **expiry enforcement
done right** (an injected clock port per Ports & Adapters, `is_live` honoring
expiry, expired grants rejecting with the committed failure semantics and an
audited rejection), and the **`Subscribe` grant check** (subscription
requests authorize against the issuer's grant like other Operations).

Does NOT cover: session/principal revocation or lockdown (sibling features);
cascade-revoke over grant provenance (explicitly out per SECURITY.md's
revocation model); descendant allowed-kinds inheritance (reserved seam).

## Epic context

- Parent epic: `epic-revocation-lifecycle`
- Position: independent start; the lockdown feature sequences after it
  (both touch the submission-authorization path and this feature establishes
  the current enforcement pattern there).

## Simplification opportunity

- The stale "intentionally not evaluated" comment and the direct `SystemTime`
  call collapse into one injected-clock design shared with the future
  session-staleness consumer (parked separately).
- Grant expiry enforcement deletes the "stored but lying" `expires_at` dead
  weight.

## Foundation references

- `docs/SECURITY.md` — revocation model (#4), grant rejection contract
  ("Missing, expired, revoked, target-mismatched, or kind-mismatched grants
  produce SubmissionOutcome = rejected")
- `docs/PROTOCOL.md` — authority, GrantRevocationPolicy, Revocation events
- `contracts/proto/patchbay/authority.proto` — grant/revocation anchors
- `core/src/authority/state.rs` — `is_live`/`is_expired`/`grant_authorizes`
