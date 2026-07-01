---
source_handle: tla-examples-diehard
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/tlaplus/Examples/master/specifications/DieHard/DieHard.tla
provenance: source-direct
---

# Per-source attestation: tla-examples-diehard

## Paraphrased summary

The TLA+ Examples `DieHard.tla` module is a compact example using constants, variables, an `Init` predicate, a `Next` action, a `Spec` formula, and state predicates used as invariants.

## Key passages

[tla-examples-diehard]{1} The module declares constants `Big`, `Small`, and `Goal`, and declares variables `big` and `small`.

[tla-examples-diehard]{2} The example defines `TypeOK == /\\ big \\in 0..Big /\\ small \\in 0..Small` as a type-correctness predicate.

[tla-examples-diehard]{3} The example defines `Init == /\\ big = 0 /\\ small = 0` as the initial predicate.

[tla-examples-diehard]{4} The example defines action operators such as `FillSmall`, `FillBig`, `EmptySmall`, and `EmptyBig`, with primed variables for next-state values.

[tla-examples-diehard]{5} The example defines `Next` as a disjunction of action operators.

[tla-examples-diehard]{6} The example defines `Spec == Init /\\ [][Next]_<<big, small>>`.

## Structural metadata

- Source type: TLA+ source file in examples repository.
- Repository branch fetched: `master`.
- Module: `DieHard`.
