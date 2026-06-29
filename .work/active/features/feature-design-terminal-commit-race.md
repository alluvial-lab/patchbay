---
id: feature-design-terminal-commit-race
kind: feature
stage: implementing
tags: [protocol, verification]
parent: epic-foundation-hardening
depends_on: [feature-command-state-ssot]
created: 2026-06-28
updated: 2026-06-29
gate_origin: null
release_binding: null
---

# Design: command terminal-commit race resolution

The "first durable terminal commit wins" race rule is currently committed v0 behavior in `docs/PROTOCOL.md` (cancellation/expiration/supersession race semantics), but it was decided inside a prose consolidation feature (`feature-command-state-ssot`) without a design pass over the alternatives. This feature reopens it as a deliberate design decision.

## What is under design review

The rule as currently committed:

> First durable terminal commit wins. The core assigns a total order to accepted state-transition events in the durable event log; the earliest committed valid terminal transition becomes authoritative. If two terminal candidates are truly concurrent before persistence, models may treat the winner as nondeterministic, but implementations must persist one total order and expose the chosen terminal state consistently in snapshots and conformance traces. Later conflicting events are audit/reconciliation events, not state rewrites.

## Alternatives to evaluate

- **First durable terminal commit wins** (current) — simplest; relies on LSN total order; late events are audit-only.
- **Last durable commit wins** — allows later events to override; simpler reconciliation but can rewrite history the operator saw.
- **Priority-ordered resolution** — e.g. operator cancellation always wins over adapter completion, or vice versa; more predictable per-stakeholder but encodes priority policy in the core.
- **Explicit conflict surface** — surface concurrent terminal candidates to the operator as a distinct state rather than silently resolving.
- **Hybrid** — first-commit-wins for most cases, priority override for specific command kinds (e.g. safety-critical cancellation).

## Design questions to resolve

- Which failure modes actually produce truly concurrent terminal candidates in v0's single-writer model? (If none, the rule is theoretical and the choice is low-stakes.)
- Does the rule interact correctly with idempotent retry — i.e. can a retry land after a terminal commit and create a false "later event"?
- How does this interact with revocation policy on already-accepted commands?
- Does the formal model need to expose the nondeterministic case, or is v0's single-writer guarantee enough to make it deterministic in practice?
- Does the choice affect the generated contract or conformance vectors materially?

## Relationship to committed docs

The rule is currently committed in `docs/PROTOCOL.md` and referenced in `docs/VERIFICATION.md` (operator intent delivery, idempotent retry). A design pass either ratifies the rule as-is (and the note is removed) or revises it (and the docs roll forward). The rule stays as committed v0 behavior until the design pass concludes.

## Acceptance criteria

- The race resolution rule is a deliberate design choice, not a prose artifact.
- The chosen rule is documented with its rationale and the alternatives considered.
- `docs/VERIFICATION.md` model obligations are consistent with the chosen rule.
- Conformance vectors for the terminal-commit race are identified (even if deferred for implementation).

## Design decisions

- **Terminal race rule**: Use first durable terminal commit wins for v0 command state. The core's durable event log order, not terminal-state priority, chooses the authoritative outcome.
- **Operator cancellation after completion**: Treat cancellation as a request submitted into a moving system. If the command already reached a durable terminal state such as `completed`, the cancel request does not rewrite that state; UX and audit history should explain that the target state changed before cancellation landed.
- **Late conflicting terminal candidates**: Record them as audit/reconciliation events, not `CommandState` rewrites and not a new durable `conflicted` state.
- **Priority overrides**: Do not add global priority ordering such as `cancelled > completed` in v0. Reserve command-kind-specific terminal-resolution policy as a future seam for safety-critical commands, but keep the generic command lifecycle simple.
- **Idempotency relationship**: This feature only defines ordering for already-accepted command-state transitions. Duplicate retry with the same command id or idempotency key returns the existing command record, including an existing terminal state. Ambiguous external execution semantics remain scoped to `feature-idempotency-ambiguous-execution`.
- **Formal model shape**: Model nondeterminism only before durable append. Once a terminal transition has an assigned log sequence number, replay, snapshots, conformance traces, and UI reconciliation are deterministic.

## Architectural choice

Ratify **first durable terminal commit wins** as the v0 command lifecycle rule.

The rejected alternatives were:

1. **Last durable terminal commit wins** — rejected because it rewrites history the operator may already have seen, makes terminal states non-terminal, and can hide real external side effects behind a later policy event.
2. **Global priority-ordered resolution** — rejected because priority is command-kind policy, not generic command lifecycle ordering. It would encode stakeholder-specific semantics in the core before v0 has enough command kinds to justify them.
3. **A durable conflict state** — rejected because conflict is an audit/timeline fact, not a command outcome. V0's single-writer log makes true post-persistence conflict impossible; adding a protocol state would complicate models, UI, and generated contracts for a rare pre-persistence race.
4. **Hybrid override policy** — deferred as an extension seam. Future safety-critical command kinds may define explicit abort/fencing semantics, but the generic v0 lifecycle should not carry that complexity.

The chosen rule aligns with Patchbay's single-authoritative-core commitment: one writer appends state-transition events to one durable log per authority domain, and the earliest valid terminal transition in that order becomes the command's authoritative terminal state.

## Implementation Units

### Unit 1: Ratify terminal race semantics in protocol prose

**File**: `docs/PROTOCOL.md`

