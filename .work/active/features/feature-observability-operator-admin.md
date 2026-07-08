---
id: feature-observability-operator-admin
kind: feature
stage: done
tags: [foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-persistence-snapshot-model]
created: 2026-06-28
updated: 2026-07-08
gate_origin: null
release_binding: null
---

# Feature: Define operator/admin observability

## Misroute note (2026-07-07)

Stripped `[prose]` — this is a design feature, not prose authoring. The scope involves genuine design decisions: (1) whether observability is v0 or post-v0 is a scope/classification decision, not consolidation of an already-settled answer (the docs don't currently settle it); (2) "the v0 control surface or CLI has enough diagnostic expectations to debug failed delivery" requires designing what diagnostic surfaces exist (delivery trace, logs, metrics, event inspection) and what "enough" means; (3) "security docs cover what must not be logged" requires deciding redaction/sensitive-payload-handling rules with security consequences. These are choosing-between-approaches / architectural-commitment decisions, not collapsed prose authoring of settled material. Routed through `feature-design`; `prose` tag removed. Same misroute pattern documented in the epic's lane-routing discipline and the 2026-07-06 codification of the prose black-box test.

Review noted that Patchbay should help the operator answer why a command did not deliver. Observability is part of the human control plane, not just implementation plumbing.

## Scope

- Health and status of core, adapters, and control surfaces.
- Delivery trace for a command: accepted, routed, adapter response, execution result.
- Logs, metrics, and event inspection expectations.
- Safe redaction and sensitive payload handling.
- CLI/admin debugging requirements.

## Acceptance criteria

- Foundation docs identify observability as v0 or post-v0 with clear scope.
- The v0 control surface or CLI has enough diagnostic expectations to debug failed delivery.
- Security docs cover what must not be logged.

## Related parked ideas

- `idea-multi-human-coordination` — v0 remains single-operator unless this feature decides otherwise, but the foundation should not foreclose future multi-human authority domains, grants, audit, handoffs, or third-party coordination surfaces.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.

## Design decisions (operator-confirmed 2026-07-07)

- **Delivery-trace scope**: v0 CLI trace + web shows current `CommandState` only. Full web trace-timeline UI is polish that can wait. — The CLI is the designated debug surface (`docs/ARCHITECTURE.md`, `docs/UX.md`); the web cockpit's v0 job is operational control, not forensic timeline UI.
- **Trace data source**: projection of the existing durable event log + audit records, filtered by command id / correlation id. No new storage. — The audit log already records every lifecycle event with correlation id + LSN (`docs/SECURITY.md` "Audit events"); a dedicated trace structure would violate SSOT and the single-writer invariant.
- **v0 diagnostic surface set**: `audit-query`, `inspect-command`, `session-health`, `adapter-status` (v0); `event-inspect`, metrics, dashboard, SIEM (defer post-v0). — The v0 set is cheap projections of already-modeled state + the audit log; the deferred set is premature for single-operator v0 (the audit log is the substrate until a real load profile exists).
- **Performance posture**: v0 carries a qualitative responsiveness floor, not quantitative targets. Observability answers "is this fast enough?" operationally against that floor, not by reporting against a spec'd SLA. — See `docs/SPEC.md` "V0 performance posture" (added this feature). No fabricated budgets constrain the implementation before a real usage pattern exists.

## Architectural choice

Observability in v0 is **a view over the durable event log + audit records, not a separate observability subsystem.** This follows the SSOT and single-writer principles: the durable event log is already the source of truth for command/session/adapter state, and the audit log already records every security-relevant decision and lifecycle event with correlation id + LSN. The v0 diagnostic surface is a set of CLI commands that project and filter these existing records — no new storage, no second writer, no metrics pipeline.

This is chosen over (b) a dedicated per-command trace structure (violates SSOT, duplicates the log) and (c) a metrics/counters subsystem (premature for single-operator v0; the audit log is the substrate until a load profile justifies counters/histograms). The trade-off: v0 observability is query-oriented ("what happened to this command?") rather than monitoring-oriented ("what's the current throughput?"). Monitoring is deferred.

## Implementation units

This is a docs-only feature (defines observability expectations in foundation docs + a CLI command inventory). Per the rolling-foundation discipline, the "units" are doc sections and a command registry, not code modules. Code implementation of the CLI commands belongs to the v0 walking-skeleton implementation features that follow.

### Unit 1: Observability scope + posture in `docs/SPEC.md`
**File**: `docs/SPEC.md` (already partly done — "V0 performance posture" section added)
**Scope**: Confirm observability is v0 (committed v0 behavior) with the scope boundary: v0 = audit log + 4 CLI diagnostic commands + web shows current `CommandState`; trace timeline, metrics, dashboard, raw event inspection, SIEM all deferred.
**Acceptance criteria**:
- [ ] SPEC identifies observability as v0 with the committed/deferred split.
- [ ] The qualitative responsiveness floor is stated (already in "V0 performance posture").
- [ ] No quantitative perf target is committed (already stated).

### Unit 2: Diagnostic surface expectations in `docs/UX.md` or `docs/PROTOCOL.md`
**File**: `docs/UX.md` (CLI diagnostic surface section) + `docs/PROTOCOL.md` (audit-query as a read surface)
**Scope**: Document the v0 CLI diagnostic command set and what each surfaces. The web cockpit's v0 observability role is limited to current `CommandState` / last transition display (already covered by UX.md's session-detail + delivery-state expectations).

