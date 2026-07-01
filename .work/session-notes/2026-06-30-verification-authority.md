# Session note — 2026-06-30 (verification & contract authority)

A durable handoff note for the next session. Read this before continuing.

## Where we are

Patchbay foundation-hardening epic (`epic-foundation-hardening`, stage: implementing). This session designed, implemented, reviewed, and closed one feature: `feature-verification-contract-authority`. It was the highest-leverage ready item — the only one that unblocked dependents.

**Epic progress: 12/21 done** (was 11/21 at session start).

## What this session did

Full feature-design → implement → deep-review → done cycle on one feature, via the `feature-design` and `implement` skills' inline paths (foundation-doc design, no code yet — matches the `feature-design-grant-shape` precedent):

1. **Design pass (`feature-design`)** — surfaced four open design questions, broke off the steering question first, locked all four via interactive Q&A, wrote the design + 5 implementation units into the item body, advanced drafting→implementing. Commit `abf6c49`.
2. **Implement (`implement`)** — edited three foundation docs (`VERIFICATION.md`, `SPEC.md`, `PROTOCOL.md`), self-verified all acceptance criteria, advanced implementing→review. Commit `f8a4f20`.
3. **Deep review (`review`, deep lane, cross-model)** — fresh-context review on `openai-codex/gpt-5.5` (different class than the GLM-5.2 implementor). Verdict: **Block** — 1 blocker + 2 important, all genuine drift/coherence gaps. Fixed all three in-stride per the nit-triage convention; re-verified; advanced review→done. Commit `af62854`.

## The four locked decisions (so the next session doesn't re-litigate)

- **Q1=B — question-type-layered authority.** Authority is partitioned by question type, not a ranked list: formal models own invariants; `.proto` owns wire shape (+ enum wire encoding); prose owns product intent + vocabulary naming; conformance vectors own executable examples; implementation is never authority. A contradiction between two promoted artifacts that each own their question type is a *surfaced reconciliation event*, not a silent override.
- **Q2=C — property-graded normative baseline (two tiers).** checked-normative (safety-critical: terminal-finality, idempotency, wrong-session, authority, crash-recovery safety, CSRF spine, **core snapshot safety**, **`TypedCorrelation`**) must clear model-promotion + have ≥1 promoted vector before v0 ships the behavior; stated-normative (liveness/cosmetic: snapshot compaction/cursor nuances, audit completeness, adapter-failure refinements, reply-correlation refinements) is a documented v0 obligation with a draft model. **Reconciles SPEC's 5-area seed with VERIFICATION's ~10 required areas** — the checked set is "the seed done right."
- **Q3=C — conformance vectors normative-once-promoted.** Vectors earn peer authority by tracing to a model property; promotion is by review; contradictions surface for reconciliation. This is the only option under which Q1=B is coherent on the model↔vector axis.
- **Q4=B — machine-readable per-vector traceability + CI coverage check.** Per-vector frontmatter (property, status, proto_fields, expected) + 4 CI checks; generates the VERIFICATION.md traceability table as a checked-in artifact. Central registry (Q4=C) reserved as migration path (greenfield, no code yet — Late-Binding).

## Key things the review caught (load-bearing for downstream features)

The cross-model review earned its keep a third time. Three findings, all fixed:

1. **Blocker (SPEC drift):** I had put *all* of "Snapshot convergence" in stated-normative, but SPEC's verification floor explicitly names "snapshots." Split the area — core snapshot safety (reject stale/cross-domain, consistent log-prefix read, late-event audit-not-rewrite) promoted to checked-normative; compaction/cursor/operational nuances stay stated. **`feature-formal-model-seed` must seed a checked snapshot-convergence model**, not just draft it.
2. **Important:** `TypedCorrelation` was misclassified as a reply-correlation edge-case refinement, but it's v0 anti-forgery safety (a reply can't masquerade as a command or cross session/authority contexts). Promoted to checked-normative. **`feature-formal-model-seed` must include `TypedCorrelation` in the checked reply-correlation model.**
3. **Important:** "`.proto` owns enum vocabulary" blurred the wire-shape-only boundary vs prose "vocabulary naming" authority. Narrowed to "enum wire encoding" in the VERIFICATION authority table and SPEC; product variant naming stays prose authority. **`feature-protocol-idl-and-conformance` must treat `.proto` as wire-encoding authority only, not as the canonical variant-name registry.**

## Two structural insights worth remembering

