---
id: research-handoff-spawn-restart-continuation-orchestration
kind: story
stage: review
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

- The claim-owned replay projection now retains delivery plus exact-claim `quiescing_prior → prior_terminated → launch_attempted → external_identity_known → handshake_reconciling → staged → success_evidence_reported` evidence. Continuation staging and promotion require that durable LSN order; post-poison reconciliation additionally requires later handshake/success evidence.
- Generated `ContinuationContextStatus` is the sole Rust/TypeScript vocabulary. Exact continuation `SessionReport` evidence must provide `resumed`, `new_context`, or `unknown`; staging copies it exactly, replay/promotion preserve it, and web/CLI derive it from promotion history rather than defaulting to `unknown`. Fresh and ordinary reports require the wire sentinel.
- Adapter disconnect still produces stale/unknown prior state. Exact prior N must remain current but unavailable (`offline`/`stale`/`failed`) with `unknown` activity; N+1 remains only reserved/staged until the atomic promotion writer commits authority/session/claim/command together.
- The web session-list spawn control is inert with an explicit reason while lockdown is active or a lockdown decision is pending. No new screen or Pi-specific supervisor mechanics were added.
- Foundation docs now state the structural phase chain, generated context carriage, no-default presentation rule, and adapter/core ownership split.

## Verification evidence

- Rust: `cargo build --workspace --all-targets`, `cargo test --workspace`, and `cargo clippy --workspace --all-targets -- -D warnings` pass, including 35 runtime-evidence/promotion, 82 server-unit, and 12 spawn-completion tests.
- Contracts: `check:drift`, `check:vectors`, `check:models`, and TypeScript build pass (55 vectors, 17 promoted vectors, 22 implementation checks, 38 mutation witnesses, 54 model-promotion blocks).
- TypeScript: operator-domain 26/26, Pi adapter 38/38 including real-core e2e, web cockpit 132/132 including browser build, and CLI 48/48 plus real-core resource projection pass.
- Focused regressions reject missing quiesce, missing prior-runtime outcome, handshake-before-identity, stage-before-handshake, and phase-before-delivery; context carriage/rendering and pending/active lockdown spawn gating are covered.
- Manual source mutation probes were killed for bypassed stage-before-handshake enforcement, omitted quiesce, omitted prior-terminated evidence, hard-coded adapter context outcome, and lockdown-enabled session-list spawn. Every source mutation was restored and the focused clean tests passed afterward.