```text
ApplyTerminalCandidate(command_id, candidate_terminal_state, candidate_event):
  current = command_state(command_id)
  if current is non_terminal and candidate transition is valid:
    append state-transition event with next LSN
    set CommandState(command_id) = candidate_terminal_state
  else if current is terminal:
    append or emit audit/reconciliation record for stale_event / late_terminal_candidate
    leave CommandState(command_id) unchanged
```

**Implementation Notes**:
- Remove the "Under design review" note from the cancellation/expiration/supersession section once this design is implemented.
- Preserve the existing terminal-state finality rule.
- Clarify that cancellation, expiration, supersession, adapter completion, adapter failure, and adapter rejection are terminal candidates competing for the same command's first durable terminal transition.
- Clarify that a cancellation submitted after completion may have its own submission/audit record, but it does not mutate the completed command.

**Acceptance Criteria**:
- [ ] `docs/PROTOCOL.md` states first durable terminal commit wins as ratified v0 behavior, not under design review.
- [ ] Late terminal candidates are described as audit/reconciliation events, not state rewrites.
- [ ] The prose distinguishes command state from operator experience of submitting a too-late cancel request.

---

### Unit 2: Add verification obligations for terminal races

**File**: `docs/VERIFICATION.md`

```text
Invariant TerminalFinality:
  once CommandState[c] in TerminalStates,
  all later events for c leave CommandState[c] unchanged

Invariant LsnDeterminesTerminalWinner:
  for any command c with terminal candidates T,
  CommandState[c] equals the valid terminal candidate with the lowest committed LSN
```

**Implementation Notes**:
- Add explicit properties under operator intent delivery and/or idempotent retry.
- The model may allow nondeterministic choice among simultaneous pre-append terminal candidates, but the selected append order must determine all later snapshots and replay.
- State that late terminal candidates produce audit/reconciliation records and do not mutate the command view.

**Acceptance Criteria**:
- [ ] `docs/VERIFICATION.md` includes terminal finality and LSN winner obligations.
- [ ] Replay/snapshot obligations mention that terminal outcomes are deterministic after durable append.
- [ ] Idempotent retry obligations mention retry-after-terminal returns the existing terminal command record.

---

### Unit 3: Preserve operator trust in UX language

**File**: `docs/UX.md`

```text
Completed; cancellation arrived after completion.
Cancelled before completion.
Expired before adapter completion.
```

**Implementation Notes**:
- Add a short presentation requirement that the UI can explain late cancellation/expiration/supersession without rewriting command state.
- Keep this as a display/timeline obligation, not a new protocol state.

**Acceptance Criteria**:
- [ ] `docs/UX.md` makes clear that downstream UX handles the operator's experience of too-late cancellation.
- [ ] UX text does not introduce local authority over durable command state.

---

### Unit 4: Identify conformance vectors for the future contract/vector feature

**File**: `contracts/vectors/command_lifecycle/terminal_commit_race/*.json` (created later by `feature-protocol-idl-and-conformance` if the contracts tree does not exist yet)

```json
{
  "name": "completion_wins_before_cancel",
  "initial_state": "running",
  "events": [
    { "lsn": 10, "candidate": "completed" },
    { "lsn": 11, "candidate": "cancelled" }
  ],
  "expected_command_state": "completed",
  "expected_late_events": ["cancelled"]
}
```

**Implementation Notes**:
- Required vector cases:
  1. completion wins before cancellation;
  2. cancellation wins before completion;
  3. expiration wins before late completion;
  4. duplicate retry after terminal returns the existing terminal state;
  5. late conflicting terminal event becomes audit/reconciliation only;
  6. replay from the same committed prefix reconstructs the same terminal state.
- If vector infrastructure is not present during implementation, record these cases in the relevant protocol/conformance feature rather than inventing a local ad hoc layout.

**Acceptance Criteria**:
- [ ] The implementation pass records the vector cases in `docs/VERIFICATION.md`, `feature-protocol-idl-and-conformance.md`, or the actual vector tree if it exists.

## Implementation Order

1. Update `docs/PROTOCOL.md` to ratify the rule and remove the under-design-review marker.
2. Update `docs/VERIFICATION.md` with terminal-finality and LSN-winner obligations.
3. Update `docs/UX.md` with the too-late cancellation presentation obligation.
4. Record the conformance-vector cases in the existing future contract/vector work if vector files do not exist yet.

No child stories are spawned. This is a single-stride documentation/verification design with tight cohesion; stories would add overhead rather than useful parallelism.

## Testing

There is no implementation code yet. Verification for this design is by document consistency and future conformance vectors:

- confirm `docs/PROTOCOL.md`, `docs/VERIFICATION.md`, and `docs/UX.md` describe the same terminal-resolution semantics;
- confirm no new durable `CommandState` is introduced for conflict;
- confirm future vector cases cover both cancellation-before-completion and completion-before-cancellation orders;
- confirm retry-after-terminal remains an idempotency case, not a terminal-race override.

## Risks

- **UX confusion**: Operators may expect cancel to retroactively dominate. Mitigation: make the command timeline explain "completed before cancellation arrived" rather than hiding the late cancel.
- **Future safety-critical commands**: Some command kinds may eventually require stronger abort/fencing semantics. Mitigation: reserve command-kind-specific terminal-resolution policy as an explicit extension seam rather than changing the generic v0 lifecycle now.
- **Coupling to provisional LSN model**: This design relies on durable log order. The current LSN model is under provisional review, but any replacement still needs a single authoritative append order for v0's single-writer core.
