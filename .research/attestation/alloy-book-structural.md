---
source_handle: alloy-book-structural
fetched: 2026-07-01
source_url: https://haslab.github.io/formal-software-design/_sources/structural-design/index.rst.txt
provenance: source-direct
---

# Attestation: Formal Software Design with Alloy 6, structural design source

## Paraphrased summary

The structural-design chapter source introduces static structural modeling in Alloy. It uses signatures, subset signatures, facts, `run`, `check`, quantification, `disj`, relational operators, transitive closure, and assertion verification in a file-system example.

## Key passages

{1} The source declares subset signatures with `sig File in Object {}` and `sig Dir in Object {}` and says the keyword `in` after the signature name followed by the including signature declares a subset signature. Source-internal anchor: "Signature declaration".

{2} The source says early validation can use a `run` command to see instances of a partial specification, and gives `run example {}`. It says an instance returned by the command is a valuation of signatures and fields satisfying all specified constraints and the formula in the `run` braces; empty braces are equivalent to true. Source-internal anchor: first `run example {}` discussion.

{3} The source gives a fact `fact { no File & Dir }`, says `&` denotes set intersection, and says `no` checks for set emptiness. Source-internal anchor: fact disjointness example.

{4} The source says `check` verifies that, up to the specified scope, the formula between braces is implied by declared facts; if not, Alloy returns a counter-example instance where the facts hold but the formula checked does not. It gives `check { no File & Dir }`. Source-internal anchor: check-command explanation near disjointness redundancy.

{5} The source gives a quantified fact for non-shared entries: `all x, y : Dir | x != y implies no (x.entries & y.entries)`. It then says Alloy provides the `disj` modifier between quantifier and variables to restrict those variables to be different, restating the property as `all disj x, y : Dir | no (x.entries & y.entries)`. Source-internal anchor: shared-entry property.

{6} The source says `lone` can check if a set contains at most one atom, and gives `all e : Entry | lone entries.e` as an alternative to the non-shared-entry fact. Source-internal anchor: backward navigation alternative.

{7} In a binary-relation note, the source says `entries in Dir lone -> Entry` forces `entries` to be injective, meaning no two atoms of the source signature point to the same target atoms. It says `Dir -> lone Entry` is simple/partial-function, `Dir -> some Entry` is entire/total, and combining properties yields functions, injections, surjections, and bijections. Source-internal anchor: "A bestiary of binary relations".

{8} The source says relational logic has set operators intersection `&`, union `+`, and difference `-`, and uses them to combine relations/predicates. Source-internal anchor: relational-logic paragraph in structural design.

{9} The source says transitive closure `^` applied to a binary relation determines the relation resulting from the union of all possible compositions; `^r` is the same as `r + r.r + r.r.r + ...`; `x.^r` is the set of atoms reachable from `x` in one or more `r` steps. Source-internal anchor: assertion verification / transitive closure explanation.