The v0 CLI diagnostic command inventory (all read-only projections of existing state):

| Command | Surfaces | Data source |
|---|---|---|
| `audit-query` | Filter audit records by actor / command / target / time / outcome | audit log (durable, queryable per SECURITY.md) |
| `inspect-command <id>` | Full lifecycle + audit trail for one command (accepted → routed → delivered → running → terminal, with timestamps + LSNs) | event log + audit records, filtered by command id / correlation id |
| `session-health` | Session connectivity × activity axes (live/stale/offline/unknown × idle/working) for one or all sessions | session state axes (PROTOCOL.md Session state axes) |
| `adapter-status` | Attached adapters, capability manifests, attach LSN, adapter generation | adapter registry (ARCHITECTURE.md adapter lifecycle) |

Deferred to post-v0: `event-inspect <lsn>` (raw event at LSN), metrics (counters/histograms/throughput), dedicated health/status dashboard, SIEM export.

**Acceptance criteria**:
- [ ] UX.md or PROTOCOL.md documents the 4 v0 CLI diagnostic commands and their data sources.
- [ ] Each command is a read-only projection (no new write path, no new storage).
- [ ] The deferred set is named as post-v0 (reserved seam, not silently absent).
- [ ] The delivery trace is documented as a view over the audit + event log (not a separate trace store).

### Unit 3: Redaction confirmation in `docs/SECURITY.md`
**File**: `docs/SECURITY.md` ("Audit events" section — already pinpoints the rules)
**Scope**: Confirm the redaction rules in SECURITY.md cover the observability surfaces. The Audit events section is the canonical no-log/redaction list for Patchbay (declared canonical during review — other docs summarize or point here, they do not maintain competing lists): audit records must not store raw session cookies, CSRF tokens, access tokens, passwords, bootstrap secrets, encryption keys, command prompt bodies by default, sensitive attachments, or adapter attachment material (`attachment_method.descriptor`); `response_contract.sensitivity` + the `secret` contract kind enforce redaction at the boundary before audit/snapshot materializes. This unit *confirms* that coverage extends to the new diagnostic commands (they project the same audit records, so the same redaction applies) and that `adapter-status` excludes raw `attachment_method.descriptor` — rather than inventing new rules. (The threat-model bullet at SECURITY line ~55 was consolidated to point at this canonical list rather than carry a competing narrower copy.)
**Acceptance criteria**:
- [ ] Confirm the 4 v0 diagnostic commands project only already-redacted audit/event records (no new raw-payload exposure path).
- [ ] If any command would surface a field not covered by existing redaction rules, flag it (expected: none — `inspect-command` shows lifecycle + audit trail, not prompt bodies).

## Testing

- **Doc consistency**: the observability scope in SPEC, the CLI command set in UX/PROTOCOL, and the redaction rules in SECURITY must agree (no command surfaces a field SECURITY says must not be logged).
- **Conformance**: once code exists, each CLI command is a read-only projection — a property test confirms no command mutates durable state or writes a new record.
- **No new storage invariant**: a test confirms the diagnostic surface introduces no new durable storage path (the audit + event logs remain the only sources).

## Risks

