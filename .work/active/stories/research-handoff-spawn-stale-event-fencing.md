---
id: research-handoff-spawn-stale-event-fencing
kind: story
stage: done
tags: [protocol, security, verification]
parent: research-handoff-spawn
depends_on: [research-handoff-spawn-generation-monotonicity-tombstoning]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-15
---

# Shared runtime-generation fence and durable evidence quarantine

## Redesign disposition

Rewritten. The old design appended a raw Observation plus separate stale audit and lacked `ClaimedSuccessor`. Both shapes are superseded.

## Checkpoint

Route every runtime-targeted adapter ingress through one reconciled classifier: SessionReport, Observation/Result, delivery acknowledgement, transcript/status/delta, and Elicitation mutation. `Current` continues through normal checks. `ClaimedSuccessor` is valid only for exact managed successor SessionReport staging. `Tombstoned`, `Unknown`, and `IdentityMismatch` reject or persist a self-contained `QuarantinedRuntimeEvidence` outer event with an atomic audit.

No normal projection may dispatch the nested quarantined candidate as an Observation or other authoritative event during hot fold or replay.

## Design

**Files**
- `core/src/session/logical_target.rs` — shared classifier implementation.
- `core/src/acceptance/{ports,observation,elicitation}.rs`, `core/src/adapter/mod.rs` — consumer-owned fence port at every ingress.
- `server/src/adapter_service.rs` — authenticated source + gate catch-up + atomic quarantine/audit append.
- Transcript/ack/Elicitation paths and enumerate-first tests.

```rust
pub trait RuntimeGenerationFence: Send + Sync {
    fn classify(
        &self,
        domain: &AuthorityDomainId,
        candidate: &RuntimeEvidenceCandidate,
    ) -> Result<RuntimeGenerationDisposition, SessionError>;
}
```

Quarantine carries the original typed candidate, exact classified/current/claim context, attachment source, and canonical reason. It is durable diagnostics/audit evidence only. It never becomes a command transition, completion fact, transcript entry, Elicitation response, session mutation, or authority input.

## Acceptance evidence

- [x] Legitimate first successor report stages through `ClaimedSuccessor` without a bypass.
- [x] Generation-N evidence after N+1 promotion yields quarantine/stale audit and no live mutation.
- [x] Stale ack/result/transcript/status/Elicitation/report candidates are equally inert.
- [x] Hot fold and replay see only the outer quarantine kind; a nested raw Observation cannot reach normal projections.
- [x] Current evidence still obeys source order, terminal finality, correlation, and authority; classifier success bypasses none.
- [x] Replaced adapter token/epoch and runtime generation independently fence evidence.
- [x] Enumeration test fails when any runtime ingress lacks the port or dispatches quarantine incorrectly.

## Unit 5 implementation evidence

- Added the consumer-owned `RuntimeGenerationFence` over the generated quarantine-candidate oneof and one reconciled implementation that delegates to the existing SessionReport and runtime-target classifiers. The boundary rejects `ClaimedSuccessor` for every non-SessionReport candidate even if a faulty fence returns it.
- Routed authenticated SessionReport, delivery acknowledgement, Result, transcript Event, Status, Delta, and protobuf Elicitation ingress through that fence before their ordinary validators or writers. Current evidence still takes those existing paths; non-current evidence takes one typed atomic quarantine/audit writer.
- Added normal Elicitation ingestion and replay projection support so stale Elicitation mutations have useful pre-state and cannot mutate it from inside a quarantine envelope.
- Removed the former unclaimed-discovery `Unknown` exception: a current authenticated attachment's first valid SessionReport is classified `Current`, while unrelated unknown runtime evidence remains fenced.
- Added generated-contract inventory oracles for `ObservationRequest`, every `ObservationKind`, and every `QuarantinedRuntimeEvidence.candidate` arm. A new generated family now requires an explicit routing decision.
- Added all-family integration and projection tests proving outer-only durability plus hot-fold/replay inertness across session, command, Elicitation, authority, diagnostics, and adapter projections. Added an independent attachment-epoch versus runtime-generation fence test.
- Killed all required local mutants: non-report `ClaimedSuccessor` bypass; quarantine writer using raw `Observation` outer kind; and command projection recursively dispatching nested quarantined Observation/transcript payloads.
- Verification passed: workspace tests; targeted stale-generation/replay suites; workspace clippy with warnings denied; formatting and generated-contract drift check. No protocol or generated contract changed.

## Implementation notes

### Fix round — behavioral enumerate-first ingress oracle

