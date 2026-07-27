---
id: epic-revocation-lifecycle-session-principal-revocation-contract-model
kind: story
stage: implementing
tags: [security, protocol, verification, foundation]
parent: epic-revocation-lifecycle-session-principal-revocation
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Generated revocation contract and model

## Checkpoint

Pin the generated RPC, durable-event, stored-event, session-generation, and
audit vocabularies for operator-session and control-surface revocation. Add the
independent-attempt formal model and traced conformance vectors before the core
implements those semantics.

This checkpoint owns Unit 1 in the parent feature. The parent design is
authoritative for exact RPC/message names, fields, failure mapping, property
ids, and the prohibition on hand-edited generated artifacts.

## Acceptance evidence

- `control.proto` exposes revoke-all, revoke-principal, and
  revoke-endpoint/device unary RPCs with bounded reason codes and generated
  result types.
- `admin.proto`, `common.proto`, and `diagnostics.proto` own the two durable
  source-event shapes, stored-event discriminators, operator-session generation
  carriage, and outcome-bearing audit kinds.
- Rust and TypeScript generated artifacts regenerate together and the drift
  check is green.
- The Quint properties use environment pre-state/independent attempted evidence;
  removing each acceptance guard produces a checker counterexample, while
  non-vacuity runs demonstrate acceptance remains reachable.
- Vectors trace old/fresh generation, exact principal, endpoint, device, and
  unaffected-endpoint cases without claiming checked-normative status early.

## Ordering constraints

No sibling dependency. This contract and model must settle before the core-state
checkpoint consumes it. As a `[verification]` story, its review uses the
project's deep lane and attacks genuine-checking/mutation evidence.
