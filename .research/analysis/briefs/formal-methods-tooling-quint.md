---
provenance: agent-synthesis
updated: 2026-07-01
facet: quint
engagement: formal-methods-tooling
---

# Formal methods tooling brief: Quint

## Bottom line

Quint is usable as Patchbay's primary authoring language if Patchbay treats safety invariants as the first-class path and treats temporal/liveness properties as a TLC-oriented path, because current Quint docs and package code show mature syntax for modules/actions/invariants, default Apalache verification for bounded invariant checking, and a TLC backend for temporal formulas; Apalache temporal checking is explicitly warned/partial. [quint-language]{2} [quint-checking-properties]{2} [quint-npm-tarball]{5} [quint-npm-tarball]{8} [quint-model-checkers]{8}

## Current installation and release facts

- Canonical npm package: `@informalsystems/quint`; install command in official docs is `npm i @informalsystems/quint -g`. [quint-getting-started]{1} [quint-npm-registry]{1}
- Latest npm dist-tag observed during this engagement is `0.32.0`, published `2026-03-31T13:40:03.331Z`; the package exports bin `quint` and requires Node `>=18`. [quint-npm-registry]{1} [quint-npm-registry]{2} [quint-npm-registry]{3}
- The latest package tarball pins the default managed Apalache distribution to `0.56.1`. [quint-npm-tarball]{6}
- Quint model checking requires a JDK `>=17`; the project environment note of Java 21 satisfies that documented floor. [quint-getting-started]{6}

## State-machine syntax to use

A Quint state-machine file should define a top-level `module`, declare mutable state with `var`, initialize it with an `action init`, define transitions as `action` definitions, and supply a `step` action that usually composes alternatives with `any { ... }`. [quint-language]{3} [quint-language]{4} [quint-language]{5} [quint-getting-started]{3}

Important syntax anchors:

```quint
module Example {
  var x: int

  action init = x' = 0

  action inc = x' = x + 1

  action step = any { inc }

  val small = x <= 3
}
```

- `pure val` / `pure def` are stateless; `val` / `def` can read state; `action` is for state transitions; `temporal` is for temporal formulas. [quint-language]{5}
- Non-determinism is written with `nondet name = Set(...).oneOf()` or similar, inside an action. [quint-language]{6} [quint-builtin]{6}
- Finite execution examples/tests can be modeled with `run`, `.then`, `.expect`, and `.fail`. [quint-language]{7} [quint-language]{8}

## Properties: invariants, temporal formulas, and `property`

- Ordinary safety properties are written as Boolean state expressions, commonly `val` definitions, then checked with `--invariant <name>` or `--invariants <names...>`. [quint-checking-properties]{2} [quint-checking-properties]{3} [quint-checking-properties]{7}
- Temporal properties are written with the `temporal` qualifier and operators such as `always`, `eventually`, `next`, `orKeep`, `mustChange`, fairness, and `leadsTo`. [quint-language]{9} [quint-builtin]{7} [quint-builtin]{8} [quint-builtin]{9} [quint-builtin]{10} [quint-changelog]{1}
- I found no Quint source syntax keyword named `property`; in the current package code, TLC config maps temporal formulas to a generated TLA+ `PROPERTY q_temporalProps`, while Quint source definitions use `temporal`. [quint-language]{5} [quint-npm-tarball]{10}
- For liveness, prefer TLC-backed checking (`--backend tlc --temporal ...`) because Quint's own docs say Apalache temporal support is partial, current package code implements the TLC backend, and the changelog records a warning for temporal formulas with Apalache. [quint-model-checkers]{8} [quint-npm-tarball]{8} [quint-changelog]{5}

## CLI commands and output surfaces

| Command | Current role | Key options / defaults |
|---|---|---|
| `quint compile <input>` | Parses/typechecks/compiles; default target is JSON, `--target tlaplus` emits TLA+ to stdout via Apalache. | `--target {json,tlaplus}`, default `json`; `--flatten`, default `true`; `--out`; `--main`; `--init`; `--step`. [quint-npm-tarball]{2} [quint-npm-tarball]{11} |
| `quint run <input>` | Simulates traces and can check invariants/witnesses. | `--max-steps` default `20`; `--max-samples`; `--init init`; `--step step`; `--invariant`; `--invariants`; `--witnesses`; `--out-itf`; `--backend {typescript,rust}`, default `rust`. [quint-npm-tarball]{3} |
| `quint test <input>` | Runs Quint `def`-based tests/runs against a spec. | `--max-samples`; `--seed`; `--match`; `--out-itf`; `--backend {typescript,rust}`, default `rust`. [quint-npm-tarball]{4} |
| `quint verify <input>` | Model checks through Apalache by default or TLC via `--backend tlc`. | `--backend {apalache,tlc}`, default `apalache`; `--max-steps` default `10`; `--invariant`; `--invariants`; `--temporal`; `--inductive-invariant`; `--out-itf`; `--tlc-config`; `--apalache-config`. [quint-npm-tarball]{5} |

