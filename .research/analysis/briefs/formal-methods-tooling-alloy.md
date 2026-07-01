---
provenance: agent-synthesis
updated: 2026-07-01
facet: alloy
engagement: formal-methods-tooling
---

# Alloy 6 brief for bounded relational invariants

## Recommendation

Use Alloy 6 for v0 as a relational model checker, not as a temporal model checker, when the property is a single-state shape constraint such as identity uniqueness, endpoint/address ambiguity, static authority-graph legality, routing legality, or anti-spoofing. Alloy 6 temporal constructs are for mutable signatures/fields, next-state values, and future/past trace constraints (`var`, prime, `after`, `always`, `eventually`, `until`, etc.) [alloy6]{1} [alloy6]{2} [alloy6]{7} [alloy6]{8}. Alloy 6 explicitly says a model without variable signatures or fields can be treated as a one-state self-looping trace and collapses to usual Alloy semantics, so relational-only v0 models do not need temporal operators [alloy6]{4}.

Use temporal operators only when the modeled question is about change across time: e.g. "after revocation, future routing attempts are denied", "authority eventually disappears", or "a route remains illegal until a grant event occurs". That crosses from static relation shape into trace behavior because Alloy 6 represents instances as lasso traces and uses `steps` to bound trace prefixes [alloy6]{3} [alloy6]{5}. If revocation is only represented as a static relation such as `revoked in Grant` plus constraints over current reachability, relational Alloy remains sufficient; if revocation semantics mention before/after effects, use temporal Alloy {inferred: property-shape classification} [alloy6]{2} [alloy-book-overview]{3}.

## Current Alloy 6 syntax essentials

- Signatures introduce sets of atoms; examples in the Alloy 6 book include `abstract sig Object {}`, `sig File extends Object {}`, `one sig Root extends Dir {}`, fields such as `entries : set Entry`, and fields with multiplicity such as `name : one Name` [alloy-book-relational]{1}.
- Subset signatures use `sig X in Parent {}`; the structural-design chapter uses `sig File in Object {}` and `sig Dir in Object {}` [alloy-book-structural]{1}.
- Facts add global constraints; the structural-design chapter uses `fact { no File & Dir }`, where `&` is set intersection and `no` checks emptiness [alloy-book-structural]{3}.
- Predicates are named formulas invoked from facts, commands, or other predicates; the overview chapter says `pred` is followed by a name, optional parameters, and a braced formula [alloy-book-overview]{3}.
- Assertions are named formulas checked by `check`; the overview chapter says assertions use `assert`, an identifier, and a braced formula [alloy-book-overview]{8}.
- `run` searches for satisfying instances of a formula; `run example {}` with empty braces imposes no extra constraint beyond the model facts [alloy-book-structural]{2}.
- `check` verifies within the command scope that the checked formula follows from declared facts; a counterexample is an instance where the facts hold and the checked formula does not [alloy-book-structural]{4}.
- Quantifiers are bounded by sets: `all`, `some`, `no`, `lone`, and `one` mean universal, existential, none, at-most-one, and exactly-one respectively [alloy-book-relational]{3}. `all disj x,y : A | P` restricts the variables to different atoms [alloy-book-relational]{4}.
- Atomic formulas include `R in S`, `R = S`, `some R`, `lone R`, `one R`, and `no R` [alloy-book-relational]{5}. Tuple membership can be written with `->`, e.g. `x->y in rel` [alloy-book-relational]{2} [alloy-book-relational]{6}.
- Common relation operators include union `+`, intersection `&`, difference `-`, dot join `.`, product `->`, domain/range restriction `<:`/`:>`, override `++`, converse `~`, transitive closure `^`, and reflexive transitive closure `*` [alloy-book-relational]{7}. Dot join composes relations, and transitive closure `^R` denotes reachability through one or more `R` steps [alloy-book-relational]{8}.

## Temporal operators: v0 scope decision

Alloy 6 adds `var` for mutable signatures/fields; a non-`var` signature or field is static and constant over time [alloy6]{1}. Prime (`e'`) denotes an expression in the next state, while `after F` denotes a formula in the next state [alloy6]{2} [alloy-book-overview]{3}. Future-time temporal syntax includes `always`, `eventually`, `after`, `until`, `releases`, and sequential composition `;` [alloy6]{7}; the official page defines their trace-state semantics [alloy6]{8}.

For v0 relational invariants, model with static signatures/fields and `fact`/`assert`/`check`. This keeps counterexamples as single-state structural instances because Alloy 6 says non-variable models can be treated as one-state loops with usual Alloy semantics [alloy6]{4}. Add `steps` only when the command actually checks trace behavior; Alloy 6 says no time horizon is equivalent to `for 10 steps`, while `for N steps` and `for M .. N steps` bound lasso-trace horizons [alloy6]{5}.

## CLI and headless checking

