---
id: epic-agent-operations-resource-plane-resource-state-integration-foundation
kind: story
stage: done
tags: [foundation, protocol, storage]
parent: epic-agent-operations-resource-plane-resource-state
depends_on: [epic-agent-operations-resource-plane-resource-state-snapshot-load]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-03
updated: 2026-08-04
---

# Close resource-state integration and foundation assertions

## Checkpoint

Exercise authenticated report → durable event → replayed resolver → resource
snapshot end to end, update every exhaustive `StoredEventKind` receiver, and
roll foundation docs forward with the committed resource state/reconnect
semantics and honest implementation-evidence tier. Keep capability-manifest,
cockpit composition, and promoted conformance evidence in their sibling
features.

## Acceptance evidence

- Real server tests prove resource identity registration is durable across
  restart, exact replacement/tombstone routing is preserved, and
  authoritative/partial/none reconnect behavior cannot fabricate current state.
- Existing command, authority, session, diagnostics, security, storage,
  subscription, CLI, and web tests stay green after the new event and snapshot
  view variants.
- Resource and projection payloads remain exact manifest-bound metadata
  envelopes; undeclared/mismatched/unspecified formats reject and docs prohibit
  data-plane traffic or credentials.
- Workspace tests/clippy, TypeScript suites, contract build/drift, vector/model
  metadata, and presentation checks pass without claiming checked-normative
  resource conformance.

## Ordering constraints

Runs after contract, projection/replay, authenticated ingress/reconciliation,
and snapshot loading are integrated. The parent feature is reviewed as the
cohesive boundary; this child does not receive an independent feature review.

## Implementation notes

Added a real-process path that attaches a manifest-declared resource adapter,
submits an authenticated typed report, verifies one durable `RESOURCE_STATE`
append, rebuilds the composite projection after restart, resolves the exact
ordinary resource target, and loads/decodes `ResourceSnapshot` through the gRPC
`LoadSnapshot(RESOURCE)` path. A generated-sequence property test varies exact
adapter/kind/local-id dimensions and all authoritative/partial/none/delta
omission branches, then compares hot state with two independent durable replays.
Existing exhaustive durable-event consumers explicitly ignore or handle
`RESOURCE_STATE`; operator subscription filtering delivers it.

Rolled `PROTOCOL`, `ARCHITECTURE`, `SECURITY`, `VERIFICATION`, and `GLOSSARY`
forward in place. The foundation now names the separate revisioned projection,
typed report and tier semantics, source-generation fencing, terminal replacement,
snapshot discrimination, metadata-only payload boundary, and implementation-
checked evidence. It does not claim checked-model or checked-normative resource
assurance; promoted vectors/formal evidence remain with the conformance sibling.

Integrated verification passed: `cargo test --workspace`;
`cargo clippy --workspace --all-targets -- -D warnings`; Rust and TypeScript
contract builds; generated drift; vector/model metadata; presentation
conformance; CLI (37), web cockpit (75), web server (31), and Pi adapter (24)
tests including real Pi/core E2E. Repository-wide `cargo fmt --check` continues
to report the documented pre-existing broad Rust formatting drift, and
repository-wide `buf lint` continues to report the pre-existing RPC naming debt;
neither check was weakened or misreported as introduced by this feature.
