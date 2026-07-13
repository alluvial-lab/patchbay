---
id: story-v0-core-acceptance-proptests
kind: story
stage: implementing
tags: [protocol, verification, foundation]
parent: feature-v0-core-acceptance
depends_on: [story-v0-core-acceptance-elicitation-slot]
created: 2026-07-12
updated: 2026-07-12
gate_origin: null
release_binding: null
---

# Story: Property tests for acceptance invariants

## Scope

Write proptest-based property tests validating the acceptance invariants: the three promoted properties (TerminalFinality, NoAcceptedToCompleted, BoundaryDedup), first-durable-terminal-wins, and replay determinism. Mutation discipline: each property catches its named bug.

## Units

- `core/tests/acceptance_proptest.rs` — proptest suite

## Key properties

- **TerminalFinality** (promoted): terminal states reject transitions out.
- **NoAcceptedToCompleted** (promoted): accepted→completed is rejected.
- **BoundaryDedup** (promoted): retry returns existing, no double-apply.
- **First-durable-terminal-wins** (stated-normative): first terminal in LSN order wins; later candidates are stale.
- **Replay determinism** (stated-normative, end-to-end): same events → same index.

## Acceptance criteria

- [ ] `terminal_state_rejects_further_transitions` passes.
- [ ] `accepted_to_completed_is_rejected` passes.
- [ ] `retry_returns_existing_no_double_apply` passes.
- [ ] `first_terminal_wins_later_is_stale` passes.
- [ ] `replay_reconstructs_identical_index` passes.
- [ ] Mutation tests prove non-vacuity for each property (inject the named bug, assert the property FAILS).

## Design reference

See `feature-v0-core-acceptance.md` § "Implementation Units" → "Unit 6".