- Execution capability: `openai-codex/gpt-5.6-sol`; the security-sensitive, mutation-driven single-test fix used direct reads and one cohesive owner rather than fan-out.
- Review weight: `thorough` from the autopilot caller; a fresh independent re-review follows this transition.
- Files changed: `server/src/adapter_service/tests.rs` plus this story record. No protocol or generated-contract files changed.
- The enumerate-first test now derives runtime families from the canonical `QuarantinedRuntimeEvidence.candidate` oneof and `ObservationKind` registry, prepares valid generation-1 ordinary-path preconditions, supersedes that runtime, then sends one stale candidate per generated family through the real authenticated `IngestObservation` RPC. Every generated family must return an outer `QuarantinedRuntimeEvidence` whose typed candidate matches the inventory-derived family; a new unmapped arm/kind fails for lack of a real ingress fixture.
- Judgment rationale: the behavioral oracle deliberately requires quarantine rather than allowing rejection because every current fixture is otherwise valid for its ordinary writer. This stronger outcome makes omission of a family-specific fence observable as a normal stored kind while retaining the separate all-family integration oracle unchanged.
- Mutation evidence: the reviewer Elicitation bypass (`runtime_target.is_some() && prepared_elicitation.is_none()`) failed the enumerate-first test with normal `Elicitation` kind `3` instead of quarantine kind `19`; an independent delivery-acknowledgement bypass failed it with normal `Observation` kind `2` instead of `19`. Each mutant was applied alone and reverted with `git restore`; production remained clean.
- Tests added/changed: upgraded `runtime_ingress_inventory_enumerates_generated_rpc_and_observation_families` from schema-name comparison to the generated-inventory-driven authenticated behavioral oracle. The existing `every_runtime_ingress_family_uses_one_fence_and_only_outer_quarantine` integration oracle remains unchanged and passes.
- Full verification passed: (1) `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`; (2) `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` (55 vectors, 17 promoted, 22 implementation checks, 38 killed mutation witnesses, 54 model-promotion blocks); (3) `cd operator-domain && npm run build && npm test` (23/23); (4) `cd pi-adapter && npm test` (38/38).
- Simplification/discrepancies/adjacent issues: no production abstraction or behavior changed; no design discrepancy and no adjacent issue was parked.

### Fix round 2 — namespace-qualified ingress enumeration

- Execution capability: `openai-codex/gpt-5.6-sol`; direct-read implementation remained the safest cohesive path for this single test-oracle correction. Review weight remains `thorough` from the autopilot caller.
- Files changed: `server/src/adapter_service/tests.rs` plus this story record. No production, protocol, or generated-contract file changed.
- Replaced the flattened `BTreeSet<String>` inventory with namespace-qualified `RuntimeIngressFamily::CandidateArm(name)` and `RuntimeIngressFamily::ObservationKind(name)` identities. Every generated quarantine-candidate arm now receives its own real authenticated ingress run and exact typed-candidate assertion; admitted Observation kinds are enumerated separately beneath the generated `observation` or `transcript_status` arms.
- Judgment rationale: preserving both registry namespaces is simpler and stronger than a merged-size assertion. A future direct `status` candidate and the existing Status Observation kind remain two independently exercised identities, while wrapper arms (`observation`, `transcript_status`) are themselves exercised in addition to their kind expansions.
- Mutation evidence: temporarily re-injected `RuntimeTranscriptStatusEvidence status = 10` into the candidate oneof; the focused inventory oracle failed with exit 101 on unmapped `CandidateArm("status")` instead of reusing/skipping `ObservationKind("status")`. Re-injected the pass-1 Elicitation bypass (`runtime_target.is_some() && prepared_elicitation.is_none()`); the same oracle failed with exit 101 because `CandidateArm("elicitation_mutation")` wrote normal Elicitation kind `3` instead of outer quarantine kind `19`. Each mutant was applied alone, reverted with `git restore`, and followed by clean diff/status checks.
- Clean focused evidence: both `runtime_ingress_inventory_enumerates_generated_rpc_and_observation_families` and `every_runtime_ingress_family_uses_one_fence_and_only_outer_quarantine` pass.
- Full verification passed: (1) `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`; (2) `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` (55 vectors, 17 promoted, 22 implementation checks, 38 killed mutation witnesses, 54 model-promotion blocks); (3) `cd operator-domain && npm run build && npm test` (23/23); (4) `cd pi-adapter && npm test` (38/38).
- Simplification/discrepancies/adjacent issues: removed cross-namespace deduplication as a representable state; no design change, no discrepancy, no residual concern, and no adjacent issue parked.

## Ordering constraint

Depends on the exact promotion/tombstone projection. Completion is blocked until the ingress inventory is complete.