Current fetched release metadata identifies the latest Alloy release as `v6.2.0` / `Alloy 6.2.0`, published `2025-01-09T16:34:04Z` [alloy-release]{1}. The release asset list includes `org.alloytools.alloy.dist.jar` at `https://github.com/AlloyTools/org.alloytools.alloy/releases/download/v6.2.0/org.alloytools.alloy.dist.jar` [alloy-release]{4}. The download page says Alloy can be run as `java -jar org.alloytools.alloy.dist.jar` [alloy-download]{2}, and the v6.2.0 release notes say the new command-line interface is documented by `java -jar alloy.jar help` [alloy-release]{2}.

Headless check pattern, using the jar asset name after download:

```bash
java -jar org.alloytools.alloy.dist.jar commands patchbay-invariants.als
java -jar org.alloytools.alloy.dist.jar exec --command ActorIdsUnique --type json --output - patchbay-invariants.als
```

The first command is grounded by the CLI source's `commands` command, which prints all commands in an Alloy file with zero-based indexes [alloy-cli]{9}. The second command is grounded by the `exec` command source: `exec` takes a path argument, has a `command` option to select a command by label/glob/index, has a `type` option whose values are `none`, `text`, `table`, `json`, and `xml`, and has an `output` option where `-` sends output to the console [alloy-cli]{1} [alloy-cli]{2} [alloy-cli]{3} [alloy-cli]{4} [alloy-cli]{8}.

Put scope in the Alloy command, not in a separate CLI flag. The book examples use `check restore_after_delete for 5 but 20 steps` to set top-level signature scope and trace-step scope [alloy-book-overview]{9}; for relational-only examples, use a command such as `check ActorIdsUnique for 5` because no mutable trace horizon is needed {inferred: applies Alloy scope syntax to static model} [alloy6]{4} [alloy-book-overview]{9}.

On a failed `check`, Alloy finds a satisfying solution to the negated assertion: the book describes this as a counterexample instance where facts hold and the checked formula does not [alloy-book-structural]{4}. The v6.2.0 CLI prints `SAT` when `solution.satisfiable()` is true and generates the selected solution output; for `check`, that satisfiable solution is the counterexample, while `UNSAT` means no counterexample was found within scope [alloy-cli]{7}. Table output for temporal solutions includes `Trace length`, `Loop state`, and per-state tables; non-temporal relational models are single-state loops under Alloy 6 semantics [alloy-cli]{10} [alloy6]{4}. JSON output writes a `SolutionDTO`, and XML output uses `A4SolutionWriter.writeInstance` [alloy-cli]{11}.

## Installation notes

The fetched release API shows a GitHub release artifact for the distributable jar and platform packages for Linux, macOS, Windows, and Debian [alloy-release]{4} [alloy-release]{5}. The repository README says Alloy runs on all operating systems with a recent JVM, "Java 6 or later", and is available as a runnable jar with Sat4j and native SAT solvers [alloy-repo]{3}. The orchestrator-verified Java 21 environment should satisfy that stated JVM floor {inferred: Java-version comparison} [alloy-repo]{3}.

Temporal complete model checking may need extra tools: the Alloy 6 page says complete model checking uses `1.. steps` and currently supports NuSMV and nuXmv installed by the user [alloy6]{6}. The download page also says Alloy 6 temporal model-checking relies on NuSMV or nuXmv in `PATH` [alloy-download]{3}. The v6.2.0 release notes add that Electrod now handles all of Alloy including integers and recommend updated NuSMV/nuXmv releases [alloy-release]{3}.

## Idioms for bounded relational shapes

### Actor-identity uniqueness

Use a total field plus injectivity:

```alloy
sig Identity {}

sig Actor {
  id: one Identity
}

fact ActorIdsUnique {
  id in Actor lone -> Identity
}
```

The field declaration follows the book's `field : one Target` idiom [alloy-book-relational]{1}. The injectivity idiom follows the structural-design note that `relation in Source lone -> Target` forces no two source atoms to point to the same target [alloy-book-structural]{7}.

Equivalent explicit form:

```alloy
fact ActorIdsUniqueExplicit {
  all disj a, b: Actor | a.id != b.id
}
```

`all disj` is the documented idiom for quantifying over different values [alloy-book-relational]{4}, and equality/inequality is grounded by Alloy atomic equality plus negation/alternate comparison syntax [alloy-book-relational]{5}.

### Authority-graph acyclicity

For a grant graph over actors:

```alloy
sig Actor {
  grants: set Actor
}

fact NoGrantCycles {
  no a: Actor | a in a.^grants
}
```

The `set` field multiplicity follows Alloy field declarations [alloy-book-relational]{1}. The acyclicity test uses transitive closure: `a.^grants` is the set reachable from `a` in one or more grant steps, and `no` checks the absence of such self-reachability [alloy-book-structural]{9} [alloy-book-relational]{5}.

If v0 removes delegation, the grant relation may be absent or always empty; in that case the acyclicity assertion is a reserved shape test rather than a live product invariant {inferred: property-shape classification} [alloy-book-structural]{9}.

### Anti-spoofing / sender identity consistency

