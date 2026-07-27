---
id: epic-revocation-lifecycle-session-principal-revocation
kind: feature
stage: drafting
tags: [security, foundation]
parent: epic-revocation-lifecycle
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Session & principal revocation

## Brief

v0.1.0 ships only "revoke current browser session"
(`RevokeOperatorSession`). SECURITY.md commits two more session-plane
controls: **revoke all browser sessions** (invalidate all operator sessions,
optionally by rotating the session-signing secret) and **revoke
endpoint/device** (mark a browser or CLI endpoint revoked, rejecting future
Operations from it).

This feature delivers both, across the web-server session store and the
core's control contract. Revoke-all covers the web session plane;
endpoint/principal revocation covers the credential plane — a compromised
principal credential becomes revocable short of rotating the core secret.
Both write durable, audited revocation events (the audit substrate exists;
`OperatorSessionRevoked` is the established pattern).

The feature also absorbs the session-record fields gap
(`backlog-session-record-fields-gap`): endpoint revocation requires records
that identify endpoints — endpoint id, revoked-at, and (per SECURITY.md:94)
session generation are added where missing rather than descoping the doc.

Does NOT cover: grant revocation (sibling feature), lockdown (sibling
feature), CLI/cockpit affordances beyond what the contract additions
naturally expose (cockpit emergency-controls UX lives with the lockdown
feature's mockup pass).

## Epic context

- Parent epic: `epic-revocation-lifecycle`
- Position: independent — parallel with grant-lifecycle; lockdown sequences
  after grant-lifecycle, not this.

## Simplification opportunity

- One revocation path pattern (`RevokeOperatorSession` + audit) generalizes
  to the new scopes instead of three bespoke flows.
- Session-record fields land once, serving both endpoint revocation and the
  SECURITY.md:94 record contract.

## Foundation references

- `docs/SECURITY.md` — revocation model (actions #2, #3), browser session
  model (record fields)
- `docs/PROTOCOL.md` — authority, Revocation events
- `contracts/proto/patchbay/control.proto` — existing revocation RPC pattern