- **Scope creep into monitoring.** The temptation is to add metrics/counters because they're "useful." Mitigation: the qualitative responsiveness floor (SPEC "V0 performance posture") makes monitoring unnecessary in v0 — if the operator *perceives* a problem, the audit/diagnostic commands answer "what happened," not "what's the throughput." Metrics are deferred until a load profile exists.
- **Redaction gap in a new command.** If a future diagnostic command surfaces a field not covered by SECURITY's redaction rules, it could leak sensitive material. Mitigation: Unit 3 confirms coverage; any new command's projection is checked against the redaction rules before adding.
- **Trace-as-authority confusion.** An operator might treat the delivery trace as authoritative command state rather than a projection. Mitigation: the trace is documented as a view over the audit + event log; `CommandState` in PROTOCOL.md remains canonical (consistent with the snapshot-correctness rule that UI state is never authoritative).

## Implementation order

1. Unit 1 (SPEC observability scope + posture) — the performance-posture section is already added; confirm/extend with the observability committed/deferred split.
2. Unit 3 (SECURITY redaction confirmation) — cheap; confirms existing coverage.
3. Unit 2 (UX/PROTOCOL diagnostic command set) — the substantive doc addition.

Single-stride implementation (one session can finish all three doc units). No child stories needed — the units are tightly cohesive (all doc edits defining one observability scope) and don't fan out.

## Extension pressure classification

- **Committed v0**: audit log (already v0); 4 CLI diagnostic commands (`audit-query`, `inspect-command`, `session-health`, `adapter-status`); web cockpit shows current `CommandState` / last transition; qualitative responsiveness floor.
- **Reserved extension seams (post-v0)**: trace timeline UI; metrics/counters/histograms; dedicated health/status dashboard; raw `event-inspect <lsn>`; SIEM export and long-retention compliance archives (already reserved in SECURITY.md); quantitative performance budgets/SLAs (reserved per SPEC "V0 performance posture").
- **Explicitly rejected for v0**: a dedicated per-command trace storage (violates SSOT); a metrics pipeline as the primary v0 observability substrate (premature; audit log is the substrate).

## Implementation notes

