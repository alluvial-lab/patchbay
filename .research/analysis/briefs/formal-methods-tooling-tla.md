---
provenance: agent-synthesis
updated: 2026-07-01
facet: tla-tlc
engagement: formal-methods-tooling
---

# TLA+/TLC tooling brief

## Bottom line for a durable checked artifact

Pin a `tla2tools.jar` release artifact, keep the `.tla` and `.cfg` files in version control, and record the exact Java invocation used to check them. The GitHub Releases `latest` endpoint fetched for this engagement returned `v1.7.4`, with a `tla2tools.jar` release asset and SHA-1 `bee4a54f3ee3d4afc347c3240ec2d9e93b075104`; the TLA+ tools documentation says the tools require Java 11+ and that `java -jar tla2tools.jar` aliases `tlc2.TLC`.[tlaplus-release-v174]{1}[tlaplus-release-v174]{2}[tlaplus-release-v174]{4}[tlaplus-use]{1}[tlaplus-use]{4}

Use the jar form without an extra `TLC` subcommand: `java -jar ./tla2tools-1.7.4.jar -config Counter.cfg -workers auto Counter.tla`. The fetched TLA+ docs document `java -jar tla2tools.jar` as the TLC alias, while the TLC source documents model checking as `java tlc2.TLC [-modelcheck] spec[.tla]` and accepts one non-option argument as the input module; adding an undocumented `TLC` token after the jar is therefore a risk, not a sourced invocation shape.{inferred: command-token mapping} [tlaplus-use]{4}[tlc-cli-v174]{1}[tlc-cli-v174]{8}

## TLA+ syntax essentials to preserve in Patchbay models

A TLA+ module has a dashed `MODULE` header, optional `EXTENDS`, module units, and a closing line of equals signs; variable declarations use `VARIABLE` or `VARIABLES`, and operator definitions use `==`.[tla-examples-grammar]{3}[tla-examples-grammar]{4}[tla-examples-grammar]{5}

The common transition-system pattern is `Init`, `Next`, a tuple of variables, and `Spec == Init /\ [][Next]_vars`; the official `DieHard` example defines constants, variables, `Init`, action operators with primed variables, `Next` as an action disjunction, and `Spec == Init /\ [][Next]_<<big, small>>`.[tla-examples-diehard]{1}[tla-examples-diehard]{3}[tla-examples-diehard]{4}[tla-examples-diehard]{5}[tla-examples-diehard]{6}

`UNCHANGED`, `[]`, `<>`, `[A]_v`, `WF_v(A)`, and `SF_v(A)` are syntactic forms in the fetched grammar/example corpus; the examples include weak and strong fairness formulas and temporal formulas such as `[]<>(hr = h)` and `[]((now # 4) => <>[](now # 4))`.[tla-examples-grammar]{2}[tla-examples-grammar]{6}[tla-examples-grammar]{7}[tla-examples-grammar]{8}[tla-examples-liveness]{2}[tla-examples-liveness]{3}[tla-examples-hourclock]{6}[tla-examples-hourclock]{8}

For TLC invariants, keep each invariant as a state predicate with no primes or temporal operators; TLC has an explicit message for an invariant that is not a state predicate.[tlc-output-messages-v174]{4}

## TLC CLI facts to pin

TLC v1.7.4 documents model checking as `java tlc2.TLC [-modelcheck] spec[.tla]`; `java -jar tla2tools.jar` is documented separately as an alias to `tlc2.TLC`.[tlc-cli-v174]{1}[tlaplus-use]{4}

`-config file` selects the configuration file, and TLC defaults to `spec.cfg` when `-config` is omitted; the v1.7.4 parser accepts `Counter.cfg` and strips the `.cfg` suffix internally.[tlc-cli-v174]{2}[tlc-cli-v174]{5}

Deadlock checking is enabled by default; the `-deadlock` flag is counterintuitive because it disables deadlock checking, and the config file also supports `CHECK_DEADLOCK TRUE|FALSE`.[tlc-cli-v174]{3}[tlc-cli-v174]{6}[tlc-config-v174]{6}

`-workers` accepts a positive integer or `auto`; `auto` maps to the JVM's available processor count in the v1.7.4 parser.[tlc-cli-v174]{4}[tlc-cli-v174]{7}

`-lncheck` requires a following strategy string; do not include it in the baseline recipe unless a model deliberately needs a documented liveness-check scheduling policy.[tlc-cli-v174]{9}

## `.cfg` shape

TLC config keywords include `SPECIFICATION`, `INIT`, `NEXT`, `INVARIANT`/`INVARIANTS`, `PROPERTY`/`PROPERTIES`, constants, constraints, action constraints, view, symmetry, and deadlock checking.[tlc-config-v174]{1}

Use either a single `SPECIFICATION Spec` line or the pair `INIT Init` and `NEXT Next`; the parser consumes one following identifier for each of `SPECIFICATION`, `INIT`, and `NEXT`, and rejects duplicates.[tlc-config-v174]{2}

