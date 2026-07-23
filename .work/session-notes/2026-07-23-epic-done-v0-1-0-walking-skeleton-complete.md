# Session note — 2026-07-23 (epic maximum review converged; v0.1.0 walking skeleton DONE)

A durable handoff note for the next session. Read this before continuing.

## What happened this session

`epic-v0-1-0-implementation` — the entire v0.1.0 walking skeleton — went
through a **maximum-weight aggregate review** (operator raised the weight:
"it's the entirety of v0.1.0") and converged after 3 passes + 2 fix waves +
2 model classes. The epic is **done**. This session also switched the
orchestrator to Kimi K3 (operator's trial), with explicit permission to spawn
`kimi-coding/*` subagents — K3-vs-gpt-5.6-sol became the cross-model review
axis (all implementers were gpt-5.6-sol).

### Pass 1 — complementary/completeness (kimi-coding/k3)

Found 4 aggregate blockers invisible to per-feature review (all suites test
pairwise seams): B1 (pi-adapter e2e red at HEAD — the trust boundary broke the
self-asserting fixture, no CI caught it), B2 (LoadSnapshot has no producer —
CLI session-health/instruct + cockpit reconcile dead), B3 (cockpit treats the
seam's normal stream completion as an error — permanent degraded banner +
locked composer), B4 (authority-domain defaults disagree — cockpit rejected
out-of-the-box). Plus I1-I5. Operator decisions: I1 ship honest partial +
re-scope docs; include I3 (login UI) + I5 (composed e2e + runbook) in the fix
wave.

### Fix wave 1 — 9 commits (2 parallel workers, orchestrator committed)

B2 snapshot materialize-on-read (orchestrator reconciled the web-server
core-smoke's stale `present:false` assertion — the cross-boundary seam neither
worker could touch), B1 pi-adapter e2e real auth, B3 cockpit clean-completion,
B4 domain templating, I3 login view, I2 CI TS suites, I5 composed e2e +
runbook, I4 docs roll-forward to honest partial.

### Pass 2 — adversarial (gpt-5.6-sol)

Found 4 more blockers (all verified): B1' (filter×reconcile wedge — the
trust-boundary filter's intentional LSN gaps deadlock the cockpit), B2' (stale
adapter generation stays authenticated), B3a (delivery-ack ordering rots
commands at delivered), B3b (no adapter-liveness mechanism), B4' (non-loopback
plaintext core transport). Plus I1' (revocation surface — parked), I2' (forged
sender), I3' (stranded sessions). Operator decisions: Q1a redeliver
delivered-not-running; Q2a stream-disconnect staleness; Q3a loopback-
constrain; B2' option 2 (per-attachment fencing token, after the first worker
correctly surfaced that a server-only fix was impossible).

### Fix wave 2 — 6 commits (3 workers: the B2' blocker needed server+pi-adapter)

B1' cockpit gap tolerance, B2' fencing token (mint at Attach, in-memory hash,
invalidate on re-attach, response-metadata return — no proto change; the
generation boundary genuinely enforced), B3a redelivery, B3b staleness, B4'
loopback constraint, I2' sender normalization, I3' stranded sessions.

### Pass 3 — convergence (kimi-coding/k3)

**Approve with comments.** All 7 pass-2 fixes verified genuine (3 probe tests
against live code; every attack rejected with evidence). 0 new blockers. 1
important (P3-I1: B3b staleness coverage narrowed by the polling fallback —
death between polls/mid-execution still presents live/working until restart).
Receiver adjudication: Important-not-Blocker; documented in RUNBOOK
(correcting the af00794 overclaim) + parked as
`backlog-adapter-staleness-full-coverage`. Converged → **epic done**.

## Where we are

**`epic-v0-1-0-implementation` is DONE.** The v0.1.0 walking skeleton is
complete: 9 features + core, all done, end-to-end verified by the composed
walking-skeleton e2e (`cd e2e && npm test`: core + Pi adapter + CLI instruct →
live/working → completed/idle). Full verification matrix green: Rust workspace
+ clippy, web-server 23/23 + core-smoke, web-cockpit 39/39, cli 16/16 +
core-smoke, pi-adapter 6/6, contracts vectors + presentation + models.

## What's next (post-v0.1.0 candidates, all parked as backlog)

- `backlog-adapter-staleness-full-coverage` — heartbeat/last-report-age
  staleness or long-poll delivery (P3-I1; also folds P3-N1 running-rot bound
  + P3-N2 per-poll rebuild perf note).
- `backlog-revocation-lifecycle-surface` — full revocation/lockdown surface
  (epic pass-2 I1'; SECURITY:203-208).
- `feature-v0-core-diagnostics` (not yet scoped) — the audit-query/inspect-
  command/adapter-status prerequisite (QuerySpec/QueryResult/AuditRecord
  schema + query projection + audit-log storage). The honest v0.1.0 partial
  ships these as stubs.
- `epic-public-product-contract` (v1.0.0) — sidelined pending this epic; now
  unblocked.
- Release: v0.1.0 has not been tagged/released. The release-deploy skill
  handles the release when the operator is ready.

## Notes for the next session

- **Model routing changed (2026-07-23):** orchestrator is Kimi K3; `kimi-coding/k3`
  subagents explicitly permitted (1M ctx — excellent for epic-scope review).
  gpt-5.6-sol remains the implementation/adversarial tier. The umans/* spawn
  ban referred to umans-provider models (glm-5.2, kimi-2.7), NOT kimi-coding/.
- Operator decisions this session (do not re-litigate): maximum review weight;
  I1 ship-honest-partial + doc re-scope; include I3/I5 in fix wave 1; Q1a
  redeliver delivered-not-running; Q2a stream-disconnect staleness; Q3a
  loopback-constrain; B2' option 2 fencing token; P3-I1 document+park.
- The composed e2e (`e2e/walking-skeleton.mjs`) + `docs/RUNBOOK.md` are the
  epic-level proof + operator guide.
- Two parked backlog items carry the honest limitations:
  staleness-full-coverage and revocation-lifecycle.
- The docs now honestly describe the v0.1.0 partial: session-health committed;
  the 3 diagnostic commands + durable audit log reserved post-v0.1.0.
- `.work/bin/work-view` still hangs on board/--blocking. `check:drift` is the
  known broken repo gap.
