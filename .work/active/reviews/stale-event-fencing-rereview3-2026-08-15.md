---
id: stale-event-fencing-rereview3-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-stale-event-fencing
created: 2026-08-15
updated: 2026-08-15
---

# Thorough re-review 3 — raw-name runtime-ingress identity

## Verdict

**CLEAN** — advance the story to `done`.

No material finding, fence bypass, silent generated-family skip, vacuous oracle, or replay divergence remains at `6e4b4d4` over the Unit 5 implementation series.

## Findings

No material findings or nits.

- `RuntimeIngressFamily::ObservationKind` now retains each raw `OBSERVATION_KIND_*` proto value name as its key. Proto enum value names are unique within the enum, and `CandidateArm` remains a separate enum namespace, so neither the pass-2 cross-namespace collision nor the pass-3 normalized-kind collision is representable.
- The assert-side `ObservationKind` match is exhaustive over `Event`, `Status`, `Delta`, `Result`, and `Unspecified`; it has no wildcard/skip arm. Regenerating a new Rust enum variant therefore breaks compilation until the reverse mapping is extended.
- The generated inventory still sends each current candidate arm and admitted Observation kind through authenticated production `IngestObservation`, requires the dedicated outer `QuarantinedRuntimeEvidence` stored kind, decodes the exact typed candidate, and checks the raw kind identity. SessionReport, Result, acknowledgement, Event, Status, Delta, and Elicitation mutation remain covered.
- The independent outer-only, `ClaimedSuccessor`, and attachment/runtime-generation regressions remain green. Quarantine candidates stay inert across normal hot/replay folds; non-SessionReport `ClaimedSuccessor` fails closed; replaced attachment tokens reject without a write while a current attachment/current runtime generation remains admitted.

## Mutation matrix

Each mutant was applied alone on the main tree, exercised with a focused test, reverted with `git restore`, and followed by clean `git status --short` and `git diff --check` checks.

| Mutant | Focused oracle result |
|---|---|
| Inject parsed `OBSERVATION_KIND_TRANSCRIPT_EVENT`, colliding only under the removed `EVENT` → `transcript_event` normalization | **Killed** — exit 101; `ObservationKind("OBSERVATION_KIND_TRANSCRIPT_EVENT")` had no distinct authenticated fixture |
| Inject parsed candidate arm `status`, colliding textually with the existing Status kind's former friendly name | **Killed** — exit 101; namespace-qualified `CandidateArm("status")` had no distinct authenticated fixture |
| Exclude prepared runtime Elicitation mutations from the shared runtime-fence block | **Killed** — exit 101; ordinary Elicitation kind `3` appeared instead of outer quarantine kind `19` |
| Exclude delivery acknowledgements from the shared runtime-fence block | **Killed** — exit 101; ordinary Observation kind `2` appeared instead of outer quarantine kind `19` |

Clean focused regressions also passed:

- `runtime_ingress_inventory_enumerates_generated_rpc_and_observation_families`
- `every_runtime_ingress_family_uses_one_fence_and_only_outer_quarantine`
- `attachment_epoch_and_runtime_generation_are_independent_ingress_fences`
- `claimed_successor_can_never_stage_a_non_session_report_family`
- `every_quarantine_family_is_outer_only_across_all_normal_hot_and_replay_folds`

## Full clean-tree suite

1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 55 vectors, 17 promoted vectors, 22 implementation checks, 38 killed mutation witnesses, and 54 model-promotion blocks.
3. `cd operator-domain && npm run build && npm test`: **PASS** — 23/23.
4. `cd pi-adapter && npm test`: **PASS** — 38/38, including the real core/adapter generation-bump, reconnect, and core-restart e2e.

The tracked tree was clean before mutations, after every restore, before the full suite, and before writing this review. `git diff --check` passed; `/` retained 61G free.

## Recommendation

**Advance to `done`.** The r3 raw-name identity and exhaustive reverse mapping close the last enumerate-first silent-skip path, and all required mutation and regression evidence is green.