Use `INVARIANT`/`INVARIANTS` for state predicates and `PROPERTY`/`PROPERTIES` for temporal properties; both forms consume identifier lists until another config keyword or EOF.[tlc-config-v174]{3}[tlc-config-v174]{4}

Constants can be assigned with `=` or overridden with `<-`; directly parsed config values include numbers, strings, booleans, set enumerations, and model values.[tlc-config-v174]{7}

## Invariant vs temporal-property checking

Invariant violations are reported as either an initial-state violation or a behavior violation, and TLC prints messages such as `Invariant %1% is violated by the initial state` or `Invariant %1% is violated.`.[tlc-output-messages-v174]{2}[tlc-output-messages-v174]{3}

Temporal-property violations are handled through TLC's liveness machinery: the liveness worker detects violating strongly connected components, prints `Temporal properties were violated.`, prints `The following behavior constitutes a counter-example:`, constructs a prefix plus cycle, prints states, and ends with a stuttering or back-to-state marker.[tlc-liveness-v174]{1}[tlc-liveness-v174]{2}[tlc-liveness-v174]{3}[tlc-liveness-v174]{4}[tlc-output-messages-v174]{5}[tlc-output-messages-v174]{6}

The v1.7.4 release notes specifically identify a fixed liveness-checking issue involving multiple workers failing to report a property violation, so Patchbay should record the TLC version next to any liveness result.[tlaplus-release-v174]{3}

## Counterexample and output shape

For command-line runs, treat TLC output as process output and capture it explicitly, for example with `| tee Counter.out`; the TLA+ tools documentation says non-Java applications should run Java on the jar and capture process output.[tlaplus-use]{6}

Toolbox models conventionally capture TLC output in `MC.out`; a fetched example `MC.out` includes TLC version, model-checking mode, SANY parsing, initial-state computation, progress, temporal-property checking, success/fingerprint statistics, coverage, state counts, depth, and finished markers.[tla-examples-mc-out]{1}[tla-examples-mc-out]{2}[tla-examples-mc-out]{3}[tla-examples-mc-out]{4}[tla-examples-mc-out]{5}[tla-examples-mc-out]{6}[tla-examples-mc-out]{7}

On a safety failure, expect an `Error: ` prefix, an invariant/property message, `The behavior up to this point is:`, and numbered state output; on a liveness failure, expect `Temporal properties were violated.`, `The following behavior constitutes a counter-example:`, numbered states, and a stuttering or back-to-state marker.[tlc-output-messages-v174]{1}[tlc-output-messages-v174]{2}[tlc-output-messages-v174]{3}[tlc-output-messages-v174]{7}[tlc-output-messages-v174]{8}[tlc-liveness-v174]{5}

## Quint-emitted TLA+ through TLC

Current Quint docs expose TLC as `quint verify --backend tlc myspec.qnt`, with `--backend` choices `apalache` and `tlc`, a `--tlc-config` JSON option, and runtime JSON fields such as `maxHeap`, `stackSize`, and `workers`.[quint-docs-cli]{4}[quint-docs-cli]{6}[quint-docs-cli]{8}

Quint docs also expose `quint compile --target tlaplus` with `--init`, `--step`, `--invariant`, and `--temporal` options, and state that TLA+ output requires flattening.[quint-docs-cli]{2}[quint-docs-cli]{3}

The Quint TLC backend source generates a TLC cfg with `INIT q_init`, `NEXT q_step`, optional `INVARIANT q_inv`, and optional `PROPERTY q_temporalProps`, then writes temporary `<module>.tla` and `<module>.cfg` files.[quint-tlc-source]{3}[quint-tlc-source]{4}[quint-tlc-source]{5}[quint-tlc-source]{7}

The same source spawns Java as `java <heap> <stack> -Djava.io.tmpdir=<tmp> -cp <jarPath> tlc2.TLC -deadlock -workers <workers> -metadir <tmp> <tmp>/<module>.tla`, where the jar path is an Apalache distribution jar rather than a separately downloaded `tla2tools.jar`.[quint-tlc-source]{1}[quint-tlc-source]{6}[quint-tlc-source]{8}

For a manual check of a Quint-emitted TLA+ file with standalone `tla2tools.jar`, mirror Quint's generated names in the cfg (`INIT q_init`, `NEXT q_step`, `INVARIANT q_inv`, `PROPERTY q_temporalProps`) or inspect the emitted TLA+ for the generated operator names before writing the cfg.{inferred: applies Quint-generated cfg names to standalone TLC} [quint-tlc-source]{3}[quint-tlc-source]{4}[quint-tlc-source]{5}[tlc-config-v174]{2}[tlc-config-v174]{3}[tlc-config-v174]{4}

## Hello-world checked artifact recipe

Download the pinned jar and verify the release checksum before running TLC; the URL and SHA-1 below are sourced from the fetched GitHub release metadata.[tlaplus-release-v174]{2}[tlaplus-release-v174]{4}

```sh
curl -L -o tla2tools-1.7.4.jar \
  https://github.com/tlaplus/tlaplus/releases/download/v1.7.4/tla2tools.jar
printf '%s  %s\n' \
  bee4a54f3ee3d4afc347c3240ec2d9e93b075104 \
  tla2tools-1.7.4.jar | sha1sum -c -
```

