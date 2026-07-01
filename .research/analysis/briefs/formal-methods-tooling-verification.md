---
provenance: adversarial-read
updated: 2026-07-01
engagement: formal-methods-tooling
verification_stage: adversarial-read
verdict: NEEDS-REVISION
---

# Formal methods tooling verification checklist

Scope read: parent synthesis, Quint/TLA/Alloy specialist briefs, relevant attestations, raw Quint sources under `.research/reference/quint/`, and `specs/seed/` artifacts.

Local empirical probes run during this verification:

- `~/.npm-global/bin/quint --version` -> `0.32.0`; Java -> OpenJDK 21.
- `quint run specs/seed/Counter.qnt --invariant not_two --max-steps 3` found the expected invariant violation and exited non-zero.
- `quint verify specs/seed/Counter.qnt --invariant not_two --max-steps 3 --server-endpoint localhost:8823` used Apalache 0.56.1, found the expected invariant violation, and exited non-zero.
- `quint verify specs/seed/Counter.qnt --backend tlc --invariant not_two --server-endpoint localhost:8824` printed `Compiling to TLA+ (via Apalache)`, ran `TLC2`, reported `Invariant q_inv is violated`, and exited non-zero.
- `quint compile specs/seed/Counter.qnt --target tlaplus` emitted TLA+ with `EXTENDS Integers, Sequences, FiniteSets, TLC, Apalache, Variants`, `q_init`, and `q_step`.
- `java -jar ./tla2tools-1.7.4.jar -config specs/seed/Counter.cfg -workers auto specs/seed/Counter.tla` completed with `Model checking completed. No error has been found.`
- `java -jar ./org.alloytools.alloy.dist.jar commands specs/seed/patchbay-invariants.als` listed `Check ActorIdsUniqueAssert for 5`; `exec --command ActorIdsUniqueAssert` exited `0`.

## (a) Semantic citation-chain walk

- **Q1 verdict / mechanism**: The mechanism is semantically supported by the attestations: `quint-npm-tarball` says `verify` has backend choices `apalache`/`tlc`, `verifyWithTlcBackend` compiles to TLA+ via Apalache and calls the TLC runner, and TLA+ compilation uses Apalache; `quint-tlc-source` gives generated `INIT q_init` / `NEXT q_step`, optional `INVARIANT q_inv` / `PROPERTY q_temporalProps`, Apalache-jar lookup, and Java `tlc2.TLC` invocation. The local rerun confirms the path actually runs.
  - **Issue**: the parent synthesis Q1 section states the empirical fact without direct citations.
  - **Issue**: the parent says `quint verify --backend tlc` finds the violation with **exit 0**. In this environment it exits non-zero on the expected violation. Revise to distinguish “toolchain works and reports the expected counterexample” from “successful process exit.”
  - **Issue**: “All three hello-world artifacts pass” is ambiguous/incorrect for `Counter.qnt`, which is intentionally violation-producing. Say “produce the expected result” or split passing checks from expected counterexample checks.

- **Cross-specialist jar-path tension**: Supported and not a real contradiction. `quint-tlc-source` supports the Quint path using the Apalache distribution jar on the classpath with `tlc2.TLC`; `tlaplus-release-v174` and `tlaplus-use` support the standalone `tla2tools-1.7.4.jar` path. The synthesis resolution is substantively correct.

- **Alloy relational-only sufficiency for v0**: General source support is adequate: `alloy6` supports static/non-`var` models collapsing to usual one-state Alloy semantics, and the Alloy book attestations support signatures, fields, facts, assertions, quantifiers, joins, and transitive closure. However, “sufficient for Patchbay v0 identity/authority-graph/anti-spoofing” is a modeling/design inference, not a source fact. The Alloy specialist marks similar claims as inferred; the parent should do the same and cite the Alloy brief/attestations directly. Preserve the Alloy brief’s anti-spoofing caveat about authenticated binding vs self-asserted identity.

- **Stale-doc resolution**: The resolution is substantively correct: the model-checkers page says TLC is not integrated, while current CLI docs, tarball/source, and changelog show `--backend tlc`. However, some citations in the Quint brief point at the wrong anchor (see (g)).

- **Quint temporal/liveness caveat**: The claim is directionally supported by raw sources and by `quint-changelog` warning about temporal formulas with Apalache, plus raw package code warning that Apalache temporal support is experimental and suggests `--backend tlc`. But the attestation/citation chain is currently broken/thin: `quint-model-checkers{8}` is cited repeatedly, yet the attestation only defines anchors `{1}` through `{7}`.

## (b) Claim-shapes the mechanical lint missed

