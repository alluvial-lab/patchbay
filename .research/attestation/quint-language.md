---
source_handle: quint-language
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/quint-co/quint/main/docs/content/docs/lang.md
provenance: source-direct
---

# Attestation: Quint language reference

## Structural metadata

- Source kind: official Quint language reference markdown (`docs/content/docs/lang.md`).
- Local fetched copy: `.research/reference/quint/lang.md`.
- Major sections used: type aliases, modes, module-level constructs, nondeterministic choice, runs, temporal operators.

## Paraphrased summary

The language reference describes Quint modules, constants, state variables, operator definitions with qualifiers (`pure val`, `pure def`, `val`, `def`, `action`, `temporal`), modes, nondeterministic choice, finite runs, and temporal operators. It distinguishes state, action, run, and temporal modes and states that action and temporal modes are intentionally incomparable.

## Key passages

### {1} type alias and uninterpreted type forms

The reference says `type` can define an alias inside a module, gives `type Temperature = int`, shows polymorphic variants such as `type Option[a] = | Some(a) | None`, and says a type identifier can introduce an uninterpreted type by defining it without constructors, e.g. `type MY_TYPE`.

Anchor: lines 97-121.

### {2} modes and action/temporal separation

The reference lists modes: Stateless, State, Non-determinism, Action, Run, Temporal. Its subsumption table says Action accepts Non-determinism/Stateless/State, Run accepts Stateless/State/Action, and Temporal accepts Stateless/State. It states that action mode and temporal mode are incomparable and that this is intentional to avoid mixing actions with temporal formulas.

Anchor: lines 124-159.

### {3} module definition grammar

A module is introduced as:

```quint
module Foo {
  // declarations
}
```

The grammar is `"module" <identifier> "{" <definitions> "}"`. The reference says a single file should contain one top-level module, top-level modules cannot be nested, and the top-level module name should match the file name when using multiple modules in a file.

Anchor: lines 163-187.

### {4} state variable declarations

State variables are declared with `var`, for example:

```quint
var name: str
var timer: int
var isArmed: bool
```

The grammar is `"var" <identifier> ":" <type>`, and the mode is State.

Anchor: lines 242-250.

### {5} operator/action/temporal definitions

The reference shows examples of `pure val`, `pure def`, `val`, `def`, `action init`, `action advance`, and `temporal neverNegative = always(timer >= 0)`. The grammar permits `("val" | "def" | "pure" "val" | "pure" "def" | "action" | "temporal") <identifier>... = <expr>`. The qualifier table maps `pure val`/`pure def` to Stateless definitions, `val`/`def` to State, `action` to Action, and `temporal` to Temporal.

Anchor: lines 261-328.

### {6} nondeterministic choice

The reference says nondeterministic choice uses:

```scala
nondet name = oneOf(expr1)
expr2
```

where `oneOf(expr1)` picks an element of a non-empty set, binds it to `name`, and makes it available in the nested action. The example `action nextSquare` chooses `i = oneOf(Int)`, constrains it with `all { ... }`, and assigns `x' = i`. The mode table lists `oneOf` as producing Non-determinism and `nondet x = e1; e2` as an Action when `e2` is an Action.

Anchor: lines 1014-1059.

### {7} finite runs

A `run` represents a finite execution. The reference shows `run run1 = (n' = 1).then(n' = 2)...`, `run run2 = (Init).then(Positive)...`, and `run run3 = (Init).then(Next)...`. It says `then` sequences actions and that a run may describe constraints over a sequence of states rather than exactly one state sequence.

Anchor: lines 1568-1655.

### {8} run assertions and expectations

The reference describes `fail(A)` as useful for runs that expect an action to be disabled. It describes `A.expect(P)` where the left-hand side is an action or run and the right-hand side is a non-action Boolean expression; if `P` evaluates false after applying `A`, a runtime error like `assert` is emitted.

Anchor: lines 1689-1727.

### {9} temporal always/eventually/next/fairness

The temporal section says temporal operators describe infinite executions. It gives `always(P)` / `P.always` as equivalent to TLA+ `[] P`, `eventually(P)` / `P.eventually` as equivalent to TLA+ `<> P`, and `next(e)` / `e.next` as equivalent to TLA+ prime in Temporal mode. It describes `orKeep(A, x)` and `mustChange(A, x)` as converting an action to temporal property forms, `enabled(A)`, and weak/strong fairness operators.

Anchor: lines 1730-1866.

### {10} leadsto expression in older language page

The “Other temporal operators” subsection says `P \leadsto Q` can be written as:

```scala
always(P implies eventually(Q))
```

Anchor: lines 1867-1873.
