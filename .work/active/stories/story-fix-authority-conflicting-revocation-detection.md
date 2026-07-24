---
id: story-fix-authority-conflicting-revocation-detection
kind: story
stage: done
tags: [security, protocol, foundation]
parent: feature-v0-core-authority
depends_on: []
release_binding: v0.1.0
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Story: Conflicting same-generation revocations must be CorruptLog, not silent no-op

## Source
Deep review of `feature-v0-core-authority` (Phase 2 adversarial, cross-model `openai-codex/gpt-5.6-sol`). Phase 1 also flagged the broader conflicting-duplicate discipline.

## Finding
`AuthorityRegistry::observe_revocation` (`core/src/authority/registry.rs:133-145`) treats a second revocation with the SAME `revocation_generation` as an exact redelivery (`Ok(())`) WITHOUT comparing the other fields (revocation_policy, revoked_at, revoked_by, reason, audit_id). Two revocations with the same `(grant_id, generation)` but different policy/timestamp/actor are silently collapsed — whichever arrived first wins.

This contradicts the rev3-review finding 1 guarantee (recorded in `feature-v0-core-authority.md:580`): "conflicting duplicate (same key, different content) = `CorruptLog` (mirrors `SessionRegistry`); exact redelivery = no-op." The spawn-tail's `insert_consistent` helper DOES compare content correctly; the registry's revocation path does not — an internal inconsistency.

## Impact
The accepted-operation policy (`Continue`/`Cancel`/`RequireReauthorization`) is security-relevant. A conflicting revocation event silently overwriting (or being silently ignored vs.) the first is a Fail-Fast violation in a committed v0.1.0 behavior. Not a direct privilege escalation (you cannot un-revoke), but a corruption-detection gap: a tampered or duplicated-with-different-content revocation is accepted instead of rejected.

## Fix
1. In `observe_revocation`, when `existing_generation == revocation_generation`, compare the incoming revocation's meaningful fields (policy, revoked_at, revoked_by, reason) against the stored record's. If identical → `Ok(())` (exact redelivery). If different → `Err(AuthorityError::CorruptLog(...))`.
2. Add a test: revoke a grant at generation 1 with policy `Continue`, then re-observe a revocation at generation 1 with policy `Cancel` → `CorruptLog`. And: re-observe the identical revocation → `Ok(())`.

## Acceptance Criteria
- [ ] A second revocation with the same generation but different content returns `CorruptLog`
- [ ] An exact-duplicate revocation (same generation, same content) is a no-op `Ok(())`
- [ ] Test covers both the conflict and the exact-redelivery cases

## Notes
- Mirrors the `insert_consistent` discipline already used by the spawn-tail (`core/src/authority/spawn_tail.rs`).
- Small, localized fix in `observe_revocation` + a test.

## Implementation notes
- Files changed: `core/src/authority/registry.rs`, `core/tests/authority_registry.rs`.
- Tests added: same-generation exact revocation redelivery remains idempotent, while a same-generation revocation with a different retained policy returns `CorruptLog` and leaves the original projection unchanged.
- Discrepancies from design: comparison is limited to the retained revocation fingerprint (`revoked_at` + `revocation_policy`), per the pinned minimal Option A; `GrantRecord` does not retain actor/reason/audit fields.
- Adjacent issues parked: none.
- Verification: `cargo build -p patchbay-core` and `cargo test -p patchbay-core --test authority_registry` pass (11 tests).

## Re-review (fast lane, 2026-07-14)
Verdict: Approve - blocker closed. `observe_revocation` now compares retained `revoked_at` + `revocation_policy` on same-generation revocations (registry.rs); identical → Ok, differing → CorruptLog. Test `same_generation_revocations_require_identical_retained_content` covers both conflict and exact-redelivery. Mirrors `insert_consistent` discipline. 174 tests, clippy clean. Blocker 2 from the feature deep review RESOLVED.

## Re-review #2 (adversarial, 2026-07-14)
Fresh-context re-review found the first fix INCOMPLETE: `observe_revocation` compared only `revoked_at` + `revocation_policy`, but `Revocation` also carries `revoked_by`, `reason`, `audit_id` — so two same-generation revocations differing only in actor/reason/audit still silently collapsed. GrantRecord retained none of those fields.

**Completed fix**: added `revoked_by: Option<ActorEndpointRef>`, `revocation_reason: String`, `revocation_audit_id: Option<EventId>` to `GrantRecord` (state.rs); populated in `observe_revocation`; same-generation redelivery now compares the COMPLETE fingerprint (generation + timestamp + policy + actor + reason + audit_id) — identical → Ok, any difference → CorruptLog. Both grant-construction sites initialize the new fields to None/empty. Test extended to cover differing-actor and differing-reason conflicts (in addition to the policy conflict + exact-redelivery cases). 174 tests green, clippy clean. Blocker 2 now GENUINELY closed.
