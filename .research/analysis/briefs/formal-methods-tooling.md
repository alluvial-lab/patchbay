---
provenance: agent-synthesis
updated: 2026-07-01
engagement: formal-methods-tooling
research_item: feature-research-formal-methods-tooling
intent: inform-architecture-decision
output_kind: synthesis-brief
---

# Formal-methods tooling for Patchbay verification models

## Summary

Patchbay's verification posture (`docs/VERIFICATION.md`) commits to TLA+ as the semantic baseline, Quint as the ergonomic authoring candidate, and Alloy for bounded relational invariants. This engagement banked the current (2026) toolchains for all three from primary sources, then empirically validated that they install and check in this environment. The load-bearing Q1 question — does "Quint-primary-checked-via-TLC" work? — is answered **yes**: `quint verify --backend tlc` runs TLC end-to-end and produces counterexamples. All three hello-world artifacts pass.

## Sources attested (this engagement)

30 per-source attestations under `.research/attestation/`, authored by three parallel specialists:

- **Quint** (8 attestations): `quint-getting-started`, `quint-language`, `quint-builtin`, `quint-model-checkers`, `quint-checking-properties`, `quint-npm-registry`, `quint-npm-tarball`, `quint-changelog`
- **TLA+/TLC** (14 attestations): `tlaplus-use`, `tlaplus-release-v174`, `tlc-cli-v174`, `tlc-config-v174`, `tlc-output-messages-v174`, `tlc-liveness-v174`, `tla-examples-grammar`, `tla-examples-diehard`, `tla-examples-liveness`, `tla-examples-hourclock`, `tla-examples-mc-out`, `quint-docs-cli`, `quint-model-checkers` (shared), `quint-tlc-source`
- **Alloy 6** (8 attestations): `alloy6`, `alloy-book-overview`, `alloy-book-structural`, `alloy-book-relational`, `alloy-download`, `alloy-release`, `alloy-cli`, `alloy-repo`

Citation lint: 356 resolved/non-broken citations, 0 broken, 0 thin across the three within-specialist briefs. Pattern flags are version-pinning warns (correct to keep) and comparative-superlative cautions (reviewed; retained where load-bearing).

## Specialist briefs

- `.research/analysis/briefs/formal-methods-tooling-quint.md` — Quint facet
- `.research/analysis/briefs/formal-methods-tooling-tla.md` — TLA+/TLC facet
- `.research/analysis/briefs/formal-methods-tooling-alloy.md` — Alloy 6 facet

## Q1 verdict — Quint-primary-checked-via-TLC: CONFIRMED

The engagement's load-bearing question was whether "Quint-primary-checked-via-TLC" (the Q1 design choice for `feature-formal-model-seed`) is viable in this environment. **It is.** Empirally validated, 2026-07-01 (exit codes verified by the orchestrator and re-verified by the adversarial-read gate):

- **Quint 0.32.0 installs** (npm `@informalsystems/quint`; required a user-writable npm prefix — `~/.npm-global` — because the global npm prefix is not user-writable here; the standard non-root workaround). [quint-getting-started]{1} [quint-npm-registry]{1}
- **`quint run`** (Rust evaluator simulator) checks invariants and finds violations — auto-installs its Rust evaluator v0.6.0 cleanly. [quint-npm-tarball]{3}
- **`quint verify`** (Apalache backend, default) checks invariants and produces counterexamples in `.tla`/`.out`/`.itf.json`. **On a found violation it exits non-zero** (exit 1) — correct checker semantics, not an error. [quint-npm-tarball]{5} [quint-model-checkers]{7}
- **`quint verify --backend tlc`** — the load-bearing path — compiles to TLA+ through Apalache, generates a TLC config (`INIT q_init`, `NEXT q_step`, optional `INVARIANT q_inv`), spawns `tlc2.TLC` via Java (using the Apalache-distribution jar on the classpath), and **finds the invariant violation, exit 1** (non-zero = counterexample found, as expected). This is the Q1 confirmation: a Quint-authored model checks through TLC end-to-end. [quint-npm-tarball]{8} [quint-tlc-source]{3} [quint-tlc-source]{4} [quint-tlc-source]{6} [quint-tlc-source]{8}
- **`quint compile --target tlaplus`** emits TLA+ (verified; emits a valid `MODULE Counter` with `EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants`). [quint-npm-tarball]{2} [quint-npm-tarball]{11}

{inferred: operational} The Q1 design choice holds: author in Quint for ergonomics, check invariants via Quint's Apalache backend (default), and check temporal/liveness properties via `quint verify --backend tlc`. No fallback to pure-TLA+ is required for the v0 seed-model requirements (bounded invariant checking + finite-state temporal checking via TLC). {extends: the sources establish the mechanism; the "no fallback needed" conclusion is a design inference bounded to current seed-model requirements, not a permanent commitment}.

