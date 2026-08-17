---
id: fix-fresh-spawn-one-shot-managed-target
kind: story
stage: drafting
tags: [protocol, design-gap]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-17
updated: 2026-08-17
---

# Design seam: a configured managed target can be fresh-spawned exactly once

## Reproduction (live UAT, 2026-08-16/17)

Configured Pi managed target declares a fixed `logicalTargetId`
(`uat-logical-target`). Core derives a fresh claim's logical target id from the
command id (`core/src/target.rs` resolve_spawn_adapter), so the CLI/cockpit must
use command id == configured logicalTargetId (CLI enforces: "fresh managed
spawn command id must equal the declared logical target id").

Sequence: fresh spawn `uat-logical-target` fails pre-launch (bad executable
path) → claim released_no_external_effect (correct). Every subsequent
`spawn pi`:

1. New idempotency key → idempotency-index miss → new-claim path.
2. `classify_claim` finds the released record; claim equality holds →
   `ExactRetry(record)`.
3. Storage maps BOTH `ExactRetry` and `Conflict` to
   `SpawnClaimConflict` (`core/src/storage/rusqlite.rs`
   do_append_spawn_claim_accepted) → pipeline returns
   `superseded/replacement_pending`.

Net: a pre-effect failure wedges the configured target forever; recovery is
reconfiguring the adapter environment with a new logicalTargetId (deployment
restart). Also `spawn-target-abandon` cannot recover it ("references an unknown
logical target") because the logical-target projection record is only created
at staging — a released-before-staging claim owns no logical target.

## Why this matters

The failure-phase table promises `released_no_external_effect` claims are
"reusable". For fresh claims the claim key is (domain, target, prior=None) and
the logical target id IS the command id, so no distinct new claim can ever
reuse the key. Combined with the adapter pinning
`deploymentTarget.logicalTargetId` (supervisor rejects mismatched claims),
re-usability is unreachable for fresh spawns on configured targets.

## Options (design decision, not a patch)

- (a) Allow a released fresh claim's exact command id to open a NEW claim
  attempt (disposition-superseding append; distinct idempotency key).
- (b) Let configured managed targets mint a fresh logical target per attempt
  (adapter matches by projectContextRef only; drop the logicalTargetId pin or
  treat it as the INITIAL target).
- (c) Let target abandonment also release a released-before-staging claim key
  (abandon by claim, not by logical target record).

Needs a feature-level decision + adversarial review before release; this is
v1-operational-risk material for any operator whose first spawn fails.
