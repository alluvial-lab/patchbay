# Session note — 2026-06-29 (semantic design pass)

A durable handoff note for the next session. Read this before continuing.

## Where we are

Patchbay foundation-hardening epic (`epic-foundation-hardening`, stage: implementing). This session closed out the entire retrospective backlog: the three reopened semantic-design features plus the provisional-semantics review story. All retrospective-flagged semantic concerns are now settled.

**Epic progress: 11/21 done** (was 8/21 at session start).

## What this session did

Worked through the retrospective backlog in dependency order, each via the `feature-design` lane (alternatives surfaced → operator decisions → design written → implementation into foundation docs → fresh-context cross-model review → close to done):

1. **`feature-design-terminal-commit-race`** — ratified "first durable terminal commit wins." Reviewed by `umans/umans-glm-5.2` (operator-requested flip; the umans concurrency slot was available). Approved with comments; nits applied.
2. **`feature-design-grant-shape`** — tightened the v0 grant record. Reviewed by `openai-codex/gpt-5.5` (initial Request changes: 1 blocker + 1 important, both contradictions between the new capability distinction and pre-existing failure-vocabulary wording; fixed in-stride; re-reviewed approve). 
3. **`feature-session-identity-adapter-contract`** — the largest; five decisions (session identity, session generation+tombstone, correlation id spaces, adapter registration lifecycle, capability manifest shape). Reviewed by `openai-codex/gpt-5.5`; approved with comments; nits applied.
4. **`story-review-provisional-semantics`** — reviewed the four weaker provisional candidates. None promoted to design; all four markers removed; two load-bearing notes added. Fast-lane review story (no sub-agent review needed — the work IS the review).

## Key decisions this session (so the next session doesn't re-litigate)

### Terminal commit race
First durable terminal commit wins. Late conflicting terminal candidates are audit/reconciliation events, not `CommandState` rewrites. No global priority ordering in v0. Command-kind-specific terminal-resolution policy reserved as a future seam for safety-critical commands. UX explains too-late cancellation ("completed before cancellation arrived") rather than rewriting state.

### Grant shape (four locked decisions)
1. **Delegation removed** from v0 grant record (`parent_grant_id` intentionally absent); reserved as prose future-direction; the "delegation cannot exceed parent" verification property moved to a precondition.
2. **Grant subject** = actor + optional endpoint/endpoint class. Device stays in identity model (audit/revocation) but is not a grant-matching field.
3. **Command kinds as authority**; no adapter-capability gate in the core. Adapter capability declarations are advisory UX-display-only; the adapter is authority on its own support, reported at delivery. `unsupported_command` is delivery-layer; unknown-to-Patchbay command kind is `validation_failed` at submission.
4. **Compound issuer tuple** — operator is grant subject; web-server endpoint is verified transport principal; core independently verifies both; exact wire evidence deferred to `feature-web-core-protocol-seam`.

### Session identity & adapter contract (five locked decisions)
- **A1**: session identity = adapter id + deployment scope + runtime session id + session generation; project/cwd/name are metadata.
- **B1**: adapter-reported session generation + core tombstone; late events are `stale_event` audit records; split retention (tombstone fact indefinite, detail bounded/compacted); strict-monotonic supersession.
- **C1**: four separate id spaces (command/message=client, reply=adapter/core with typed correlation, event=core/LSN); command id and idempotency key are separate fields.
- **D1**: adapter-as-principal with explicit registration lifecycle; trust-root mechanism adapter-specific (not mTLS-mandated).
- **E1a+E2a**: ratified 3-tier snapshot model; capability shapes per where the core branches (snapshot=3-tier, idempotency=enum, streaming/cancellation/replacement=boolean).
- **Naming**: "generation" retained (a `generation → incarnation` rename was considered and walked back — "generation" is deployed, understood, not wrong); scope-qualified (core/session/adapter) with assigner-foregrounded glossary entry.

