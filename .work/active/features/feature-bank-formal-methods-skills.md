---
id: feature-bank-formal-methods-skills
kind: feature
stage: done
tags: [prose, verification, foundation]
parent: epic-foundation-hardening
depends_on: [feature-research-formal-methods-tooling]
created: 2026-07-01
updated: 2026-07-01
gate_origin: null
release_binding: v0.1.0
research_origin: formal-methods-tooling
---

# Bank formal-methods reference skills

Craft three auto-loading `.agents/skills/` reference skills — Quint, TLA+/TLC, and Alloy 6 — distilled from the completed formal-methods-tooling research brief, so future model-authoring agents have verified, version-pinned API knowledge instead of relying on training-recall. This is the "bank the library" final form: the brief is the source material; the reference skills are the distilled, auto-loading form. This is the first `.agents/skills/` content in the Patchbay repo.

## Scope

- `.agents/skills/quint/SKILL.md` — current Quint authoring syntax (modules, `action`, `var`, `pure def`, types, invariants), the checking path (`quint run` simulator; `quint verify` Apalache default for bounded invariants; `quint verify --backend tlc` for temporal/liveness; `quint compile --target tlaplus`), install (npm `@informalsystems/quint`, user-prefix workaround), exit-code semantics (non-zero = counterexample found), and idioms for the Patchbay property shapes (terminal-finality, idempotent retry, monotonic generation). Version-pinned to Quint 0.32.0.
- `.agents/skills/tla-plus/SKILL.md` — TLA+/TLC syntax essentials (`EXTENDS`, `VARIABLES`, `Init`/`Next`/`Spec`, `UNCHANGED`, fairness, invariants, temporal), the CLI (`java -jar tla2tools-1.7.4.jar -config <spec>.cfg -workers auto <spec>.tla`; do NOT add a `TLC` token), `.cfg` shape (`SPECIFICATION`/`INVARIANT`/`PROPERTY`), deadlock-checking default, liveness multi-worker caveat, and how Quint-emitted TLA+ is checked. Version-pinned to tla2tools v1.7.4 (SHA-1 `bee4a54f3ee3d4afc347c3240ec2d9e93b075104`).
- `.agents/skills/alloy/SKILL.md` — Alloy 6 relational syntax (`sig`, `pred`, `fact`, `assert`, `fun`, `check`, quantifiers, joins, set ops, transitive closure for acyclicity), the headless CLI (`java -jar org.alloytools.alloy.dist.jar exec --command <label> --type json --output - <file>.als`; `commands` to list), scope-in-command convention, the v0 relational-only sufficiency for identity/authority-graph/anti-spoofing shapes (temporal Alloy needs NuSMV — out of v0), and the idioms for the three Patchbay relational shapes. Version-pinned to Alloy 6.2.0.

## Skill format (per the agile-workflow `research` skill's reference-skill spec)

Each skill:
- Named after the technology, not the research (e.g. `quint`, not `research-quint`).
- `description` with specific trigger keywords (library names, CLI commands, syntax terms) so it auto-loads when model-authoring work reaches that language.
- `user-invocable: false` — auto-loads by keyword match.
- Under 200 lines — move detailed reference tables to `references/` files if needed.
- Version-specific (pin to the versions the brief verified).
- Concrete code examples + pitfalls from the research (not generic).

## Acceptance criteria

- `.agents/skills/quint/SKILL.md`, `.agents/skills/tla-plus/SKILL.md`, `.agents/skills/alloy/SKILL.md` exist.
- Each carries the version pin from the brief and the exact CLI invocations verified in `specs/seed/`.
- Each includes the idioms for at least one Patchbay property shape from its specialist brief.
- The Quint skill documents the exit-code semantics (non-zero = counterexample) and the `--backend tlc` path for temporal properties.
- The Alloy skill states the v0 relational-only scope and the NuSMV dependency for temporal (out of v0).
- The TLA+ skill documents the jar-path distinction (standalone `tla2tools.jar` vs Apalache-distribution jar used by `quint verify --backend tlc`).

## Research grounding

**Source**: `.research/analysis/briefs/formal-methods-tooling.md` (slug: `formal-methods-tooling`)

The research engagement banked current (2026) toolchains for Quint, TLA+/TLC, and Alloy 6 from primary sources, empirically validated they install and check in this environment, and confirmed the Q1 verdict (Quint-primary-checked-via-TLC). The specialist briefs under `.research/analysis/briefs/formal-methods-tooling-{quint,tla,alloy}.md` and the 30 per-source attestations under `.research/attestation/` are the source material for these reference skills. Distilling them into auto-loading skills makes the verified API knowledge available to future model-authoring agents (notably `feature-formal-model-seed`) without re-reading the full research substrate.

## Implementation notes

- Files created: `.agents/skills/quint/SKILL.md`, `.agents/skills/tla-plus/SKILL.md`, `.agents/skills/alloy/SKILL.md`. First `.agents/skills/` content in the repo.
- Authored from the verified research brief + 3 specialist briefs + 30 attestations + the empirically-validated `specs/seed/` hello-world artifacts. Every CLI invocation, version pin, and exit-code claim is grounded in the research substrate.
- Each skill under 200 lines (93/104/102), `user-invocable: false`, version-pinned (Quint 0.32.0, tla2tools v1.7.4, Alloy 6.2.0), with specific trigger keywords for auto-loading.
- Discrepancies from design: none. All six acceptance criteria verified met.
- Adjacent issues parked: none.
- Verification: `rg` confirmed version pins, exit-code semantics, `--backend tlc` path, Alloy v0 relational-only scope + NuSMV caveat, TLA+ jar-path distinction, and `user-invocable: false` across all three skills. Line counts under 200 each.

## Review (2026-07-01)

**Verdict**: Approve with comments (after fixes)

**Review lane**: fresh-context cross-model review on `openai-codex/gpt-5.5` (high thinking).

**Blockers** (resolved in review stride):
- **Quint idiom snippets used non-Quint syntax** — the initial snippets used invented forms (`all cmd in commands =>`, `requires(...)`, `? ... else ...`) not in the Quint grammar. Replaced with source-grounded idioms condensed from the specialist brief (`all {}`/`any {}`/`nondet ... oneOf()`/`temporal ... = always(... next(...))`).
- **`quint parse` mislabeled as typecheck** — the skill said `quint parse` does "Parse + typecheck"; the research attestation shows `compile` is the parse+typecheck command. Fixed: `quint parse` = parse only; `quint compile` = parse+typecheck+compile.

**Implementation discovery during review** (load-bearing): parse-checking the idiom snippets against the installed Quint 0.32.0 revealed that **typed action parameters (`action receive(key: str)`) fail to parse** — the Quint grammar accepts *untyped* params (`action receive(key)`), which is what the getting-started docs' `action deposit(account, amount)` examples use. Fixed all idiom snippets to untyped params; all three now parse clean (`quint parse` exit 0). This means the specialist brief's idiom snippets (`formal-methods-tooling-quint.md`) carry the same typed-param defect — they were authored from docs but never runtime-validated (the research explicitly deferred parse-validation as an enriching acquisition candidate). Recorded for a future brief refresh.

**Notes**: The empirical parse-check discharged the research's own enriching-acquisition-candidate ("validate whether the three Patchbay idiom snippets parse/typecheck under the current package"). The TLA+ and Alloy skills needed no fixes — the reviewer confirmed both match the substrate on all checked points. This review bar caught a real defect the second time (grant-shape found blocker+important; verification-authority found blocker+2-important; the research adversarial-read found a real exit-code error; this review found two real Quint-syntax errors). Nits: none.
