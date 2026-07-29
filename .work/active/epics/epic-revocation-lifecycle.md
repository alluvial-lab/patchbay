---
id: epic-revocation-lifecycle
kind: epic
stage: review
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

## Aggregate completion note (2026-07-29)

All three child features are `done`. The SECURITY.md emergency-control
contract is now implemented end to end:

- `grant-lifecycle` — Clock port + expiry enforcement, `RevokeGrant` (durable,
  policy-aware), Subscribe grant check. 4 review blockers fixed (shared
  CoreDecisionGate, test evidence, CLI validation, audit attribution).
- `session-principal-revocation` — revoke-all, principal/endpoint/device
  revocation fences, CLI controls; session-record fields gap absorbed. 3
  review blockers fixed (stale-issuer race, audit misattribution, count bug).
- `lockdown` — durable posture (survives restart), all-Operation rejection,
  stale-session clamp, session-generation invalidation, authorized entry,
  bootstrap-channel-only exit, entry/exit audit producers (deferred
  obligation discharged), cockpit nav shell + security view per signed-off
  mockup, CLI lockdown-enter/exit. 4 review blockers fixed (adapter
  projection race, query-validation ordering, security inventory wiring,
  test evidence).

Also landed: single-generator-of-record for contract bindings (build.rs no
longer races buf), cockpit nav-shell architecture (research-grounded),
SECURITY.md status notes rolled forward (#1–#5 now implemented).

Verification at epic close: cargo 33 suites + clippy, cli 36, web-server 31,
web-cockpit 73, pi-adapter 24, e2e (incl. lockdown → restart → bootstrap
exit), contracts drift/vectors/models/presentation — all green. Formal
properties/vectors remain honestly draft/stated-normative.

## Epic review findings (standard pass 1, 2026-07-29 — independent reviewer: gpt-5.6-sol)

Verdict: blockers-found. Receiver-confirmed blockers (fix before `done`):

1. **Stale adapter token survives replacement race** — adapter
   attach/token replacement isn't under CoreDecisionGate;
   ingest/report/deliveries authenticate before gate acquisition without
   re-authentication after. Fix: serialize attachment + decision
   establishment through the gate; re-auth + re-read current attachment
   under it; barrier tests for stale IngestObservation/ReportDiagnostics/
   ReceiveDeliveries.
2. **Revoking the last broad grant bricks the deployment** — RevokeGrant
   accepts the sole bootstrap authority-domain grant; no recovery path
   exists (setup secret consumed, admin has only Bootstrap+ExitLockdown).
   Fix (receiver's choice, least-irreversible): REFUSE routine revocation of
   the last recovery-capable authority-domain grant (typed failure) +
   high-impact confirmation for broad grants in CLI/cockpit; an admin
   grant-recovery op is a possible future design, not this fix.
3. **Subscribe cursor validation before gate/catch-up** — valid resume
   cursors from adapter-originated events can be rejected "beyond current
   LSN" while the projection lags. Fix: cursor comparison under the gate
   after catch-up + issuer re-verification; reconnect regression test.
4. **Aggregate audit coverage gaps** — lockdown-blocked mutations return
   unaudited; repeated lockdown entry/exit unaudited; LoadSecuritySnapshot
   decisions unaudited; current-session revocation mutates before auditing
   (audit failure → unaudited success). Fix: audit all of these; reorder
   mutation/audit; audit-query + injected-write-failure tests.
5. **Cockpit claims lockdown before core confirmation** — sets
   lockdown.active=true before the EnterSecurityLockdown result; on denial
   it falsely presents containment. Fix: lockdown_submitting local state
   while awaiting; "active" only from confirmed result or reconciled
   snapshot; on denial restore prior posture; on unknown outcome force
   reconcile without claiming success.
