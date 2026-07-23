# Session note — 2026-07-23 (epic maximum review: pass 1 + fix wave done; pass 2 adversarial in flight)

A durable handoff note for the next session. Read this before continuing.

## What happened this session

The operator raised the epic review weight to **maximum** (the epic is the
entirety of v0.1.0). The orchestrator model was switched to Kimi K3 (the
operator is trying it out), with explicit permission to spawn `kimi-coding/*`
subagents — the `umans/*` spawn ban referred to umans-provider models (glm-5.2,
kimi-2.7), not the new `kimi-coding/` provider. This makes K3-vs-gpt-5.6-sol a
genuine cross-model axis: all implementers were gpt-5.6-sol.

### Pass 1 — complementary/completeness (kimi-coding/k3, fresh context)

Verdict: **Request changes** — 4 verified aggregate blockers + 5 important
findings, all invisible to per-feature review (every suite tests pairwise
seams; the composed system was broken in the operator's daily paths). The
per-feature mutation-verified properties are genuinely real; the composition
was not.

**Blockers (all orchestrator-verified before fixing):**
- **B1** — pi-adapter e2e RED at HEAD (trust boundary broke the self-asserting
  fixture; no CI caught it).
- **B2** — `LoadSnapshot` has no producer (materializer deferred) → CLI
  `session-health`/`instruct` + cockpit reconcile dead in production.
- **B3** — cockpit treats the seam's *normal* stream completion as an error →
  permanent degraded banner + locked composer.
- **B4** — authority-domain defaults disagree (cockpit hard-coded
  `operator-domain`; core/CLI `default`) → cockpit rejected out-of-the-box.

**Important:** I1 core-diagnostics scope (OPERATOR DECISION: ship honest
partial, re-scope foundation docs — done), I2 no TS suites in CI, I3 no
browser login UI (OPERATOR: include in fix wave), I4 README/SECURITY drift,
I5 no composed e2e + runbook (OPERATOR: include).

### Fix wave — 9 commits, all verified

Two parallel workers (disjoint write sets; no worker commits — the orchestrator
verified + committed; cross-boundary smokes deferred to the orchestrator):

- `428c219` B2 — LoadSnapshot materializes on read from the rebuilt projection
  (orchestrator reconciled the web-server core-smoke's stale `present:false`
  assertion — the cross-boundary seam neither worker could touch).
- `80c76d0` B1 — pi-adapter e2e through the real trust boundary (6/6).
- `29fb9f9` B3 — clean completion = normal polling boundary; degrade only on
  errors/gaps. Mutation tests re-verified.
- `5f4690e` B4 — web-server templates the configured domain into served HTML.
- `c08f963` I3 — cockpit login view (401 → form → /login → startup).
- `cadddfe` I2 — CI runs all 4 TS suites + core build.
- `6c1d748` I5 — composed walking-skeleton e2e (core + adapter + CLI instruct
  → live/working → completed/idle). PASSES — the epic-level proof.
- `6db620f` I4 — README + SPEC/PROTOCOL/UX/SECURITY rolled forward to the
  honest partial (3 diagnostic commands + durable audit log → reserved/
  post-v0.1.0; session-health committed).
- `a8f6b2e` — `docs/RUNBOOK.md` (written by the orchestrator; worker A couldn't
  touch docs/).

Final integration verification: all suites green (Rust workspace, 4 TS
packages, contracts, composed e2e).

### Pass 2 — adversarial (gpt-5.6-sol, fresh context) — IN FLIGHT

Attacking the fixed system: trust-boundary end-to-end (core-secret blast
radius, loopback-binding edge cases, session/principal/actor cross-binding),
cross-feature "earned not asserted" claims (identity-before-submission core-
side, Subscribe filter fail-closedness for future kinds, adapter report
authentication, concurrent first-answer), the fixes themselves (B2 staleness
window, B3 gap window, B4 template injection, I3 throttle/error-oracle),
composed operational failure modes, compromised-principal blast radius.
Severity grounded in the v0.1.0 threat model (single-operator, localhost-ish).

## Where we are

Epic at `stage: review`. Pass 2 in flight. After it returns: adjudicate, fix
any receiver-confirmed material blockers, converge (re-review until a pass is
clean), then close the epic → done.

## Notes for the next session

- Model routing changed: orchestrator is now Kimi K3; `kimi-coding/k3`
  subagents are explicitly permitted by the operator (1M ctx — good for
  epic-scope reviews). gpt-5.6-sol remains the implementation/adversarial tier.
- Operator decisions this session (do not re-litigate): maximum review weight
  for the epic; I1 ship-honest-partial + doc re-scope; include I3 + I5 in the
  fix wave.
- The web-server core-smoke reconciliation pattern: when a fix changes core
  behavior, grep ALL test packages for assertions of the old behavior
  (`present, false`) — the cross-boundary seam belongs to the orchestrator.
- `docs/RUNBOOK.md` exists now (startup order, env matrix, bootstrap flow).
- `.work/bin/work-view` still hangs on board/--blocking. `check:drift` is the
  known broken repo gap.
