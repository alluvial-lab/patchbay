---
name: quint
description: >
  Quint formal-specification language and model-checker (Informal Systems). Auto-loads when
  authoring or checking Quint specifications (.qnt files), Quint state machines, Quint
  invariants/temporal properties, or invoking quint run / quint verify / quint compile.
  Patchbay's chosen primary authoring language for verification models.
user-invocable: false
---

# Quint reference (v0.32.0)

Quint is Patchbay's primary authoring language for formal verification models. Author in Quint for ergonomics; check invariants via the Apalache backend (default); check temporal/liveness properties via `--backend tlc`. Verified working in the Patchbay environment 2026-07-01.

**Install** (non-root; the global npm prefix is not user-writable here):
```sh
mkdir -p ~/.npm-global && npm config set prefix '~/.npm-global'
npm i @informalsystems/quint -g
export PATH="$HOME/.npm-global/bin:$PATH"   # add to shell profile
quint --version   # 0.32.0
```
Node 24 verified. Quint auto-installs its Rust evaluator (v0.6.0) and Apalache server (v0.56.1) on first `run`/`verify`.

## Module syntax

```quint
module Counter {
  var x: int

  action init = x' = 0
  action step = x' = x + 1

  val not_two = x != 2
}
```
- `var x: int` — typed state variable.
- `action init`/`action step` — state transitions; `x'` is the next-state value.
- `val` — pure expression (use for invariants and derived values).
- `pure def` — pure function; `nondet` — nondeterministic assignment.

## Checking commands

| Command | Purpose |
|---|---|
| `quint parse <file>` | Parse (syntax check; does not typecheck or execute). |
| `quint compile <file>` | Parse + typecheck + compile (default target JSON; `--target tlaplus` emits TLA+). Use this for typecheck validation. |
| `quint run <file> --invariant <v> --max-steps N` | Simulator (Rust evaluator); finds invariant violations by random trace. |
| `quint verify <file> --invariant <v> --max-steps N` | Model-check via Apalache (default backend); bounded invariant checking. |
| `quint verify <file> --backend tlc --invariant <v>` | Model-check via TLC (temporal/liveness; finite-state). |
| `quint verify <file> --backend tlc --temporal <p>` | TLC temporal-property checking. |
| `quint test <file> --match <run-name>` | Run named Quint `run` tests. |
| `quint compile <file> --target tlaplus` | Emit TLA+ to stdout (through Apalache's `TLA` command). |

**Exit-code semantics (load-bearing):** `quint run` and `quint verify` exit **non-zero (1)** when a counterexample/violation is found. This is correct checker behavior, not an error — do not treat exit 1 as a tool failure. Exit 0 = no violation found within bounds.

## Backends — when to use which

- **Apalache (default)** — bounded invariant checking. Needs `--max-steps` (default 10). An Apalache pass proves the invariant holds up to the bound; for unbounded claims use inductive invariants or cross-check via TLC on finite state spaces.
- **TLC (`--backend tlc`)** — temporal/liveness properties and exhaustive finite-state checking. Compiles to TLA+ via Apalache, generates a TLC config (`INIT q_init`, `NEXT q_step`, optional `INVARIANT q_inv`, optional `PROPERTY q_temporalProps`), spawns `tlc2.TLC`. Uses the **Apalache-distribution jar** on the classpath (not a standalone `tla2tools.jar`).

**Temporal/liveness caveat:** Apalache temporal support is *partial* (Quint docs: "Temporal properties have partial support"). For temporal/liveness properties, **always use `--backend tlc`**.

## Counterexample output

On a violation, Quint emits a trace and writes artifacts under `_apalache-out/server/<timestamp>/`: `violation1.tla`, `MCviolation1.out`, `violation1.json`, `violation1.itf.json`. Console shows `[violation] Found an issue` and `error: found a counterexample`.

## Patchbay property idioms

These patterns are condensed from the specialist brief's source-grounded snippets. State machines compose transitions with `any { ... }` (alternation) and `all { ... }` (conjunction); nondeterminism uses `nondet x = Set(...).oneOf()`.

**Terminal-finality** (once terminal, later events don't mutate state) — an action-level no-op for terminal retries plus a temporal next-state property:
```quint
module TerminalFinality {
  var phase: str
  var payload: int
  pure val TERMINAL = Set("done", "failed")
  action init = all { phase' = "open", payload' = 0 }
  action mutate = all { not(phase.in(TERMINAL)), payload' = payload + 1, phase' = phase }
  action finish = all { phase' = "done", payload' = payload }
  action retryAfterTerminal = all { phase.in(TERMINAL), phase' = phase, payload' = payload }
  action step = any { mutate, finish, retryAfterTerminal }
  temporal terminal_finality =
    always(phase.in(TERMINAL) implies (next(phase) == phase and next(payload) == payload))
}
// check: quint verify TerminalFinality.qnt --backend tlc --temporal terminal_finality
```

**Idempotent retry** (same key doesn't double-apply) — model the applied-key set as state, repeated keys are no-ops, assert total ≤ unique keys:
```quint
module IdempotentRetry {
  var applied: Set[str]
  var total: int
  action init = all { applied' = Set(), total' = 0 }
  action receive(key) = any {
    all { key.in(applied), applied' = applied, total' = total },
    all { not(key.in(applied)), applied' = applied.union(Set(key)), total' = total + 1 },
  }
  action step = { nondet key = Set("a", "b").oneOf(); receive(key) }
  val no_double_apply = total <= applied.size()
  run same_key_retry_noops = (init).then(receive("a")).then(receive("a")).expect(total == 1)
}
// check: quint run IdempotentRetry.qnt --invariant no_double_apply
//        quint test IdempotentRetry.qnt --match same_key_retry_noops
```

**Monotonic generation** (strictly-greater supersession; lower/equal are no-ops):
```quint
module MonotonicGeneration {
  var generation: int
  var value: int
  action init = all { generation' = 0, value' = 0 }
  action submit(newGen, newValue) = any {
    all { newGen > generation, generation' = newGen, value' = newValue },
    all { newGen <= generation, generation' = generation, value' = value },
  }
  action step = { nondet newGen = 0.to(5).oneOf(); nondet newValue = 0.to(10).oneOf(); submit(newGen, newValue) }
  temporal generation_monotonic = always(next(generation) >= generation)
}
// check: quint verify MonotonicGeneration.qnt --backend tlc --temporal generation_monotonic
```

## Pitfalls

- **Stale docs hazard:** the Quint model-checkers doc page says TLC is "not integrated with Quint" — this is stale. The current npm tarball + changelog (v0.31.0+) confirm `--backend tlc` is implemented and working.
- `_apalache-out/` directories are tool output, not version control — gitignore them.
- `quint compile --target tlaplus` output requires flattening; the emitted TLA+ uses generated names (`q_init`, `q_step`, `q_inv`) — mirror these in any manual TLC config.

## Reference

Hello-world artifact: `specs/seed/Counter.qnt`. Source briefs: `.research/analysis/briefs/formal-methods-tooling-quint.md` + `.research/analysis/briefs/formal-methods-tooling.md`. Attestations: `.research/attestation/quint-*.md`.
