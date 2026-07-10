---
name: tla-plus
description: >
  TLA+ specification language and TLC model-checker. Auto-loads when authoring or checking
  TLA+ specifications (.tla files), TLC config files (.cfg), temporal properties, invariants,
  or invoking tla2tools.jar / tlc2.TLC. Patchbay's semantic baseline for durable protocol models.
user-invocable: false
---

# TLA+ / TLC reference (tla2tools v1.7.4)

TLA+ is Patchbay's semantic baseline for durable, long-lived protocol models. TLC is the explicit-state model-checker. Author TLA+ directly for hand-written baseline models, or check Quint-emitted TLA+ via TLC (Quint's `--backend tlc` produces TLA+ internally).

**Install** (download the pinned jar; do not commit the binary):
```sh
curl -L -o tla2tools-1.7.4.jar \
  https://github.com/tlaplus/tlaplus/releases/download/v1.7.4/tla2tools.jar
printf '%s  %s\n' bee4a54f3ee3d4afc347c3240ec2d9e93b075104 tla2tools-1.7.4.jar | sha1sum -c -
```
Java 11+ required; verified on Java 21. SHA-1 pins the v1.7.4 release artifact.

## Module syntax

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
- `EXTENDS Naturals` — imports (also `Integers`, `Sequences`, `FiniteSets`, `TLC`).
- `VARIABLE x` — state variable; `vars == <<x>>` — tuple for `[][Next]_vars`.
- `Init` — initial predicate; `Next` — next-state relation; `x'` — next-state value.
- `UNCHANGED x` — state unchanged.
- `Spec` — the complete behavior: Init ∧ always [Next]_vars ∧ fairness.
- `WF_vars(Inc)` — weak fairness; `SF_vars(Inc)` — strong fairness.
- Invariants: `TypeOK == x \in 0..3`. Temporal: `EventuallyThree == <>[](x = 3)`.

## Config file (.cfg)

```cfg
SPECIFICATION Spec
INVARIANT TypeOK
PROPERTY EventuallyThree
```
- `SPECIFICATION <Spec>` — names the behavior spec to check.
- `INVARIANT <inv>` — state invariant (checked every state).
- `PROPERTY <temporal>` — temporal property (liveness/safety over behaviors).
- TLC defaults to `<spec>.cfg` if `-config` omitted.

## Checking with TLC

```sh
java -jar ./tla2tools-1.7.4.jar -config Counter.cfg -workers auto Counter.tla
```
- **Do NOT add a `TLC` token after the jar** — `java -jar tla2tools.jar` aliases `tlc2.TLC`; an extra `TLC` token is undocumented and a risk.
- `-config <file>` — selects the config; defaults to `spec.cfg`.
- `-workers auto` — uses all CPU cores (or a positive integer).
- `-deadlock` — **disables** deadlock checking (counterintuitive); default is deadlock-checking ON.

**Exit-code semantics:** exit 0 = "Model checking completed. No error has been found." A violation produces a non-zero exit and a counterexample trace.

## Counterexample output

On an invariant violation, TLC prints the trace (`State 1: <Initial predicate>` ... `State N: <action>`). On a temporal-property failure, it prints a behavior trace. Output goes to stdout; TLC also writes `.out` files when configured.

## Liveness caveat

TLC checks `<>[]`-style temporal properties. The v1.7.4 release notes flag a fixed liveness-checking issue involving multiple workers failing to report — prefer `-workers auto` (sourced/safe) over explicit multi-worker liveness runs until verified.

## Checking Quint-emitted TLA+

`quint verify --backend tlc` does this internally, but for a **manual** check of Quint-emitted TLA+:
1. `quint compile <file>.qnt --target tlaplus > Counter.tla` (emits TLA+ with generated names).
2. Inspect the emitted TLA+ for the generated operator names: `q_init`, `q_step`, `q_inv`, `q_temporalProps`.
3. Write a `.cfg` mirroring them: `INIT q_init`, `NEXT q_step`, `INVARIANT q_inv`, `PROPERTY q_temporalProps`.
4. Run TLC: `java -jar tla2tools-1.7.4.jar -config Counter.cfg Counter.tla`.

**Jar-path distinction (load-bearing):** `quint verify --backend tlc` uses an **Apalache-distribution jar** on the classpath, not the standalone `tla2tools.jar`. For manual checks, use the standalone `tla2tools-1.7.4.jar`. Both reach TLC; they are different classpaths.

## Pitfalls

- `-deadlock` disables checking — the name is the opposite of intent.
- Default config filename is `<spec>.cfg`, not `<module>.cfg` — they usually match but not always.
- Quint-emitted TLA+ `EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants` — needs the Apalache jar on the classpath if run manually without flattening.

## Reference

Hello-world artifact: `.agents/skills/tla-plus/examples/Counter.tla` + `.agents/skills/tla-plus/examples/Counter.cfg`. Source briefs: `.research/analysis/briefs/formal-methods-tooling-tla.md` + `.research/analysis/briefs/formal-methods-tooling.md`. Attestations: `.research/attestation/tla*-*.md` + `.research/attestation/tlc-*-v174.md` + `.research/attestation/quint-tlc-source.md`.
