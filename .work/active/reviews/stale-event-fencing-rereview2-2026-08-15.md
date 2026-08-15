---
id: stale-event-fencing-rereview2-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-stale-event-fencing
created: 2026-08-15
updated: 2026-08-15
---

# Thorough re-review 2 — namespace-qualified runtime-ingress enumeration

## Verdict

**MATERIAL** — return the story to `implementing`.

The r2 change closes the pass-2 cross-namespace collision: every current `QuarantinedRuntimeEvidence.candidate` arm now has a distinct `CandidateArm(name)` iteration and exact typed-candidate assertion. Both the requested direct `status` collision and a unique new candidate arm fail as unmapped. Current Result/Event/Status/Delta expansion remains typed, and the Elicitation/acknowledgement bypass, outer-only, `ClaimedSuccessor`, and independent attachment/runtime-generation regression oracles pass or kill their mutants.

The inventory still has one same-namespace collision, however. `ObservationKind::Event` is renamed to `"transcript_event"` before insertion into the `BTreeSet`, so a future generated `OBSERVATION_KIND_TRANSCRIPT_EVENT` is silently deduplicated and reuses the existing Event fixture. That generated kind survived the behavioral enumeration with exit 0.

## Findings

### MATERIAL — normalized ObservationKind identities can still silently collide

**Locations:** `server/src/adapter_service/tests.rs:493-520`, `server/src/adapter_service/tests.rs:657-676`, `server/src/adapter_service/tests.rs:746-765`

`generated_runtime_ingress_families` preserves the candidate-arm namespace, but it does not preserve each generated `ObservationKind` member's canonical identity. The existing `OBSERVATION_KIND_EVENT` is normalized to `ObservationKind("transcript_event")`; all other kinds are lowercased. The result is then inserted into a `BTreeSet<RuntimeIngressFamily>`.

Reviewer mutation: adding the valid generated member

```proto
OBSERVATION_KIND_TRANSCRIPT_EVENT = 5;
```

made both that new member and the existing `OBSERVATION_KIND_EVENT` normalize to `ObservationKind("transcript_event")`. The focused behavioral inventory stayed green (exit 0), exercised only the existing Event ingress, and asserted the existing `Candidate::TranscriptStatus` payload. The new generated kind had no distinct authenticated fixture or typed assertion.

This contradicts the story's claim that every generated `ObservationKind` is enumerated and that a new unmapped arm/kind fails. It is the same class of assurance defect as pass 2, now within the ObservationKind namespace rather than across candidate/kind namespaces.

**Required direction:** preserve the exact generated enum member in the inventory identity (for example `ObservationKind("OBSERVATION_KIND_EVENT")`) and keep any friendly fixture/routing key separate. A new enum member must create a distinct set entry and fail until it receives its own authenticated request and exact typed-kind assertion. Add the `OBSERVATION_KIND_TRANSCRIPT_EVENT` collision as a mutation witness.

## Mutation matrix

Every mutant was applied alone on the main tree, exercised with the focused inventory test, reverted with `git restore`, and followed by a clean status check.

| Mutant | Oracle result |
|---|---|
| Add direct candidate arm `RuntimeTranscriptStatusEvidence status = 10`, colliding with existing Status kind | **Killed** — exit 101 on unmapped `CandidateArm("status")` |
| Add unique direct candidate arm `future_runtime_family` | **Killed** — exit 101 on unmapped `CandidateArm("future_runtime_family")` |
| Exclude prepared runtime Elicitation from the shared fence | **Killed** — exit 101; ordinary Elicitation kind 3 observed instead of quarantine kind 19 |
| Exclude delivery acknowledgement from the shared fence | **Killed** — exit 101; ordinary Observation kind 2 observed instead of quarantine kind 19 |
| Add `OBSERVATION_KIND_TRANSCRIPT_EVENT`, colliding after Event's friendly-name normalization | **SURVIVED** — exit 0; material finding |

## Regression evidence

- `runtime_ingress_inventory_enumerates_generated_rpc_and_observation_families`: **PASS** on the restored clean tree.
- `every_runtime_ingress_family_uses_one_fence_and_only_outer_quarantine`: **PASS**; current acknowledgement, Result, Event, Status, Delta, Elicitation, and SessionReport families remain outer quarantine only.
- `claimed_successor_can_never_stage_a_non_session_report_family`: **PASS**.
- `every_quarantine_family_is_outer_only_across_all_normal_hot_and_replay_folds`: **PASS**.
- `attachment_epoch_and_runtime_generation_are_independent_ingress_fences`: **PASS**.
- Source inspection confirms Result uses `Candidate::Observation`, while Event/Status/Delta use `Candidate::TranscriptStatus` and retain the exact nested generated `ObservationKind`.

## Clean-tree full suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 55 vectors, 17 promoted, 22 implementation checks, 38 killed mutation witnesses, 54 model-promotion blocks.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 23/23.
4. `cd pi-adapter && npm test`: **PASS** — 38/38, including the real core/adapter generation-bump, reconnect, and restart e2e.

Disk remained healthy with 61G free on `/`. The tracked tree was clean before the suite and after every mutation restore; `git diff --check` passed.

## Recommendation

**Return to `implementing`.** Preserve canonical generated `ObservationKind` identities separately from fixture routing names, add the same-namespace collision witness, and require the focused behavioral enumeration to fail for that new unmapped kind before advancing to `done`.
