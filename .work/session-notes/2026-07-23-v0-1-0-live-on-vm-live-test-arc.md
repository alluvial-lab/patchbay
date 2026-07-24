# Session note — 2026-07-23 (v0.1.0 walking skeleton live on the VM; live-test arc complete)

A durable handoff note for the next session, written before a context reset. Read this first.

## Where we are

`epic-v0-1-0-implementation` is **done** (maximum-weight review converged pass 3 —
see `2026-07-23-epic-done-v0-1-0-walking-skeleton-complete.md`). This session then took the
milestone from "reviewed" to **actually running and operator-tested on the VM**: the full
four-process stack was brought up against a real Pi session (kimi-coding/k3, thinking high)
with the machine's real provider, and the operator drove it from their phone over the LAN.
Roughly ten real bugs surfaced in live use; all were fixed inline and committed.

## The running stack (live as of this note)

- **Core** (`patchbay-core-server`, pid 62063): `devup` authority domain, db at
  `tmp/devup/core.db`, listeners `127.0.0.1:50051` (network) + `127.0.0.1:50052` (loopback admin).
- **Pi adapter** (pid 1336464): attached, hosting `dev-session-1` (kimi-coding/k3), fencing-token auth.
- **Web-server** (pid 1321940): `https://192.168.50.110:3000` (TLS, LAN-reachable; self-signed cert
  in `tmp/devup/`). The cockpit has a login form.
- **CLI:** `node cli/dist/src/main.js` (not installed globally). Credentials at
  `tmp/devup/cli-credentials.json` (0600).
- **Login (both cockpit form + CLI):** `operator-dev` / `<redacted — rotated 2026-07-24>`.
  This note originally carried the live password in cleartext; gate-security flagged it
  (Critical) and the credentials were rotated. **Lesson, now convention: tracked notes
  never carry live secrets — describe where they live (tmp/devup, 0600), not their values.**
- Scratch state is all under `tmp/devup/` (db, credentials, TLS cert, workspace). `rm -rf tmp/devup`
  resets everything. A start script for the adapter lives at `/tmp/start-adapter.sh` (edit env as needed).

### Bring-up / manage

- Core: see the core's `nohup` invocation in git history of `tmp/devup/core.log` env (or
  `docs/RUNBOOK.md` — the runbook is accurate and current).
- Adapter: `setsid /tmp/start-adapter.sh > /tmp/adapter-live.log 2>&1 < /dev/null &`
  (background launches via plain `nohup ... &` were flaky in this shell; setsid+disown works).
- Web-server: `nohup env PATCHBAY_CORE_ADDR=http://127.0.0.1:50051 PATCHBAY_CORE_SECRET=<redacted — rotated 2026-07-24>
  PATCHBAY_AUTHORITY_DOMAIN_ID=devup PATCHBAY_OPERATOR_ID=operator-dev
  PATCHBAY_WEB_BIND_ADDR=192.168.50.110:3000 PATCHBAY_TLS_CERT=tmp/devup/tls-cert.pem
  PATCHBAY_TLS_KEY=tmp/devup/tls-key.pem node web-server/dist/src/main.js > tmp/devup/web.log 2>&1 &`
- **Stale web-server processes hold :3000** across restarts — `pkill -9 -f "node web-server"` then
  check `ss -tlnp | grep :3000` before rebinding.
- After adapter restarts, the CLI's core session can be invalidated; re-run `cli login` to recover
  (trust boundary failing closed, by design).

## Live-use fixes landed this session (all committed)

In order:
1. `119ec81` — cockpit fetch binding (login threw "called on an object that does not implement Window").
2. `b87362d` — CSRF interceptor matched lowercase `submit`; Connect method names are proto-case
   (`Submit`), so every Submit 403'd. +3 regression tests.
3. `a12bcf7` — three UI fixes: viewport/body reset (white frame), Enter-to-send (Shift+Enter=newline),
   instruct dedup (command.lsn advances to completed LSN → matched/placed by accepted LSN instead).
