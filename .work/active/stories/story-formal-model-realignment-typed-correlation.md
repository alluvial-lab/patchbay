---
id: story-formal-model-realignment-typed-correlation
kind: story
stage: done
tags: [verification, protocol, foundation]
parent: feature-formal-model-realignment
depends_on: [story-formal-model-realignment-elicitation]
created: 2026-07-08
updated: 2026-07-09
gate_origin: null
release_binding: null
---

# Story: TypedCorrelation extension (Unit TC)

Implements Unit TC from `feature-formal-model-realignment`. Extends the checked `TypedCorrelation` model to cover response Operation → Elicitation typed correlation. Depends on Unit EL (the ElicitationId space must be defined).

## Scope

Extend `specs/seed/reply_correlation.qnt` in place. Add `ELICITATION_ID_SPACE` and response-Operation correlation. A response Operation (`kind = approval-response | elicitation-response`) correlates by typed reference to a known prior `ElicitationId` in the same authority/session/responder context. The five id spaces (CommandId, MessageId, ReplyId, EventId, ElicitationId) remain disjoint.

**Checked property:** the existing `TypedCorrelation` invariant is extended to cover the response→Elicitation case. The `@promotion` block's `semantics` field is updated; the property id remains `TypedCorrelation` (coverage expansion, not a new property). No new `OperationResponseCorrelationTyped` id is introduced — `TypedCorrelation` is the single checked property for all typed-correlation cases.

## Bounds and invocation (N2)

Bounds carry forward from `reply_correlation.qnt` (command_ids: 2, message_ids: 2, reply_ids: 2) plus `ELICITATION_ID_SPACE` (2 ids) and response-Operation ids (2). Invocation: `quint verify reply_correlation.qnt --invariant typed_correlation --max-steps 12`.

## Acceptance Criteria

- [x] `quint parse` + `quint compile` exit 0.
- [x] Extended `TypedCorrelation` passes (now covering response→Elicitation).
- [x] Mutation test: a response Operation using a `ReplyId`/`EventId`/`CommandId` as `ElicitationId` is rejected (forgery prevented).
- [x] `reply_correlation.emitted.tla` regenerated.
- [x] VERIFICATION.md "TypedCorrelation extension" bullet narrowed (response→Elicitation now covered).
- [x] `@promotion` block updated (no `tier` field); `check-models.mjs` exits 0.

## Key files

- Edit: `specs/seed/reply_correlation.qnt` (+ regenerate `.emitted.tla`)
- Edit: `docs/VERIFICATION.md`
- Design reference: `.work/active/features/feature-formal-model-realignment.md` Unit TC

## Implementation notes
- Files changed: `specs/seed/reply_correlation.qnt`, `specs/seed/reply_correlation.emitted.tla`, `docs/VERIFICATION.md`.
- Tests/verification run: `quint parse specs/seed/reply_correlation.qnt`; `quint compile specs/seed/reply_correlation.qnt`; `quint verify specs/seed/reply_correlation.qnt --invariant typed_correlation --max-steps 12` (`[ok] No violation found`); mutation test on `/tmp/reply_correlation_mut.qnt` weakening `typedResponseReferenceOk` to accept any `elicitation` corrId (`[violation]`, counterexample recorded `responseCorrelatesTo.ro1 = "c1"`); regenerated TLA via `quint compile reply_correlation.qnt --target tlaplus`; `node contracts/scripts/check-models.mjs`; `node contracts/scripts/check-vectors.mjs`.
- Genuine-checking confirmation: response Operation recording uses an action-side helper (`typedResponseReferenceOk` / `responseOperationRecordable`), while `recordedResponseOpIndependentOk` checks raw state/id-space/context facts and does not call the helper. The mutation broke the helper, not the oracle, and the invariant failed.
- Discrepancies from design: represented authority-domain + target-session/generation as the bounded context atom already used by `reply_correlation.qnt`; responder actor is a separate map. This preserves the model's compact projection while covering same authority/session/responder correlation.
- Adjacent issues parked: none.