- Parent synthesis contains multiple load-bearing claims with no citations: empirical installation/checking results, Q1 “confirmed,” jar-path distinction, Alloy v0 sufficiency, and stale-doc resolution. A source-bound synthesis should cite direct attestations, not rely on adjacent specialist briefs implicitly.
- “No pure-TLA+ fallback needed for v0” is an operational/design inference. Mark it as such and bound it to current seed-model requirements.
- “Current (2026) toolchains” is mostly source-supported, but should cite npm registry/tarball, GitHub releases, and Alloy release attestations in the parent synthesis.
- “All three hello-world artifacts pass” is a smoothed phrase over mixed outcomes: Quint counterexample expected/found, standalone TLC no error, Alloy no counterexample.

## (c) Coherence-read for smoothed contradictions

- No smoothing found in the Apalache-jar vs standalone-`tla2tools.jar` tension; the synthesis correctly treats them as two valid TLC paths with different classpaths.
- No smoothing found in the stale Quint docs resolution at the conceptual level; the contradiction is named and resolved toward current tarball/changelog/source.
- Minor smoothing risk in Alloy: “temporal operators / NuSMV out of v0 scope” is acceptable only if v0 properties remain static relational shape checks. If revocation/lease/future-routing semantics enter v0, the Alloy brief itself says that crosses into temporal modeling.

## (d) Noise-domination / relevance-weighting

- For Q1 mechanism, the most relevant attestations are `quint-tlc-source` and `quint-npm-tarball`, not the older/stale model-checkers page. Parent should cite those directly.
- For stale-doc resolution, the relevant support is the combination `quint-model-checkers{5}` versus `quint-docs-cli{4,6}`, `quint-npm-tarball{5,8,10}`, and `quint-changelog{3}`.
- For Alloy v0, the most relevant support is `alloy6{4}` for relational collapse, `alloy6{6}` / `alloy-download{3}` for NuSMV/nuXmv temporal dependency, and book attestations for the three relational idioms.

## (e) Quote-context walk

- The quote/paraphrase “TLC is not integrated with Quint” is contextually real in the raw `model-checkers.mdx` TLC outline. It is not being stripped of a qualifier, but several brief citations point to the wrong anchor.
- The Apalache temporal warning is under-supported at the attestation level. Raw package source says Apalache has experimental support and “might give incorrect results,” and recommends `--backend tlc`; the existing attestations do not quote that passage directly.
- Alloy NuSMV/nuXmv framing preserves the important qualifier: complete/temporal model checking, not static relational checking.

## (f) Analytical-tier-inheritance walk

- Parent synthesis appears to inherit specialist-brief conclusions without re-citing the descriptive attestations. Under the lens-not-substrate guard, specialist briefs are analytical artifacts; load-bearing parent claims should cite the underlying fetched-source attestations.
- The empirical validation claims are not backed by checked-output artifacts in the repo. Either add a small evidence section/artifact with exact commands and outputs, or soften to “verification rerun by orchestrator/adversarial pass observed...” with exact command notes.

## (g) Line-reference / anchor walk

- **Broken anchor**: `.research/analysis/briefs/formal-methods-tooling-quint.md` cites `[quint-model-checkers]{8}` five times, but `.research/attestation/quint-model-checkers.md` defines only `{1}` through `{7}`. This contradicts the reported lint result.
- **Wrong semantic anchor**: Quint brief citations to `[quint-model-checkers]{4}` for “not integrated” or “TLC checks temporal properties” are wrong. `{4}` is about TLC not picking from all integers; `{5}` is “not integrated”; `{6}` is “checks invariants and temporal properties.”
- `quint-npm-tarball{10}` is semantically useful but has a weak line reference (“lines corresponding to ...”) rather than a precise line range. Not blocking, but revision should tighten it because this is load-bearing.

## (h) Thin-attestation check (semantic)

- `quint-model-checkers` is semantically thin for the Apalache temporal partial-support claim: the raw source contains “Temporal properties have partial support,” but the attestation omits that as an anchored passage. Add it or stop citing it.
- `quint-npm-tarball` is not thin for the TLC mechanism, but the load-bearing `{10}` passage should be given precise source line anchors.
- Alloy attestations are adequate for relational syntax and static semantics; the only thinness is the Patchbay-specific sufficiency inference, which should be marked as inferred rather than presented as source-attested.

## Required revision checklist

1. Fix the parent Q1 empirical wording: `quint verify --backend tlc` reports the expected violation but exits non-zero in this environment.
2. Clarify “hello-world artifacts pass” to separate expected counterexample runs from no-error checks.
3. Add direct citations in the parent synthesis for all load-bearing claims.
4. Repair `quint-model-checkers` anchors: add an attested `{8}` for Apalache temporal partial support or rewrite citations to existing anchors/sources; fix `{4}` misuse to `{5}`/`{6}` as appropriate.
5. Mark “No pure-TLA+ fallback needed for v0” and “Alloy relational-only is sufficient for v0 identity/authority/anti-spoofing” as design inferences, with citations to the source facts they extend.
6. Preserve the Alloy anti-spoofing modeling caveat from the specialist brief.

## Verdict

**NEEDS-REVISION**
