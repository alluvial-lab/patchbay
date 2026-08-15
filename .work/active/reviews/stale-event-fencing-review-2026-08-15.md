---
id: stale-event-fencing-review-2026-08-15
kind: story
stage: done
tags: [review, spawn]
parent: research-handoff-spawn-stale-event-fencing
created: 2026-08-15
updated: 2026-08-15
---

# Thorough review — shared runtime-generation fence and durable evidence quarantine

## Verdict

**MATERIAL** — return the story to `implementing`.

The production paths reviewed at `21ae3c2` plus `006a9e3` route SessionReport, Result, delivery acknowledgement, transcript Event/Status/Delta, and runtime Elicitation mutation through the shared classifier before ordinary writes. Current evidence continues to its existing validator/writer, non-report `ClaimedSuccessor` fails closed, and non-current evidence uses the atomic outer quarantine/audit writer. The `RuntimeTranscriptStatusEvidence` wrapper preserves legitimate Current Observations unchanged while preventing a generic direct-Observation unwrap from reaching transcript/status evidence; the clean Pi e2e suite also passes with the corrected diagnostic test reader.

The required enumerate-first assurance is not real, however. A fresh removal of the Elicitation family from the production fence survived the test claimed to enumerate fenced ingress. The separate hand-built all-family integration test caught the behavior, but the story explicitly requires the generated-contract inventory oracle itself to fail when any family loses the port. For this security-critical containment boundary, the surviving fence-removal mutant is material verification debt.

## Findings

### MATERIAL — the enumerate-first inventory does not prove that enumerated families use the fence

**Locations:** `server/src/adapter_service/tests.rs:476-500`, `server/src/adapter_service.rs:1993-2013`, `server/src/adapter_service/tests.rs:2796`

`runtime_ingress_inventory_enumerates_generated_rpc_and_observation_families` parses generated-contract source, but then compares it only with two manually repeated name sets. It never invokes production ingress, never observes `RuntimeGenerationFence::classify`, and has no relationship to the routing branches at `adapter_service.rs:1993-2013`.

Reviewer mutation: changing the production condition to exclude prepared runtime Elicitations (`runtime_target.is_some() && prepared_elicitation.is_none()`) made stale Elicitation evidence append as a normal `StoredEventKind::Elicitation` instead of outer quarantine. The claimed enumerate-first test still passed (exit 0). Only the separately hand-authored `every_runtime_ingress_family_uses_one_fence_and_only_outer_quarantine` test failed (exit 101, observed kind 3 instead of quarantine kind 19).

This contradicts the story acceptance item “Enumeration test fails when any runtime ingress lacks the port or dispatches quarantine incorrectly” and leaves the generated inventory disconnected from the security property it purports to guard.

**Concrete fix:** replace or join the schema-name assertion with an inventory-driven routing oracle. Derive the admitted runtime candidate set from the generated `QuarantinedRuntimeEvidence.candidate` registry and the Observation enum/oneof descriptors, map every generated member to an authenticated ingress fixture, and assert both (a) exact generated-set equality and (b) exactly one shared-fence classification before any normal writer. Make the production router injectable with a counting test fence, or centralize the complete runtime-family dispatch behind one testable function used unconditionally by `ingest_observation`. Re-run the Elicitation-bypass mutant and equivalent ack/transcript/Result bypasses; the enumerate-first oracle itself must kill each one.

## Disposition and invariant assessment

- **Inventory completeness (production):** pass by source tracing and end-to-end behavior for SessionReport, Result, acknowledgement, transcript Event/Status/Delta, and runtime Elicitation. Exact staged-SessionReport retry reconciliation is byte/source-exact and read-only; it does not append or reproject fresh evidence.
- **Disposition semantics:** pass. `Current` continues to existing source-order, terminality, correlation, target, and Elicitation lifecycle checks. Only SessionReport can retain `ClaimedSuccessor`; other families fail at the consumer boundary. Tombstoned/unknown/mismatched evidence cannot reach ordinary writers.
- **Outer-only durability/replay:** pass. Quarantine is one dedicated stored kind plus an atomically linked stale-event audit. Session, command, claim, Elicitation, authority, diagnostics, and adapter folds ignore the outer event and do not recursively apply nested candidates.
- **Independent fences:** pass. Attachment-token replacement rejects before classification/writes, while a current attachment carrying old runtime-generation evidence reaches runtime quarantine; current attachment/current runtime evidence remains admitted.
- **Diagnostics-only quarantine:** pass. Nested evidence supplies audit/diagnostic context only and cannot create command transition/completion, transcript Observation, Elicitation mutation, session state, claim state, or authority.
- **Transcript wrapper:** pass by code path and clean e2e. Event/Status/Delta are wrapped only for quarantine classification; `Current` still sends the original Observation to the ordinary writer. Replay dispatch remains on the outer stored kind.
- **Assurance completeness:** fail for the enumerate-first mutation requirement described above.

## Mutation matrix

Every mutant was applied alone on the main tree, run through focused tests, reverted with `git restore`, and followed by a clean status check.

| Mutant | Oracle | Result |
|---|---|---|
| Exclude prepared runtime Elicitation from the production runtime-fence block | `runtime_ingress_inventory_enumerates_generated_rpc_and_observation_families` | **SURVIVED** — exit 0; material finding |
| Same Elicitation fence bypass | `every_runtime_ingress_family_uses_one_fence_and_only_outer_quarantine` | **Killed** — exit 101; normal Elicitation kind 3 observed instead of quarantine kind 19 |
| Recursively dispatch a direct nested quarantined Observation through `CommandIndex` | `every_quarantine_family_is_outer_only_across_all_normal_hot_and_replay_folds` | **Killed** — exit 101; nested Observation created deferred spawn-success state |
| Remove the consumer-boundary guard rejecting non-SessionReport `ClaimedSuccessor` | `claimed_successor_can_never_stage_a_non_session_report_family` | **Killed** — exit 101 on the first admitted non-report candidate |

## Clean-tree verification

- `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** — all workspace unit/integration/property/doctests and warnings-denied clippy.
- `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** — 55 vectors, 17 promoted vectors, 22 implementation checks, 38 killed mutation witnesses, 54 model-promotion blocks.
- `cd operator-domain && npm run build && npm test`: **PASS** — 23/23.
- `cd pi-adapter && npm test`: **PASS** — 38/38, including the real core/adapter/generation-bump/reconnect/restart e2e.
- Final pre-review-file `git status --short`, `git diff --check`, and disk check: **clean / pass**, 61G free on `/`.

## Recommendation

**Return to implementing.** Connect the generated ingress inventory to observable shared-fence invocation, add mutation-sensitive inventory coverage for every runtime candidate family, and rerun the Elicitation fence-removal mutant before advancing to `done`.