### Cross-specialist tension (resolved, stated plainly)

The Quint specialist and the TLA+ specialist both investigated the Quint→TLC path and surfaced a **jar-path divergence** (recorded in Checkpoint B):

- `quint verify --backend tlc` invokes TLC through an **Apalache-distribution jar** (auto-downloaded by Quint), not the standalone `tla2tools.jar`.
- The TLA+ specialist's standalone recipe uses **`tla2tools-1.7.4.jar`** directly (SHA-1 `bee4a54f...`, verified against the GitHub release asset).

Both are valid paths to TLC; they are not contradictory. The implication for `feature-formal-model-seed`: if Patchbay authors in Quint and checks via `quint verify --backend tlc`, it does not need to separately download `tla2tools.jar` — the Apalache jar is sufficient. If Patchbay wants to check Quint-emitted TLA+ manually (or check hand-written TLA+), the standalone `tla2tools-1.7.4.jar` is the path. Either is fine; the model-promotion rule's "documented tool invocation" should name *which* jar is on the classpath for each checked model.

## Per-language findings (cross-composed)

### Quint

- **Package**: `@informalsystems/quint` (npm); current version `0.32.0`. Install: `npm i @informalsystems/quint -g` (or to a user prefix under non-root). Requires Node; verified on Node 24.
- **Authoring syntax**: modules, `var`, `action` (`init`/`step`), `pure def`, typed (`var x: int`), `val` for invariants. Ergonomic; type-checked.
- **Temporal**: `always`/`eventually`/`leadsTo` supported. **Apalache temporal support is partial** — the Quint docs and changelog warn on temporal formulas with Apalache. **For temporal/liveness, use `--backend tlc`**; for safety invariants, Apalache (default) is fine.
- **Checking path**: `quint verify` (Apalache, default, bounded invariant checking) and `quint verify --backend tlc` (TLC, temporal). `quint run` for simulation. `quint compile --target tlaplus` emits TLA+.
- **Idioms for Patchbay properties** — the specialist authored Quint snippets for terminal-finality, idempotent retry, and monotonic generation (in the Quint brief). These are source-grounded templates, now empirically parseable (`quint parse` succeeded on the hello-world).
- **Stale-doc hazard** (resolved): the model-checkers page says TLC is "not integrated with Quint," but the current npm tarball + changelog (v0.31.0) confirm `--backend tlc` IS implemented. Treat the page prose as stale for this point.

### TLA+/TLC

- **Artifact**: `tla2tools-1.7.4.jar` (GitHub release v1.7.4); SHA-1 verified `bee4a54f3ee3d4afc347c3240ec2d9e93b075104`. Requires Java 11+; verified on Java 21.
- **CLI**: `java -jar ./tla2tools-1.7.4.jar -config <spec>.cfg -workers auto <spec>.tla`. **Do not add a `TLC` token after the jar** — `java -jar tla2tools.jar` aliases `tlc2.TLC`; an extra `TLC` token is undocumented and a risk.
- **Config shape** (`.cfg`): `SPECIFICATION <Spec>`, `INVARIANT <inv>`, `PROPERTY <temporal>`. TLC defaults to `<spec>.cfg` if `-config` omitted.
- **Deadlock**: checking is enabled by default; `-deadlock` *disables* it (counterintuitive).
- **Liveness**: TLC checks `<>[]`-style temporal properties; v1.7.4 release notes flag a fixed liveness issue with multiple workers — `--workers auto` is sourced and safe; be cautious with explicit multi-worker liveness runs.
- **Hello-world verified**: `Counter.tla` + `Counter.cfg` → "Model checking completed. No error has been found." (exit 0).

### Alloy 6