- Files changed: `docs/SPEC.md` (V0 observability scope section; V0 performance posture section added in the prior design stride), `docs/UX.md` (CLI section expanded with the 4-command diagnostic inventory + projection/redaction caveats), `docs/SECURITY.md` (Audit events section extended with a confirmation that the v0 diagnostic commands project already-redacted records).
- Tests added: none (docs-only feature; no code). Verification was an acceptance-criteria walk + cross-reference resolution + no-new-storage invariant + redaction-coverage check, all passing.
- Discrepancies from design: none. The design's Unit 2 noted the commands are "uses of the existing `query` OperationKind" — confirmed against `docs/PROTOCOL.md` OperationKind registry (line 155: `query` = "Read status, snapshot, capabilities, lists, history, metadata, or diagnostics"). (Initial implementation prose loosely allowed "CLI-local reads of the audit log"; review caught this as a persistence-boundary violation — all 4 commands now route as `query` Operations against the core, and a no-lifecycle bypass read is an explicitly reserved seam, not v0 behavior.)
- Adjacent issues parked: `story-fix-failurecode-execution-outcome-unknown` (FailureCode proto enum missing `FAILURE_CODE_EXECUTION_OUTCOME_UNKNOWN` — drift from PROTOCOL line ~356, introduced by `feature-idempotency-ambiguous-execution`; surfaced during this feature's pass-4 review).
- Cross-doc consistency verified: SPEC observability scope, UX CLI command set, and SECURITY redaction rules agree — no command surfaces a field SECURITY says must not be logged; the deferred set (trace timeline UI, metrics, dashboard, `event-inspect`, SIEM) is named consistently across SPEC and UX.

## Review findings (fresh-context deep review, 2026-07-08) → bounced → fixed

Verdict: **Block** (3 findings, all fixed this pass):

1. **BLOCKER (fixed): CLI-local audit reads violated the persistence boundary.** UX.md said commands may be "a CLI-local read of the queryable audit log," contradicting PROTOCOL.md (control surfaces never touch persistence directly) and ARCHITECTURE.md (storage port). Fixed: commands are now `query` Operations against the core; the no-lifecycle bypass read is explicitly a reserved seam, not v0 behavior. UX.md CLI prose + the `audit-query`/`inspect-command` data-source column updated.
2. **BLOCKER (fixed): `session-health` omitted canonical session states.** UX.md listed connectivity as `live/stale/offline/unknown` and activity as `idle/working`, but PROTOCOL.md's canonical registries include connectivity `failed` and activity `unknown`. Fixed: the `session-health` row now lists the full canonical registries (`SessionConnectivityState` 5 states × `SessionActivityState` 3 states).
3. **IMPORTANT (fixed): observability seams not in the extension-seams registry.** SPEC/UX classified trace timeline, metrics, dashboard, `event-inspect`, SIEM, perf budgets as reserved, but PROTOCOL's cross-cutting registry had no observability rows. Fixed: added 4 rows to `docs/PROTOCOL.md` extension-seams registry (C: v0 observability surface; R: trace UI/metrics/dashboard/event-inspect/SIEM/perf-budgets; X: dedicated trace storage + metrics-as-primary-substrate; R: no-lifecycle bypass read).

No nit-level findings. Re-review pending.

## Re-review (fresh-context, 2026-07-08) → APPROVED

Verdict: **Approve** (zero blockers, zero important, zero nits). All three prior findings confirmed FIXED:
1. Persistence boundary — diagnostic commands route as `query` Operations against the core; CLI-local/no-lifecycle bypass read explicitly reserved.
2. Session states — `session-health` lists the full canonical registries (5 connectivity × 3 activity states).
3. Extension-seams registry — 4 observability rows added to PROTOCOL.md (C/R/X/R), consistent with SPEC/UX/SECURITY.

Acceptance criteria met. SSOT/no-new-storage invariant holds. Redaction coverage holds. No new drift introduced by the fixes.

## Pass-3 review (full deep, 2026-07-08) → blocker fixed + important carry-forward

Pass 3 found a 4th blocker (SSOT drift: PROTOCOL's own audit-redaction summary omitted adapter attachment material after SECURITY was extended) and one important (generated-contracts derivability gap):

- **BLOCKER (fixed): PROTOCOL redaction summary was stale.** `docs/PROTOCOL.md` had its own audit-redaction list that SECURITY was extended to cover adapter attachment material, but PROTOCOL's summary was not updated → the two foundation docs disagreed on what must not be logged. Fixed: PROTOCOL summary now includes adapter attachment material and defers to SECURITY as the canonical list (prevents future drift).
- **IMPORTANT (carry-forward, not blocking this docs feature): query-contract shapes not first-class in `.proto`.** `audit-query` and `inspect-command` can route as `OperationKind = QUERY` with generic `PayloadEnvelope` carriage, but the typed filter/response proto shapes for audit-query filters and inspect-command results do not yet exist in `contracts/proto/`. This is a derivability gap for the CLI implementation feature, not a defect in this docs feature (which defines the diagnostic surface, not the wire contracts). Flagged here so the implementation feature derives the typed query payload/response contracts (registry-owned or generated) rather than hand-copying CLI DTOs. Not filed as a separate item — it belongs to the v0 walking-skeleton implementation scope.

## Pass-5 review (full deep, 2026-07-08) → SSOT consolidation landed

Pass 5 found the same redaction-drift class one more time: SECURITY line 55 (threat-model bullet) still enumerated the full no-log list in parens despite the pass-4 "consolidation," preserving the drift mechanism. Fixed properly this time — line 55 is now a pure pointer (no enumeration) to the canonical Audit events list. Also fixed stale item-body notes ("CLI-local reads" prose clarified as pre-fix state; "Adjacent issues parked: none" updated to reference the parked FailureCode story).

The recurring class has been redaction-list SSOT drift across passes 3b/4/5. The consolidation (one canonical list in SECURITY Audit events; all other mentions are pure pointers) is the structural fix; pass 5 confirmed everything else is converged. Re-review pending to confirm the pure-pointer consolidation holds.

## Pass-6 review (full deep, 2026-07-08) → APPROVED

Verdict: **Approve** (zero blockers, zero important, zero nits). All 11 lenses pass. The pure-pointer consolidation at SECURITY:55 structurally eliminated the redaction-drift class: exactly one canonical no-log/redaction list (SECURITY Audit events, declared canonical), SECURITY:55 is a pure pointer (no enumeration), PROTOCOL defers, no competing lists anywhere. ACs met. SSOT/no-new-storage invariant holds. Redaction coverage holds across all docs.

Feature closed on a full deep review (not a fix-verification). Six review passes total — each found a real drift; the recurring redaction-list SSOT class converged via the pure-pointer consolidation, not by patching competing lists.
