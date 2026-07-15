---
id: story-v0-core-authority-spawn-tail
kind: story
stage: review
tags: [security, protocol, foundation]
parent: feature-v0-core-authority
depends_on: [story-v0-core-authority-registry, story-v0-core-authority-ingest, story-sessions-spawn-origin-field]
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Story: Descendant-grant-on-spawn log-tail reactor (order-independent, tested via replay)

## Scope
Implement Unit 4 of `feature-v0-core-authority` (revision 3): a pure fold that produces a descendant-grant issuance on observing a completed spawn + correlated `SessionRegistered.spawn_origin`. **Order-independent** (rev2 finding D). Exercised via replay/direct observe in tests — NO live consumer loop (rev2 finding E dropped).

## Units
- `core/src/authority/spawn_tail.rs` — `SpawnDescendantTail`, `DescendantGrantIssuance`

## Implementation
See `feature-v0-core-authority.md` Unit 4 for exact signatures. Key points:
- **Depends on `story-sessions-spawn-origin-field`** (the `SessionRegistered.spawn_origin` field).
- **Order-independent** (rev2 finding D): three separate maps — `spawn_ops` (command_id → SpawnOpInfo), `completed` (HashSet), `registrations` (spawn_origin command_id → RegistrationInfo). After ANY insertion, call `try_issue(command_id)`: issue iff spawn_ops.has(cid) && completed.has(cid) && registrations.has(cid) && !issued.has(cid). This handles all 6 arrival orders (e.g. SessionRegistered before Completed).
- `issued` = in-memory idempotency for the fold. Durable idempotency for tests = deterministic grant_id from `(authority_domain_id, spawn_command_id)`; the test harness calls `ingest_descendant_grant` with that ID (re-observe → same ID → no-op).
- The issuance's `spawned_session_scope` comes from the `SessionRegistered` event (NOT the spawn op's fleet target).
- `spawning_grant_id`: from the spawn op's authorization if available; may be `None` in v0.1.0 (documented — full durable acceptance-metadata is follow-on).
- NO live consumer loop, NO composition layer (rev2 finding E dropped). Pure fold; tests feed events directly.
- Read `core/src/acceptance/elicitation.rs` FIRST — `ElicitationSlotLayer` is the direct template (read-only log consumer).

## Acceptance Criteria
- [ ] A Spawn OPERATION + Completed transition + SessionRegistered(spawn_origin=that command) produces exactly one `DescendantGrantIssuance`, **regardless of arrival order** (test all 6 permutations)
- [ ] A spawn reaching a non-Completed terminal produces NO issuance
- [ ] A SessionRegistered without `spawn_origin` does NOT trigger an issuance
- [ ] Replay (re-observing events) does not produce duplicate issuances (idempotent via `issued` + deterministic grant_id)
- [ ] The issuance's allowed-kinds match `DESCENDANT_GRANT_ALLOWED_KINDS` exactly
- [ ] The issuance's `spawned_session_scope` comes from the `SessionRegistered` event

## Notes
- Depends on stories 1 (registry), 3 (ingest), AND the sessions prerequisite `story-sessions-spawn-origin-field`.
- Add tests in `core/tests/authority_spawn_tail.rs`. The 6-permutation order test is key (rev2 finding D).
- No composition layer — this story is the reactor only.

## Implementation notes
- Files changed: `core/src/authority/spawn_tail.rs`, `core/src/authority/mod.rs`, `core/tests/authority_spawn_tail.rs`.
- Tests added: six real arrival-order permutations; non-Completed terminals; absent `spawn_origin`; replay idempotency; conflicting duplicate rejection; event/message/tail domain isolation.
- Verification: `CARGO_HOME=/tmp/cargo-home cargo build -p patchbay-core`; `CARGO_HOME=/tmp/cargo-home cargo test -p patchbay-core --test authority_spawn_tail` (6 passed).
- Delivery mode: direct-read only; the integration surface and templates were explicit, and both bundled items intentionally share `authority/mod.rs` ownership.
- Discrepancies from design: generated `CommandTransition` has no `authority_domain_id`, so its decoded-domain equality cannot be checked; the fold validates its event-envelope domain and the tail's single-domain binding instead. Missing spawn sender actors are retained as designed, then fail fast if the three facts would otherwise issue a grant because `DescendantGrantIssuance.subject_actor_id` is required.
- Documented v0.1.0 gaps: `spawning_grant_id` and `audit_id` remain `None`; no live consumer/composition layer was added.
- Adjacent issues parked: none.

## rev3-review fixes (in-stride, 2026-07-13)
Design review #3 found 2 blockers in this unit; both are mechanical (protocol/pattern-pinned), resolved here:
- **Domain isolation (finding 1):** all three maps keyed by `(AuthorityDomainId, CommandId)`, NOT bare `CommandId` — events are domain-scoped; client command IDs aren't globally unique. Conflicting duplicate (same key, different content) = `CorruptLog` (mirrors `SessionRegistry`); exact redelivery = no-op.
- **Deterministic grant_id in issuance (finding 1):** computed inside a canonical helper `descendant_grant_id(domain, spawn_op)`, included as `DescendantGrantIssuance.descendant_grant_id` — NOT delegated to the caller. Re-observe → same id → no-op.
- **audit_id (finding 2):** `DescendantGrantIssuance.audit_id: Option<EventId>` = `None` in v0.1.0. The protocol requires a spawn-completion audit link (`DescendantGrant.audit_id` field 14), but the audit producer is deferred (R4). The descendant grant is **component-tested, not protocol-complete**. Documented gap (`backlog-authority-durable-acceptance-metadata`).
