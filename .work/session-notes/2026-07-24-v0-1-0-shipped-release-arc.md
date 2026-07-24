# Session note — 2026-07-24 (v0.1.0 SHIPPED: fix wave, gates, tag, stack on release bits)

A durable handoff note for the next session, written before a context reset. Read this first.

## Where we are

**v0.1.0 is tagged and shipped.** Tag `v0.1.0` (annotated, at `a752ade`) pushed to
origin; release collapsed at `5c13b9c`; the single summary doc is
`.work/releases/v0.1.0/release-v0.1.0.md` (145 items; bodies prune to git history
per delete-refs — recover with `git show a752ade:<former active path>`).
The live stack runs the release binaries.

## What happened this session (in order)

1. **Pre-release fix wave** (3 features, all done + reviewed standard weight):
   - `feature-adapter-staleness-liveness` — long-lived `ReceiveDeliveries`
     subscription replaced the polling fallback; stream-drop staleness now spans
     the adapter's lifetime; running-rot reconciles to `failed(execution_outcome_unknown)`.
     Reviewed by kimi-coding/k3 (APPROVE; dual-projection deviation validated sound;
     RUNBOOK honesty fixes in-wave).
   - `feature-cockpit-icon-set` — Lucide inline-SVG catalog + `.icon` primitive,
     bound into check-presentation. Reviewed by sol (APPROVE; square icon geometry fixed).
   - `feature-session-model-field` — `SessionModelChanged` mutation, model end-to-end
     (proto→core→adapter→cockpit/CLI). Review BLOCKED (fabricated idle + unreplayable-log
     race), fixed (`6718292`, `d6fa8a2`), receiver-verified, done.
2. **Release v0.1.0** — bound 131 items (+13 gate items +1 patterns record = 145),
   mapping `tag-based` recorded in CONVENTIONS.md, binding guard clean.
3. **Gates ran** (first ever): security 5 (1 critical, 4 medium), tests 1 (high),
   cruft 1, docs 6, patterns 6 promoted. All findings fixed and verified before ship.
4. **The Critical**: the 2026-07-23 session note had live credentials in cleartext.
   Rotated FOR REAL (full re-bootstrap — no rotation RPC exists), old creds verified
   dead, note scrubbed. **New convention: tracked notes NEVER carry live secrets.**
5. **ENOSPC incident**: `/tmp` (7.9G tmpfs) filled with **201,272 leaked SQLite temp
   files (6.6G)** from the Rust test suites → Pi crashed, 2 workers died. Cleaned;
   structural fix parked (`backlog-test-tempfile-hygiene`). Dead workers' uncommitted
   work was intact and sound; resumed and completed.
6. **Ship + rebuild**: changelog → tag → push → collapse; full stack rebuilt from
   tagged source and restarted with the rotated credentials.

## The running stack (live as of this note)

- **Core**: `devup` authority domain, db `tmp/devup/core.db` (0600, tightened by the
  new binary), listeners `127.0.0.1:50051` + `127.0.0.1:50052` (loopback admin).
- **Web-server**: `https://192.168.50.110:3000` (TLS, LAN-reachable).
- **Pi adapter**: attached (generation 2), hosting `dev-session-1` (kimi-coding/k3).
- **Credentials**: rotated 2026-07-24. They live ONLY in `tmp/devup/.secrets.env`
  (0600, gitignored) — source it for env. CLI stores: `~/.patchbay/cli-credentials.json`
  + `tmp/devup/cli-credentials.json`. Old deployment preserved at
  `tmp/devup-prerotate-20260724/` (gitignored; safe to delete).
- **Bring-up**: `docs/RUNBOOK.md` is current. Adapter: `setsid /tmp/start-adapter.sh
  > /tmp/adapter-live.log 2>&1 < /dev/null &` (bump `PATCHBAY_ADAPTER_GENERATION`
  on each re-attach — fencing). After core restarts, CLI sessions invalidate →
  re-run `cli login` (trust boundary failing closed, by design).
- **Do not** `pkill -f patchbay-core-server` from a shell whose own cmdline contains
  the pattern — it self-matches. Use explicit PIDs.

## Behavioral changes the operator should know

- **CLI rejects `--password`/`--setup-secret` on argv.** Use
  `PATCHBAY_OPERATOR_PASSWORD` / `PATCHBAY_SETUP_SECRET` env or the TTY prompt.
- **Operations now require a validity window** — CLI/web set a 5-minute default;
  expired/not-yet-valid is rejected at acceptance (zero-skew v0.1.0 policy).
- **Cockpit/CLI show the session model** (`kimi-coding/k3` etc.).
- **Sessions now go `stale` on adapter stream drop** — kill the adapter and watch
  session-health; re-attach restores truth. Running commands on a dead adapter
  reconcile to `failed(execution_outcome_unknown)` — retry judgment per
  idempotency_strength (RUNBOOK).
- **e2e suite** (`cd e2e && npm test`) is green and documented in RUNBOOK as the
  composed smoke to run after cross-component changes.

## Parked / backlog (notable)

- `backlog-test-tempfile-hygiene` — test suites leak SQLite temp files into /tmp
  (the ENOSPC root cause). Real fix: scoped temp root + cleanup.
- `backlog-session-report-source-ordering` — stale report can roll mutable fields
  backward (contract hardening: adapter-side revision).
- `backlog-revocation-lifecycle-surface` — includes the missing password-rotation path.
- Pre-existing `buf lint` RPC naming violations (12, unrelated protos) — unaddressed.
- WAL/SHM reopen only tightens the main db file to 0600 (manual chmod needed once).

## What's next (operator's call)

- **Dogfood v0.1.0**: the running-rot fix wants a live kill-mid-turn test; phone
  testing over LAN works (cockpit login = operator-dev + rotated password).
- **v1.0.0 arc**: `epic-public-product-contract` (implementing) is unblocked —
  5 features at drafting: public-compatibility, adapter-portability-proof,
  self-hosted-operations, publication-governance, executable-release-assurance.
  Design work that doesn't need running code can proceed any time.

## Model routing (unchanged from 2026-07-23 note)

- Orchestrator: umans/umans-glm-5.2. Subagents: openai-codex only (never umans/*).
- Implementation/design: gpt-5.6-terra (mid) / gpt-5.6-sol (complex+correctness-critical).
- Cross-model review axis: sol ↔ kimi-coding/k3 (operator-permitted).