### Provisional-semantics review (four candidates)
1. Session state axes (5×3) — **accept**.
2. Enrollment posture — **accept** (with coupling made explicit: enrollment channel must be distinct from routine web login — load-bearing for lockdown).
3. Five revocation actions — **accept-with-note**: lockdown exit = bootstrap channel, not routine web re-auth; restart doesn't clear it (durable posture); channel distinction is load-bearing.
4. Gap-free per-domain LSN — **accept-with-note**: event/cursor/revision identity is `(authority_domain_id, LSN)` tuple (federation hygiene); HLC deferred as premature.

## Two load-bearing insights worth remembering

- **Lockdown↔enrollment coupling**: lockdown only protects if its exit channel differs from routine login. The operator's "hostile actor stops the core" question found this; the "trust level = bootstrap trust level" framing resolved it. Restart is not an escape hatch because lockdown is durable and crash-recovery replays it.
- **LSN key shape as federation seam**: the domain demarcator lives in the durable key from day one, so federation is "add a layer," not "migrate the data." This is the operator's "sensible defaults with federation in mind" hygiene principle applied.

## Workflow notes for the next session

- **Subagent routing**: per `AGENTS.md`, subagents run on `openai-codex`, never `umans` (umans budget reserved for orchestration). This session used `gpt-5.5` for the grant-shape and session-identity reviews. The terminal-commit-race review used `umans/umans-glm-5.2` because the operator explicitly requested the flip (one umans concurrency slot) — that was an explicit exception, not the default.
- **Review nit convention** (added to `.agents/rules/substrate-discipline.md` this session): the dispatching/reviewing agent triages nits before advancing to done — cheap/local nits applied in-stride, nits not applied explicitly recorded as deferred/not-worth-changing, so they aren't silently swallowed.
- **Reviewer sandbox artifacts**: the GPT-5.5 reviewer subagents created zero-byte placeholder files in the repo root (`.bashrc`, `.env`, literal `*.pem`, etc.) each run. I removed them each time. If a future reviewer run leaves them, they're safe to delete — they're sandbox setup residue, not project content. (Worth a future fix to suppress at source, but not blocking.)
- **Cross-model review bar**: features got fresh-context sub-agent review (different model class than the implementor). Review found real issues both times it ran on features (grant-shape: blocker+important; session-identity: nits only). The review bar is earning its keep.
- **The `implement` skill's inline path is correct for these foundation-doc features** — they're no-coordination prose-like doc implementations, not code. The orchestrator would be overhead.

## Recommended next pickups

With the retrospective backlog closed, the queue is now "new ground" rather than "reopened decisions":

1. **`feature-extension-seams-non-foreclosure`** — now fully unblocked. Its ordering note said to wait until the reopened semantic design work + provisional-semantics review concluded; both are done. This sweep classifies committed v0 assertions against future directions now that the assertion set is settled. Route through prose-author (genuine prose: classification rules, inventories, mappings).
2. **`feature-research-v0-stack-tooling`** — still drafting, independent of all semantic work. Grounds TS web framework, Rust core primitives, browser operator-domain libs, reference control-plane projects. Light-path research (web-search-and-attest). Can run in parallel with anything.
3. **The remaining drafting design features**: `feature-idempotency-ambiguous-execution`, `feature-lease-scope-decision`, `feature-ux-v0-acceptance`, `feature-verification-contract-authority`. Each routes through `feature-design`.

`feature-web-core-protocol-seam` is in the active queue (drafting, no parent) — it consumes session-identity, grant-shape, and persistence (all now done), so it's unblocked too, but it's a larger design effort.

## Key committed decisions recap (don't re-litigate)

- V0 two-process topology: Rust core (authority, no HTTP) + TS web server (control surface, generated Protobuf/Connect to core).
- Protocol contract: Protobuf + Buf; Rust via prost, TS via Protobuf-ES. Connect-ES/prost server-side fit deferred to v0-stack-tooling research.
- Single authoritative core: one writer → one durable log per authority domain; no HA/replication in v0; crash recovery via log replay.
- All retrospective-flagged semantics now settled (see above).

## No untracked files of note

Working tree is clean. `.pi/remote-pi/config.json` remains intentionally uncommitted (local mesh config, gitignored).