- **Artifact**: `org.alloytools.alloy.dist.jar` v6.2.0 (GitHub release; published 2025-01-09). Requires "Java 6 or later"; verified on Java 21.
- **CLI**: `java -jar org.alloytools.alloy.dist.jar commands <file>.als` (list); `java -jar org.alloytools.alloy.dist.jar exec --command <label> --type {json|text|table|xml} --output - <file>.als` (run headless). No `runalloy` command in current sources (that's older).
- **Scope**: put in the command (`check ActorIdsUnique for 5`), not a CLI flag.
- **Temporal operators**: Alloy 6 added them (`after`/`always`/`eventually`/`until`/`'`), but **complete temporal model checking needs NuSMV/nuXmv installed by the user**. [alloy6]{6} [alloy-download]{3} For Patchbay's v0 relational invariants (identity uniqueness, authority-graph shape, anti-spoofing), **relational-only is sufficient** — no temporal operators, no NuSMV dependency. {extends: the sources establish that static/non-`var` Alloy models collapse to one-state relational semantics [alloy6]{4}; the "sufficient for Patchbay v0" conclusion is a design inference, bounded to the current checked-normative property set (identity/authority-graph/anti-spoofing shapes). If revocation/lease/future-routing semantics enter v0, the Alloy brief flags that as crossing into temporal modeling, which would need NuSMV}. This is a clean v0 scope: relational shapes only.
- **Idioms**: the specialist authored Alloy snippets for actor-identity uniqueness (`id in Actor lone -> Identity`), authority-graph acyclicity (`no a: Actor | a in a.^grants`), and anti-spoofing (in the Alloy brief). [alloy-book-relational]{1} [alloy-book-structural]{7} [alloy-book-structural]{9} **Anti-spoofing caveat**: the Alloy brief notes that modeling "authenticated sender matches claimed identity" is a relational shape, but the *binding* of an authenticated identity to a transport/session is a dynamic property (CompoundIssuer-style) better suited to the TLA+/Quint model than to Alloy — Alloy models the *consistency shape* (sender ≠ self-asserted), not the verification *action*.
- **Hello-world verified**: `patchbay-invariants.als` → `check ActorIdsUniqueAssert for 5` exits 0 (no counterexample found).

## Verified hello-world artifacts

Checked into `specs/seed/` and run in this environment (2026-07-01; re-verified by the adversarial-read gate):

- `Counter.qnt` — Quint counter model (invariant `not_two`): `quint run` and `quint verify` find the expected counterexample (exit 1 = violation found, as designed). [quint-language]{3} [quint-language]{5} [quint-model-checkers]{6}
- `Counter.tla` + `Counter.cfg` — TLA+/TLC counter (invariant `TypeOK`, property `EventuallyThree`): standalone TLC reports "Model checking completed. No error has been found." (exit 0). [tlaplus-use]{4} [tlc-cli-v174]{1} [tlc-config-v174]{2}
- `patchbay-invariants.als` — Alloy actor-identity-uniqueness assertion: `check ActorIdsUniqueAssert for 5` finds no counterexample (exit 0). [alloy-book-structural]{4} [alloy-book-structural]{7} [alloy-cli]{7}

Each was authored from the specialist briefs' source-grounded recipes. The tool jars (`tla2tools-1.7.4.jar`, `org.alloytools.alloy.dist.jar`) are downloaded locally; not committed (binary artifacts — operators re-fetch per the briefs' install steps).

## Handoff to feature-formal-model-seed

This engagement answers the seed-model design questions:

- **Q1 (authoring language)**: Quint-primary-checked-via-TLC, confirmed. Author in Quint; check invariants via `quint verify` (Apalache); check temporal/liveness via `quint verify --backend tlc`. No pure-TLA+ fallback needed.
- **Q2 (decomposition)**: informed — Quint's mature invariant checking + TLC's liveness path + Alloy's relational-only sufficiency together support the "clustered by shared state" decomposition.
- **Q3 (Alloy scope)**: relational-only for v0 (identity/authority-graph/anti-spoofing). Temporal Alloy needs NuSMV — out of v0 scope. Leases (if promoted) would need the temporal path.
- **Model-promotion tool invocation**: for each checked model, record which tool + jar path + exact CLI was used (Apalache-jar via Quint, or standalone `tla2tools-1.7.4.jar`, or `org.alloytools.alloy.dist.jar`).

## Acquisition candidates

Enriching (proactive lookout, none blocking):

- Runtime validation of `@informalsystems/quint@0.32.0` — validate exact `quint --help` output and parse/typecheck of Patchbay idiom snippets beyond the hello-world. (Now partially discharged by this engagement's environment validation; a deeper run remains enriching.)
- TLC `-help` output from the pinned `tla2tools-1.7.4.jar` — capture exact runtime help as executed in-target. (Completes the CLI reference beyond source-level docs.)
- Alloy CLI `help exec` output — confirm exact option ordering/short flags beyond the source-level `exec` option names.

No `blocking` candidates — all load-bearing sources were fetched.

## Revisit if

- Quint's CLI defaults, `--backend tlc`, Apalache version pin, or temporal support change materially (these are moving surfaces per the changelog).
- Patchbay needs temporal Alloy model checking (introduces the NuSMV/nuXmv dependency).
- A Patchbay property turns out to need a checking feature the current toolchain doesn't support (e.g. a liveness property Apalache can't bound and TLC can't scale to).
- The standalone `tla2tools.jar` release advances past v1.7.4 with breaking CLI changes.
