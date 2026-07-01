---
source_handle: alloy-book-relational
fetched: 2026-07-01
source_url: https://haslab.github.io/formal-software-design/_sources/relational-logic/index.rst.txt
provenance: source-direct
---

# Attestation: Formal Software Design with Alloy 6, relational-logic source

## Paraphrased summary

The relational-logic chapter source explains Alloy's relational logic, including signatures and fields as relations, quantifiers, atomic formulas, membership, set and relational operators, dot join, product, and transitive closure.

## Key passages

{1} The source uses declarations including `abstract sig Object {}`, `sig File extends Object {}`, `sig Dir extends Object { entries : set Entry }`, `one sig Root extends Dir {}`, and `sig Entry { name : one Name, object : one Object }`. Source-internal anchor: file-system specification at the beginning of the chapter.

{2} The source says a relation is also known in logic as a predicate, and that membership of tuple `(x,y)` in relation `name` can be denoted in Alloy as `x->y in name`. Source-internal anchor: relation/predicate explanation.

{3} The source says all Alloy quantifications are bounded by a set and use English words instead of mathematical notation. Its quantifier table says `all x : A | P` means `P` is true for all `x` in set `A`; `some x : A | P` means true for some `x`; `no x : A | P` means false for all `x`; `lone x : A | P` means true for at most one `x`; and `one x : A | P` means true for exactly one `x`. Source-internal anchor: quantifier table.

{4} The source says multiple variables can be quantified at once and that `disj` can quantify over different values; `all disj x,y : A | P` abbreviates `all x,y : A | x != y implies P`. Source-internal anchor: disjoint quantification paragraph.

{5} The source says Alloy atomic formulas include `R in S` for subset-or-equal, `R = S` for equality, `some R` for at least one tuple, `lone R` for at most one tuple, `one R` for exactly one tuple, and `no R` for empty. Source-internal anchor: atomic formula table.

{6} The source says tuple membership can be written as `x₁->…->xₐ in R`, using `->` to denote a relation with a single tuple containing those variables. Source-internal anchor: membership explanation after atomic formula table.

{7} The relational-operator table names `+` union, `&` intersection, `-` difference, `.` composition/dot join, `[]` box join, `->` product, `<:` domain restriction, `:>` range restriction, `++` override, `~` transpose/converse, `^` transitive closure, and `*` reflexive transitive closure. Source-internal anchor: relational operators table.

{8} The formal-meaning table says `x₁->…->xₐ₋₁->y₂->…->yₑ in R.S` iff there are tuples in `R` and `S` whose joined columns match (`xₐ = y₁`); it also says `x₁->x₂ in ^R` iff `x₁->x₂` is in `R + R.R + R.R.R + …`, and `x₁->x₂ in *R` iff it is in `^R + iden`. Source-internal anchor: formal relational operator meanings.
