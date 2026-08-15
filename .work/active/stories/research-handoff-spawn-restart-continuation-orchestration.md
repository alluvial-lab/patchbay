---
id: research-handoff-spawn-restart-continuation-orchestration
kind: story
stage: implementing
tags: [adapter, protocol, security]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-completion-promotion-driver, deployment-authority-workspace-scoped-revocable-keys]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-15
---

# Restart as typed spawn continuation orchestration

## Redesign disposition

Rewritten to consume the safe shared contracts. Concrete Pi RPC process/session-file mechanics belong to the following Pi-adapter redesign, not this spawn-side checkpoint.

## Checkpoint

Expose intentional restart as a new `spawn` Operation with a new command/key and exact continuation payload. Apply the phase/effect contract, pending-replacement fence, compound authority, claim, staged successor, and completion driver across the generic orchestration. Do not add a `restart` OperationKind or hide replacement in `session-management`.

Continuation restores adapter-native logical context, not arbitrary process state. The adapter reports `resumed`, `new_context`, or `unknown`; only promotion makes N+1 current.

## Design

**Files**
- Server/core orchestration seams and generated payload delivery.
- Operator action construction in `web-cockpit` and `cli` using existing action surfaces.
- Rolling foundation docs when implemented.
- Pi-specific supervisor implementation explicitly deferred to `research-handoff-pi-adapter-capability` redesign.

Required generic phase order:
1. accept exact claim + compound provenance and activate N delivery fence;
2. deliver claim/target spec to the adapter;
3. record phase evidence for quiesce and old-runtime outcome;
4. adapter terminates/replaces/reconciles using its declared implementation;
5. exact successor report stages and reserves external identity;
6. successful Result/phase evidence enters completion readiness;
7. completion driver atomically promotes or claim remains active/poisoned.

Failure mapping follows the parent table: pre-offer no-effect may release atomically; delivered/launch ambiguity poisons; prior clean termination leaves N offline/current until promotion; stream loss yields stale/unknown and poison; no failure allocates or publishes N+1.

## Acceptance evidence

- [x] Fresh and continuation remain one `spawn` kind; continuation has exact prior and both authority Grants.
- [x] New N work rejects after claim acceptance and old work is explicitly quiesced/resolved.
- [x] Every orchestration phase emits the typed effect/connectivity evidence expected by the parent table.
- [x] N+1 remains staged until the atomic promotion driver commits descendant authority/current/completed.
- [x] `resumed/new_context/unknown` does not claim arbitrary process-state continuity.
- [x] Ambiguous failure poisons; no automatic relaunch or same-generation new claim occurs.
- [x] Existing web/CLI actions show canonical lifecycle/failure/retry risk without new protocol states.

## UI fallback

No new screen. Reuse session-list spawn and session-detail restart actions plus canonical Operation/failure/Grant presentation.

## Ordering constraint

Depends on verified atomic completion/promotion and adapter-local credential-reference boundary. Reconnect/cursor convergence follows.

## Implementation evidence

- `core/src/session/spawn_orchestration.rs` is the generic phase/outcome and continuation-prior precondition authority. Claim replay retains ordered exact-runtime identity/handshake/stage/success checkpoints; promotion requires them, and poison requires later handshake/success before explicit reconciliation.
- Adapter disconnect now produces stale/unknown prior state. Continuation staging and both hot/replay promotion reject unless exact prior N remains current but unavailable (`offline`/`stale`/`failed`) with `unknown` activity. N+1 remains only reserved/staged until the existing atomic promotion writer commits authority/session/claim/command together.
- Operator subscriptions expose redacted spawn claim/promotion facts. Shared `operator-domain` construction, CLI `spawn`/`restart`, web session-list spawn/session-detail restart, and projection folding keep fresh/restart under `OperationKind.SPAWN`, reconstruct exact managed identity, and present context as `resumed`/`new_context`/`unknown` without process-restoration claims.
- Foundation docs now state the seven generic phases, prior-N availability fence, ordered readiness, post-poison reconciliation, context vocabulary, and adapter/core ownership split. No proto or Pi-supervisor mechanics changed.

## Verification evidence

- Rust: `cargo fmt --all -- --check`, `cargo check --workspace --all-targets`, `cargo test --workspace --all-targets --no-fail-fast`, and `cargo clippy --workspace --all-targets -- -D warnings` pass.
- TypeScript: contracts build plus operator-domain, Pi adapter, web server, CLI, web cockpit, token-commune adapter, and walking-skeleton test suites pass. Added shared spawn, CLI fresh/restart, and web fresh/restart construction tests.
- Protocol: `cd contracts && buf build proto && buf generate` pass and produce no generated/proto diff. Standalone `buf lint proto` still reports the repository's pre-existing RPC request/response naming findings documented in `.work/session-notes/2026-07-15-three-surface-layers-done-4-of-6.md`; fixing them would require prohibited, unrelated proto edits.
- Required mutation tests pass: terminal command state cannot release/clear a claim fence; distinct commands cannot reclaim one active generation; web identity-before-submission survives generated incomplete/superseded targets; operator-domain independent honesty oracles kill representative composition mutants. New restart tests additionally kill prior-availability, known-activity, claim-release, and early-N+1-publication mutants.
- The requested `.work/bin/work-view check` cannot run because the installed `work-view` has no `check` subcommand (`--help` exposes query/board/serve only); this story is the only `.work` file changed.
