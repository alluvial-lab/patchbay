---
id: story-verification-correction-retained-semantics
kind: story
stage: done
tags: [verification]
parent: epic-public-product-contract-verification-claim-correction
depends_on: [story-verification-correction-prose]
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Narrow retained promoted property semantics and fix stale model header comments

## Scope

Seven retained promoted properties have formulas that are genuine (mutation-survivable, independent oracle) but whose `@promotion` semantics text overclaims what the formula establishes. Narrow the semantics text to match the formula. Also fix stale model header comments that no longer reflect the model's promoted property set.

## Unit

`Unit 6` from `epic-public-product-contract-verification-claim-correction` design.

## Files

- `specs/seed/command_lifecycle.qnt` — `NoAcceptedToCompleted` semantics + header comment
- `specs/seed/authority.qnt` — `FleetAuthorityForSpawn`, `SpawnRevocationDoesNotCascade`, and `ElicitationResponderAuthority` semantics
- `specs/seed/subscription_authority.qnt` — `SubscriptionGrantChecked` semantics
- `specs/seed/csrf_browser.qnt` — `browser_local_state_not_authority` semantics
- `specs/seed/elicitation_lifecycle.qnt` — `ElicitationStaleTargetInert` semantics
- `docs/VERIFICATION.md` — `ElicitationResponderAuthority` checked-model description
- `specs/seed/patchbay-relational.als` — header comment
- `specs/seed/snapshot_recovery.qnt` — header comment

## Implementation

### Narrow retained promoted property semantics

For each of the seven properties, update only the `semantics:` field in the `@promotion` block. Do not change `status` (stays promoted), `invocation`, or the `val`/`temporal` formula.

- `NoAcceptedToCompleted` (`command_lifecycle.qnt`): current semantics says "must pass through `delivered`" but the formula permits either `delivered` OR `running` immediately before completion. Narrow to: "a command cannot transition directly from `accepted` to `completed`; it must pass through `delivered` or `running`".
- `FleetAuthorityForSpawn` (`authority.qnt`): current semantics claims "authenticated actor" but the model has no authentication evidence — it proves grant-subject matching for the modeled actor. Narrow to: "spawn acceptance requires a live fleet-scope spawn Grant whose subject matches the submitting actor; per-session grants alone cannot authorize spawning a not-yet-existing session".
- `SubscriptionGrantChecked` (`subscription_authority.qnt`): same issue — claims "authenticated actor" but no authentication evidence. Narrow to: "subscription establishment succeeds only with a live subscribe-kind Grant record whose subject matches the submitting actor and stream/filter scope".
- `ElicitationResponderAuthority` (`authority.qnt`): current semantics claims endpoint authentication, but the model only checks that the modeled submitting endpoint maps to the expected responder actor and that the claimed actor matches. Narrow to: "response Operations are accepted only when the modeled submitting endpoint maps to the expected responder actor and the claimed actor matches that responder". Apply the same narrowing to `docs/VERIFICATION.md:127`.
- `browser_local_state_not_authority` (`csrf_browser.qnt`): current semantics claims protection of "grant checks" but the model has no grant state — it checks operator-session status and CSRF evidence. Narrow to: "browser-local UI claims cannot grant authority or override server-side session/CSRF checks".
- `ElicitationStaleTargetInert` (`elicitation_lifecycle.qnt`): semantics says "do not mutate live Elicitation state" but the formula only excludes `answered` and answer data; it has no response-attempt discriminator or next-state equality, so a stale response mutation to another state could pass. Narrow to: "responses to stale target/session generations do not cause the Elicitation to become answered or record answer data".
- `SpawnRevocationDoesNotCascade` (`authority.qnt`): when `gDescOs3Live != "yes"`, the descendant condition becomes `true`, so a cascade that deletes the descendant grant passes. Narrow to: "revoking the fleet spawn grant blocks future spawns and, when a descendant grant exists, does not revoke it".

### Fix stale model header comments

