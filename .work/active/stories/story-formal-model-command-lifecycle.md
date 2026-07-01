---
id: story-formal-model-command-lifecycle
kind: story
stage: implementing
tags: [verification, protocol, foundation]
parent: feature-formal-model-seed
depends_on: []
created: 2026-07-01
updated: 2026-07-01
gate_origin: null
release_binding: null
---

# Story: Author command_lifecycle.qnt (the fused terminal-race + dedup model)

This is the trickiest unit of `feature-formal-model-seed` — the fused `command_lifecycle.qnt` state machine modeling accepted-command durability, the terminal race (first-durable-terminal-commit-wins), and idempotency-boundary dedup in one model. It carries 7 of the seed's checked properties. Spawned as a separate story because the terminal-race nondeterminism + dedup-map fusion + the TLC temporal path is the highest-unknown surface, and a fresh implementation agent should absorb this one model rather than the whole feature design.

## Scope

- Author `specs/seed/command_lifecycle.qnt` per Unit 1 of the parent feature design.
- State: `state: str -> str` (CommandId -> CommandState), `idemKey: str -> str`, `appliedKeys: Set[str]`, `lsn: int`, `terminalLsn: str -> int`.
- Permissive actions: `init`, `commitTerminal`, `lateTerminalCandidate` (the no-op that TerminalFinality is checked against), `retry`, `step`.
- 7 checked properties with inline `@promotion` blocks: `CommandDurability`, `TerminalFinality`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner` (TLC temporal); `BoundaryDedup`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting` (Apalache invariant; `RetryAfterTerminalReturnsExisting` also has a TLC temporal form).
- Bounds: 3 command ids, 3 idempotency keys, `--max-steps 12`.

## Acceptance criteria

- [ ] `quint parse command_lifecycle.qnt` exits 0 (parse; catches the typed-action-parameter pitfall — use untyped params `action commitTerminal(cmd, candidate)`).
- [ ] `quint compile command_lifecycle.qnt` exits 0 (typecheck).
- [ ] `quint verify command_lifecycle.qnt --backend tlc --temporal terminal_finality` exits 0 (no counterexample; default workers per the liveness caveat).
- [ ] `quint verify command_lifecycle.qnt --invariant boundary_dedup --max-steps 12` exits 0 (Apalache).
- [ ] `quint verify command_lifecycle.qnt --backend tlc --temporal lsn_determines_terminal_winner` exits 0.
- [ ] `quint verify command_lifecycle.qnt --backend tlc --temporal pre_append_terminal_choice` exits 0.
- [ ] `command_lifecycle.emitted.tla` committed (via `quint compile --target tlaplus`); never hand-edited.
- [ ] All 7 `@promotion` blocks present and grep-able; each `invocation` field names the exact CLI + jar-path classpath.

## Implementation notes

- The genuine-checking discipline is load-bearing: the `lateTerminalCandidate` action must be a *permissive* no-op (it's *allowed* to happen), and `terminal_finality` proves the no-op holds. Do NOT bake the property into the action guard (that would check a tautology).
- Use the verified Quint map/set syntax from `.research/attestation/quint-builtin.md`: `str -> int` map type, `.put(k,v)`, `.get(k)`, `.set(k,v)`, `.contains(e)`, `.union(s)`, `.size()`, `Set(...)`, `s.oneOf()`, `1.to(3)`, `s.mapBy(f)`.
- Bounds are the smallest that exercise the race (≥2 terminal candidates competing for one command). If TLC can't exhaust the state space, reduce to 2 command ids — but record the change in the `@promotion` bounds field.
- If `intToString` or string `++` fails to parse, switch tombstone keys to `(str, int)` tuples — but this model doesn't use tombstone keys (that's `session_generation.qnt`), so this risk does not apply here.

## Risks (from parent feature)

- **Composed-model parse risk (highest).** This is the first composed Patchbay model; if it fails to parse, resolve the Quint syntax before the other units are attempted. The skill idioms parse individually but composition is unvalidated.
- **TLC bounds scalability.** 3×6×3×12 may hit limits; reduce bounds if needed.