Text output uses `[ok] No violation found` on successful verification and `[violation] Found an issue` with a printed trace when a trace-backed issue is found; `--out-itf` writes the trace in Informal Trace Format, and `--out` writes selected stage/result fields as JSON. [quint-checking-properties]{5} [quint-checking-properties]{6} [quint-npm-tarball]{12}

## Checking path: Apalache, TLC, and TLA+

- Default `quint verify` checks with Apalache. The package creates Apalache configs, connects to an existing server or downloads/spawns the pinned Apalache server, and sends `CHECK` requests. [quint-npm-tarball]{5} [quint-npm-tarball]{6} [quint-npm-tarball]{7}
- Apalache is bounded: the docs say it checks executions up to `--max-steps` and defaults to 10, so an Apalache `[ok]` is bound-relative unless using the inductive-invariant workflow. [quint-model-checkers]{7} [quint-checking-properties]{8} [quint-checking-properties]{10}
- `quint verify --backend tlc` is now implemented in the package; it compiles to TLA+ through Apalache, ensures the Apalache distribution/JAR is present, generates TLC config, and invokes `tlc2.TLC` through Java. [quint-npm-tarball]{8} [quint-npm-tarball]{10} [quint-changelog]{3}
- `quint compile --target tlaplus` emits TLA+ to stdout through Apalache's `TLA` command; {inferred: from implementation} the emitted TLA+ can be checked independently by TLC if the operator supplies an equivalent TLC config (`INIT q_init`, `NEXT q_step`, optional `INVARIANT q_inv`, optional `PROPERTY q_temporalProps`), but the current Quint user docs do not provide a stable manual TLC command recipe. [quint-npm-tarball]{2} [quint-npm-tarball]{10} [quint-npm-tarball]{11}

## Idioms for Patchbay-shaped properties

The snippets below are templates, not executed artifacts; they instantiate documented `module`/`var`/`action`/`temporal` syntax, nondeterministic finite choices, set membership/update operators, and `run` expectations. [quint-language]{3} [quint-language]{4} [quint-language]{5} [quint-language]{6} [quint-language]{8} [quint-builtin]{1} [quint-builtin]{5}

### Terminal finality

Use an action-level no-op for terminal retries plus a temporal next-state property for “once terminal, later events do not mutate terminal state.” [quint-language]{9} [quint-builtin]{1}

```quint
module TerminalFinality {
  var phase: str
  var payload: int

  pure val TERMINAL = Set("done", "failed")

  action init = all {
    phase' = "open",
    payload' = 0,
  }

  action mutate = all {
    not(phase.in(TERMINAL)),
    payload' = payload + 1,
    phase' = phase,
  }

  action finish = all {
    phase' = "done",
    payload' = payload,
  }

  action retryAfterTerminal = all {
    phase.in(TERMINAL),
    phase' = phase,
    payload' = payload,
  }

  action step = any { mutate, finish, retryAfterTerminal }

  temporal terminal_finality =
    always(phase.in(TERMINAL) implies (next(phase) == phase and next(payload) == payload))
}
```

Check with TLC for stronger temporal support:

```sh
quint verify TerminalFinality.qnt --backend tlc --temporal terminal_finality
```

TLC is the preferred backend for this temporal shape because Quint docs describe TLC as checking temporal properties and Apalache temporal support as partial. [quint-model-checkers]{6} [quint-model-checkers]{8}

### Idempotent retry at a boundary

Model the idempotency key set as state, make repeated keys no-ops, and assert that total applied effects never exceeds the number of unique keys. [quint-builtin]{1} [quint-builtin]{2} [quint-builtin]{11} [quint-builtin]{12}

