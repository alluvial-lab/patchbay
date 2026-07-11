---
id: idea-csrf-trace-fidelity
kind: backlog
created: 2026-07-01
updated: 2026-07-11
tags: [verification, security, foundation]
research_refs: []
---

# Backlog: acceptance invariants should verify against attempted-evidence state, not recorded trace

Filed from the deep re-review of `feature-formal-model-seed`. The general pattern for safety-claiming models that verify server-side acceptance: the invariant should check against the **attempted/request evidence** (the raw submitted values, captured as distinct state), not the action's recorded trace variables. An action that lies about what was submitted (sets the recorded field to the bound value regardless of input) fools the invariant. This is the "drop the check and lie about recorded evidence" defect the genuine-checking discipline guards against.

## Status (updated 2026-07-11)

The CSRF models (`csrf_browser.qnt`) were fixed in `story-fix-csrf-trace-and-ssot-drift` to inspect `attemptedSession`/`attemptedProof` (raw evidence) rather than `lastSession`/`lastProof` (recorded trace). The 4 CSRF promoted properties (`CsrfRejectsMissingProof`, `CsrfRejectsUnauthenticated`, `RevokedSessionCannotCommand`, `browser_local_state_not_authority`) and the 3 lifecycle properties (`TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted`, `GenerationMonotonic`) do NOT share the defect.

The defect IS present in the authority/subscription models and was the basis for Unit 7 demotions (`FleetAuthorityForSpawn`, `ElicitationResponderAuthority`, `SpawnRevocationDoesNotCascade`, `SubscriptionGrantChecked` — all demoted in `epic-public-product-contract-verification-claim-correction`).

## Open question routed to the v1 formal gate

The round-3 deep review of `epic-public-product-contract-verification-claim-correction` claimed the defect is more pervasive — affecting 9 surviving promoted properties across elicitation (6), subscription (2), and reply-correlation (`TypedCorrelation`). The host partially disputed this: an independent mutation test of `ElicitationPendingFinality` (the reviewer claimed it passed) was CAUGHT by the property (the temporal `__saved_` baseline mechanism protects it). `SubscriptionAudited` has a narrower confirmed defect (detects counter mismatch, not a coordinated lie).

The systematic question — whether each surviving promoted property's formula is an independent oracle against the model's own actions, or whether the model families need re-architecting to introduce immutable attempted-evidence state — is **model-architecture work owned by `epic-public-product-contract-executable-release-assurance`**, which already says it must "run the real formal checker where a property is formally gated" and that "metadata is never sufficient behavioral evidence." It is NOT absorbed as further demotions in the verification-claim-correction feature, because (a) the reviewer's claim was partially wrong, (b) each property needs individual mutation verification, and (c) re-architecting 3 model families is v1-gate-scale work, not a claim-correction stride.

When the v1 gate reaches design, it should: mutation-test each surviving promoted property individually; demote, narrow, or re-architect per the result; and apply the attempted-evidence discipline uniformly across all server-side-acceptance models.
