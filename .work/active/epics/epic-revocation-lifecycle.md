---
id: epic-revocation-lifecycle
kind: epic
stage: implementing
tags: [security, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Epic: Revocation and lockdown lifecycle

## Brief

v0.1.0 shipped only current-session revocation (`RevokeOperatorSession`), but
`docs/SECURITY.md` (Lockdown / revocation; committed-behavior list) commits a
broader emergency-control contract: revoke-all sessions, endpoint/principal
revocation, grant revocation, durable security lockdown with a
bootstrap-channel exit, and grant expiry enforcement (`expires_at` is stored
but ignored — `GrantRecord.is_live()`). The 2026-07-23 adversarial review
surfaced this as Important; the 2026-07-27 docs audit confirmed the doc/code
gap claim by claim.

This epic implements the contract to match the doc: the SECURITY.md claims
stand, and the code catches up. Emergency controls are the operator's
incident-response surface — descoping them would be a product decision, and
the operator has chosen to keep them. The durable audit substrate from
`epic-observability-dogfooding` gives every new decision point (revocations,
lockdown entry/exit, grant expiry rejections) its committed audit home,
satisfying the deferred lockdown-producer obligation recorded in
`epic-observability-dogfooding-core-diagnostics`.

## Strategic decisions

- **Scope = revocation-lifecycle + grant-expiration, nothing more.** Combines
  `backlog-revocation-lifecycle-surface` and
  `backlog-grant-expiration-enforcement` (the expiration item is literally a
  bullet of the lifecycle item). The wider authority-hardening backlog
  cluster is explicitly NOT pulled in; it keeps its own timing.
- **Implement to match the doc.** SECURITY.md's committed emergency-control
  claims are the product; no descoping of the prose. (Interim doc honesty
  note added to SECURITY.md: these controls are contract-committed, landing
  with this epic.)
- **Clock port built here, staleness stays parked.** Grant expiry enforcement
  needs a clock port (injected, like `Storage`) — this epic builds it. The
  sessions feature's time-driven staleness timers are a future consumer of
  that port but are NOT in scope (per the expiration item's own coupling
  note, deferred).
- **Endpoint revocation may grow session-record fields.** The docs-audit gap
  `backlog-session-record-fields-gap` (no endpoint id / generation / revoked
  time on session records) is adjacent: endpoint revocation likely wants
  those fields. Not a declared dependency — the designing feature may absorb
  the field additions; flag for epic-design.
- **UI-bearing.** Lockdown and revocation are operator-facing emergency
  controls; cockpit affordances (and CLI commands) are part of delivery.
  Mockups land at epic-design tier per the mockup-first convention.

## Decomposition sketch (for epic-design)

1. **Session & principal revocation** — revoke-all-sessions,
   endpoint/device/principal revocation; contract additions to the control
   service; session-record field additions as needed.
2. **Grant lifecycle** — public grant-revocation path, `expires_at`
   enforcement via the new clock port (`is_live` honors expiry; expired
   grants reject per SECURITY.md), `Subscribe` grant check.
3. **Lockdown & exit** — durable lockdown posture per SECURITY.md § Lockdown,
   bootstrap-channel exit, lockdown entry/exit audit producers (discharges
   the deferred producer obligation).

## Simplification opportunity

- Retires the doc/code honesty gap: SECURITY.md's committed claims become
  true rather than aspirational; the interim status note is removed on
  completion.
- One clock port replaces two deferred ad-hoc clock needs (grant expiry now,
  session staleness later).
- Grant expiry enforcement deletes the "stored but ignored" `expires_at`
  dead weight — a committed field that currently lies.

## Extension pressure classification

- **Committed by this epic** (already contract-committed in SECURITY.md;
  this is delivery, not a new seam): revoke-all, endpoint/principal
  revocation, grant revocation + expiry, lockdown/exit, Subscribe grant
  check.
- **Reserved (unchanged)**: multi-operator revocation scope, cross-actor
  delegation lineage, lease-backed coordination.
- **Interaction with parked ideas**: none foreclosed — revocation is
  per-authority-domain and endpoint-scoped, compatible with multi-human
  coordination later.

## Origin

Combines `backlog-revocation-lifecycle-surface` (2026-07-23 maximum-review
finding + 2026-07-27 docs-audit evidence) and
`backlog-grant-expiration-enforcement` (2026-07-13 authority design review).

## Decomposition

Three features, matching the sketch. Session/principal revocation and grant
lifecycle start in parallel (disjoint write sets: web-server/session plane vs
core authority plane); lockdown sequences after grant-lifecycle because both
touch the submission-authorization path and grant-lifecycle establishes the
current enforcement pattern there. Grounding note: `GrantRecord.is_expired()`
already evaluates `expires_at` (via direct `SystemTime`, no clock port) — the
grant-lifecycle feature owns the injected-clock cleanup plus a stale
"intentionally not evaluated" comment at `core/src/authority/state.rs:45`.

### Child features

- `epic-revocation-lifecycle-session-principal-revocation` — revoke-all-sessions, endpoint/principal revocation, session-record fields — depends on: `[]`
- `epic-revocation-lifecycle-grant-lifecycle` — grant-revocation RPC, clock-port expiry enforcement, Subscribe grant check — depends on: `[]`
- `epic-revocation-lifecycle-lockdown` — durable lockdown + bootstrap exit + audit producers + cockpit UX (mockups required at feature design) — depends on: `[epic-revocation-lifecycle-grant-lifecycle]`

### Simplification arcs

- `grant-lifecycle` — clock port unifies expiry enforcement; deletes the stale comment and the "stored but lying" `expires_at` dead weight.
- `lockdown` — rejection gate rides the existing submission-authorization path; exit reuses the loopback admin/bootstrap channel.
- `session-principal-revocation` — one revocation path pattern generalizes; session-record fields land once.

### Decomposition risks

- **Lockdown UX is safety-critical and unmocked** — the epic's one net-new
  surface; mockups are explicitly required at that feature's design pass
  (deferred from epic tier to keep momentum; do not implement without them).
- **Self-lockout footgun** — revoke-all / lockdown exercised by the sole
  operator against their own live session. Feature design must address
  re-entry (bootstrap channel) and the CLI recovery path before
  implementation.
- **Write-set collisions** between lockdown and grant-lifecycle if
  parallelized anyway — the depends_on edge exists to prevent that.
