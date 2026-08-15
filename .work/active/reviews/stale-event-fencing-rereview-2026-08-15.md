---
id: stale-event-fencing-rereview-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-stale-event-fencing
created: 2026-08-15
updated: 2026-08-15
---

# Thorough re-review — behavioral runtime-ingress inventory oracle

## Verdict

**MATERIAL** — return the story to `implementing`.

The pass-1 behavioral gap is closed for every current family: the generated-contract-derived inventory now sends stale candidates through authenticated production `IngestObservation`, and both the pass-1 Elicitation bypass and an independently isolated transcript-Event bypass fail on a normal inner write instead of outer quarantine. The inventory still has a future-staleness hole, however: it flattens candidate arms and expanded Observation kinds into one unqualified string set, so a new candidate arm can collide with an existing kind-derived name and be silently skipped.

## Findings

### MATERIAL — colliding generated family names are deduplicated before behavioral routing

**Locations:** `server/src/adapter_service/tests.rs:476-514`, `server/src/adapter_service/tests.rs:641-666`, `server/src/adapter_service/tests.rs:688-740`

`generated_runtime_ingress_families` inserts direct `QuarantinedRuntimeEvidence.candidate` field names and `ObservationKind`-derived subfamily names into the same `BTreeSet<String>`. This loses the registry and arm identity. For example, the current `OBSERVATION_KIND_STATUS` becomes `"status"`; a future direct candidate arm named `status` is then deduplicated against it.

Reviewer mutation: adding the valid contract arm

```proto
RuntimeTranscriptStatusEvidence status = 10;
```

to `QuarantinedRuntimeEvidence.candidate` left `runtime_ingress_inventory_enumerates_generated_rpc_and_observation_families` green (exit 0). The oracle exercised only the pre-existing Status Observation fixture and asserted `Candidate::TranscriptStatus`; it never requested or observed the new direct candidate arm. This is exactly a generated candidate family being silently skipped.

Generated-artifact drift and exhaustive Rust matches would force other contract work during a real regeneration, but they do not make this behavioral oracle represent the new arm once those compile sites are updated. The acceptance requirement is that the inventory itself cannot go stale and that a new arm requires a real authenticated ingress fixture.

**Concrete fix:** keep qualified inventory identities instead of flattening both registries into strings, for example `CandidateArm("status")` versus `ObservationKindFamily("status")`. Require a distinct real-ingress fixture and exact typed-candidate assertion for every oneof arm, while separately expanding `ObservationKind` only beneath the existing `observation`/`transcript_status` arms. Add the colliding-`status` contract mutation as a witness alongside the unique-arm and new-kind witnesses.

## Current behavioral coverage

The current derivation does cover every present oneof arm and all admitted Observation kinds:

| Generated candidate arm | Generated kind expansion | Authenticated production ingress exercised | Required durable candidate |
|---|---|---|---|
| `observation` | `RESULT` | `ObservationRequest.event` Result | outer quarantine / `Candidate::Observation` |
| `session_report` | n/a | `ObservationRequest.session_report` | outer quarantine / `Candidate::SessionReport` |
| `delivery_acknowledgement` | n/a | acknowledgement-shaped `ObservationRequest.event` | outer quarantine / `Candidate::DeliveryAcknowledgement` |
| `transcript_status` | `EVENT`, `STATUS`, `DELTA` | each corresponding `ObservationRequest.event` | outer quarantine / `Candidate::TranscriptStatus` with the same kind |
| `elicitation_mutation` | n/a | Elicitation-shaped `ObservationRequest.event` | outer quarantine / `Candidate::ElicitationMutation` |

`OBSERVATION_KIND_UNSPECIFIED` is deliberately non-ingress and excluded. The surrounding `ObservationRequest.observation` equality oracle still forces an explicit classification decision for a new RPC arm.

## Mutation matrix

Every source mutant was applied alone on the main tree, exercised with the focused inventory test, reverted with `git restore`, and followed by clean status/diff checks.

| Mutant | Oracle result |
|---|---|
| Pass-1 Elicitation bypass: exclude prepared runtime Elicitations from the shared runtime-fence block | **Killed** — exit 101; `elicitation_mutation` wrote ordinary Elicitation kind `3` instead of outer quarantine kind `19` |
| Isolated transcript Event bypass while preserving acknowledgement and Elicitation routing | **Killed** — exit 101; `transcript_event` wrote ordinary Observation kind `2` instead of outer quarantine kind `19` |
| Add unique valid `future_runtime_family` arm to `QuarantinedRuntimeEvidence.candidate` | **Killed** — exit 101; generated family had no real authenticated fixture |
| Add `OBSERVATION_KIND_PROGRESS` | **Killed** — exit 101; generated `progress` family had no real authenticated fixture |
| Add valid candidate arm `status`, colliding with the kind-derived Status family | **SURVIVED** — exit 0; the `BTreeSet<String>` deduplicated the new arm and reused the old `Candidate::TranscriptStatus` fixture |

The clean-tree inventory oracle passed after all mutations were reverted.

## Regression spot-checks

- `runtime_ingress_inventory_enumerates_generated_rpc_and_observation_families`: **PASS** on the clean tree.
- `every_runtime_ingress_family_uses_one_fence_and_only_outer_quarantine`: **PASS**; all seven current expanded families remain outer-only and replay-inert.
- `claimed_successor_can_never_stage_a_non_session_report_family`: **PASS**.
- `attachment_epoch_and_runtime_generation_are_independent_ingress_fences`: **PASS**.
- `every_quarantine_family_is_outer_only_across_all_normal_hot_and_replay_folds`: **PASS**; every current generated oneof arm, including the transcript/status wrapper, remains inert across repeated hot/replay folds and cannot be recursively redispatched.

## Clean-tree full verification

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 55 vectors, 17 promoted vectors, 22 implementation checks, 38 killed mutation witnesses, 54 model-promotion blocks.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 23/23.
4. `cd pi-adapter && npm test`: **PASS** — 38/38, including the real core/adapter generation-bump, reconnect, and restart e2e.

Pre-mutation, between-mutation, pre-suite, and final tracked-tree checks were clean apart from this review file; `git diff --check` passed. Disk remained healthy with 61G free on `/`.

## Recommendation

**Return to `implementing`.** Preserve registry-qualified identities in the derived inventory, require one distinct real authenticated fixture per candidate arm/kind family, and kill the colliding-candidate mutation before advancing to `done`.
