# 2026-07-27 — Observability epic shipped; dogfooding live; 5 cockpit fixes

## Where things stand

`epic-observability-dogfooding` is **done** (all 4 features + 5 stories, 23
review blockers fixed across 5 independent review passes). The dogfooding
stack is **live with real use**: core (50051/admin 50052 loopback),
pi-adapter (one preprovisioned session `patchbay-main`, cwd
`/home/agent/projects/patchbay`, model `openai-codex/gpt-5.6-sol`), web-server
(TLS on `192.168.50.110:3000`, self-signed cert at `~/.config/patchbay/tls/`).
Durable state now lives outside /tmp: DB at
`~/.local/state/patchbay/patchbay.sqlite3`, adapter JSONL diagnostics at
`~/.local/state/patchbay/adapter.log`, env (0600) at `~/.config/patchbay/env`.
Previous runtime state was tmpfs and is gone; operator re-bootstrapped.

Git: main is ~10 commits ahead of origin (operator pushes manually).

## What happened this session (after epic close)

- Drift/CI story done: contracts regenerated with pinned toolchain (buf
  1.71.0 / protoc-gen-prost 0.5.0 / protoc-gen-es 2.12.1), `check:drift`
  wired into CI, README pins documented. CI follow-up noted: protoc-gen-prost
  cargo install is uncached.
- Backlog triage: 4 research handoffs pruned (absorbed by v0.1.0), resolved
  flake item pruned. Backlog ≈ 26 items.
- Stack bring-up: no processes were running; DB/tmpfs state lost. Fresh
  bootstrap. Web-server bound LAN+TLS (was loopback-only default).
- Dogfooding fixes (all operator-confirmed, each a standalone story):
  1. **Render amplification** — no agent text: ~1,900 events/turn × one sync
     full re-render each froze the tab. Fixed with rAF render coalescing.
  2. **Tool-call args preview** — "Running bash" with no content; now Pi-style
     preview inside one card (plain-text, never markdown; 240-char cap).
  3. **Scroll anchor** — rebuild reset scrollTop to 0 for users reading
     history; anchor capture/restore across rebuilds. Mobile page-scroll
     variant noted as not covered.
  4. **In-timeline activity indicator** — working/thinking/using-X row at the
     transcript tail. Also first-ever CSS for `.activity-indicator` (session
     list was unstyled bare text).
  5. **Expired session startup crash** — 8h web-session TTL; startup only
     treated 401 as login-needed, 403 (expired/revoked) crashed. Now both →
     login flow.

## Watch items / lessons

- **Process lessons recorded in story bodies**: verify edits actually landed
  (a rejected multi-edit batch silently shipped CSS-only once — caught by
  operator screenshot); verify the BUILT bundle contains the change.
- **Two agents commit to this repo concurrently** (this session + the
  cockpit-driven `patchbay-main` session, which scoped
  `epic-agent-operations-resource-plane` on its own arc). Disjoint files so
  far; linear history; watch for same-file collisions.
- pi-adapter e2e had an intermittent flake — root-caused as test-side
  cancellation race, fixed; item pruned.
- `docs/RUNBOOK.md` covers the bring-up incl. `PATCHBAY_ADAPTER_LOG`; web
  sessions are 8h TTL (login page now appears correctly on expiry).

## Held for next session: cockpit UX batch (mockup-first)

Four dogfooding-sourced items, one `/ux-ui-design:screens` pass:
1. `idea-session-list-row-redesign` — hierarchy, path wrapping; HIGH mobile
   impact (session list IS the mobile home at ≤760px).
2. `idea-delivery-line-layout-stability` — status bar reflows on state
   change (interrupt button appears/disappears); fold into instruction card.
3. `idea-cockpit-settings-section` — first settings: hide tool calls,
   preview expansion/cap, density.
4. Tool-call single-row lifecycle (Running→finished in place, Pi-style) —
   noted in `story-fix-tool-call-args-preview` body.

## Board

- Active: `epic-public-product-contract` (implementing; 5 features drafting,
  v1.0.0 arc) + the cockpit agent's `epic-agent-operations-resource-plane`
  (drafting, owned by that session — coordinate before touching).
- Backlog: ~26 items (authority hardening cluster is the largest coherent
  group; sessions/protocol; hygiene).
- Nothing at review; all observability + dogfooding-fix items done.
