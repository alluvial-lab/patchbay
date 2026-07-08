---
id: story-formal-model-realignment-adjacency
kind: story
stage: implementing
tags: [verification, protocol, foundation]
parent: feature-formal-model-realignment
depends_on: [story-formal-model-realignment-traceability]
created: 2026-07-08
updated: 2026-07-08
gate_origin: null
release_binding: null
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