4. `32b15c3` — thinking blocks are not message content (thinking streams via `activityDetail`).
5. `7a89fba` — **the big one: per-message delta ordinal.** The dedup was dropping ~46% of streamed
   deltas because the eventId's content-length suffix repeated for consecutive deltas (verified
   Q/P/O all at len 23, etc.). Root of the "scrambled gibberish while streaming."
6. `353c96a` — serialize per-session transcript reports (preserve stream arrival order; complements
   the ordinal fix).
7. `c672128` — bound the body to the viewport (`height:100dvh; overflow:hidden`) so the timeline
   scrolls internally instead of the page — matched against the locked mockup `option-2.html`.
8. `4a903cd` — paperclip icon for the Attach button (was inflating the composer into a big circle).

Every fix was verified against the real stack (not just tests) — several were mock-vs-real gaps the
test suites couldn't see. The pattern held: live testing on the real composed system flushes what
pairwise tests can't.

## Parked / backlog items (all in `.work/backlog/`)

- `backlog-adapter-staleness-full-coverage` — **now proven to bite in real use**, not just review:
  a taco turn stuck at `running` forever after an adapter restart mid-turn (P3-N1 running-rot), and
  the B3b staleness coverage gap (P3-I1). Consider bumping priority.
- `backlog-revocation-lifecycle-surface` — full revocation/lockdown surface (epic pass-2 I1).
- `backlog-session-model-field` — surface the agent model in session reports. Operator decision:
  proper proto field (mutable session state, models change mid-session), NOT the observation hack.
- `backlog-icon-set-adoption` — **Lucide, resolved** (matches the operator's parallel project
  `projects/SNC/platform`, which uses `lucide-react`). Implement as inline SVG paths (no build step).
- Core-diagnostics (the CLI's 3 stubbed commands: audit-query/inspect-command/adapter-status) —
  NOT yet scoped as a feature. The honest v0.1.0 partial ships them stubbed.

## Known live issues / honest caveats

- The taco turn stuck at RUNNING in the cockpit is a real P3-N1 instance (killed adapter mid-turn).
  Fresh turns complete normally.
- Old pre-fix messages (thinking-leak gibberish, dropped-delta scrambling) are still in the durable
  log and replay as-is — that's the honest record; the fixes prevent NEW occurrences.
- `docs/RUNBOOK.md` documents the v0.1.0 limitations honestly (staleness coverage, running-rot).

## Decisions NOT to re-litigate

- All epic-review operator decisions (maximum weight; I1 ship-honest-partial + doc re-scope; include
  I3/I5 in fix wave; Q1a redeliver delivered-not-running; Q2a stream-disconnect staleness; Q3a
  loopback-constrain the general listener; B2 option 2 fencing token; P3-I1 document+park).
- CLI auth posture option 1 (full transport principal).
- Trust boundary D1 (core as source-of-truth) / D2 (dedicated local-console listener).
- select-many renders as select-one; DENIED→Declined; Q2 delivery-trace reduced to a reserved seam.
- Model-surfacing: proper proto field. Icon set: Lucide.

## Model routing (changed 2026-07-23)

- The orchestrator is now **Kimi K3** (operator's trial). `kimi-coding/k3` subagents are explicitly
  permitted by the operator (1M ctx — excellent for epic-scope review). The `umans/*` spawn ban
  referred to umans-provider models (glm-5.2, kimi-2.7), NOT `kimi-coding/`.
- `openai-codex/gpt-5.6-sol` remains the implementation / adversarial-review tier.
- Cross-model review axis this arc: K3 (complementary/epic passes) ↔ gpt-5.6-sol (implementers +
  adversarial pass).

## What's next (operator's call)

- v0.1.0 is built and operator-tested but **not released** (no tag). `release-deploy` when ready —
  the gates (security/tests/cruft/docs/patterns) will run; expect gate items given the live-use arc.
- Priority candidates from live evidence: `backlog-adapter-staleness-full-coverage` (running-rot
  bites), core-diagnostics (if v0.1.0 should ship the diagnostic commands after all — but the
  operator already chose the honest partial).
- The stack is left running for continued operator testing.
