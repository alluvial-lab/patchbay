---
id: story-verification-correction-trace-fidelity-demotion
kind: story
stage: done
tags: [verification, protocol, bug]
parent: epic-public-product-contract-verification-claim-correction
depends_on: [story-verification-correction-retained-semantics]
release_binding: v0.1.0
gate_origin: null
created: 2026-07-10
updated: 2026-07-11
---

# Demote trace-fidelity-defective promoted authority properties and fix GenerationMonotonic prose

## Scope

Demote four promoted properties whose formulas do not independently establish the behavior their names and semantics claim, discovered in the round-2 deep review of this feature. Three authority/subscription properties inspect state recorded by the same action that decides acceptance (a trace-fidelity defect: a mutation that accepts arbitrary inputs while recording the expected values passes the invariant). The fourth, `SpawnRevocationDoesNotCascade`, does not detect deletion of a descendant grant during fleet revocation — verified by a mutation test that sets `gDescOs3Live' = "no"` in `revokeSpawnGrant` and the property still passes. Also fix `GenerationMonotonic` hand-authored prose that presents action-enforced strict-supersession as checked-model behavior.

## Unit

`Unit 7` from `epic-public-product-contract-verification-claim-correction` review (round 2). This is a review-discovered correction: the feature advanced to `done` before round 2 surfaced these defects. Filing this story re-opens the feature's review surface honestly rather than patching a done feature inline.

## Origin

Deep review round 2 (`openai-codex/gpt-5.6-sol`, xhigh, fresh-context adversarial). The reviewer ran mutation tests against the formulas and found they are not independent oracles. The host (umans orchestrator) independently verified each mutation:

- `SpawnRevocationDoesNotCascade`: mutating `revokeSpawnGrant` to set `gDescOs3Live' = "no"` (deleting the descendant) — `quint verify --temporal spawn_revocation_does_not_cascade` still reported "No violation found." Confirmed at HEAD.
- `FleetAuthorityForSpawn` / `ElicitationResponderAuthority` / `SubscriptionGrantChecked`: the invariants inspect `LastSpawnActor`, `ResponseEndpoint*`/`ResponseClaimedActor*`, and `LastSubscriptionActor`/`LastSubscriptionScope` — fields written by the accepting action (`attemptSpawn` sets `LastSpawnActor' = actor` at line 192). A mutation that accepts arbitrary inputs while recording the expected values passes the invariant. This is the "drop the check and lie about recorded evidence" defect the genuine-checking discipline guards against.

Round 1 had filed `SpawnRevocationDoesNotCascade` as a backlog item (`idea-spawn-revocation-model-coverage`) calling it "a genuine mutation-survivable independent oracle." That classification was wrong; the mutation test proves it. This story supersedes that backlog note.

## Files

