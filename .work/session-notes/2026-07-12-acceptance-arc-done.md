# Session Note — Acceptance Arc: Command Lifecycle + Acceptance Pipeline

## Context

Continued from the storage arc session. Started with `feature-v0-core-persistence` done (4/4 stories, 52 tests) and `feature-v0-core-acceptance` at `stage: drafting`. Ended with both persistence and acceptance done (2/4 core features), 111 tests green, and authority + sessions unblocked.

The session ran as an autopilot goal (`autopilot epic-v0-core`, tier: highest / `openai-codex/gpt-5.6-sol`), with interactive design checkpoints for the feature-design phase.

## What happened

### 1. Feature design: 5 decisions resolved interactively

`feature-v0-core-acceptance` — the operation acceptance pipeline and command lifecycle state machine. 5 design decisions, each unpacked before resolving:

| Q | Decision | Key reason |
|---|---|---|
| Q1 command state model | hybrid (event log SSOT + in-memory index hot path) | matches how persistence was built; recovery returns raw materials, domain layer applies them |
| Q1b event payload shape | uniform `CommandTransition` events; accept reuses `OPERATION` | deriving transitions from Observations breaks on core-sourced transitions (delivered, expired, cancelled); late candidates are audit-only `stale_event` Observations — first-durable-terminal-wins is structural (first terminal `COMMAND_TRANSITION` in LSN order) |
| Q2 port shapes | two async RPITIT ports: `GrantCheck` + `TargetResolver` | consistency with `Storage`; async > sync for forward-compat even though authority state is in-memory |
| Q3 observation ingestion | acceptance owns ingestion + state reflection; streaming/subscription is protocol-seam | separate method on the acceptance service (distinct adapter→core ingress), not a separate trait |
| Q4 elicitation scope | A2: acceptance accepts response Ops as plain operations; Elicitation-slot terminalization is an independent log consumer | decouples acceptance from `ElicitationState`; first-answer-wins is structurally identical to the command terminal race |

Added `CommandTransition` proto message + `STORED_EVENT_KIND_COMMAND_TRANSITION = 8` variant (Generated Contracts). 6 child stories with declared `depends_on` chains.

### 2. Implementation: 6 stories, each deep-reviewed