- **The authority model only becomes enforced when CI is wired.** The rules in VERIFICATION.md (artifact authority order, vector promotion, the 4 CI checks) hold vacuously until `feature-protocol-idl-and-conformance` creates actual `.proto` + vectors + the CI script. This is intentional Late-Binding, but it means the *real* test of Q1=B/Q3=C/Q4=B comes when that feature implements them. If that feature finds the frontmatter shape insufficient, only the field set evolves — the 4 CI checks are the load-bearing commitment.
- **`feature-formal-model-seed` and `feature-protocol-idl-and-conformance` have a natural ordering.** The formal models define the invariants/properties; the conformance vectors in the IDL feature *trace to* those properties (Q3=C). So the seed models should land first — they're the authority source the IDL feature's vectors will reference. Picking them in that order also means the IDL feature isn't waiting on an unbuilt property registry.

## Workflow notes for the next session

- **Subagent routing**: per `AGENTS.md`, subagents run on `openai-codex`, never `umans`. This session's review used `gpt-5.5` (high thinking) — the designated outside-reviewer slot. Pass `model` explicitly on every `subagent` dispatch.
- **The bash-tool sandbox defect is still live and affects subagents too.** The `.claude` (char device / regular file) self-poisoning issue hit the reviewer subagent — it couldn't run `git show` and read files directly instead. Didn't affect review quality. The operator relaunched this session with no sandbox, so the host bash tool works; subagent bash may still be affected. Upstream report filed by the operator this session (see the earlier conversation): bwrap setup creates `<cwd>/.claude` as a file, which then bricks the next `mkdir .claude/commands`. The `background`/`monitor` tools (exec `/bin/sh` directly) are the stable escape hatch.
- **Review nit convention**: cheap/local nits applied in-stride; nits not applied explicitly recorded as deferred/not-worth-changing. This session had no nits — all findings were blocker/important and were applied.
- **Cross-model review bar**: features got fresh-context sub-agent review on `gpt-5.5`. Three-for-three features in this epic have found real issues (grant-shape: blocker+important; session-identity: nits; verification-authority: blocker+2-important). The bar is consistently earning its keep.
- **The `implement` skill's inline path is correct for these foundation-doc features** — no-coordination prose-like doc implementations, not code. The orchestrator would be overhead.

## Recommended next pickups

With the retrospective backlog closed *and* the verification-authority question settled, the two items that were blocked on it are now ready. Both are leaves (no further items depend on them). **Pick them up in this order:**

1. **`feature-formal-model-seed`** — author the first TLA+/Quint + Alloy models. Routes through `feature-design` (it's a design feature, tags `[verification, protocol, foundation]`, no specialized tag). The checked-normative property list in VERIFICATION.md (now including core snapshot safety + `TypedCorrelation`) is the direct target: seed those properties to *passing* checked models, and the stated-normative ones to draft models. Model-promotion metadata (property + bounds + tool invocation + pass/fail + product-semantics note) is required for each promoted model. **Watch the two review-driven promotions**: snapshot-convergence core safety and `TypedCorrelation` must be in the checked set, not deferred to draft.
2. **`feature-protocol-idl-and-conformance`** — create `contracts/proto/patchbay/v1/*.proto`, `buf.yaml`/`buf.gen.yaml`, generation targets (prost for Rust, Protobuf-ES for TS), and golden conformance vectors with the Q4=B frontmatter (property/status/proto_fields/expected). The `.proto` is wire-shape authority only. Routes through `feature-design`. **Depends on the seed models existing** for the vectors to trace to — hence the ordering above.

**Other still-ready items (independent of the two above, can interleave):**
- `feature-extension-seams-non-foreclosure` (prose-author) — unblocked since last session; classifies committed v0 assertions against future directions now that the assertion set is settled.
- `feature-research-v0-stack-tooling` (research, light path) — grounds TS web framework, Rust core primitives, browser operator-domain libs. Independent of everything.
- `feature-idempotency-ambiguous-execution`, `feature-lease-scope-decision`, `feature-ux-v0-acceptance` — drafting design features, all deps met.

## Key committed decisions recap (don't re-litigate)

- V0 two-process topology: Rust core (authority, no HTTP) + TS web server (control surface, generated Protobuf/Connect to core).
- Protocol contract: **Protobuf + Buf** (now ratified in SPEC, no longer "default candidate"); Rust via prost, TS via Protobuf-ES.
- Single authoritative core: one writer → one durable log per authority domain; no HA/replication in v0; crash recovery via log replay.
- Authority is question-type-layered (Q1=B); normativity is property-graded (Q2=C); vectors are normative-once-promoted (Q3=C); traceability is machine-readable per-vector metadata + CI (Q4=B).
- All retrospective-flagged semantics settled (terminal-commit-race, grant-shape, session-identity-adapter-contract, provisional-semantics).

## Files of note

Working tree is clean (all committed). Three foundation docs changed this session: `docs/VERIFICATION.md`, `docs/SPEC.md`, `docs/PROTOCOL.md`. The leaked zero-byte dotfiles in the repo root (`.bashrc`, `.env`, `*.pem`, etc., from subagent sandbox setup) remain untracked and harmless without the sandbox; not committed.
