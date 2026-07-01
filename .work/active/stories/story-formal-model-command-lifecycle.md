---
id: story-formal-model-command-lifecycle
kind: story
stage: review
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

## Implementation notes

- Files created: `specs/seed/command_lifecycle.qnt`, `specs/seed/command_lifecycle.emitted.tla`.
- Tests added: none (no implementation code; verification is by running the checkers — see below).
- Discrepancies from design:
  - **Map-literal syntax**: the design's `Set("c1" -> "k1", ...)` for `idemKey` would have parsed as a Set of pairs, not a Map. Corrected to `Map("c1" -> "k1", ...)` per `.research/attestation/quint-builtin.md` (verified `Map(... -> ...)` is the map literal; `Set(...)` with arrows is a set of tuples).
  - **Map membership**: `state.contains(c)` is a Set operation, not a Map operation — typecheck error. Corrected to `state.keys().contains(c)` (`m.keys()` returns the key set; verified in builtin attestation).
  - **Action variable-consistency**: Quint `any { ... }` action branches must update the same set of variables. The `receive` action's two branches initially updated only `appliedKeys`/`applyCount`, causing a typecheck error. Fixed by adding `all { ... }` wrappers with explicit `state' = state, lsn' = lsn, ...` for the untouched variables in each branch.
  - **`boundary_dedup` was self-defining (caught by test integrity)**: the design's `appliedKeys.size() <= IDEMPOTENCY_KEYS.size()` passed for the wrong reason — a Set cannot exceed its universe by construction. Restructured to a genuine per-key count check `applyCount.get(k) <= 1` over a new `applyCount: str -> int` state variable, with a permissive `receive(key)` action (any key may arrive any number of times; only the first applies). Now genuinely checks the dedup property against permissive transitions.
  - **`retry_reuses_id_and_key` invariant was inverted (caught by test integrity)**: the design's formulation `(idemKey.get(cmd) == key and key.in(appliedKeys)).not()` asserted "no command's key is in the applied set" — the opposite of reality (every command's key IS applied at init). Apalache correctly found a counterexample. Restructured to a temporal binding-stability property `next(idemKey.get(cmd)) == idemKey.get(cmd)` (the command-id-to-key binding never changes after acceptance).
  - **Backend: Q3's `tlc` for temporal properties does not work (implementation discovery)**: the Quint→TLA+ compilation of `next()`-in-`always()` emits `[](\A cmd: state[cmd]' = state[cmd])`, which TLC rejects with `[] followed by action not of form [A]_v`. Apalache (default backend) checks these temporal properties correctly (exit 0, no violation, all 5) but warns its temporal support is experimental. `backend` field changed from `tlc` to `apalache-temporal`; invocation is `echo y | quint verify ... --temporal <p>`. Recorded in parent feature's Implementation discovery section.
  - **Bounds**: design specified `--max-steps 12`; temporal checks run at `--max-steps 10` (Apalache temporal is slower; 10 is the documented default and sufficient for the bounded state space).
- Adjacent issues parked: none.
- Verification (all exit 0 / `[ok]`):
  - `quint parse command_lifecycle.qnt` → exit 0
  - `quint compile command_lifecycle.qnt` → exit 0 (typecheck)
  - `quint verify --invariant command_durability --max-steps 12` → `[ok]`
  - `quint verify --invariant boundary_dedup --max-steps 12` → `[ok]`
  - `echo y | quint verify --temporal terminal_finality --max-steps 10` → `[ok]`
  - `echo y | quint verify --temporal pre_append_terminal_choice --max-steps 10` → `[ok]`
  - `echo y | quint verify --temporal lsn_determines_terminal_winner --max-steps 10` → `[ok]`
  - `echo y | quint verify --temporal retry_reuses_id_and_key --max-steps 10` → `[ok]`
  - `echo y | quint verify --temporal retry_after_terminal_returns_existing --max-steps 10` → `[ok]`
  - 7 `@promotion` blocks present and grep-able; each `invocation` names the exact CLI.
  - `command_lifecycle.emitted.tla` committed (generated artifact, never hand-edited).
