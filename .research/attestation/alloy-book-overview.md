---
source_handle: alloy-book-overview
fetched: 2026-07-01
source_url: https://haslab.github.io/formal-software-design/_sources/overview/index.rst.txt
provenance: source-direct
---

# Attestation: Formal Software Design with Alloy 6, overview source

## Paraphrased summary

The overview chapter source introduces Alloy 6 modeling through a mutable Trash example. It covers `var sig`, subset signatures, transition-system traces, predicates for events, `after` and prime for next-state references, `run`, `assert`, `check`, temporal operators, and command scopes.

## Key passages

{1} In the Trash state specification, the source declares `var sig File {}` and `var sig Trash in File {}`. It explains that `File` is a top-level signature and `Trash` is a subset signature, and that top-level signatures are disjoint. Source-internal anchor: lines around the "Specifying a software design" state declaration.

{2} The source says Alloy has no special syntax to declare a transition system; instead it specifies transition systems implicitly using a temporal logic formula that recognizes valid execution traces, and a trace is an infinite sequence of states. Source-internal anchor: transition-system discussion after the mutable signature declarations.

{3} In the event-modeling section, the source says `after` evaluates a formula in the next state, and a trailing prime evaluates an expression in the next state. It says a predicate is a named formula that only holds when invoked, and declares predicates with `pred`, followed by a name, optional parameters, and a braced formula. Source-internal anchor: event specification / predicate paragraph.

{4} The same section gives an example predicate:

```alloy
pred empty {
  some Trash and       // guard
  after no Trash and   // effect on Trash
  File' = File - Trash // effect on File
}
```

Source-internal anchor: `pred empty` code block.

{5} The source gives an empty command `run example {}` and explains that named commands appear in the Analyzer execute menu by their names; unnamed commands are generated from `run` or `check` plus a sequential identifier. Source-internal anchor: `run example {}` paragraph.

{6} The source says temporal operator `eventually` is satisfied by a trace if the formula following it is true in at least one state of the trace. Source-internal anchor: `eventually no File` discussion.

{7} The source says Alloy uses the same logic to specify the system and expected properties; it summarizes `always` as requiring the following formula to hold in all states of a trace, and `eventually` as requiring it in at least one state. Source-internal anchor: "Verifying expected properties" temporal operator discussion.

{8} The source says properties to be verified by `check` commands should be written as assertions; an assertion uses `assert`, an identifier, and a braced formula. It gives an example `assert restore_after_delete { always (all f : File | restore[f] implies once delete[f]) }`. Source-internal anchor: assertion declaration paragraph.

{9} The source says a `check` command followed by an assertion name verifies that assertion; `check restore_after_delete` yields no counter-example instances in the example, meaning the assertion is most likely valid. It says commands have an implicit scope on top-level signatures and also a scope on the finite prefix of traces before the mandatory back loop. By default this trace scope is 10, and `check restore_after_delete for 5 but 20 steps` changes the signature and step scopes. Source-internal anchor: `check restore_after_delete` paragraph.
