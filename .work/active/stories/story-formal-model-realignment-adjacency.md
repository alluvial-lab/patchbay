---
id: story-formal-model-realignment-adjacency
kind: story
stage: done
tags: [verification, protocol, foundation]
parent: feature-formal-model-realignment
depends_on: [story-formal-model-realignment-traceability]
created: 2026-07-08
updated: 2026-07-08
gate_origin: null
release_binding: v0.1.0
---

# Story: V1 transition-adjacency strengthening (Unit CL)

Implements Unit CL from `feature-formal-model-realignment` — the trickiest unit, highest regression risk. Strengthens `command_lifecycle.qnt` to enforce the PROTOCOL transition adjacency and adds `NoAcceptedToCompleted`.

## Scope

Strengthen `commitTerminal` with an `allowedTransition` guard using the **exact** PROTOCOL table (`docs/PROTOCOL.md:116-132`):

```text
accepted  -> delivered | rejected | failed | expired | cancelled | superseded
delivered -> running | completed | rejected | failed | expired | cancelled | superseded
running   -> completed | failed | expired | cancelled | superseded
```

Add `advance(cmd, candidate)` action for non-terminal→non-terminal transitions (`accepted → delivered`, `delivered → running`) — **non-vacuity guarantee**: without it, `completed` is unreachable from `accepted` under the strengthened adjacency, making `NoAcceptedToCompleted` vacuously true.

New checked property (temporal, stutter-safe — checks transitions INTO `completed`, not the static state):

```quint
temporal no_accepted_to_completed =
  always(CMD_IDS.forall(cmd =>
    (state.get(cmd) != "completed" and next(state.get(cmd)) == "completed")
      .implies(state.get(cmd).in(Set("delivered", "running")))))
```

`@promotion` block (no `tier` field per Q1): `property: NoAcceptedToCompleted, status: promoted, backend: apalache-temporal, invocation: echo y | quint verify command_lifecycle.qnt --temporal no_accepted_to_completed --max-steps 10`.

## Acceptance Criteria

- [ ] `quint parse` + `quint compile` exit 0.
- [ ] **Regression gate (mandatory):** all 7 existing checked properties still pass (5 temporal: `echo y | quint verify --temporal <p> --max-steps 10`; 2 invariants: `quint verify --invariant <v> --max-steps 12`).
- [ ] `NoAcceptedToCompleted` passes (`echo y | quint verify --temporal no_accepted_to_completed --max-steps 10`, exit 0).
- [ ] **Mutation test:** breaking `allowedTransition` to permit `accepted → completed` causes `NoAcceptedToCompleted` to fail (genuine-checking proof — the invariant must not re-use `allowedTransition`).
- [ ] **Non-vacuity:** a reachability witness (`run`) confirms `completed` is reachable from `accepted` via the `advance` action (B3 — the mutation test alone does not prove non-vacuity).
- [ ] `command_lifecycle.emitted.tla` regenerated and committed.
- [ ] `@promotion` block present (no `tier` field); `check-models.mjs` exits 0.
- [ ] VERIFICATION.md updated: `NoAcceptedToCompleted` added to checked-model list; transition-adjacency stated-normative bullet narrowed (no-`accepted → completed` now checked-model; full adjacency graph + read fast-path remain stated-normative — I5).

## Key files

- Edit: `specs/seed/command_lifecycle.qnt`
- Regenerate: `specs/seed/command_lifecycle.emitted.tla`
- Edit: `docs/VERIFICATION.md`
- Design reference: `.work/active/features/feature-formal-model-realignment.md` Unit CL

## Implementation notes

- Files changed: `specs/seed/command_lifecycle.qnt`, `specs/seed/command_lifecycle.emitted.tla`, `docs/VERIFICATION.md`, `contracts/scripts/check-vectors.mjs`.
- Tests added: no separate test files; added the `NoAcceptedToCompleted` promoted temporal property and `completeViaAdvanceWitness` reachability driver in `command_lifecycle.qnt`.
- Discrepancies / mechanical deviations from design: used `candidateState` instead of `to` in `allowedTransition` because `to` is a Quint built-in name; pinned the default lifecycle driver to `c1` as a symmetry reduction so `--max-steps 10` temporal checks remain tractable while retaining `CMD_IDS = 3` and id/key quantification; added an explicit `completeViaAdvanceWitness` run step to print the accepted → delivered → completed non-vacuity trace.
- Adjacent issues parked: none.

Verification output (final run after reverting the mutation):

```text
parse/compile:
quint parse command_lifecycle.qnt: exit 0
quint compile command_lifecycle.qnt: exit 0

Regression gate:
temporal terminal_finality: PASS exit=0 duration=98s — [ok] No violation found (96635ms).
temporal pre_append_terminal_choice: PASS exit=0 duration=259s — [ok] No violation found (258515ms).
temporal lsn_determines_terminal_winner: PASS exit=0 duration=257s — [ok] No violation found (256140ms).
temporal retry_reuses_id_and_key: PASS exit=0 duration=16s — [ok] No violation found (15247ms).
temporal retry_after_terminal_returns_existing: PASS exit=0 duration=103s — [ok] No violation found (102000ms).
invariant command_durability: PASS exit=0 duration=8s — [ok] No violation found (6477ms).
invariant boundary_dedup: PASS exit=0 duration=7s — [ok] No violation found (6045ms).

New property:
temporal no_accepted_to_completed: PASS exit=0 duration=107s — [ok] No violation found (106015ms).

Mutation test:
Temporarily added "completed" to the accepted branch of allowedTransition, then re-ran
`echo y | quint verify command_lifecycle.qnt --temporal no_accepted_to_completed --max-steps 10`.
Result: FAIL as expected, exit=1 duration=8s — [violation] Found an issue; counterexample showed c1 accepted -> completed at LSN 4. Mutation reverted.

Non-vacuity witness:
`quint run command_lifecycle.qnt --step completeViaAdvanceWitness --max-steps 2 --witnesses completed_reachable`
Result: exit 0; printed c1 accepted at State 0, c1 delivered at State 1, c1 completed at State 2; completed_reachable witnessed in 10000/10000 traces.

Traceability / vector checks:
node contracts/scripts/check-vectors.mjs: exit 0 — All vector checks passed.
node contracts/scripts/check-models.mjs: exit 0 — All model-promotion checks passed.

Generated artifact:
quint compile command_lifecycle.qnt --target tlaplus > command_lifecycle.emitted.tla: exit 0 (protobuf warning only).
```
