---
source_handle: quint-getting-started
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/quint-co/quint/main/docs/content/docs/getting-started.mdx
provenance: source-direct
---

# Attestation: Quint getting-started guide

## Structural metadata

- Source kind: official Quint documentation page source (`docs/content/docs/getting-started.mdx`).
- Local fetched copy: `.research/reference/quint/getting-started.mdx`.
- Page title: `Getting Started`.

## Paraphrased summary

The guide introduces installation, editor setup, a small `bank` model, simulator-based invariant checking, and model-checker-based verification. It shows the npm installation command, a module with `var`, `pure val`, `action`, `nondet`, `any`, and a state invariant, then demonstrates `quint run` and `quint verify` against that invariant.

## Key passages

### {1} npm installation command

The NPM tab says to install node/npm and run:

```sh
npm i @informalsystems/quint -g
```

Anchor: lines 12-18 in the fetched copy.

### {2} editor language server npm package

The Vim/Neovim editor setup says to install the language server via npm:

```sh
npm i @informalsystems/quint-language-server -g
```

Anchor: lines 81-85.

### {3} sample model shape

The guide's `bank` example defines `module bank`, a state variable `var balances: str -> int`, a pure value `pure val ADDRESSES = Set("alice", "bob", "charlie")`, `action deposit`, `action withdraw`, `action init`, `action step`, and a `val no_negatives` invariant. The `step` action uses `nondet account = ADDRESSES.oneOf()`, `nondet amount = 1.to(100).oneOf()`, and `any { deposit(...), withdraw(...) }`.

Anchor: lines 107-147.

### {4} simulator command and purpose

The guide says the `run` subcommand simulates executions while checking an invariant, and gives:

```sh
quint run bank.qnt --invariant=no_negatives
```

Anchor: lines 153-158.

### {5} MBT flag metadata

The guide says `--mbt` includes additional metadata on a violation trace, “usually used for testing purposes,” and identifies MBT as Model Based Testing.

Anchor: lines 160-165.

### {6} model checker command, Java requirement, and default bound

The guide says to use the `verify` subcommand for the model checker, notes that the model checker requires Java Development Kit `>= 17`, gives:

```sh
quint verify bank.qnt --invariant=no_negatives
```

and says this verifies all possible executions of up to 10 steps and should produce an `[ok]` result when fixed.

Anchor: lines 187-196.
