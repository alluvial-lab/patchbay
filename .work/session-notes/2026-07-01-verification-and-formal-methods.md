# Session note — 2026-07-01 (verification authority + formal-methods tooling)

A durable handoff note for the next session. Read this before continuing.

## Where we are

Patchbay foundation-hardening epic (`epic-foundation-hardening`, stage: implementing). This session closed two features and ran a full research engagement: designed/implemented/reviewed `feature-verification-contract-authority`, then scoped/ran/closed `feature-research-formal-methods-tooling`, then filed and closed the handoff `feature-bank-formal-methods-skills`. The formal-methods toolchain is now banked and verified; the first `.agents/skills/` reference skills exist.

**Epic progress: 14/23 done** (was 11/21 at session start — +3 done, +2 new items filed).

## What this session did (in order)

1. **`feature-verification-contract-authority`** — design → implement → deep review → done. Four locked decisions (Q1=B layered authority, Q2=C property-graded normativity, Q3=C vectors-normative-once-promoted, Q4=B machine-readable traceability). Cross-model review on `gpt-5.5` found blocker+2-important (snapshot demotion despite SPEC floor; TypedCorrelation misclassified; enum-vocabulary boundary blurred) — all fixed in-stride.

2. **`feature-research-formal-methods-tooling`** — scoped as `[research]`, ran the agentic-research `research-orchestrator` engagement end-to-end: kickoff (dials + §9 depth gate) → substrate-check → decompose (Checkpoint A: 3 parallel specialists) → fan-out (Quint, TLA+/TLC, Alloy on `gpt-5.5`) → lint (356 resolved, 0 broken) → Checkpoint B (cross-specialist jar-path tension) → cross-synthesis + environment validation (installed all 3 tools, ran all 3 hello-worlds) → adversarial-read (NEEDS-REVISION → revised) → spot-check → close to done.

3. **`feature-bank-formal-methods-skills`** (research-handoff emission) — filed via `research-handoff`, then authored three `.agents/skills/` reference skills (quint, tla-plus, alloy) via `prose-author`, reviewed on `gpt-5.5` (found 2 blockers in Quint skill — invented idiom syntax + `quint parse` mislabel — both fixed), done.

## Key decisions this session (don't re-litigate)

### Verification & contract authority (feature-verification-contract-authority, done)
- **Q1=B question-type-layered authority**: models own invariants; `.proto` owns wire shape + enum wire encoding; prose owns product intent + vocabulary naming; vectors own executable examples; implementation never authority. A contradiction between two promoted artifacts is a surfaced reconciliation event, not a silent override.
- **Q2=C property-graded normativity**: checked-normative (safety-critical: terminal-finality, idempotency, wrong-session, authority, crash-recovery safety, CSRF spine, **core snapshot safety**, **TypedCorrelation**) + stated-normative (liveness/cosmetic). Reconciles SPEC's 5-area seed with VERIFICATION's ~10 required areas.
- **Q3=C vectors normative-once-promoted**: vectors earn peer authority by tracing to a model property; promotion by review.
- **Q4=B machine-readable traceability**: per-vector frontmatter + 4 CI checks; generates the VERIFICATION.md table. Central registry reserved as migration path.

### Formal-methods tooling (feature-research-formal-methods-tooling, done)
- **Q1 verdict: Quint-primary-checked-via-TLC CONFIRMED.** `quint verify --backend tlc` runs TLC end-to-end (compiles to TLA+ via Apalache, generates TLC config, spawns `tlc2.TLC`, finds counterexamples). No pure-TLA+ fallback needed for v0.
- **All three tools install and check in this environment**: Quint 0.32.0 (npm, user-prefix workaround), tla2tools v1.7.4 (jar, SHA-1 `bee4a54f...`), Alloy 6.2.0 (jar). Java 21 + Node 24 present.
- **Alloy v0 scope: relational-only** (identity/authority-graph/anti-spoofing). Temporal Alloy needs NuSMV — out of v0. Leases (if promoted) would need the temporal path.
- **Jar-path distinction**: `quint verify --backend tlc` uses an Apalache-distribution jar; standalone TLC uses `tla2tools-1.7.4.jar`. Both valid; different classpaths.

## Implementation discoveries worth remembering

