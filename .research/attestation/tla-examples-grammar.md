---
source_handle: tla-examples-grammar
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/tlaplus/Examples/master/specifications/SpecifyingSystems/Syntax/TLAPlusGrammar.tla
provenance: source-direct
---

# Per-source attestation: tla-examples-grammar

## Paraphrased summary

The TLA+ Examples repository includes a TLA+ grammar module that encodes core lexical and syntactic forms used by TLA+ modules, declarations, operators, temporal operators, unchanged expressions, and fairness operators.

## Key passages

[tla-examples-grammar]{1} The grammar source begins as a TLA+ module with a module header and `EXTENDS Naturals, Sequences, BNFGrammars`.

[tla-examples-grammar]{2} The reserved-word set includes `MODULE`, `EXTENDS`, `VARIABLE`, `VARIABLES`, `UNCHANGED`, `WF_`, and `SF_`.

[tla-examples-grammar]{3} The module grammar form is `AtLeast4("-") & tok("MODULE") & Name & AtLeast4("-")`, optionally followed by `EXTENDS` and a comma-list of names, then module units, then a line of `=` characters.

[tla-examples-grammar]{4} The variable declaration grammar accepts either `VARIABLE` or `VARIABLES` followed by a comma-list of identifiers.

[tla-examples-grammar]{5} Operator definitions use a left-hand side, `==`, and an expression.

[tla-examples-grammar]{6} Prefix operators include `[]`, `<>`, and `UNCHANGED`.

[tla-examples-grammar]{7} Expression forms include `[ expression ]_ expression` and `<< expression >>_ expression`.

[tla-examples-grammar]{8} Expression forms include `WF_ expression ( expression )` and `SF_ expression ( expression )`.

[tla-examples-grammar]{9} Expression forms include conjunction sequences prefixed by `/\\` and disjunction sequences prefixed by `\\/`.

## Structural metadata

- Source type: TLA+ source file in examples repository.
- Repository branch fetched: `master`.
- Module: `TLAPlusGrammar`.