For a message whose authenticated sender must match the claimed actor identity:

```alloy
sig Actor {}

sig Message {
  sender: one Actor,
  claimed: one Actor
}

fact NoSpoofing {
  no m: Message | m.sender != m.claimed
}
```

This encodes "no message has a sender different from its claimed identity" using bounded quantification and equality/inequality over `one` fields [alloy-book-relational]{1} [alloy-book-relational]{3} [alloy-book-relational]{5}. If the intended property is instead "self-asserted identity must not be trusted unless it is bound to an authenticated sender," split the model into `claimed` and `authenticated` fields and assert their equality only after the trusted binding relation is in scope {inferred: modeling caveat} [alloy-book-relational]{1}.

## Hello-world recipe

Author `patchbay-hello.als`:

```alloy
sig Identity {}

sig Actor {
  id: one Identity
}

fact UniqueIds {
  id in Actor lone -> Identity
}

assert ActorIdsUnique {
  all disj a, b: Actor | a.id != b.id
}

check ActorIdsUnique for 4
```

The model uses a signature, a `one` field, a fact, an assertion, `all disj`, and a scoped `check`, each grounded in the syntax sources above [alloy-book-relational]{1} [alloy-book-structural]{7} [alloy-book-overview]{8} [alloy-book-relational]{4} [alloy-book-overview]{9}. The command to check it headlessly after downloading the v6.2.0 jar is:

```bash
curl -L -o org.alloytools.alloy.dist.jar \
  https://github.com/AlloyTools/org.alloytools.alloy/releases/download/v6.2.0/org.alloytools.alloy.dist.jar
java -jar org.alloytools.alloy.dist.jar exec --command ActorIdsUnique --type json --output - patchbay-hello.als
```

The download URL is a fetched release asset [alloy-release]{4}. The `exec` path, command selection, JSON output type, and console output are grounded in the v6.2.0 CLI source [alloy-cli]{1} [alloy-cli]{2} [alloy-cli]{3} [alloy-cli]{4}. This engagement did not install or run Alloy locally, so the recipe is source-verified rather than environment-validated.

## Disconfirming analysis

- I checked whether Alloy 6 requires temporal semantics for every model. The Alloy 6 page disconfirms that: a model without variable signatures or fields can be viewed as a one-state self-loop and collapses to usual Alloy semantics [alloy6]{4}. This supports the relational-only recommendation for static v0 invariants.
- I checked whether CLI use should rely on an older `runalloy` command. No fetched current source names `runalloy`; the v6.2.0 release notes point to `java -jar alloy.jar help`, and the CLI source defines `exec` and `commands` behaviors [alloy-release]{2} [alloy-cli]{1} [alloy-cli]{9}. Therefore this brief does not recommend `runalloy` as an attested current command.
- I checked whether temporal model checking is zero-dependency. The Alloy 6 page and download page both mention NuSMV/nuXmv for complete/temporal model checking, and the v6.2.0 release notes recommend updated NuSMV/nuXmv releases [alloy6]{6} [alloy-download]{3} [alloy-release]{3}. This does not affect relational-only v0 use, but it matters if later models use unbounded/complete temporal checks.

## Contradictions and tensions

| Handles | Relationship | Note |
|---|---|---|
| `alloy-download`, `alloy-release`, `alloy6` | tension / qualifies | The download page says Alloy 6 temporal model-checking relies on NuSMV or nuXmv in `PATH` [alloy-download]{3}; the Alloy 6 page narrows this around complete model checking with `1.. steps` [alloy6]{6}; the v6.2.0 release notes say Electrod now handles all of Alloy including integers and recommend updated NuSMV/nuXmv [alloy-release]{3}. Treat NuSMV/nuXmv as relevant for temporal/complete workflows, not for relational-only SAT4J checks. |
| `alloy-download`, `alloy-release`, `alloy-cli` | qualifies | The download page documents `java -jar org.alloytools.alloy.dist.jar` as executing the jar [alloy-download]{2}; the release notes say CLI help is available through `java -jar alloy.jar help` [alloy-release]{2}; the CLI source specifies the `exec` and `commands` subcommands [alloy-cli]{1} [alloy-cli]{9}. The jar invocation is stable across these sources, while detailed CLI options come from source/help. |

## Revisit if

- Patchbay models revocation as an event with post-state obligations rather than as a static relation; then add `var`, event predicates, `after`/prime, and `always`/`eventually` properties [alloy6]{1} [alloy6]{2} [alloy-book-overview]{3}.
- Operators want complete temporal model checking; then validate NuSMV/nuXmv installation and `steps` scopes in the actual environment [alloy6]{6} [alloy-download]{3}.
- The project upgrades beyond Alloy 6.2.0; refresh release assets, CLI help, and CLI source before encoding commands in automation [alloy-release]{1} [alloy-cli]{1}.
- The CLI exact spelling of short flags matters; fetch or generate `java -jar org.alloytools.alloy.dist.jar help exec` output from the release jar before documenting short-form commands [alloy-release]{2}.