- `specs/seed/authority.qnt` — demote 3 `@promotion` blocks + remove 3 `val`/`temporal` definitions
- `specs/seed/subscription_authority.qnt` — demote 1 `@promotion` block + remove 1 `val` definition
- `contracts/scripts/check-vectors.mjs` — move 4 ids from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES`
- `docs/VERIFICATION.md` — checked-model property lists, authority-tier section, seed-model summaries, generated tables, `GenerationMonotonic` prose (~line 204), `FleetAuthorityForSpawn`/`SubscriptionGrantChecked` descriptions (~122, 144)
- `docs/ADAPTER-PI.md` — `GenerationMonotonic` prose (~line 78)
- `docs/SPEC.md` — verification floor (~line 70)
- `docs/PROTOCOL.md` — any checked-model references to the 4 properties

## Implementation

### Demote the 4 trace-fidelity-defective properties

For each of the four properties:

1. In the `@promotion` block:
   - Change `status: promoted` → `status: draft`
   - Replace the concrete `invocation` with `<TBD — demoted; formula does not independently establish the claimed behavior; v1 formal gate owns the real property>`
   - Add `demotion_reason:` (see per-property reasons below)

2. Remove the `val`/`temporal` definition entirely (same discipline as Unit 4: removing the definition ensures `quint verify --invariant <name>` fails because the invariant doesn't exist, rather than passing vacuously or passing on a defective formula). Keep the `@promotion` block so the property id survives as a stated-normative obligation.

3. In `contracts/scripts/check-vectors.mjs`:
   - Remove `FleetAuthorityForSpawn`, `ElicitationResponderAuthority`, `SpawnRevocationDoesNotCascade`, `SubscriptionGrantChecked` from `CHECKED_MODEL_PROPERTIES`
   - Add them to `STATED_NORMATIVE_PROPERTIES`
   - Keep both arrays alphabetically sorted

**Per-property demotion reasons:**

- `FleetAuthorityForSpawn` (`authority.qnt`, `@promotion` at ~345, `val fleet_authority_for_spawn` at ~356): the invariant inspects `LastSpawnActor`, which `attemptSpawn` sets to the attempted `actor` (line 192) — the same actor the acceptance guard checks. The formula proves the *recorded* accepting actor matches the grant subject, not that an independent submitting-actor claim was verified against the grant. A mutation that accepts arbitrary spawn inputs while recording the grant's subject as `LastSpawnActor` passes. The formula is not an independent oracle for submitting-actor authority.

- `ElicitationResponderAuthority` (`authority.qnt`, `@promotion` at ~387, `val elicitation_responder_authority` at ~398): the invariant inspects `ResponseEndpoint*` and `ResponseClaimedActor*` fields written by `attemptElicitationResponse`. The formula proves the *recorded* endpoint/actor match the expected responder, not that an independent submitting-endpoint/claimed-actor claim was verified. A mutation that accepts arbitrary endpoint/claim inputs while recording the expected values passes. The formula is not an independent oracle for responder authority.

- `SpawnRevocationDoesNotCascade` (`authority.qnt`, `@promotion` at ~373, `temporal spawn_revocation_does_not_cascade` at ~384): the formula checks descendant status only when `gDescOs3Live == "yes"` in the post-state. Mutating `revokeSpawnGrant` to set `gDescOs3Live' = "no"` (deleting an existing descendant during fleet revocation) still passes — verified by mutation test. The descendant's preservation is encoded in the action (`revokeSpawnGrant` copies `gDescOs3Live` at line 210), not proved by the formula. The formula is not a mutation-survivable oracle for non-cascade behavior.

- `SubscriptionGrantChecked` (`subscription_authority.qnt`, `@promotion` at ~208, `val subscription_grant_checked` at ~219): the invariant inspects `LastSubscriptionActor`/`LastSubscriptionScope` written by the subscription-establishing action. Same trace-fidelity defect as the authority properties: the formula proves the *recorded* actor/scope match the grant, not that an independent submitting-actor/scope claim was verified. A mutation that accepts arbitrary inputs while recording the grant's subject/scope passes.

### Fix `GenerationMonotonic` hand-authored prose (Blocker B)

The `@promotion` `semantics:` (SSOT) explicitly says: "the live session generation never decreases (checked). Strict-supersession (equal/lower reports are no-ops) is additionally enforced by the action guard (`if gen > generation`) but is NOT a checked temporal property." But hand-authored prose presents strict-supersession as checked:

- `docs/VERIFICATION.md:204`: `- **GenerationMonotonic**: session supersession requires a strictly-greater generation; lower reports are rejected as audit and the live generation is unchanged; equal reports are a no-op.` — narrow to: the checked property proves the live session generation never decreases; strict-supersession (lower rejected, equal no-op) is enforced by the action guard, not a checked temporal property.

- `docs/ADAPTER-PI.md:78`: `- `GenerationMonotonic` (**checked-model**) — session supersession (`session_new` / fresh-session restart) requires a strictly-greater generation; a lower report is rejected; an equal report is a no-op.` — narrow the same way.

### Update VERIFICATION.md prose

