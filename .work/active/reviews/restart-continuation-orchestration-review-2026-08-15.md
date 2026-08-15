---
id: restart-continuation-orchestration-review-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-restart-continuation-orchestration
created: 2026-08-15
updated: 2026-08-15
---

# Thorough review — Unit 9 generic restart-continuation orchestration

## Verdict

**MATERIAL** — return `research-handoff-spawn-restart-continuation-orchestration` to `implementing`.

The landed claim/fence, ambiguity poison, staged-successor, sole promotion-owner, no-auto-relaunch, and atomic-publication guarantees remain intact under the requested mutations. The composition is not complete, however: continuation readiness can skip the typed quiesce/old-runtime phases and can order staging before handshake, the adapter-reported context outcome has no generated evidence carriage, and the newly enabled web spawn action remains active during lockdown.

Review mode: independent fresh-context story review, effective weight `thorough`, implementation range `85dad25..5f145cd`.

## Findings

### MATERIAL — the seven-phase continuation spine is neither complete nor structurally ordered

**Locations:** `core/src/session/spawn_orchestration.rs:306-311,319-352`; `server/src/adapter_service.rs:1674-1685`; `core/tests/runtime_evidence_promotion.rs:2320-2336`; `docs/PROTOCOL.md:416-424`

`SpawnCompletionPhaseRecord` discards `offered`, `quiescing_prior`, `prior_terminated`, and `launch_attempted` evidence, so completion readiness cannot require the story's typed quiesce/old-runtime phase. Successor staging checks only that the prior session projection is currently unavailable with unknown activity; it does not require any exact-claim `quiescing_prior` or `prior_terminated` evidence. The committed continuation fixture consequently promotes from identity → handshake → stage → result → success without either typed prior phase.

The retained suffix is also ordered incorrectly. `is_ready` requires `handshake_lsn > identity_lsn` and `success_lsn > handshake_lsn` plus `success_lsn > staged_lsn`, but never requires `staged_lsn > handshake_lsn`. A reviewer probe moved the staged event to LSN 9 and handshake evidence to LSN 10; `next_spawn_promotion` still returned a ready promotion. The temporary fail-closed assertion failed with exit 101: `stage before handshake must not become ready`.

This contradicts the binding phase order and allows later-phase evidence to precede or replace earlier continuation evidence. Session unavailability alone is not the required typed old-runtime outcome.

**Concrete fix:** extend the claim-owned readiness projection to consume the complete exact-claim continuation sequence after claim/fence acceptance and delivery. Require typed quiesce plus old-runtime outcome before launch/identity, and enforce `identity < handshake < stage < success` (including post-poison suffix rules) by LSN. Make staging and promotion reject or remain unready when any required phase is absent or out of order. Add focused continuation tests for no quiesce evidence, no old-runtime outcome, handshake before identity, stage before handshake, and later evidence before delivery; retain the existing acceptance/delivery fence barrier.

### MATERIAL — logical-context status is duplicated locally but cannot be reported or folded

**Locations:** `contracts/proto/patchbay/adapter_control.proto:85-98`; `contracts/proto/patchbay/sessions.proto:115-122`; `core/src/session/spawn_orchestration.rs:19-47`; `operator-domain/src/spawn.ts:28-40`; `cli/src/commands/spawn.ts:96-100`; `web-cockpit/src/ui/session-detail.ts:428-436`

The Rust `ContinuationContextStatus` and TypeScript `ContinuationContextStatus` independently enumerate the same three values, but neither generated spawn-execution nor staged-successor evidence carries that status. No production core path consumes the Rust type. Both operator surfaces therefore hard-code `unknown`; an adapter can never report `resumed` or `new_context`, the core cannot validate or retain the result, and replay cannot reconstruct it.

This leaves the checkpoint's adapter-reporting promise unimplemented and violates the generated-contract/single-source-of-truth discipline. Deferring Pi process mechanics does not justify deferring the adapter-neutral evidence field that the downstream Pi supervisor must consume.

**Concrete fix:** add one generated adapter-neutral context-status registry and carry it on the appropriate exact-claim execution/staged evidence. Validate the closed vocabulary at ingress/replay, preserve it through promotion/operator projection, derive Rust and TypeScript types from the generated contract, and render the folded value. The downstream Pi supervisor should choose the value; Unit 9 should own its generic carriage and semantics.

### MATERIAL — the new session-list spawn action remains enabled during lockdown