Create `Counter.tla` using the module/declaration/operator/fairness forms attested above.[tla-examples-grammar]{3}[tla-examples-grammar]{4}[tla-examples-grammar]{5}[tla-examples-grammar]{7}[tla-examples-grammar]{8}[tla-examples-hourclock]{6}

```tla
---- MODULE Counter ----
EXTENDS Naturals

VARIABLE x
vars == <<x>>

Init == x = 0
Inc == /\ x < 3
       /\ x' = x + 1
Stay == /\ x = 3
        /\ UNCHANGED x
Next == Inc \/ Stay

Spec == /\ Init
        /\ [][Next]_vars
        /\ WF_vars(Inc)

TypeOK == x \in 0..3
EventuallyThree == <>[](x = 3)
====
```

Create `Counter.cfg` with a `SPECIFICATION`, one invariant, and one temporal property; this is the config shape parsed by TLC v1.7.4.[tlc-config-v174]{2}[tlc-config-v174]{3}[tlc-config-v174]{4}

```cfg
SPECIFICATION Spec
INVARIANT TypeOK
PROPERTY EventuallyThree
```

Run TLC without `-deadlock` so default deadlock checking remains enabled, with `-workers auto` sourced from the v1.7.4 parser, and capture output for promotion evidence.[tlc-cli-v174]{3}[tlc-cli-v174]{6}[tlc-cli-v174]{7}[tlaplus-use]{6}

```sh
java -jar ./tla2tools-1.7.4.jar -config Counter.cfg -workers auto Counter.tla | tee Counter.out
```

A promotion bundle should include `Counter.tla`, `Counter.cfg`, `Counter.out`, the jar version/download URL/checksum, and the exact command above; those fields are the stable checked artifact and documented invocation required for repeatability.{inferred: artifact bundle from sourced invocation and output-capture facts} [tlaplus-release-v174]{1}[tlaplus-release-v174]{2}[tlaplus-release-v174]{4}[tlc-cli-v174]{2}[tlc-cli-v174]{7}[tlaplus-use]{6}

## Disconfirming analysis

I looked for support for `java -jar tla2tools.jar TLC ...` and did not find it in the fetched TLA+ usage document or TLC v1.7.4 source; the fetched usage document instead says `java -jar tla2tools.jar` aliases `tlc2.TLC`, and the TLC parser treats the non-option token as the input module.[tlaplus-use]{4}[tlc-cli-v174]{8}

I did not use master-branch-only CLI conveniences in the baseline recipe: master `USE.md` mentions JSON trace dump/load, but the v1.7.4 TLC source comment fetched for the pinned release does not provide the same attested baseline in its documented option list, so the recipe relies only on `-config`, `-workers`, default deadlock checking, and ordinary stdout capture.[tlaplus-use]{6}[tlc-cli-v174]{2}[tlc-cli-v174]{3}[tlc-cli-v174]{4}

I checked Quint's docs and source because docs say TLC is used via transpilation and the source shows the concrete runtime path; the source's current TLC backend uses an Apalache-distribution jar on the classpath, not the standalone `tla2tools.jar` release artifact.[quint-model-checkers]{2}[quint-model-checkers]{5}[quint-tlc-source]{6}[quint-tlc-source]{8}

## Contradictions

| Handles | Relationship | Note |
|---|---|---|
| `quint-model-checkers`, `quint-docs-cli`, `quint-tlc-source` | tension | The model-checkers page says TLC is “not integrated with Quint” and requires transpilation, while the CLI docs expose `quint verify --backend tlc` and the source implements that backend by compiling to TLA+ and spawning TLC.[quint-model-checkers]{5}[quint-docs-cli]{6}[quint-tlc-source]{8} |
| `tlaplus-use`, `tlc-cli-v174` | qualifies | The usage document says `java -jar tla2tools.jar` aliases TLC, while the v1.7.4 TLC source documents the underlying class invocation as `java tlc2.TLC`; together they support jar-without-subcommand and classpath forms, not a jar `TLC` subcommand.[tlaplus-use]{4}[tlc-cli-v174]{1} |

## Revisit if

- Revisit when `tlaplus/tlaplus` `/releases/latest` no longer returns `v1.7.4`, because the pinned download URL, checksum, and release-note caveats would change.[tlaplus-release-v174]{1}[tlaplus-release-v174]{2}[tlaplus-release-v174]{4}
- Revisit if Patchbay adopts Quint as the authoring surface, because Quint's TLC backend currently invokes TLC through an Apalache jar and generated `q_*` cfg names rather than the standalone `tla2tools.jar` flow.[quint-tlc-source]{3}[quint-tlc-source]{4}[quint-tlc-source]{5}[quint-tlc-source]{6}[quint-tlc-source]{8}
- Revisit before relying on liveness results with multiple TLC workers, because the fetched v1.7.4 release notes specifically call out a liveness multi-worker unsoundness fix.[tlaplus-release-v174]{3}
