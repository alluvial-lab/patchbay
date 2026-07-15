---
id: backlog-authority-payload-actor-in-descendant-issuance
kind: feature
stage: backlog
tags: [security, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Backlog: Descendant-grant issuance must not trust self-asserted Operation.sender for the subject actor

## Source
Deep review of `feature-v0-core-authority` (Phase 1 + Phase 2, both reviewers independently). This is the top-flagged finding.

## Finding
`SpawnDescendantTail` (`core/src/authority/spawn_tail.rs`) derives the descendant grant's `subject_actor_id` from `Operation.sender.actor_id` — a self-asserted payload field. The spawn itself is authorized against the verified `IssuerContext` (R2), but the descendant grant's *subject* (who gets authority over the spawned session) is taken from the untrusted payload. An actor A with a valid spawn grant can submit a spawn whose payload claims `sender = B`, and the reactor issues the descendant grant to B. An empty payload actor makes a completed spawn fail descendant issuance.

This is the same compound-issuer concern R2 resolved for `GrantCheck` (verified `IssuerContext`, not `Operation.sender`), but the spawn-tail was not brought under the same discipline because durable acceptance metadata doesn't exist yet (`backlog-authority-durable-acceptance-metadata`).

## Scope vs. the design
The design defers durable acceptance metadata (verified actor/endpoint/authorizing-grant surviving replay) to `backlog-authority-durable-acceptance-metadata` and marks `spawning_grant_id` as may-be-None. It does NOT explicitly say "the descendant subject may be derived from a self-asserted payload field." The rev3 R3/provenance note speaks to `spawning_grant_id` optionality, not subject-actor trust. So this is a gap in the design's deferral honesty, surfaced by review.

## Direction
When durable acceptance metadata lands (the same work as `backlog-authority-durable-acceptance-metadata`), the spawn-tail must consume the verified actor from that metadata, not `Operation.sender`. Until then:
- Option A: the spawn-tail treats a payload `sender` that doesn't match the verified issuer as `CorruptLog` (but it has no access to the verified issuer — it's a pure log fold). Not feasible without the metadata.
- Option B: document this as an explicit known limitation of the component-complete (not-live) v0.1.0 authority — the descendant-grant subject is payload-derived because durable verified identity isn't available to the fold yet. Add a code comment at `spawn_tail.rs` subject derivation noting the trust assumption and the backlog link.
- Option C (preferred long-term): couple with `backlog-authority-durable-acceptance-metadata` — once the verified actor is durable on the command record, the spawn-tail reads it.

## Priority
Not blocking v0.1.0 component-complete authority (no live path; tests inject grants), but MUST be resolved before the live spawn path is exercised. Couples with `backlog-authority-durable-acceptance-metadata` and `backlog-authority-live-composition`. Becomes blocking when live composition lands.