**Locations:** `web-cockpit/src/ui/shell.ts:137-139,561-574`; `docs/UX.md:106`

The shell passes `actions.spawn` to the sidebar unconditionally, and the sidebar enables the button whenever it can infer exactly one adapter. Unlike restart and session-detail actions, it never consults `model.lockdown.active` or pending lockdown submission. The active-lockdown cockpit therefore exposes a state-changing spawn control even though the signed-off surface is required to keep all controls stale/read-only. Core authorization still rejects the command, so this is not an authority bypass, but it is a material wiring and presentation-contract failure in the UI surface added by this story.

**Concrete fix:** thread the canonical lockdown/submitting posture into `renderSidebar`, disable the spawn action with the same explicit lockdown reason used by other controls, and add a shell-level test proving the callback cannot fire while lockdown is pending or active.

## Checklist disposition

- **Composition / landed-unit guarantees:** partial pass. Fence activation, poison retention, staged-until-promotion, external identity reservation, and the sole completion driver remain intact; complete phase composition fails as finding 1.
- **Failure-table fidelity:** pass for pre-offer release, delivered/launch ambiguity poison, clean prior remaining current/unavailable, stream-loss stale/unknown plus poison, no same-generation reclaim, and no automatic relaunch.
- **Vocabulary:** one `spawn` OperationKind and no new protocol lifecycle state pass; generated context-status carriage fails as finding 2.
- **UI wiring:** web/CLI compile and use `OperationKind.SPAWN` with exact continuation payloads; lockdown wiring fails as finding 3.
- **Replay/publication:** pass. No staged event publishes N+1, and promotion remains the only authority/session/claim/command aggregate publication owner.

## Mutation matrix

Every source mutant was applied alone on the main tree, run with one focused test, reverted with `git restore`, and followed by a clean `git status --short`. The additional order probe was also removed before full verification.

| Mutant / probe | Focused oracle | Result |
|---|---|---|
| Skip the accepted continuation fence by forcing `delivery_fence_matching` open | `continuation_acceptance_activates_exact_fence_and_explicit_effects_atomically` | **KILLED**, exit 101; expected `ReplacementPending`, observed `Open`. The production acceptance/delivery barrier also passed on the clean tree. |
| Omit handshake evidence by replacing it with a second identity checkpoint | `promotion_producer_keeps_earliest_exact_success_result_retry_on_both_sides_of_staging` | **KILLED**, exit 101; the complete-evidence promotion assertion received no candidate. |
| Publish N+1 from the staged-successor fold before the driver | `exact_continuation_report_stages_n_plus_one_without_publishing_it` | **KILLED**, exit 101; the successor appeared in the live-session index. |
| Allow automatic relaunch by disabling durable delivery suppression | `abnormal_stream_loss_poisons_managed_spawn_and_prevents_redelivery` | **KILLED**, exit 101; the poisoned exact command was offered again. |
| Put staged-successor evidence before handshake evidence | temporary fail-closed probe over `next_spawn_promotion` | **SURVIVED implementation**; promotion was returned and the expected-`None` assertion exited 101, producing finding 1. |
| Omit typed quiesce/prior-terminated evidence from a continuation | existing `continuation_requires_both_live_grants_and_tombstones_n_on_n_plus_one_promotion` fixture | **SURVIVED implementation**; the clean focused test promoted successfully, producing finding 1. |

## Full clean-tree suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — including 34 runtime-evidence/promotion, 39 spawn-claim, 82 server-unit, 12 spawn-completion, and all doctests; warnings-denied clippy passed.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 55 vectors, 17 promoted vectors, 22 implementation checks, 38 registered mutation witnesses, and 54 model-promotion blocks.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 26/26.
4. `cd pi-adapter && npm test`: **PASS** — 38/38, including the real core/adapter generation-bump, reconnect, and core-restart e2e.
5. `cd web-cockpit && npm test`: **PASS** — 129/129, including browser build.
6. `cd cli && npm test`: **PASS** — 48/48 plus the real-core resource projection smoke test.

The tracked tree was clean before mutation work, after every `git restore`, before the full suite, and before this review file was written. `git diff --check` passed. No temporary worktree was created; `/` retained 60G free.

## Recommendation

**Return to implementing.** Make the complete seven-phase sequence structural, provide generated context-status evidence carriage, disable fresh spawn during lockdown, add the focused regressions above, rerun the clean-tree suite, and submit a new thorough review pass.