```quint
module IdempotentRetry {
  var applied: Set[str]
  var total: int

  pure val KEYS = Set("a", "b")

  action init = all {
    applied' = Set(),
    total' = 0,
  }

  action receive(key: str) = any {
    all {
      key.in(applied),
      applied' = applied,
      total' = total,
    },
    all {
      not(key.in(applied)),
      applied' = applied.union(Set(key)),
      total' = total + 1,
    },
  }

  action step = {
    nondet key = KEYS.oneOf()
    receive(key)
  }

  val no_double_apply = total <= applied.size()

  run same_key_retry_noops =
    (init).then(receive("a")).then(receive("a")).expect(total == 1)
}
```

Check both the invariant and the concrete retry run:

```sh
quint run IdempotentRetry.qnt --invariant no_double_apply
quint verify IdempotentRetry.qnt --invariant no_double_apply
quint test IdempotentRetry.qnt --match same_key_retry_noops
```

`run` is the simulator/invariant path, `verify` is model checking, and `test` is the test runner. [quint-npm-tarball]{3} [quint-npm-tarball]{4} [quint-npm-tarball]{5}

### Monotonic generation / supersession

Encode lower and equal generations as no-ops, strictly greater generations as state changes, and optionally add a temporal monotonicity property. [quint-language]{9} [quint-builtin]{6}

```quint
module MonotonicGeneration {
  var generation: int
  var value: int

  action init = all {
    generation' = 0,
    value' = 0,
  }

  action submit(newGen: int, newValue: int) = any {
    all {
      newGen > generation,
      generation' = newGen,
      value' = newValue,
    },
    all {
      newGen <= generation,
      generation' = generation,
      value' = value,
    },
  }

  action step = {
    nondet newGen = 0.to(5).oneOf()
    nondet newValue = 0.to(10).oneOf()
    submit(newGen, newValue)
  }

  temporal generation_monotonic = always(next(generation) >= generation)

  run lower_rejected =
    (init).then(submit(2, 20)).then(submit(1, 99)).expect(generation == 2 and value == 20)

  run equal_noop =
    (init).then(submit(2, 20)).then(submit(2, 99)).expect(generation == 2 and value == 20)
}
```

Check temporal monotonicity with TLC and scenario tests with `quint test`:

```sh
quint verify MonotonicGeneration.qnt --backend tlc --temporal generation_monotonic
quint test MonotonicGeneration.qnt --match "lower_rejected|equal_noop"
```

## Hello-world recipe: minimal counter model

This is a source-grounded recipe for an operator to run later; it was not executed in this engagement. It combines the documented module/action/invariant syntax and the model-checker example pattern where `x` starts at `0`, increments by one, and violates `x != 2` at depth 2. [quint-language]{3} [quint-language]{5} [quint-model-checkers]{6}

Create `Counter.qnt`:

```quint
module Counter {
  var x: int

  action init = x' = 0
  action step = x' = x + 1

  val not_two = x != 2
}
```

Commands:

```sh
npm i @informalsystems/quint -g
quint run Counter.qnt --invariant not_two --max-steps 3
quint verify Counter.qnt --invariant not_two --max-steps 3
quint compile Counter.qnt --target tlaplus > Counter.tla
quint verify Counter.qnt --backend tlc --invariant not_two
```

The install command is from the official guide, `run` and `verify` are the documented simulator/model-checker commands, `compile --target tlaplus` is supported by the current package, and the TLC backend is present in current package code. [quint-getting-started]{1} [quint-getting-started]{4} [quint-getting-started]{6} [quint-npm-tarball]{2} [quint-npm-tarball]{8}

Expected failure shape: the simulator/verification path should emit a violation trace and a `[violation] Found an issue` style result when the invariant fails; exact state values and elapsed timings are runtime-dependent. [quint-checking-properties]{6} [quint-npm-tarball]{12}

## Risks for Patchbay adoption

- Temporal/liveness checking is the highest-risk area: Quint syntax supports temporal formulas, but Apalache support is partial/experimental and TLC should be the default for temporal checks. [quint-language]{9} [quint-model-checkers]{8} [quint-changelog]{5}
- Apalache verification is bounded by `--max-steps` unless using inductive invariants, so safety claims should either be phrased as bounded checks, backed by inductive invariants, or cross-checked through TLC on finite state spaces. [quint-model-checkers]{7} [quint-checking-properties]{8} [quint-checking-properties]{10}
- TLC requires finite state spaces; Quint docs call out that TLC cannot pick from all integers and needs constrained finite sets. [quint-model-checkers]{3}
- Current official docs are not fully synchronized with the current package: one official model-checker page still says TLC is “not integrated,” while the current package and changelog show `--backend tlc`. [quint-model-checkers]{5} [quint-npm-tarball]{5} [quint-changelog]{3}

