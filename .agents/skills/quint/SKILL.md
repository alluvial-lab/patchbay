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
| `quint parse <file>` | Parse + typecheck (no execution). |
| `quint run <file> --invariant <v> --max-steps N` | Simulator (Rust evaluator); finds invariant violations by random trace. |
| `quint verify <file> --invariant <v> --max-steps N` | Model-check via Apalache (default backend); bounded invariant checking. |
| `quint verify <file> --backend tlc --invariant <v>` | Model-check via TLC (temporal/liveness; finite-state). |
| `quint verify <file> --backend tlc --temporal <p>` | TLC temporal-property checking. |
| `quint compile <file> --target tlaplus` | Emit TLA+ to stdout (through Apalache's `TLA` command). |

**Exit-code semantics (load-bearing):** `quint run` and `quint verify` exit **non-zero (1)** when a counterexample/violation is found. This is correct checker behavior, not an error — do not treat exit 1 as a tool failure. Exit 0 = no violation found within bounds.

## Backends — when to use which

- **Apalache (default)** — bounded invariant checking. Needs `--max-steps` (default 10). An Apalache pass proves the invariant holds up to the bound; for unbounded claims use inductive invariants or cross-check via TLC on finite state spaces.
- **TLC (`--backend tlc`)** — temporal/liveness properties and exhaustive finite-state checking. Compiles to TLA+ via Apalache, generates a TLC config (`INIT q_init`, `NEXT q_step`, optional `INVARIANT q_inv`, optional `PROPERTY q_temporalProps`), spawns `tlc2.TLC`. Uses the **Apalache-distribution jar** on the classpath (not a standalone `tla2tools.jar`).

**Temporal/liveness caveat:** Apalache temporal support is *partial* (Quint docs: "Temporal properties have partial support"). For temporal/liveness properties, **always use `--backend tlc`**.

## Counterexample output

On a violation, Quint emits a trace and writes artifacts under `_apalache-out/server/<timestamp>/`: `violation1.tla`, `MCviolation1.out`, `violation1.json`, `violation1.itf.json`. Console shows `[violation] Found an issue` and `error: found a counterexample`.

## Patchbay property idioms

Terminal-finality (once terminal, later events don't mutate state):
```quint
temporal terminalFinality = always(all cmd in commands => cmd.state == "completed" implies always(cmd.state == "completed"))
```

Idempotent retry (same idempotency key doesn't double-apply):
```quint
val no_double_apply = all k in appliedKeys => countApplied(k) <= 1
```

Monotonic generation (strictly-greater supersession; lower rejected, equal no-op):
```quint
action report_generation(s, g) = all {
  requires(g > currentGen(s) or g == currentGen(s)),
  g > currentGen(s) ? currentGen' = g else currentGen' == currentGen
}
```

## Pitfalls

- **Stale docs hazard:** the Quint model-checkers doc page says TLC is "not integrated with Quint" — this is stale. The current npm tarball + changelog (v0.31.0+) confirm `--backend tlc` is implemented and working.
- `_apalache-out/` directories are tool output, not version control — gitignore them.
- `quint compile --target tlaplus` output requires flattening; the emitted TLA+ uses generated names (`q_init`, `q_step`, `q_inv`) — mirror these in any manual TLC config.

## Reference

Hello-world artifact: `specs/seed/Counter.qnt`. Source briefs: `.research/analysis/briefs/formal-methods-tooling-quint.md` + `.research/analysis/briefs/formal-methods-tooling.md`. Attestations: `.research/attestation/quint-*.md`.
