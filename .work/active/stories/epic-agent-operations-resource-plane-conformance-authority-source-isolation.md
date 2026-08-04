---
id: epic-agent-operations-resource-plane-conformance-authority-source-isolation
kind: story
stage: implementing
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
