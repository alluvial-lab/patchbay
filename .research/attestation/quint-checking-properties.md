---
source_handle: quint-checking-properties
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/quint-co/quint/main/docs/content/docs/checking-properties.mdx
provenance: source-direct
---

# Attestation: Quint checking-properties documentation

## Structural metadata

- Source kind: official Quint documentation page source (`docs/content/docs/checking-properties.mdx`).
- Local fetched copy: `.research/reference/quint/checking-properties.mdx`.
- Page title: `Checking Properties`.

## Paraphrased summary

The page explains properties in Quint, focusing on safety properties as invariants and discussing liveness as temporal formulas. It demonstrates `--witnesses`, negated invariants for finding interesting traces, multiple invariants with `--invariants`, inductive invariants, and how inductive invariant checking is decomposed into Apalache calls.

## Key passages

### {1} simulator and model checkers as property tools

The page says the main way to interact with a Quint model is by checking properties; the simulator can increase confidence in a property, and model checkers can formally verify properties.

Anchor: lines 9-13.

### {2} safety and liveness distinction

The page defines safety examples as “something bad never happens” and liveness examples as “something good eventually happens.” It says safety properties are much easier to write and check as invariants, while liveness properties require temporal formulas and often fairness assumptions.

Anchor: lines 15-21.

### {3} invariant as Boolean expression over state

The page gives:

```quint
val no_negatives = ADDRESSES.forall(addr => balances.get(addr) >= 0)
```

It says that while checking `no_negatives` as an invariant, Quint tools check it on every state, including the initial state and every possible reachable state.

Anchor: lines 23-27.

### {4} witnesses

The page says witnesses are for checking that something is true in some state rather than every state. It says there are two current ways: `--witnesses` in the simulator and an invariant of the simulator or model checker.

Anchor: lines 31-39.

### {5} `--witnesses` command and output shape

The page gives:

```sh
quint run gettingStarted.qnt --witnesses=alice_more_than_bob --max-steps=5
```

The reported output includes `[ok] No violation found`, hint lines, and a `Witnesses:` section with the witnessed trace count and seed.

Anchor: lines 55-64.

### {6} violation trace output shape

For a negated invariant, the page gives:

```sh
quint run gettingStarted.qnt --invariant="not(alice_more_than_bob)" --max-steps=5
```

The reported output prints an example execution with numbered states, `[violation] Found an issue`, hints, a seed, and `error: Invariant violated`.

Anchor: lines 73-93.

### {7} multiple invariants

The page says `--invariants` checks multiple invariant definition names at once, gives `quint run bank.qnt --invariants no_negatives accounts_match total_positive`, and shows violation output with specific failed invariant names. It says `--invariants` also works with `verify`.

Anchor: lines 95-125.

### {8} inductive invariant checking commands

The page says an inductive invariant can be checked with:

```sh
quint verify file.qnt --inductive-invariant ind_inv
```

and, with an ordinary invariant implication check:

```sh
quint verify file.qnt --inductive-invariant ind_inv --invariant inv
```

It says simulation (`quint run`) does not support inductive invariants.

Anchor: lines 173-187.

### {9} TypeOK requirement for inductive invariants

The page says each state variable `x` in an inductive invariant must first have either `x == <value>` or `x.in(<set of possible values>)`; it says predicates defining variable domains are usually called `TypeOK`.

Anchor: lines 189-195.

### {10} Apalache calls for inductive invariants

The page says Quint calls Apalache 2-3 times for inductive invariants: base case equivalent to `quint verify file.qnt --invariant ind_inv --max-steps 0`; preservation equivalent to `quint verify file.qnt --init ind_inv --invariant ind_inv --max-steps 1`; and, when an ordinary invariant is present, implication equivalent to `quint verify file.qnt --init ind_inv --invariant inv --max-steps 0`.

Anchor: lines 199-213.
