---
id: story-formal-model-realignment-subscription
kind: story
stage: implementing
tags: [verification, protocol, foundation]
parent: feature-formal-model-realignment
depends_on: [story-formal-model-realignment-spawn]
created: 2026-07-08
updated: 2026-07-08
gate_origin: null
release_binding: null
---

# Story: Subscription authority (Unit SUB — promote into authority.qnt)

Implements Unit SUB from `feature-formal-model-realignment`. Promotes 3 subscription-authority properties into `specs/seed/authority.qnt` (same file as Unit SA; must follow SA sequentially per Q4 Option α). Subscription is the grant-checked-without-lifecycle second authority mechanism.

## Scope

Extend `specs/seed/authority.qnt` with subscription-specific state. **Subscription authorization queries real `Grant` records (B5 fix), not a boolean side-channel:** a subscription's authorization is a `Grant` with `GrantCommandKinds` containing `subscribe` and `GrantScopeById` = the stream/filter scope.

State variables: `subscriptionId`, `subscriptionGrantId` (→ authorizing Grant id), `subscriptionCursor`, `subscriptionStream`, `subscriptionFilter`, `auditRecords`, `operationRecordsCreated` (counter; must stay 0 — no OperationState), `eventLsn`, `eventStream`, `eventFilter`, `replayedEvents`.

The `operationRecordsCreated` counter makes `SubscriptionAudited` genuine: a subscription allow/deny creates an audit record but does NOT increment the Operation-record counter.

Actions (permissive): `attemptEstablish` (succeeds only if live `Grant` for scope with actor as subject + `subscribe` kind; always creates audit; never creates Operation record), `emitEvent`, `replayByCursor`.

## Checked properties (3, `status: promoted`; tier derived by Unit TR)

- `SubscriptionGrantChecked` (invariant) — genuine check: queries `Grant`/`GrantScopeById`/`GrantCommandKinds`/`GrantStatus` tuples, not a boolean side-channel.
- `SubscriptionAudited` (invariant) — every establish attempt creates an audit record; `operationRecordsCreated` stays 0.
- `SubscriptionCursorReplayAuthorized` (invariant) — replay returns only events with `LSN > cursor` within authorized filter; `replayedEvents` contains no out-of-cursor/out-of-filter events.

## Bounds and invocation (N2)

- Bounds: 3 actors × 2 streams × 2 filters × 2 grant statuses × 5 cursors. `--max-steps 12`.
- Invariants: `quint verify authority.qnt --invariant subscription_grant_checked --max-steps 12` (and the other 2).

## State-space bloat mitigation (N3)

Adding subscription cursor/replay/event state alongside spawn state and existing grant infrastructure may push `authority.qnt` past Apalache's tractable state space. Set `--max-steps` bounds per spec; reduce atom-set bounds (actors/sessions/scopes) if Apalache doesn't complete. **Fallback:** if verification remains intractable, split subscription authority into its own `subscription_authority.qnt` (reverting to standalone for SUB only, keeping SA in `authority.qnt`). This split trigger is an implementation-time decision, not a design re-open — properties and genuine-checking proofs are unchanged by placement.

## Acceptance Criteria

- [ ] `quint parse` + `quint compile` exit 0 (extended `authority.qnt` compiles after both SA and SUB additions).
- [ ] All 3 new properties pass.
- [ ] Mutation test `SubscriptionGrantChecked`: allowing establish without a live grant fails the property.
- [ ] Mutation test `SubscriptionAudited`: an establish that creates an Operation record (`operationRecordsCreated > 0`) fails the property (B5).
- [ ] Mutation test `SubscriptionCursorReplayAuthorized`: replay returning an out-of-cursor or out-of-filter event fails the property.
- [ ] `@promotion` blocks present (no `tier` field); `check-models.mjs` exits 0; VERIFICATION.md updated.

## Key files

- Edit: `specs/seed/authority.qnt` (same file as Unit SA)
- Edit: `docs/VERIFICATION.md`, `contracts/scripts/check-vectors.mjs` (arrays)
- Design reference: `.work/active/features/feature-formal-model-realignment.md` Unit SUB