- `command_lifecycle.qnt:3-6`: currently says "models accepted-command durability, the first-durable-terminal-commit-wins race, and idempotency-boundary dedup" and "carries the 7 promoted model properties of feature-formal-model-seed plus the NoAcceptedToCompleted transition-adjacency property". After demotion, the model carries 3 promoted properties (`TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted`) and does not model durability or the terminal-race boundary. Update to reflect the 3 retained promoted properties and the actual scope.
- `patchbay-relational.als:31-32`: currently says "Promoted model: the one relational invariant that is genuinely checkable in a v0 static snapshot without becoming tautological." After demotion, no Alloy property is promoted. Update to reflect that the model contains draft/reserved properties only; Alloy remains the reserved relational tool for future delegation/authority-graph/lease problems.
- `snapshot_recovery.qnt:15,21-23`: currently says draft properties "typecheck cleanly" and "are exercised against the LSN/cursor/revision/domain/generation core". After removing the `val` definitions (Unit 4), the properties don't typecheck (they don't exist). Update to reflect that the property ids are reserved stated-normative obligations with no executable formula.

## Acceptance criteria

- [ ] Seven `@promotion` semantics fields narrowed to match their formulas.
- [ ] `docs/VERIFICATION.md:127` uses the same narrowed description for `ElicitationResponderAuthority`.
- [ ] `command_lifecycle.qnt` header comment updated to reflect 3 retained promoted properties and actual scope.
- [ ] `patchbay-relational.als` header comment and the adjacent NOTE at lines 50-52 updated: remove the claim that the check "verifies non-vacuity" and the claim that non-vacuity is "observed via the check finding a satisfying instance" — an UNSAT assertion check does not establish that.
- [ ] `snapshot_recovery.qnt` header comment updated to reflect reserved stated-normative obligations with no executable formula.
- [ ] `node contracts/scripts/check-models.mjs` exits 0 (semantics text changes don't affect tier derivation).
- [ ] `quint parse` exits 0 for all affected model files.


## Review (2026-07-11)

**Verdict**: Approve - fast-lane advance.

Story verified by implement (green `quint parse` + checkers); the cumulative diff across all 8 units was covered by the feature's 6-round deep-review convergence loop, which confirmed the final state (8 promoted / 39 stated-normative, 24 demotions, 24 formulas removed, 8 survivors mutation-confirmed sound).
## Implementation notes

- `NoAcceptedToCompleted`: clarified that completion may follow either `delivered` or `running`, while still excluding a direct `accepted` → `completed` transition.
- `FleetAuthorityForSpawn`: replaced the unsupported authentication claim with the modeled Grant-subject/submitting-actor match.
- `SubscriptionGrantChecked`: replaced the unsupported authentication claim with the modeled Grant-subject/submitting-actor and stream/filter-scope match.
- `ElicitationResponderAuthority`: narrowed the claim to the modeled endpoint-to-responder mapping plus claimed-actor match.
- `browser_local_state_not_authority`: removed the unsupported grant-check claim; the semantics now name only server-side session/CSRF checks.
- `ElicitationStaleTargetInert`: narrowed the claim from general state immutability to exclusion of the `answered` state and answer data.
- `SpawnRevocationDoesNotCascade`: limited descendant non-revocation to traces where a descendant grant exists.
- `docs/VERIFICATION.md`: updated the hand-authored `ElicitationResponderAuthority` checked-model description; `check-models.mjs` regenerated all seven corresponding model-table semantics cells from their `@promotion` blocks.
- `command_lifecycle.qnt` header: now names the three retained promoted properties and explicitly leaves durability and the terminal-race failure boundary to the v1 formal gate.
- `patchbay-relational.als` header and adjacent note: now classify all Alloy properties as draft/reserved and state that the bounded UNSAT assertion result does not establish non-vacuity.
- `snapshot_recovery.qnt` header: now identifies the reserved stated-normative property ids as having no executable formulas and defers their complete models to the v1 formal gate.
- Verification: `quint parse` exited 0 for `command_lifecycle.qnt`, `authority.qnt`, `subscription_authority.qnt`, `csrf_browser.qnt`, `elicitation_lifecycle.qnt`, and `snapshot_recovery.qnt`; the first `check-models.mjs` run regenerated `docs/VERIFICATION.md` and exited 1 as expected, and the confirming second run exited 0. `git diff --check` also passed.
- Discrepancies from design: the Scope and Implementation preamble said “five” properties and the Files list omitted `elicitation_lifecycle.qnt`; corrected both to match the seven-property acceptance criteria. No formula, status, invocation, or bounds changed.
- Dispatch: direct-read only; the targets and required wording were fully specified, so exploratory fan-out would not improve the evidence.
- Adjacent issues parked: none.