Update the non-generated prose to reflect the 4 demotions:

- Checked-model property lists (the `authority.qnt` and `subscription_authority.qnt` lines) — remove the 4 demoted names; keep the genuinely promoted properties.
- The `authority.qnt promotion status` section (~lines 116-145): the checked-model spawn list now has zero promoted spawn properties (all 3 demoted); the subscription section loses `SubscriptionGrantChecked`. Move the 4 to stated-normative.
- `FleetAuthorityForSpawn` description (~line 122) and `SubscriptionGrantChecked` description (~line 144) — these were flagged by round 2 as not matching their generated `semantics:` rows. Now that the properties demote, remove them from the checked-model description list entirely (they appear in the stated-normative list instead).
- Seed-model summary tables — move the 4 to the draft column; update the summary counts.
- `GenerationMonotonic` line (~204) per above.

### Update SPEC.md verification floor (~line 70)

The current text says "fleet-spawn and Elicitation-responder authority with non-cascading spawn-grant revocation, subscription authority" as checked-model coverage. After demotion, these are stated-normative. Update to remove them from the checked-model seed coverage list and add to the stated-normative list.

### Update PROTOCOL.md

Grep for any checked-model references to the 4 demoted properties and mark them stated-normative. (Phase 1 review found none at HEAD for PROTOCOL.md, but verify.)

### Verify the other 2 subscription properties are NOT defective

`SubscriptionAudited` and `SubscriptionCursorReplayAuthorized` (both promoted) inspect structural invariants (`operationRecordsCreated == 0`, audit-record counts, replayed-event authorization) rather than state recorded by an accepting action to verify a submitting-actor claim. They do NOT have the trace-fidelity defect. Confirm by reading their formulas; they stay promoted. Record the assessment in implementation notes.

### Verification

Run `node contracts/scripts/check-vectors.mjs` (exits 0, regenerates conformance table), then `node contracts/scripts/check-models.mjs` (exits 1, regenerates model table), then `node contracts/scripts/check-models.mjs` again (exits 0, confirms current).

```
export PATH="$HOME/.npm-global/bin:$PATH"
quint parse specs/seed/authority.qnt
quint parse specs/seed/subscription_authority.qnt
node contracts/scripts/check-vectors.mjs
node contracts/scripts/check-models.mjs
node contracts/scripts/check-models.mjs
```

## Demotion reasons (consolidated for the record)

- `FleetAuthorityForSpawn`: trace-fidelity defect — invariant inspects `LastSpawnActor` written by the accepting action; proves recorded-actor consistency, not submitting-actor authority. Not an independent oracle.
- `ElicitationResponderAuthority`: trace-fidelity defect — invariant inspects endpoint/actor fields written by the accepting action; proves recorded-endpoint consistency, not responder authority. Not an independent oracle.
- `SpawnRevocationDoesNotCascade`: mutation-fragile — descendant deletion during fleet revocation passes the formula (verified by mutation test); preservation is action-encoded, not formula-proved. Not a mutation-survivable oracle.
- `SubscriptionGrantChecked`: trace-fidelity defect — invariant inspects `LastSubscriptionActor`/`LastSubscriptionScope` written by the accepting action; proves recorded-actor/scope consistency, not submitting-actor/scope authority. Not an independent oracle.

The real independent-actor/endpoint/scope verification and the genuine non-cascade formula (pre/post state, separate-revocation path in the step relation) are v1 formal-gate work owned by `epic-public-product-contract-executable-release-assurance`.

## Acceptance criteria

