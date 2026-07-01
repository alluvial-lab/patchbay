---
source_handle: alloy6
fetched: 2026-07-01
source_url: https://alloytools.org/alloy6.html
provenance: source-direct
---

# Attestation: Alloy 6 page

## Paraphrased summary

The Alloy 6 page describes additions in Alloy 6: mutable signatures/fields via `var`, next-state expression syntax with a trailing prime, temporal logic connectives, lasso-trace instances, time-horizon scoping through `steps`, and the relationship between Alloy 6 syntax and older Alloy semantics.

## Key passages

{1} Under "Mutable signatures and fields", the page says Alloy 6 extends previous versions with a `var` keyword for mutable signatures or fields; signatures or fields not preceded by `var` are static and assumed constant over time. Source-internal anchor: "Mutable signatures and fields".

{2} Under "Value of an expression in the next state", the page says the value of expression `e` in the next state is denoted by `e'`; it also says constraints are extended with linear-time temporal logic with past connectives for reasoning about future and past states along a trace. Source-internal anchor: "Value of an expression in the next state".

{3} Under "Instances are traces", the page says Alloy instances are now infinite sequences of states, represented as lasso traces: finite sequences with a loop from the last state to a former state. Source-internal anchor: "Instances are traces".

{4} The same "Instances are traces" section says an instance for a model without variable signatures or fields can be thought of as a trace made of a single state looping to itself, and that in such a case the visualizer works as in older Alloy versions. It also says plain old Alloy models that do not use Alloy 6 syntactic constructs collapse to usual Alloy semantics. Source-internal anchor: "Instances are traces".

{5} Under "Time horizon", the page says analyses proceed by bounding signatures, and that scope specifications may also constrain the time horizon of lasso traces using the reserved `steps` keyword; `for N steps` is equivalent to `for 1 .. N steps`, and no time horizon is implicitly equivalent to `for 10 steps`. Source-internal anchor: "Time horizon".

{6} Under "Complete model-checking", the page says Alloy 6 offers complete model checking over all possible traces when the time scope is set to `1.. steps`; it also says NuSMV and nuXmv are supported and must be installed by the user. Source-internal anchor: "Complete model-checking".

{7} Under the future-time temporal formula grammar, the page gives `expr ::= unOp expr | expr binOp expr`, `unOp ::= always | eventually | after`, and `binOp ::= until | releases | ;`. Source-internal anchor: temporal-connectives grammar.

{8} The temporal-operator semantics section says `after F` is true in state `i` iff `F` is true in state `i + 1`; `always F` is true in state `i` iff `F` is true in every state at or after `i`; `eventually F` is true in state `i` iff `F` is true in some state at or after `i`; `F until G` requires `G` in some state `j >= i` and `F` in every state from `i` before `j`; `F releases G` is described as `G` holding until and including a state where `F` holds, or forever if no such state exists; `F ; G` is true in state `i` iff `F` is true in state `i` and `G` is true in `i + 1`. Source-internal anchor: future-time temporal formula semantics.