- **Quint typed action parameters FAIL to parse.** `action receive(key: str) = ...` is rejected by Quint 0.32.0's grammar; the correct form is **untyped params** (`action receive(key) = ...`), matching the getting-started docs (`action deposit(account, amount)`). The specialist brief's idiom snippets carry this same defect (authored from docs, never runtime-validated — parse-validation was an explicitly-deferred enriching acquisition candidate). The `quint` reference skill uses the corrected untyped form; the brief `.research/analysis/briefs/formal-methods-tooling-quint.md` still has the defect (recorded for refresh).
- **`quint parse` is parse-only; `quint compile` is parse+typecheck+compile.** The research attestation supports this; the skill was initially mislabeled and fixed.
- **Quint exit codes: non-zero (1) = counterexample found.** `quint run` and `quint verify` exit 1 on a violation — correct checker semantics, not an error. This was a defect the adversarial-read gate caught (I'd initially claimed exit 0).
- **The cross-model review bar earned its keep four times running**: grant-shape (blocker+important), session-identity (nits), verification-authority (blocker+2-important), formal-methods-skills (2 Quint blockers). The adversarial-read gate on the research also caught a real exit-code error. Every independent gate is surfacing real defects.

## Two load-bearing structural insights

- **The two-step research→skills path works.** The `[research]` orchestrator produces ARD-checked `.research/` artifacts but NOT `.agents/skills/` reference skills; `research-handoff` emits a `[prose]` item; `prose-author` crafts the skills from the checked brief. This keeps the API knowledge source-grounded before it's distilled into auto-loading form. The three skills are the first `.agents/skills/` content in the repo.
- **Empirical validation is the highest-value step.** The Q1 verdict was "confirmed" by the specialists' source-attestation, but actually running `quint verify --backend tlc` and seeing the counterexample is what made it trustworthy. Parse-checking the idiom snippets caught a defect the source-attested brief itself carried. Run the tools; don't just read about them.

## Workflow notes for the next session

- **Subagent routing**: per `AGENTS.md`, subagents run on `openai-codex`, never `umans`. This session used `gpt-5.5` (high thinking) for all three research specialists, the adversarial-read, and the skills review. Pass `model` explicitly on every `subagent` dispatch.
- **The bash-tool sandbox defect is still live for subagents.** The `.claude/commands` bwrap issue hit the adversarial-reader subagent (couldn't run `git show`, read files directly). The host bash tool works (operator relaunched with no sandbox this session). The `background`/`monitor` tools remain the stable escape hatch for subagent-context commands.
- **Research engagement shape**: the orchestrator's walk (kickoff → substrate-check → decompose/Checkpoint A → fan-out → lint → Checkpoint B → cross-synthesis → adversarial-read → spot-check → close) is the established precedent now. 3 parallel `gpt-5.5` specialists worked well for a 3-facet tooling engagement; ~10 min each.
- **`.gitignore` now excludes `*.jar`** (the tool binaries) and `specs/seed/_apalache-out/` + `specs/seed/states/` (tool output).

## Recommended next pickups

With the formal-methods toolchain banked, `feature-formal-model-seed` is fully unblocked and its design questions are now grounded in verified toolchain capability:

1. **`feature-formal-model-seed`** — author the first TLA+/Quint + Alloy models. Routes through `feature-design`. The checked-normative property list in VERIFICATION.md (now including core snapshot safety + TypedCorrelation) is the direct target. Q1=Quint-primary-checked-via-TLC (confirmed); Q2=clustered decomposition (informed by the tooling); Q3=Alloy relational-only for v0. Use the new `.agents/skills/{quint,tla-plus,alloy}/SKILL.md` reference skills when authoring — they have the verified syntax + idioms + pitfalls. **Watch the untyped-params gotcha** the skill documents.

2. **`feature-protocol-idl-and-conformance`** — create `contracts/proto/patchbay/v1/*.proto` + `buf.yaml`/`buf.gen.yaml` + generation targets + golden conformance vectors with Q4=B frontmatter. Depends on `feature-formal-model-seed` for the vectors to trace to model properties. Routes through `feature-design`.

**Other still-ready items (independent):**
- `feature-extension-seams-non-foreclosure` (prose-author), `feature-observability-operator-admin` (prose-author), `feature-pi-parity-checklist` (prose-author) — all unblocked.
- `feature-research-v0-stack-tooling` (research, light path) — independent.
- `feature-idempotency-ambiguous-execution`, `feature-lease-scope-decision`, `feature-ux-v0-acceptance` — drafting design features, all deps met.

## Deferred / parked

- **Refresh the Quint specialist brief** to fix the typed-param defect in its idiom snippets (now that parse-validation has been done). Low priority — the `.agents/skills/quint/SKILL.md` has the corrected form; the brief is historical substrate.
- **The research's enriching acquisition candidates** (TLC `-help` output, Alloy CLI `help exec` output, deeper Quint idiom validation) — recorded in the synthesis; not blocking.

## Key committed decisions recap (don't re-litigate)

- V0 two-process topology: Rust core (authority, no HTTP) + TS web server (control surface, generated Protobuf/Connect to core).
- Protocol contract: Protobuf + Buf (ratified in SPEC); Rust via prost, TS via Protobuf-ES.
- Authority is question-type-layered (Q1=B); normativity is property-graded (Q2=C); vectors are normative-once-promoted (Q3=C); traceability is machine-readable per-vector metadata + CI (Q4=B).
- Formal-methods toolchain: Quint 0.32.0 primary (checked via Apalache for invariants, `--backend tlc` for temporal); TLA+/TLC v1.7.4 baseline; Alloy 6.2.0 relational-only for v0. All verified working in-environment.
- All retrospective-flagged semantics settled.

## Files of note

Working tree is clean (all committed). This session's substrate additions:
- `docs/VERIFICATION.md`, `docs/SPEC.md`, `docs/PROTOCOL.md` (verification-authority edits).
- `.research/analysis/briefs/formal-methods-tooling{,-quint,-tla,-alloy,-verification}.md` + 30 `.research/attestation/*.md`.
- `specs/seed/{Counter.qnt,Counter.tla,Counter.cfg,patchbay-invariants.als}` (verified hello-worlds).
- `.agents/skills/{quint,tla-plus,alloy}/SKILL.md` (first banked reference skills).
- Tool jars (`tla2tools-1.7.4.jar`, `org.alloytools.alloy.dist.jar`) downloaded locally, gitignored.
