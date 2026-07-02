# Session note — 2026-07-01 (formal-model-seed: design → implement → 3-deep-review arc)

A durable handoff note for the next session. Read this before continuing.

## Where we are

Patchbay foundation-hardening epic (`epic-foundation-hardening`, stage: implementing). This session took `feature-formal-model-seed` from `drafting` through design, full implementation (7 models via 2-wave parallel sub-agent dispatch), and **three deep cross-model reviews** that each caught real defects the prior pass missed. The feature is `done`; the seed formal models are now a trustworthy, procedurally-clean foundation. The arc also produced a gloss-audit discipline that changed how I present design options.

**Epic progress: 15/23 done** (was 14/23 at session start — `feature-formal-model-seed` done, +1 backlog item filed). All 5 child stories of the seed feature are terminal.

## What this session did (in order)

1. **Design (`feature-design`)** — 5 interactive design questions, dependency-ordered (which most constrain the others' choice set). Resolved: Q1=B focused-cluster-checked-to-pass + vocabulary-for-all; Q2=B-wide clustered-by-shared-state + Alloy; Q3=C mixed backends; Q4=A Quint-primary + emitted TLA+ as generated artifact; Q5=1 inline `@promotion` comment blocks. 1 child story spawned (Unit 1, the trickiest).

2. **Gloss audit** — user pressed on Q5; I'd conflated two things and offered an infeasible option (YAML frontmatter in `.qnt` files — tool parsers own line 1). Re-derived honestly. User then asked "did we gloss over anything similar in the prior design decisions?" — I audited Q1–Q4 and found Q4's "standalone `tla2tools.jar` re-check path" was **empirically false** (tested: emitted TLA+ `EXTENDS ... Apalache, Variants`, standalone TLC can't parse it). Corrected the Q4 rationale; the decision (A) still held.

3. **Implement Unit 1 (`command_lifecycle.qnt`)** — the trickiest model (fused terminal-race + dedup). 3 syntax fixes grounded in the attestation (Map literal, `.keys().contains`, `any{}` variable-consistency). Test-integrity caught 2 self-defining properties (`boundary_dedup` was a tautology; `retry_reuses_id_and_key` inverted). Q3's `backend:tlc` for temporal **does not work** (TLC rejects `[] followed by action not of form [A]_v`); switched to Apalache-temporal (`echo y | quint verify --temporal`).

4. **Implement Units 2–7** (orchestrator, 2-wave parallel) — 4 checked models + Alloy + 2 draft models. All file-independent; dispatched 4 + 2 `openai-codex` sub-agents.

5. **Deep review #1** (cross-model adversarial, `gpt-5.5` xhigh) — **Block**. Found 6 promoted properties self-defining or vacuously true (B1 TypedCorrelation self-referential; B2 CSRF self-referential; B3 LateGenerationInert vacuous dead-stutter; B4 GenerationMonotonic weaker than claimed; B5 Alloy AuthorityGraphAcyclicAssert vacuous via `fact { no Grant }`; B6 Alloy SenderMatchesClaimAssert tautological via forcing fact). Filed `story-fix-formal-model-genuine-checks`. I verified the 2 most damning via mutation tests before classifying.

6. **Fix #1** — separated action predicates from independent property oracles; mutation tests confirmed each broken predicate now fails the invariant. B4 honestly downgraded to non-decrease floor (strict-supersession exceeded Apalache's experimental temporal support).

7. **Deep review #2** — **Block again**. The B5/B6 *fixes* regressed: removing forcing facts turned vacuous-true into **actually-false** (Alloy found counterexamples — self-grants, sender≠claimedSender). Plus B2-trace (CSRF trusts recorded `lastProof`) and B4-overclaim. Filed `story-fix-alloy-relational-assertions` + `story-fix-formal-model-disclosure-drift`. **I caught my own measurement error**: `--type json`/file-count gave false UNSAT; `--type text` skolem-witness is the reliable method.

8. **Fix #2** — demoted B5/B6 to `status: draft` (not checkable relationally in v0 without becoming tautological); added CSRF `attemptedSession`/`attemptedProof` state; narrowed B4 semantics.

9. **Deep review #3** — Approve with comments. All promoted properties genuinely hold (mutation-test proven). Filed `story-fix-csrf-trace-and-ssot-drift` (B2-trace incomplete: `attemptedProof` still action-recorded, so a combined mutation passes; plus SSOT drift). Feature advanced to done.

10. **Follow-up fix + procedural correction** — closed the CSRF trace gap (split request capture from server processing: raw submitted evidence is now pre-state the accepting action reads but cannot rewrite). User caught that filing a child under a done feature re-opens the review surface → final parent re-review confirming no regressions.

## The load-bearing lessons (don't relearn these)

### 1. The gloss-audit discipline (from the design phase)
**For mechanism claims (a path works, a parser accepts a construct), ground empirically BEFORE presenting as an option.** I offered Q4 "standalone re-check path stays available" and Q5 "YAML frontmatter in .qnt" — both infeasible as stated, both caught only when pressed. The fix: when a design option asserts a mechanism is feasible, run it before the user sees it. For scope choices (how much to build), options can be genuine without empirical grounding. **When asked "did we gloss over anything similar?", audit honestly rather than defend** — that's how Q4's false claim surfaced.

### 2. Genuine-checking discipline (the recurring theme of all 3 reviews)
A formal model is worthless if the checked property is **self-defining** — the invariant uses the same predicate the action's guard uses, so it can never catch a broken predicate. The test: **mutate the predicate (break it to `true` or invert it); if the invariant still passes, it's self-defining.** This caught 6 properties in review #1 and 1 more (B2-trace) in review #3. The fix pattern is uniform: the invariant must check **raw state facts via an independent oracle**, never the helper the action consults. `command_lifecycle`'s `boundary_dedup` is the gold standard (independent `applyCount.get(k) <= 1` over a permissive `receive` action).

### 3. The trace-fidelity extension (B2-trace, the deepest finding)
Even an independent oracle is insufficient if it checks **action-recorded state** rather than **environment pre-state**. The CSRF invariants read `attemptedProof` — but `attemptedProof` was action-assigned, so a combined mutation (drop the proof check AND lie about `attemptedProof`) still passed. The fix: raw submitted evidence must be **pre-state** set by a separate `arriveRequest` action; the accepting action reads it but cannot rewrite it. This is the `idea-csrf-trace-fidelity` pattern — generalize it to any server-side-acceptance model (e.g. `authority.qnt` CompoundIssuer when promoted).

### 4. Removing a forcing fact without a real constraint trades vacuous-true for actually-false (B5/B6)
The Alloy "fix" that removed `fact { no Grant }` and `fact { sender = claimedSender }` turned tautologies into **provably-false asserts** (Alloy found counterexamples). The honest resolution: B5 (acyclicity) needs a delegation edge that's out of v0; B6 (sender==claimedSender) is a dynamic CompoundIssuer binding, not relational. **Both demoted to `status: draft`.** Only `ActorIdsUniqueAssert` remains promoted in Alloy. Lesson: a genuine Alloy check must check a property true because of OTHER constraints, or be demoted if none exist.

### 5. Apalache temporal is experimental; `--backend tlc` doesn't work for `next()`-in-`always()`
Q3 specified TLC for two-state temporal properties. Empirically: the Quint→TLA+ compilation emits `[](\A cmd: x'=x)`, which TLC rejects (`[]` needs `[A]_v` form). Apalache default checks them (all pass) but warns temporal support is "experimental." All checked temporal properties are `always(...)` safety (not `eventually` liveness) — the conservative end. Residual filed as `idea-tlc-temporal-workaround`. **B4's strict-supersession form exceeded even Apalache** (false counterexamples on valid 0→1→2 traces when reading `next()` on attempted-event vars in an implication antecedent) → downgraded to non-decrease floor + structural guard.

### 6. Measurement discipline: `--type text` + skolem-witness is the reliable Alloy UNSAT method
`--type json` / file-count gave false UNSAT (I "confirmed" B5/B6 held when they actually failed). The reliable method: `java -jar ... exec --command <label> --type text --output - <file>.als`; a `skolem $<AssertName>_...` line means a counterexample was found (assert FAILS); absence = UNSAT (holds). **Never trust a recorded green without re-running it yourself** — this applies to Quint checks too (the implementer's "honest encoding" claims were wrong twice).

### 7. Procedural: filing a child under a done feature re-opens its review surface
When I filed `story-fix-csrf-trace-and-ssot-drift` under the already-done feature, I skipped the parent re-review when it landed. User caught it. The substrate doesn't distinguish "refinement" from "scope" — a child is a child. **Re-review the parent when any child changes under it, even post-hoc refinements.**

## The review-arc shape (what each pass caught)

Three deep reviews, each attacking from a fresh angle, each catching what the prior couldn't:
- **#1**: 6 self-defining properties (the basic genuine-checking audit).
- **#2**: a fix-pass regression (vacuous-true → actually-false) — the B5/B6 demotion lesson.
- **#3**: the B2-trace fix was itself incomplete (action-recorded state, not pre-state) + SSOT drift.

This is the review bar working as designed for a safety-claiming artifact. The cost (3 deep reviews + 4 fix stories) is honest — each pass found a real defect class. A single pass would have shipped self-defining "safety" properties that let real violations through undetected.

## Key decisions this session (don't re-litigate)

### `feature-formal-model-seed` design (5 questions)
- **Q1=B** focused-cluster-checked-to-pass + vocabulary-for-all: checked = command delivery + wrong-session + idempotency-boundary + TypedCorrelation + CSRF spine (pinned by done deps); draft = snapshot/recovery + authority (large, deferred). Full property-id vocabulary established as SSOT even for draft.
- **Q2=B-wide** clustered-by-shared-state: one Quint model per state cluster with projection seams; idempotency folded into `command_lifecycle.qnt` (tightest coupling); TypedCorrelation+CSRF pulled into checked.
- **Q3=C** mixed backends → **corrected to Apalache for both** (TLC doesn't work for `next()`-in-`always()`; see lesson 5).
- **Q4=A** Quint-primary + emitted TLA+ as generated inspection artifact (NOT an independent re-check lane — empirically grounded).
- **Q5=1** inline `@promotion` comment blocks (drift-impossible; CI greps them).

### Promotion-metadata shape
`@promotion` blocks inline above each property: `property`, `tier`, `status` (draft|promoted), `model`, `language`, `backend` (apalache | apalache-temporal | alloy-cli), `invocation`, `bounds`, `expected`, `proto_fields`, `semantics`. `status: promoted` + a promoted vector tracing to it = "checked-normative." A future CI script greps these to generate the `docs/VERIFICATION.md` traceability table.

## Implementation discoveries worth remembering

- **Quint map literal is `Map("k" -> v, ...)`, NOT `Set("k" -> v)`** (latter is a Set of tuples). Map membership is `m.keys().contains(k)`, not `m.contains(k)`. `any{}` action branches must update the same variable set (add `x' = x` for untouched vars).
- **`intToString`/string `++` are unverified** — not in the attestation. `session_generation.qnt` used `(str, int)` tuple keys instead.
- **`pure def` cannot read state variables** — helpers inspecting state use `def`. (Units 3 & 4 found this.)
- **Quint exit codes**: `quint verify`/`quint run` exit 1 = counterexample found (correct), exit 0 = no violation. Apalache temporal prompts interactively → `echo y |` in non-interactive runs.
- **Apalache jar**: `/home/agent/.quint/apalache-dist-0.56.1/apalache/lib/apalache.jar`. Quint installs to `~/.npm-global` (user-prefix workaround; `export PATH="$HOME/.npm-global/bin:$PATH"` before any quint command).
- **Alloy CLI**: `java -jar org.alloytools.alloy.dist.jar commands <file>` (list); `exec --command <label> --type text --output - <file>` (run). Scope in the command (`check X for 5`), not a CLI flag. `SAT` = counterexample found (assert fails); `UNSAT` = holds. Use `--type text` + skolem check, not json/file-count.

## Workflow notes for the next session

- **Subagent routing** (per `AGENTS.md`): all implementation/review sub-agents on `openai-codex` (never `umans`). This session: `gpt-5.5` (high/xhigh) for the 3 deep reviews + Wave-1 implementation; `gpt-5.3-codex-spark` (medium) for the 2 draft-model Wave-2 agents. Pass `model` explicitly on every dispatch.
- **The 2-wave orchestrator shape** worked well for 6 independent files: 4 parallel (checked models + Alloy) then 2 parallel (draft models). One agent per file (no bundles) when write-ownership is disjoint.
- **Fast-lane story review** permits "run cheap verification yourself" — for a story whose deliverable IS the mutation-test proof, re-running that proof is the verification, not over-scoping. I re-ran mutation tests myself on every fix-story review.
- **`.gitignore`** excludes `*.jar`, `specs/seed/_apalache-out/`, `specs/seed/states/`. Root-level `_apalache-out/` is NOT ignored (only `specs/seed/` one) — clean it manually.

## State of the seed models (what exists now)

`specs/seed/`:
- **Checked Quint** (promoted properties, mutation-test proven): `command_lifecycle.qnt` (7), `session_generation.qnt` (4), `reply_correlation.qnt` (1 TypedCorrelation), `csrf_browser.qnt` (3). Each has a `.emitted.tla` inspection artifact.
- **Checked Alloy**: `patchbay-relational.als` (1 promoted: `ActorIdsUniqueAssert`; B5/B6 demoted to draft).
- **Draft Quint** (compile-only, reserved property-ids): `snapshot_recovery.qnt` (6 ids), `authority.qnt` (4 ids).
- **Retained hello-worlds**: `Counter.qnt`/`.tla`/`.cfg`, `patchbay-invariants.als` (superseded, annotated).
- `docs/VERIFICATION.md` has a "Seed models (v0)" section (checked/stated tables + toolchain note).

## Backlog items filed this session

- `idea-tlc-temporal-workaround` — pursue TLC-checkable form for temporal properties (Apalache experimental; options: hand-author `[A]_vars`, history-variable invariants, cross-check via TLC on hand-written TLA+, or accept + document).
- `idea-csrf-trace-fidelity` — the environment-evidence pattern (raw submitted evidence as pre-state); now properly applied in `csrf_browser.qnt`, to be reused for `authority.qnt` CompoundIssuer.

## Recommended next pickups

The seed formal models are a trustworthy foundation. Downstream work can now derive obligations from the property-id vocabulary (SSOT) with confidence the checked properties are genuine:

1. **`feature-protocol-idl-and-conformance`** — `.proto` contracts + `buf.yaml`/`buf.gen.yaml` + golden conformance vectors with `@promotion`-style frontmatter tracing to the model property-ids. The seed's `@promotion` blocks are the model-layer analog of the vector frontmatter the authority feature specified. Depends on the seed (now done).

2. **Promote a draft property** (when its semantics are needed) — e.g. snapshot core safety, or `authority.qnt` CompoundIssuer (use the `idea-csrf-trace-fidelity` pre-state-evidence pattern). Per-property promotion, not a baseline re-open.

3. **`idea-tlc-temporal-workaround`** — if the experimental-temporal residual becomes blocking before a release treats the temporal properties as product semantics. The `always(...)` safety mitigation is adequate for v0 seed; pursue a TLC path before durable product claims.

**Other still-ready items** (independent of the formal-model arc): `feature-idempotency-ambiguous-execution`, `feature-lease-scope-decision`, `feature-extension-seams-non-foreclosure`, `feature-ux-v0-acceptance`, `feature-research-v0-stack-tooling`, `feature-observability-operator-admin`, `feature-pi-parity-checklist`. Check `.work/bin/work-view --ready`.

## The one-sentence takeaway

The seed formal models are done and trustworthy — but only because three adversarial reviews, a gloss audit, and a procedural catch each forced a layer of honesty the prior pass had accepted. The discipline that produced them (ground mechanism claims empirically; mutate-to-prove genuine checks; re-review when children change) is more valuable than the models themselves.
