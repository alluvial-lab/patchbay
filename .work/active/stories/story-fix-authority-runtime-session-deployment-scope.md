---
id: story-fix-authority-runtime-session-deployment-scope
kind: story
stage: done
tags: [security, protocol, foundation]
parent: feature-v0-core-authority
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-14
updated: 2026-07-14
---

# Story: RuntimeSession scope match must include deployment_scope

## Source
Deep review of `feature-v0-core-authority` (Phase 2 adversarial + Phase 1 completeness, cross-model `openai-codex/gpt-5.6-sol`). Both reviewers independently flagged this.

## Finding
`target_scope_matches` for `RuntimeSession` (`core/src/authority/state.rs`, `same_session`) compares adapter_id + runtime_session_id + session_generation but OMITS `deployment_scope`. The feature design (Unit 1, `feature-v0-core-authority.md:165`) explicitly pins the exact-tuple as "adapter+deployment+runtime+generation", and this is "committed v0.1.0 behavior" (the full grant-matching matrix, rev2 finding #3 pinned).

The existing test (`core/tests/authority_registry.rs:348-385`) blesses the omission: both the grant scope (`runtime_scope(...)`) and the requested scope use an empty/default `deployment_scope`, so the match passes without exercising the field.

## Impact
A grant for `(pi, machine-a, session-1, gen-7)` would authorize a request targeting `(pi, machine-b, session-1, gen-7)` if such a live session existed. Deployment scope is part of stable session identity (`PROTOCOL.md`, `VERIFICATION.md:177`). This is a real authority-bypass in the scope-containment matrix — a committed v0.1.0 safety claim.

## Fix
1. `same_session` in `core/src/authority/state.rs`: require non-empty `deployment_scope` on BOTH grant and requested scopes, and compare them, as part of the exact-tuple match. Reject (return false) if either is empty (Fail Fast at the boundary; `Unspecified`-style defense).
2. Update `runtime_scope(...)` test helper and the matrix test in `core/tests/authority_registry.rs` to set a real `deployment_scope` and add a negative case: same adapter+runtime+generation but DIFFERENT deployment_scope must NOT match.

## Acceptance Criteria
- [ ] `same_session` compares `deployment_scope` as part of the exact-tuple
- [ ] A runtime-session grant with deployment_scope "machine-a" does NOT authorize a request with deployment_scope "machine-b"
- [ ] Empty deployment_scope on a runtime-session grant does not match (Fail Fast)
- [ ] Existing matrix tests updated; cross-deployment negative test added

## Notes
- This is a code bug contradicting a pinned design decision, not a deferral.
- The fix is small and localized to `same_session` + its tests.

## Implementation notes
- Files changed: `core/src/authority/state.rs`, `core/tests/authority_registry.rs`.
- Tests added: the scope-containment matrix now rejects cross-deployment RuntimeSession requests and grants with empty deployment scope.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Verification: `cargo build -p patchbay-core` and `cargo test -p patchbay-core --test authority_registry` pass (10 tests).

## Re-review (fast lane, 2026-07-14)
Verdict: Approve - blocker closed. `same_session` now compares non-empty `deployment_scope` (state.rs:157-159). Cross-deployment negative test (`machine-b`, asserts `!match`) and empty-deployment rejection both present. The matrix test's `requested_session` updated to `deployment_scope: "machine-a"` so the positive case stays green. 174 tests, clippy clean. Blocker 1 from the feature deep review RESOLVED.
