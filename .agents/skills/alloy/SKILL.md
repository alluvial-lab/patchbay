---
name: alloy
description: >
  Alloy 6 specification language and analyzer (AlloyTools / MIT). Auto-loads when authoring or
  checking Alloy specifications (.als files), relational invariants, sig/pred/fact/assert/check,
  or invoking the Alloy CLI headless checker. Patchbay's tool for bounded relational invariants.
user-invocable: false
---

# Alloy 6 reference (v6.2.0)

Alloy is Patchbay's tool for bounded relational invariants: actor-identity uniqueness, authority-graph constraints, anti-spoofing relationships. **Patchbay v0 uses relational-only Alloy** — no temporal operators, no NuSMV dependency. Temporal Alloy is out of v0 scope.

**Install** (download the pinned jar; do not commit the binary):
```sh
curl -L -o org.alloytools.alloy.dist.jar \
  https://github.com/AlloyTools/org.alloytools.alloy/releases/download/v6.2.0/org.alloytools.alloy.dist.jar
```
Java 6+ required (the repo's stated floor); verified on Java 21. Release v6.2.0 published 2025-01-09.

## Module syntax

```alloy
sig Identity {}
sig Actor { id: one Identity }

fact ActorIdsUnique {
  id in Actor lone -> Identity
}

assert ActorIdsUniqueAssert {
  all disj a, b: Actor | a.id != b.id
}

check ActorIdsUniqueAssert for 5
```
- `sig` — signature (a set of atoms); fields declare relations.
- `one` / `lone` / `set` / `some` — multiplicities (`one Identity` = exactly one; `lone ->` = injective).
- `pred` — predicate; `fact` — always-true constraint; `fun` — function; `assert` — claim to check.
- `check <label> for N` — run a check up to scope N (atoms per top-level sig). **Put scope in the command, not a CLI flag.**
- `all disj a, b: Actor` — quantify over distinct atoms; `^` — transitive closure; `.` — join.

## Headless CLI

```sh
# list commands in a file
java -jar org.alloytools.alloy.dist.jar commands <file>.als

# run a check headless (no Analyzer GUI)
java -jar org.alloytools.alloy.dist.jar exec --command <label> --type json --output - <file>.als
```
- `commands` — prints all `run`/`check` commands with zero-based indexes.
- `exec` — runs a command headless; `--command <label>` selects by label/glob/index; `--type {none|text|table|json|xml}` output format; `--output -` sends to console.
- No `runalloy` command in current sources (that's older tooling).

**Exit-code semantics:** exit 0 = command ran. For `check`, a counterexample found prints `SAT` (the negated assertion is satisfiable); `UNSAT` = no counterexample found within scope (the assertion holds). Examine output, not just exit code.

**Measurement discipline (load-bearing):** verify assertions with `--type text --output -` and look for a `skolem $<AssertName>_...` line. A skolem witness means a counterexample was found (assertion FAILS); its absence means `UNSAT` (assertion holds). **Do NOT use `--type json` or output-file-count to judge UNSAT** — both give false positives (reported a passing assertion on an actually-failing check in the seed-model arc). The reliable invocation: `java -jar org.alloytools.alloy.dist.jar exec --command <label> --type text --output - <file>.als | grep -c skolem` (0 = holds).

## Counterexample output

On a failed `check`, Alloy finds a satisfying solution to the negated assertion (a counterexample instance where facts hold but the checked formula does not). JSON output writes a `SolutionDTO`; text output shows the instance. For relational-only models, output is a single-state instance (no trace).

## v0 scope: relational-only

Patchbay v0 relational invariants (identity uniqueness, authority-graph shape, anti-spoofing) need **only static relational Alloy** — no `var`, no temporal operators. Static/non-`var` models collapse to usual one-state Alloy semantics. This keeps v0 dependency-free (no NuSMV).

**Temporal Alloy (out of v0):** Alloy 6 added `after`/`always`/`eventually`/`until`/`'` (next-state), but complete temporal model checking needs NuSMV or nuXmv installed by the user. If revocation/lease/future-routing dynamics enter v0, that crosses into temporal modeling — re-open this scope decision then.

## Patchbay relational idioms

**Genuine-checking discipline (load-bearing):** a relational `check` must not be a tautology over a `fact`, and removing a forcing `fact` to make a check "genuine" without adding a real constraint turns vacuous-true into **actually-false** (Alloy will find a counterexample). A relational check is genuine only if the asserted property is true because of *other* constraints in the model — or it should be **demoted to draft** if no such constraint exists (the property may be inherently dynamic and belong in a TLA+/Quint model). Test: temporarily remove any fact that duplicates the assertion; if Alloy then finds a counterexample (`SAT`), the check was tautological and the property is not genuinely checkable relationally.

Actor-identity uniqueness (no two actors share an identity):
```alloy
sig Identity {}
sig Actor { id: one Identity }
fact ActorIdsUnique { id in Actor lone -> Identity }
```
The `lone ->` forces injectivity (no two source atoms point to the same target).

Authority-graph acyclicity (no grant cycles — trivially true in v0 since delegation is removed, but the shape):
```alloy
sig Actor { grants: set Actor }
fact NoGrantCycles { no a: Actor | a in a.^grants }
```
`a.^grants` is the set reachable from `a` in one or more grant steps; `no` checks absence of self-reachability.

Anti-spoofing (sender identity consistency — relational shape only):
```alloy
sig Actor {}
sig Message { sender: one Actor, claimedSender: one Actor }
fact SenderMatchesClaim { all m: Message | m.sender = m.claimedSender }
```
**Caveat:** this models the *consistency shape* (sender ≠ self-asserted). The *binding* of an authenticated identity to a transport/session is a dynamic verification action (CompoundIssuer-style) — that belongs in the TLA+/Quint model, not Alloy.

## Pitfalls

- Scope goes in the `check`/`run` command (`check X for 5`), not a CLI flag.
- `check` finding `SAT` means a counterexample WAS found (the assertion fails); `UNSAT` means it holds.
- Relational-only models: use `check X for N` (no `but N steps`); temporal models add `but N steps`.
- No `runalloy` command — use `java -jar org.alloytools.alloy.dist.jar exec`.

## Reference

Hello-world artifact: `.agents/skills/alloy/examples/patchbay-invariants.als`. Source briefs: `.research/analysis/briefs/formal-methods-tooling-alloy.md` + `.research/analysis/briefs/formal-methods-tooling.md`. Attestations: `.research/attestation/alloy*.md`.