| Story | Deep review | Key catches |
|---|---|---|
| state-machine | 1 round | Mutation-vacuous `from_state` test (used a disallowed transition, so removing the guard wouldn't let it through); missing `command_id` identity check |
| pipeline | 1 round (Approve) | Clean on first pass — pre-acceptance-failure-leaves-no-trace and BoundaryDedup held |
| observation-ingestion | 1 round | TOCTOU race (`CommandStateLookup` + separate `append`); resolved via `AlreadyTerminal` variant + replay-fold skip |
| replay | 1 round | Snapshot discriminator gap (command-only snapshot in the global authority-domain slot hides sibling projections); resolved via snapshot deferral |
| elicitation-slot | 1 round | Non-response command could terminalize (fixed: fold OPERATION events to confirm response OperationKind); A2 coupling (fixed: local `operation_state_is_terminal`); every-terminal→Answered (fixed: only Completed→Answered) |
| proptests | 1 round | Non-representative race shape (second transition used first terminal as from_state, not the pre-terminal state); mutation test didn't prove double persistence (fixed: directly read mutant log) |

Stories 3 and 4 ran in **parallel** (non-overlapping write paths: `observation.rs` vs `index.rs`+`replay.rs`), reconciled cleanly in `mod.rs`.

### 3. Feature-level deep review: 5 blockers

After all 6 stories reached `done`, the feature-level deep review found 5 blockers:

- **Blocker 1** (real bug): retry returns hardcoded `Accepted`, not the existing command's state. Fixed: `submit` takes a `CommandStateLookup` parameter; the `Duplicate` path looks up the existing state.
- **Blocker 5** (real design gap): A2 correlations don't flow end-to-end. Fixed: `CommandStateLookup` returns `CommandSnapshot` (state + correlations); `ingest_observation` merges the command's correlations into the derived transition.
- **Blockers 2-4** (forward-dependencies): target binding shape, authenticated principal, observation source-auth — the ports are the seams; authority/sessions/protocol-seam fill them in. Documented as reserved seams.

### 4. Re-review after blocker fixes (the gap you caught)

I fixed blockers 1+5 in-place on already-`done` stories but **did not re-review**. You asked "did we re-review the stories that went through review?" — the answer was no. The re-review found one remaining blocker: the `Duplicate` path's `unwrap_or(OperationState::Accepted)` fallback silently reproduced the original bug when the lookup returned `None`. Fixed: fail fast with `AcceptanceError::CorruptRecord`.

## Key lessons

1. **Fixing feature-level blockers modifies already-`done` stories, and those modifications need re-review.** A fix can introduce a regression or a new issue — here, the `None` fallback reproduced the exact bug the fix was supposed to close. The persistence arc's 4-round convergence loop was the same pattern at the story level. I should have re-reviewed immediately after the fixes, not waited for the operator to ask. The cost of a re-review is low; the cost of shipping a regression in a safety-claiming formal-model feature is high.

2. **The `[verification]` deep-lane convergence loop is the mechanism that makes evidence honest.** Every story in this arc went through at least one deep-review round that found a real issue — vacuous tests, TOCTOU races, snapshot discriminator gaps, non-representative race shapes. None of these would have been caught by a fast-lane "confirm green tests, advance" review. The CONVENTIONS.md rule routing `[verification]` stories to the deep lane is load-bearing.

3. **Parallel fan-out produces integration seams that need explicit reconciliation.** Stories 3 and 4 ran in parallel and both defined pieces the other needed (`CommandStateLookup` trait in story 3, `CommandIndex` impl in story 4). The integration gap (CommandIndex didn't implement CommandStateLookup) surfaced at the feature-level review, not during the parallel run. The orchestrator should flag cross-story trait seams explicitly when fanning out.

4. **"Forward dependency" is not "blocker."** The feature-level review flagged the event registry, target binding, and authenticated-principal concerns as blockers, but these are the sibling features' design decisions — the ports are the seams. Calling them persistence/acceptance blockers would block the foundation on downstream work. The honest framing: reserved seam, documented in the extension classification.

5. **Disk-full is a recurring sandbox issue.** The `open_in_memory()` test helper leaks one temp file per call (documented), and the cargo registry cache fills `/tmp`. Mid-session test failures from "No space left on device" look like code failures but are environment. Clean `/tmp` before re-running.

## Current state

```
epic-v0-1-0-implementation  (drafting)
├── epic-v0-core  (implementing)
│   ├── feature-v0-core-persistence   ✅ done (4/4 stories)
│   ├── feature-v0-core-acceptance    ✅ done (6/6 stories)
│   ├── feature-v0-core-authority     (drafting — UNBLOCKED)
│   └── feature-v0-core-sessions      (drafting — UNBLOCKED)
├── feature-v0-protocol-seam  (drafting, blocked on core)
├── feature-v0-pi-adapter      (drafting, blocked on core)
├── feature-v0-web-server      (drafting, blocked on seam)
├── feature-v0-web-cockpit     (drafting, blocked on web-server)
└── feature-v0-cli             (drafting, blocked on seam)
```

111 tests green across `patchbay-core`. Clippy + rustfmt clean.

## Next

Authority and sessions are both unblocked and ready for `feature-design`. Each needs its own design pass to define its port and how it implements the acceptance seams (`GrantCheck` for authority, `TargetResolver` for sessions). They can proceed in parallel.

The critical path to the phone-usable walking skeleton is: core (authority/sessions) → protocol-seam → web-server → web-cockpit, with the Pi adapter parallel.
