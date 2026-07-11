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

The defect was confirmed pervasive across the formal-model substrate via a 4-round review convergence loop on `epic-public-product-contract-verification-claim-correction`:

- 4 authority properties demoted in Unit 7 (`FleetAuthorityForSpawn`, `ElicitationResponderAuthority`, `SpawnRevocationDoesNotCascade`, `SubscriptionGrantChecked`).
- 9 more properties demoted in Unit 8 after the host retracted its round-3 dispute: 6 Elicitation (`ElicitationPendingFinality`, `ElicitationFirstAnswerWins`, `ElicitationCorrelationTyped`, `ElicitationInvalidResponseRejected`, `ElicitationStaleTargetInert`, `ElicitationWithdrawalFinality`), 2 subscription (`SubscriptionAudited`, `SubscriptionCursorReplayAuthorized`), and `TypedCorrelation`. The host's round-3 test mutated the wrong branch; round 4 found the right mutation and the host reproduced it. All 9 confirmed mutation-fragile.
- The 8 surviving promoted properties were independently mutation-tested in round 5 and confirmed to catch their claim-breaking mutations — they are genuinely sound (CSRF via attempted evidence; lifecycle/transition via structural invariants not recorded by the accepting action).

The systematic question — re-architecting the elicitation/subscription/reply-correlation model families to introduce immutable attempted-evidence state — is **model-architecture work owned by `epic-public-product-contract-executable-release-assurance`**, which already says it must "run the real formal checker where a property is formally gated" and that "metadata is never sufficient behavioral evidence." When the v1 gate reaches design, it must build the genuine formulas for the 24 demoted properties and apply the attempted-evidence discipline uniformly across all server-side-acceptance models.
