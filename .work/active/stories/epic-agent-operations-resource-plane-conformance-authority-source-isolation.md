---
id: epic-agent-operations-resource-plane-conformance-authority-source-isolation
kind: story
stage: review
tags: [verification, protocol, security]
parent: epic-agent-operations-resource-plane-conformance
depends_on: [epic-agent-operations-resource-plane-conformance-vector-execution-bridge]
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Prove resource authority and authenticated-source isolation

## Checkpoint

Execute the resource cases added to `command-acceptance.json` and
`failure-missing-grant.json`, plus dedicated Observation-source,
cross-adapter-collision, and core-state-injection vectors. Exercise the real
acceptance, grant, target-resolution, authenticated adapter ingress, durable
event-kind dispatch, and replay paths. A resource target must be authorized by
the same canonical OperationKind/grant pipeline as a session target; an adapter
channel or payload must not become grant authority or a core-owned
`RESOURCE_STATE` writer.

Extend the existing Rust/server proptest suites with independently generated
adapter/kind/local-id dimensions and source-claim attempts. Add explicit mutant
checks that omit each tuple dimension, trust a claimed source, bypass adapter
authentication, or fold an opaque Observation payload as core state; the oracle
must reject each mutant.

## Primary files

- `contracts/vectors/command-acceptance.json`
- `contracts/vectors/failure-missing-grant.json`
- `contracts/vectors/resource-observation-source-authenticated.json` (new)
- `contracts/vectors/resource-identity-collision-fenced.json` (new)
- `contracts/vectors/resource-core-state-injection-rejected.json` (new)
- `core/tests/conformance_vectors.rs`
- `server/tests/conformance_vectors.rs`
- `core/tests/authority_proptest.rs`
- `core/tests/acceptance_proptest.rs`
- `server/src/adapter_service/tests.rs`

## Acceptance evidence

- Exact live resource grant + registered exact target accepts and appends once;
  missing, expired, revoked, kind-mismatched, cross-adapter, cross-kind, and
  cross-id grants reject before an Operation append or delivery.
- An unauthenticated/stale attachment or an authenticated adapter targeting
  another adapter's resource cannot append an Observation or resource report.
- A forged Observation sender/payload remains evidence only: it creates no
  Grant/Operation/ResourceState authority and cannot terminalize a command for a
  different exact resource target.
- An Observation carrying encoded `ResourceStateEvent` bytes remains stored as
  `OBSERVATION`; rebuilding `ResourceRegistry` ignores it. Only the typed,
  authenticated report path can cause the core to normalize and append
  `RESOURCE_STATE` with core-assigned domain/LSN/revision.
- Every claim-breaking injected mutant is caught by an independent expected
  outcome derived from raw generated identities/source context.

## Ordering constraints

Depends on the shared execution bridge. Do not satisfy the vectors with a
resource-specific acceptance path or capability-derived authority.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` at high reasoning, explicit caller selection for authority/source-isolation evidence.
- Review weight: `thorough`, explicit caller override; left at `review` for the project verification deep lane.
- Files changed: the shared `command-acceptance.json` and `failure-missing-grant.json`, three resource-specific vectors, core/server conformance runners, `core/tests/authority_proptest.rs`, `core/tests/resource_replay.rs`, `server/src/adapter_service/tests.rs`, and generated conformance traceability in `docs/VERIFICATION.md`.
- Tests added: vector-driven exact acceptance/missing-grant execution through `submit_with_clock`; full tuple collision execution through production containment and target-key code; authenticated current/stale/missing/cross-owner server ingress; opaque encoded `ResourceStateEvent` Observation replay isolation; explicit tuple/source/dispatch mutant oracles.
- Mutation evidence: (1) replaced the acceptance durable append with a fabricated successful `EventId`; `command-acceptance` failed because no Operation existed. (2) changed the empty-grant path to authorize `mutant-bypass`; `failure-missing-grant` failed. (3) removed adapter equality from production `same_resource`; both the generated authority property and collision vector failed. (4) made production `require_same_adapter` unconditional; the source vector failed when the cross-adapter request appended. (5) made `ResourceRegistry` decode generic Observation payload bytes as `ResourceStateEvent`; the core-state-injection vector failed on the forged-domain event. Every mutation was reverted.
- Verification: all five focused implementation checks reported their exact ids; Rust authority/source/replay properties passed; `cargo test --workspace` passed; clippy with warnings denied passed; contracts build/vector/drift/presentation/model checks passed; web cockpit passed 103 tests.
- Simplification: resource acceptance reuses the production Operation/grant/target pipeline; source evidence reuses authenticated adapter ingress; opaque bytes remain governed by the durable event-kind discriminator.
- Discrepancies from design: vectors remain `draft` until the final promotion checkpoint, so focused package runner requests execute them directly while the umbrella checker correctly executes zero promoted checks at this checkpoint. Existing authority proptest already generated every tuple dimension; this unit added explicit mutation witnesses rather than duplicating the generator.
- Adjacent issues parked: none.
