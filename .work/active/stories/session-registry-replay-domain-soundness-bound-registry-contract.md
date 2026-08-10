---
id: session-registry-replay-domain-soundness-bound-registry-contract
kind: story
stage: implementing
tags: [protocol, foundation]
parent: session-registry-replay-domain-soundness
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-10
updated: 2026-08-10
---

# Bound session-registry contract

## Checkpoint

Make each `SessionRegistry` an explicitly authority-domain-bound projection,
then give its owned event fold exact identity-plus-payload redelivery semantics.
Route report ingestion and target resolution through that domain boundary, and
fold every successful append into the supplied hot projection before returning.

## Acceptance evidence

- Construction rejects an empty authority domain; session-event/security-clamp
  folds, report lookup/ingest, adapter-stale derivation, and runtime-session
  resolution reject a different domain before mutation or append.
- An exact projection-owned `(authority_domain_id, LSN, StoredEventPayload)`
  redelivery is inert. Reusing the same event identity with different kind or
  bytes, replaying an unseen older owned event, or registering the same live
  slot at a new LSN is corrupt history rather than a content-blind no-op.
- The old per-record `event_lsn <= last_authoritative_lsn` skips and partial
  tombstone/registration duplicate checks are removed; one event ledger owns
  replay equality for registrations, generation bumps, state/metadata changes,
  and security-lockdown clamps.
- Every single- and multi-delta report append uses the same append-then-fold
  helper. A returned success has warmed the supplied projection; a failed
  append has not, and a fold failure after commit forces rebuild-before-reuse.
- The duplicate inherent session resolver is removed; the acceptance-owned
  `TargetResolver` is the one runtime-session resolution boundary.
- The production `CoreDecisionGate` and rebuild-before/after adapter-service
  path remain intact and independently responsible for cross-request
  serialization; the core writer makes no global multi-projection concurrency
  claim.

## Ordering constraints

This checkpoint is first. It establishes the constructor, lookup, replay, and
writer contract consumed by the integration/property evidence checkpoint. It
is semantically independent of `replay-integrity-prefix-discipline`: that
feature validates newly read complete prefixes, while this checkpoint validates
exact redelivery at the session projection boundary.
