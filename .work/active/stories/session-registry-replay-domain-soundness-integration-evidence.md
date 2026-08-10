---
id: session-registry-replay-domain-soundness-integration-evidence
kind: story
stage: implementing
tags: [verification, protocol]
parent: session-registry-replay-domain-soundness
depends_on: [session-registry-replay-domain-soundness-bound-registry-contract]
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Session integration and property evidence

## Checkpoint

Exercise the bound registry through the real acceptance target-resolution seam,
attack malformed/conflicting replay records as a table, and broaden the session
property oracle across collision-prone adapter/scope/runtime identities. Keep
production decision-gate serialization as separate composition-root evidence.

## Acceptance evidence

- `acceptance_pipeline` uses a real populated `SessionRegistry` for at least one
  accepted runtime-session Operation and one cross-domain rejection; the latter
  creates no command record even though the grant test double otherwise
  authorizes it.
- Table-driven registry cases cover missing/empty/wrong outer domain, missing
  LSN, inner/outer domain mismatch, missing mutation/identity/state, unknown
  enum values, exact redelivery, conflicting same-LSN payload, unseen older
  owned events, and duplicate registration at a new LSN. Rejected cases leave
  the projection unchanged.
- Ingest tests prove all single- and multi-delta successes warm immediately,
  domain mismatch appends nothing in either domain, partial-append failure
  preserves only the committed prefix, and exact caller redelivery remains
  inert.
- A multi-identity proptest varies adapter id, deployment scope, runtime id,
  and generation using deliberate one-dimension collisions. Its independent
  per-identity oracle checks non-decreasing live generation, retained
  tombstones, hot/rebuilt equality, and that a report cannot mutate another
  identity.
- Mutation evidence catches a faulty identity key that omits an adapter, scope,
  or runtime dimension; no test derives its expected key/equality from the
  production registry helper.
- The existing server regression
  `concurrent_conflicting_model_reports_leave_a_replayable_log` stays green and
  is reported only as evidence for the composition-root `CoreDecisionGate`, not
  as a core-writer safety guarantee.
- Evidence remains implementation-checked. No formal property, conformance
  vector, wire contract, or normative assurance tier is promoted.

## Ordering constraints

Depends on
`session-registry-replay-domain-soundness-bound-registry-contract`; these tests
verify the integrated behavior instead of defining a second replay/domain
predicate. Because this story is tagged `[verification]`, its later story-level
review follows the project deep lane and attacks the equality and identity-key
mutation witnesses before advancing to `done`.
