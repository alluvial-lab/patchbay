---
id: story-v0-core-acceptance-proptests
kind: story
stage: done
tags: [protocol, verification, foundation]
parent: feature-v0-core-acceptance
depends_on: [story-v0-core-acceptance-elicitation-slot]
created: 2026-07-12
updated: 2026-07-13
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

- [x] `terminal_state_rejects_further_transitions` passes.
- [x] `accepted_to_completed_is_rejected` passes.
- [x] `retry_returns_existing_no_double_apply` passes.
- [x] `first_terminal_wins_later_is_stale` passes.
- [x] `replay_reconstructs_identical_index` passes.
- [x] Mutation tests prove non-vacuity for each injectible named bug; replay determinism's structural exception is documented rather than represented by a fake mutant.

## Design reference

See `feature-v0-core-acceptance.md` § "Implementation Units" → "Unit 6".

## Implementation notes

- Files changed: `core/tests/acceptance_proptest.rs`.
- Tests added: five 100-case proptests for TerminalFinality, NoAcceptedToCompleted, BoundaryDedup, first-durable-terminal-wins, and replay determinism; four mutation-discipline tests for terminal overwrite, the forbidden accepted→completed edge, dedup double-apply, and last-terminal-wins.
- Mutation discipline: the transition properties share their production oracle with explicit buggy appliers, and BoundaryDedup runs the same pipeline-level oracle against an always-append `Storage` adapter. Replay determinism has no clean injected mutant because `CommandIndex` replay is a pure fold with no clock, randomness, iteration-order output, or injectable choice; changing events in a storage wrapper would change the input rather than inject nondeterminism, so no fake mutation test was added.
- Verification: `CARGO_HOME=/tmp/cargo-home cargo build -p patchbay-core`; `CARGO_HOME=/tmp/cargo-home cargo test -p patchbay-core`; `CARGO_HOME=/tmp/cargo-home cargo clippy -p patchbay-core --all-targets -- -D warnings`; `CARGO_HOME=/tmp/cargo-home cargo fmt -p patchbay-core -- --check` all pass.
- Discrepancies from design: the suite adds explicit non-vacuity mutants for NoAcceptedToCompleted and first-durable-terminal-wins beyond the two concrete mutants named in the story; the replay-mutation requirement uses the documented structural escape hatch.
- Adjacent issues parked: none.
- Dispatch: direct-read only; the integration surface was limited to the acceptance modules, storage port, and existing test patterns named by the story.
