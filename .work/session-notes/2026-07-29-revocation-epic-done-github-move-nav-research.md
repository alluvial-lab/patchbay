# 2026-07-29 — Repo moved to GitHub; revocation/lockdown epic done; nav architecture researched

## Where things stand

`epic-revocation-lifecycle` is **done** — the SECURITY.md emergency-control
contract (#1–#5) is fully implemented: revoke current/all sessions,
endpoint/principal/device revocation, grant revocation + expiry + Subscribe
grant checks, durable lockdown with bootstrap-channel-only exit. 3 features,
13 stories, 16 blockers fixed across 4 independent review passes.

Git: ~60 commits ahead of origin. **Repo home is now GitHub**:
`origin` = `git@github.com:alluvial-lab/patchbay.git` (private), Forgejo
kept as `forgejo` secondary (mirror pushes need `git push forgejo main`).

Runtime: stack still up (core 50051/50052, adapter, web-server TLS on
192.168.50.110:3000) but running the **pre-epic build** — restart to get
revocation controls, lockdown, and the new cockpit shell. State/env:
`~/.config/patchbay/env`, `~/.local/state/patchbay/`.

## What happened (since the 2026-07-27 note)

- **Repo move**: Forgejo → GitHub org (alluvial-lab), private, GitHub
  primary / Forgejo secondary. Only out/ (gitignored) and the release doc
  referenced the old host.
- **Docs audit** (pre-v0.1+ hygiene): 14 findings. 13 prose corrections
  landed (fake CI claims, phantom shared operator domain, snapshot
  checkpointing, core-generation). 4 implementation gaps filed as backlog:
  session-record-fields (since absorbed), core-generation persistence,
  snapshot checkpoint writer, revocation surface (became the epic).
- **epic-revocation-lifecycle** end to end: scoped from 2 merged backlog
  items (implement-to-match-doc decision), 3-feature decomposition,
  delegated design + implementation waves, 3 feature reviews + 1 epic
  aggregate review. Headline blockers caught: last-grant-revocation
  bricking (now refused), stale-issuer race, adapter token race, adapter
  projection replay-corruption race, optimistic cockpit lockdown claim.
- **Cockpit navigation architecture** — researched and locked:
  `.research/analysis/briefs/cockpit-navigation-architecture.md`
  (Material 3 / Apple HIG canonical + VS Code/Slack/Linear practice + SNC
  prior-art gap analysis). Icon-only left rail (VS Code activity-bar model),
  destinations punch out contextual panels, left-accent highlighter,
  bottom tabs + drill-in + back on mobile, inspector-as-sheet.
- **Lockdown mockup saga**: 4 options → hybrid → ~8 revision rounds to
  sign-off (`.mockups/screens/epic-revocation-lifecycle-lockdown/`).
  Security single-column, inline banner over read-only cockpit, two-step
  LOCKDOWN ritual, production session-detail chat parity. Signed off as
  "good MVP" — it is the UI reference for the shipped lockdown feature AND
  the future cockpit shell.

## Lessons (learned the hard way, again)

- **Verify claims, don't trust reports**: two workers self-closed features
  skipping mandatory review (bounced; reviews then found 7 real blockers).
  "Clippy passed" was false twice. Orchestrator now re-runs every suite at
  every wave boundary.
- **Bundle verification**: a UI fix once shipped as CSS-only because a
  rejected multi-edit batch silently dropped the JS half. Now: after any
  web-cockpit change, grep the change into `dist/assets/cockpit.js`.
- **Grid + hidden children = auto-placement trap** (TWICE in the mockup:
  hidden banner row, hidden lock-reason row). Explicit row placement or
  flex, never implicit templates with conditional children.
- **Single generator of record**: build.rs regenerating contracts raced the
  committed buf baseline; build.rs no longer writes bindings; the allows it
  injected moved to buf.gen.yaml.
- **Mock review infrastructure**: serve with `Cache-Control: no-store` —
  lost an hour to a stale cached mock. Also: index-iframes make everything
  look 1/4 height; review options at full size.
- **Subagent edits can silently revert your earlier fixes** when they
  reorganize a file (the chat-pane port reverted the highlighter selector
  fix). Re-verify after every delegation round.

## Board

- Done: epic-observability-dogfooding, epic-revocation-lifecycle,
  6 dogfooding-fix stories, drift/CI story.
- Active: `epic-public-product-contract` (implementing, 5 features
  drafting); `epic-agent-operations-resource-plane` (drafting — owned by
  the cockpit-driven session, coordinate before touching).
- Held: cockpit UX batch (session-list rows, settings section,
  delivery-line layout) — the nav-shell research + lockdown mock now
  INFORM it; when picked up, use the locked nav architecture.
- Backlog: ~24 items (authority cluster, sessions/protocol, hygiene,
  core-generation persistence, snapshot checkpoint writer, parked ideas).

## Next candidates

Restart the stack for the new build; dogfood revocation/lockdown; the held
UX batch (mockups exist now for the shell); v0.2.0 release cut; or
`epic-public-product-contract` feature design.