## Disconfirming analysis

- I checked the official getting-started guide, language reference, builtin operator docs, checking-properties page, model-checkers page, npm registry metadata, npm tarball source, and changelog rather than relying on memory. These sources agree on the npm package name, core module/action syntax, invariant checking, and Apalache as the default verification path. [quint-getting-started]{1} [quint-language]{5} [quint-checking-properties]{3} [quint-npm-registry]{1} [quint-npm-tarball]{5}
- I found disconfirming/stale documentation around TLC integration: the model-checkers page says TLC is “not integrated with Quint,” but the current npm tarball exposes `--backend tlc`, the implementation invokes TLC, and the changelog says TLC was added as an alternative backend in v0.31.0. [quint-model-checkers]{5} [quint-npm-tarball]{5} [quint-npm-tarball]{8} [quint-changelog]{3}
- I found disconfirming/stale generated CLI documentation around defaults in `docs/content/docs/quint.md` during source review, but I did not cite it as authoritative in the synthesis because the npm tarball is the fetched current release artifact and the tarball's CLI source matches the v0.31/v0.32 changelog for Rust defaults and TLC backend. [quint-npm-tarball]{3} [quint-npm-tarball]{4} [quint-changelog]{3} [quint-changelog]{4}
- I did not install or run Quint, per engagement instruction; therefore snippets and the hello-world recipe are source-grounded templates, not runtime-validated artifacts. [quint-getting-started]{4} [quint-getting-started]{6}

## Contradictions

| Handles | Relationship | Divergence | Treatment |
|---|---|---|---|
| `quint-model-checkers`, `quint-npm-tarball`, `quint-changelog` | contradicts | Official model-checker docs say TLC is not integrated; current package code and changelog say `quint verify --backend tlc` exists and is implemented. [quint-model-checkers]{5} [quint-npm-tarball]{5} [quint-npm-tarball]{8} [quint-changelog]{3} | Use current npm tarball/changelog for CLI behavior; treat page prose as stale for this point. |
| `quint-language`, `quint-builtin`, `quint-changelog` | qualifies | Older language page says leadsto can be written as `always(P implies eventually(Q))`; builtin docs and changelog show a current `leadsTo` operator. [quint-language]{10} [quint-builtin]{10} [quint-changelog]{1} | Use `leadsTo` as available, while `always(...eventually(...))` remains a portable spelling. |
| `quint-model-checkers`, `quint-npm-tarball` | qualifies | Model-checkers page says Apalache is integrated and auto-downloaded; package source specifies the concrete mechanism and pinned version. [quint-model-checkers]{5} [quint-npm-tarball]{6} [quint-npm-tarball]{7} | Use package source for exact version/mechanism. |

## Revisit if

- npm `@informalsystems/quint` latest changes beyond `0.32.0`, because CLI defaults, `--backend tlc`, Apalache version pin, and temporal support are moving surfaces. [quint-npm-registry]{1} [quint-changelog]{1}
- Quint publishes updated docs resolving TLC integration and generated CLI-default drift. [quint-model-checkers]{5} [quint-npm-tarball]{5}
- Patchbay needs unbounded liveness guarantees rather than bounded safety or finite-state TLC checks, because current source evidence makes temporal/liveness the least stable path. [quint-model-checkers]{8} [quint-changelog]{5}

## Acquisition candidates

- **enriching** — source: npm registry/tarball for `@informalsystems/quint@0.32.0`; class: runtime validation in controlled environment; web-availability: npm tarball and registry are public; completes: exact installed `quint --help`, actual counterexample text/ITF JSON, and whether the three Patchbay idiom snippets parse/typecheck under the current package. [quint-npm-registry]{3} [quint-npm-tarball]{1}
- **enriching** — source: TLC URL named by Quint's model-checkers page; class: independent backend documentation; web-availability: linked from fetched Quint docs; completes: a fully sourced manual `java tlc2.TLC` recipe for `quint compile --target tlaplus` output outside `quint verify --backend tlc`. [quint-model-checkers]{1} [quint-npm-tarball]{10}
