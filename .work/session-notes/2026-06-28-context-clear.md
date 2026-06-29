# Session note — 2026-06-28 (context clear)

A durable handoff note for the next session, since context is being cleared. Read this before continuing.

## Where we are

Patchbay foundation-hardening epic (`epic-foundation-hardening`, stage: implementing). The prose lane produced three done features (command-state-ssot, security-threat-model, persistence-snapshot-model) plus research (contract-tooling, web-control-security, both done). The operator then asked for a retrospective, which reshaped the queue significantly.

## What happened this session (the important reorder)

A retrospective on the done prose features found that several **semantic/architectural decisions were made through the collapsed prose-author lane without a design pass** — the prose black-box misroute test should have caught them. Three were reopened as design features; four weaker candidates were filed as a review story; four more queued prose features were retagged from `[prose]` to design.

This was the key correction: **design decisions had been slipping through the prose lane, which skips the design gate, pre-mortem, and alternatives evaluation.** The queue is now reshaped so design goes through `feature-design`, prose stays prose, and committed assertions that were pre-decided are explicitly marked "under design review" in the docs.

## Current queue state (active)

Done (7): `feature-command-state-ssot`, `feature-persistence-snapshot-model`, `feature-security-threat-model`, `feature-v0-walking-skeleton`, `feature-research-contract-tooling`, `feature-research-web-control-security`, `story-bootstrap-substrates`.

Reopened as design features (drafting, route through `feature-design`):
- `feature-design-terminal-commit-race` — the "first durable terminal commit wins" race rule. Smallest, most self-contained.
- `feature-design-grant-shape` — grant field list + parent-grant delegation seam. Cross-cuts security and the web-core seam.
- `feature-session-identity-adapter-contract` — **retagged from prose to design**. Owns session generation semantics + adapter capability tiers. Also carries the three-tier adapter snapshot model reopened from persistence-snapshot-model.

Retagged from prose to design (drafting):
- `feature-idempotency-ambiguous-execution` — `maybe_executed` state, idempotency-key semantics.
- `feature-lease-scope-decision` — leases in/out of v0, fencing design if in.
- `feature-ux-v0-acceptance` — screen inventory, navigation, timeline behavior. Can invoke ux-ui-design skills (`screens`, `flows`).
- `feature-verification-contract-authority` — artifact authority order, generation targets.

Genuine prose (drafting, correctly in prose lane):
- `feature-extension-seams-non-foreclosure` — **DO NOT PICK UP FIRST.** See ordering note below.
- `feature-observability-operator-admin`
- `feature-pi-parity-checklist` — downstream of `feature-session-identity-adapter-contract`.

Review story (drafting):
- `story-review-provisional-semantics` — four weaker semantic candidates (session axis 5×3 decomposition; enrollment posture; five revocation actions; LSN gap-free model). Each gets a decision: accept / accept-with-note / promote-to-design.

Backlog:
- `feature-web-core-protocol-seam` — design-bearing; parked until foundation prose + the reopened design features settle (it consumes session-identity, grant-shape, persistence).
- `feature-research-v0-stack-tooling` — **active, in `.work/active/features/`** (filing discrepancy: it's in active not backlog). Grounds TS web-server stack, Rust core primitives, browser operator-domain primitives, reference control-plane projects. Independent of the semantic design work — can run in parallel.
- `idea-multi-human-coordination`, `idea-desktop-app-surface` — parked extensibility ideas, treated as inputs to `feature-extension-seams-non-foreclosure`.
- `research-handoff-*` (5 items) — implementation follow-ons from the two research engagements; superseded one (`web-control-security-1`) was dropped.

## Ordering decision the operator and I reached

**`feature-extension-seams-non-foreclosure` is NOT the next pickup.** Its sweep classifies committed v0 assertions against future directions, but the committed-assertion set is currently shifting (three reopened design features + one review story will change what the sweep classifies against). Running it now = classifying a moving target + likely re-sweep. The feature carries an explicit "Ordering note" in its body saying run it **after** the reopened semantic design work + provisional-semantics review conclude. Nothing in the active queue hard-blocks on it meanwhile (the "coordinate with extension-seams" blocks on other features are satisfiable by local per-feature classification).

## Recommended next pickups

1. **A reopened design feature** — `feature-design-terminal-commit-race` (smallest) or `feature-design-grant-shape` (cross-cutting). Route through `feature-design`, which gives the HITL (alternatives, tradeoffs, operator taste) that was missing from the prose lane.
2. **`feature-research-v0-stack-tooling`** can run in parallel — it's independent of the semantic design work and grounds implementation picks (TS web framework, Rust log/state-machine primitives, browser operator-domain libraries, reference control-plane projects). Route through the research-orchestrator.

## Key committed decisions (so the next session doesn't re-litigate)

- **V0 two-process topology** (`docs/ARCHITECTURE.md` "V0 process topology"): Rust coordination core (authority, no HTTP) + TS web server (control surface, terminates HTTP, speaks generated Protobuf/Connect contract to core). Server-side operator-domain reuse and split deployment are reserved seams. This was a real discussion with the operator, legitimately settled.
- **Protocol contract source** (research-grounded): Protobuf + Buf, Rust via prost, TS via Protobuf-ES. Committed direction, not yet implemented. Deepened questions (Connect-ES server-side fit, prost server-side) deferred to the v0-stack-tooling research.
- **Single authoritative core** = one writer to one durable log per authority domain, single-writer, no HA/replication in v0. Crash recovery via log replay. The web server is a control surface, not a core.
- **Lane routing discipline** recorded on `epic-foundation-hardening.md`: apply the prose black-box test honestly; when in doubt prefer design. Several retags already made (see above).

## Marked provisional / under-design-review in the docs

`docs/PROTOCOL.md`:
- "Cancellation, expiration, supersession, and race semantics" → under design review (`feature-design-terminal-commit-race`).
- "Authority grants" → under design review (`feature-design-grant-shape`).
- "Adapter snapshot capability tiers" → under design review (`feature-session-identity-adapter-contract`).
- "Revisions and cursors" → provisional (`story-review-provisional-semantics` #4).
- "Session state axes" → provisional (`story-review-provisional-semantics` #1).

`docs/SECURITY.md`:
- "Enrollment and authentication" → provisional (#2).
- "Revocation model" → provisional (#3).

These notes stay until the corresponding design/review item concludes.

## Workflow notes for the next session

- The concurrent-umans-sessions constraint from this session may be lifted — check with the operator before spawning subagents. Codex was available for fresh-context review (`openai-codex/gpt-5.5` worked well for the persistence review; different model class from the inline author, satisfying the cross-model review principle).
- Research engagements: the prior two were run **light path** (inline, no specialist fan-out) successfully. The v0-stack-tooling research could go light path too — it's web-search-and-attest territory.
- Review bar: features need a fresh-context sub-agent review (cross-model when possible), not inline self-review. We learned this the hard way with persistence (advanced prematurely, then reverted and ran proper Codex review which found a blocker + 2 important).
- The two research briefs and their attestations live in `.research/`. The work substrate is `.work/`. Don't put operational state in `.research/` or research claims only in `.work/` prose.

## Untracked file

`.pi/remote-pi/config.json` — local Pi mesh config, intentionally not committed.