- [x] Four `@promotion` blocks changed to `status: draft` with `demotion_reason` and `<TBD>` invocation.
- [x] Four `val`/`temporal` definitions removed from `authority.qnt` (3) and `subscription_authority.qnt` (1); `@promotion` blocks preserved.
- [x] Four ids moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [x] `node contracts/scripts/check-vectors.mjs` exits 0; `node contracts/scripts/check-models.mjs` exits 0 on second run.
- [x] `quint parse specs/seed/authority.qnt` and `quint parse specs/seed/subscription_authority.qnt` exit 0.
- [x] VERIFICATION.md prose updated: checked-model property lists, authority-tier section, seed-model summaries, `GenerationMonotonic` prose (~204), `FleetAuthorityForSpawn`/`SubscriptionGrantChecked` descriptions removed from checked-model list.
- [x] ADAPTER-PI.md:78 `GenerationMonotonic` prose narrowed.
- [x] SPEC.md:70 verification floor updated: the 4 demoted properties moved from checked-model to stated-normative.
- [x] PROTOCOL.md verified: no surviving checked-model references to the 4 demoted properties.
- [x] `GenerationMonotonic` stays promoted; its prose now says "never decreases (checked)" and describes strict-supersession as action-enforced, not checked.
- [x] `SubscriptionAudited` and `SubscriptionCursorReplayAuthorized` assessed and remain promoted (no trace-fidelity defect — structural invariants). Assessment recorded in implementation notes.
- [x] Final promoted count: 17 (21 − 4). Final stated-normative: 30 (26 + 4).


## Review (2026-07-11)

**Verdict**: Approve - fast-lane advance.

Story verified by implement (green `quint parse` + checkers); the cumulative diff across all 8 units was covered by the feature's 6-round deep-review convergence loop, which confirmed the final state (8 promoted / 39 stated-normative, 24 demotions, 24 formulas removed, 8 survivors mutation-confirmed sound).
## Implementation notes

- Delivery mode: direct-read inline implementation; the affected model blocks, tier registry, and prose integration points were explicit, so no exploratory agent fan-out was needed.
- Files changed: `specs/seed/authority.qnt`, `specs/seed/subscription_authority.qnt`, `contracts/scripts/check-vectors.mjs`, `docs/VERIFICATION.md`, `docs/ADAPTER-PI.md`, and `docs/SPEC.md`.
- Demoted `FleetAuthorityForSpawn`, `ElicitationResponderAuthority`, `SpawnRevocationDoesNotCascade`, and `SubscriptionGrantChecked` to draft metadata and removed their executable definitions. Each retained `@promotion` block names the concrete trace-fidelity or mutation-survivability defect and reserves the property id for the v1 formal gate.
- Tier SSOT and generated artifacts: moved all four ids to `STATED_NORMATIVE_PROPERTIES`; final registry/model counts are 17 checked-model and 30 stated-normative (27 modeled draft plus 3 reserved-unmodeled). `docs/VERIFICATION.md` generated tables were regenerated only through the checker scripts.
- `GenerationMonotonic` remains promoted. Hand-authored prose in `docs/VERIFICATION.md` and `docs/ADAPTER-PI.md` now distinguishes checked non-decrease from strict-supersession enforced by the action guard.
- Retained-subscription assessment: `SubscriptionAudited` checks structural counters (`operationRecordsCreated == 0`, `auditRecords == SubscriptionEstablishAttempts`, and the attempt bound), rather than action-recorded submitting evidence. `SubscriptionCursorReplayAuthorized` checks replayed-event structure through cursor, LSN, stream/filter, and grant facts. Neither uses an accepting action's recorded actor/scope as proof of that same submitted claim, so both remain promoted.
- `docs/PROTOCOL.md` was searched for all four property ids; it contains no checked-model references requiring an edit.
- Verification results: both Quint parses exited 0; `check-vectors.mjs` exited 0; the first `check-models.mjs` run exited 1 as expected while regenerating the model table; the second run exited 0 with 44 promotion blocks, 17 checked-model properties, and 30 stated-normative properties.
- Discrepancies resolved in-stride: none. The generated model summary expresses the stated-normative total as 27 modeled drafts plus 3 reserved-unmodeled properties, matching the requested total of 30.
- Tests added: none; verification is provided by the existing Quint parser and traceability checkers.
- Adjacent issues parked: none.
