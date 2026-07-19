# Session note — 2026-07-19 (component-layer conformance earned; cockpit next via EC1-EC3 option A)

A durable handoff note for the next session. Read this before continuing.

## Where we are

`epic-v0-1-0-implementation` — **5/6 layers done** (core + seam + web-server + pi-adapter + presentation-component-layer). This session shipped the **shared presentation-component layer** (`feature-v0-presentation-component-layer`) — but only after bouncing it from `done` back to `drafting` and rebuilding it as a genuine machine-checkable conformance layer through a 7-pass thorough review.

The remaining work on the phone-usable critical path: **the cockpit** (`feature-v0-web-cockpit`, `stage: implementing`, deps now both `done`), then the **CLI** (`feature-v0-cli`, `stage: drafting`). The cockpit is implementation-ready except for one open design decision — EC1–EC3 (see below) — which the operator has now decided: **option A (typed proto messages)**.

## What happened this session

### The component layer was shipped twice — the second time it was real

**First attempt (land-mode).** `feature-v0-presentation-component-layer` was originally mockup-generated CSS (`tokens.css` + `components.css` + showcase) declared "locked + implementing" by the `palette`/`components` skills in the same stride. A land-mode implementation pass verified it by re-checking the implementer's own claims — and asserted "all three protocol registries fully bound" while ElicitationState sat at 3/9, and shipped an invisible `.toast` rule (1:1 contrast: `--color-bg-inverse` and `--color-text-primary` were the same hex value in both themes).

**Operator review caught it.** The operator questioned whether the layer had gone through rigorous design and implementation. Honest answer: no. The feature's Brief claimed the layer "makes the conformance floor machine-checkable," but the design gate never specified the machine-checkable mechanism as a deliverable — it named the obligation in prose, shipped the CSS, and advanced on the artifacts alone.

**Bounce + re-design.** The feature went `done → drafting` (implement skill's design-flaw escape hatch). Re-design (Q1–Q5): a **sibling-but-separate conformance check** (`contracts/scripts/check-presentation.mjs`), a **descriptive runtime contract** (not executable — Q2A), a **full a11y harness** (contrast + axe-core — Q3B), review weight pinned to **thorough** (Q4). The one genuine 50/50 in the mockup-pass's inline decisions (Option C: `working` stays a 3-value axis, thinking-vs-executing is Observation-composed) was re-opened and **retained** — the reversibility asymmetry settles it (C→B is additive and clean; B→C is destructive and breaking).

### The thorough review converged after 7 passes (4 → 7 → 5 → 4 → 5 → 1 → 0)

Each pass did adversarial mutation testing and found the check was self-defining in progressively subtler ways. Cumulatively, every load-bearing oracle was transformed from hand-maintained (gameable) to genuinely independent:

- Registry bindings: hand-maintained list → **parsed from `.proto` enums** (parity catches drift)
- Retry matrix: hand-maintained rows → **derived from `docs/UX.md` table**
- Contrast: hand-maintained token pairs → **derived from CSS rules** (effective-theme overlay, key-set parity between explicit-dark and system-follow-dark)
- Showcase coverage: raw-text `includes` → **DOM-aware `querySelector`** (exact class, same-element `data-state`)
- Dominance: class presence, `some()` → **both paths (`:has()` + wrapper fallback), exact selector, opacity < 1, per-selector**
- Reduced-motion: count media blocks → **per-selector exact + override detection**
- Invented states: not checked → **rejected** (CSS scanned for non-registry modifiers)
- Meta-test: output grep → **asserts exit status** (neutered exitCode caught)

Pass 7 returned `ready` with no findings. The check is genuinely non-self-defining for realistic defect classes. The meta-test proves it fails on real defects (missing binding, invisible toast, neutered exitCode). **The "machine-checkable" claim is earned, not asserted.** Feature advanced to `done` (commit `aeaa45d`).

### Key artifacts produced this session

- `contracts/scripts/check-presentation.mjs` — the conformance check (registry parity, contrast, dominance, reduced-motion, retry matrix, showcase, axe-core)
- `contracts/scripts/test-presentation-check.mjs` — meta-test proving the check fails on defects
- `.github/workflows/ci.yml` — CI running presentation check, meta-test, vectors, models, build, Rust clippy/test (does NOT run `check:drift` — pre-existing repo gap, documented)
- `.mockups/design-system/` — tokens.css + components.css + components.html (with the fixes from review passes 1–6: ElicitationState 9/9, toast contrast, dominance fallback, delivery-line showcase, reduced-motion guards, data-state markers, AA-compliant token values)
- `docs/PROTOCOL.md:592` — layer reclassified R→C (implemented v0.1.0)
- `docs/UX.md` — generated presentation conformance traceability block + runtime-contract cross-reference

### Commits this session

```
aeaa45d review: feature-v0-presentation-component-layer (Approve — thorough converged pass 7; -> done)
ff8cbd6 review: feature-v0-presentation-component-layer thorough-pass-6 fix (dark-token key-set parity + effective-theme overlay)
4d5cb76 review: feature-v0-presentation-component-layer thorough-pass-5 fixes (...)
7d682de review: feature-v0-presentation-component-layer thorough-pass-3 fixes (...)
4caa1dc review: feature-v0-presentation-component-layer thorough-pass-4 fixes (...)
721cec0 review: feature-v0-presentation-component-layer thorough-pass-2 fixes (...)
6a011eb review: feature-v0-presentation-component-layer thorough-pass-1 fixes (...)
7e306a3 implement: feature-v0-presentation-component-layer (conformance check + a11y harness + runtime contract)
ad1a9ad feature-design: feature-v0-presentation-component-layer (...)
da702ea feature-v0-presentation-component-layer: bounce to drafting (design-flaw discovery)
efcc666 review: feature-v0-presentation-component-layer (Approve with comments)  [the first, bounced review]
b3c3268 implement: feature-v0-presentation-component-layer  [the first, bounced land-mode pass]
```

## What's next: the cockpit, gated on EC1–EC3 option A

`feature-v0-web-cockpit` (`stage: implementing`) is the next layer. **Both deps are now `done`** (`feature-v0-web-server`, `feature-v0-presentation-component-layer`). But it cannot proceed cleanly until the **EC1–EC3 wire-shape decision** is resolved — a semantic 50/50 the cockpit's own Risks section flags, which stopped the original orchestrator run at the start of this whole arc.

### The blocker (still open — nothing changed it)

`contracts/proto/patchbay/elicitations.proto` has `ResponseContract` (contract_kind + `ui_hints: repeated string` + a free-form `schema_ref` string) but **no typed `QuestionContract`/`ResponseOption`/`ElicitationResponsePayload` message**. The core's `ElicitationSlotLayer` (`core/src/acceptance/elicitation.rs`) treats response payloads as opaque bytes — it only tracks command lifecycle + terminal transition, never response content. No conformance vector pins the shape. The pi-adapter does not emit question Elicitations today (only approval).

The cockpit's design Units 2, 4, 5 (presentation model fold, elicitation handling, shell) reference types that don't exist: `ElicitationView { contract: ResponseContract; options?: Option[] }` — but the proto `ResponseContract` has no `options` field and there is no `Option` message.

### The operator decision (this session): Option A — typed proto messages

Three options were surfaced; the operator chose **(A)**:
- **(A) Typed proto messages** — add `QuestionContract { options[], allow_free_text }` + `ElicitationResponsePayload { selected_option_id, free_text, clarification }`, bind into `Elicitation`/`Operation.payload`, regen Rust+TS. Chosen because: it aligns with Single-Source-of-Truth + Generated-Contracts; it gives the cockpit typed bindings; and having just spent 7 passes making the component layer genuinely non-self-defining, building the cockpit against *untyped* JSON would create exactly the un-checked surface the component-layer arc existed to prevent. The conformance-check infrastructure now exists — a typed contract means the cockpit's elicitation handling can be checked the same way.
- (B) Untyped `PayloadEnvelope` JSON — rejected (the "hand copies" anti-pattern; un-checkable).
- (C) Defer EC1–EC3, approval-only cockpit — rejected for now (operator chose the full typed path).

### The EC1–EC3 design context (from the cockpit feature body)

These three response-contract-shape decisions were grounded against `docs/PROTOCOL.md` during the cockpit's original `feature-design` pass:
- **EC1 — Free-text option within a `question` contract: v0.1.0 committed.** A `select-one`/`select-many` question may append a free-text option ("or type your own answer"). The response Operation carries the free-text string instead of a selected option id. This is a `free-text` ui_hint within the committed `question` contract_kind — no contract-kind promotion.
- **EC2 — "Answer-and" composed response (structured selection + free-text clarification): v0.1.0 committed.** A question response may carry a selected option *plus* an appended free-text clarification in one Operation (the "And..." field). The clarification is supplementary context; the structured selection remains the primary answer.
- **EC3 — Grouped multi-question (N independent single-answer Elicitations as one visual card): v0.1.0 committed as the grouping; the multi-answer contract is reserved.** Claude's nested multi-question maps to N independent Elicitations opened as a batch, rendered as one visual card, each independently single-answer and independently terminal. A true multi-answer contract (one Elicitation carrying multiple questions) is a reserved seam ("multi-answer accumulation", PROTOCOL:312).

### How to proceed (operator's stated intent: scope option A, then pick up with fresh context)

The proto extension is itself a small design act. Recommended sequence for the next session:

1. **Scope the proto extension as a focused design + implementation act.** This is a protocol-semantics-bearing change across two codegen targets (`contracts/buf.gen.yaml`: `protoc-gen-prost` for Rust, `protoc-gen-es` for TS). The exact message shapes for `QuestionContract`/`ResponseOption`/`ElicitationResponsePayload` need to be designed — they must satisfy EC1 (free-text option), EC2 (answer-and clarification), and EC3 (grouped = N independent single-answer, so the *payload* stays single-answer; grouping is a presentation concern, not a proto concern). The shapes must NOT quietly promote the reserved multi-answer seam (one Elicitation carrying multiple questions) — EC3 is explicitly the *grouping* of independent single-answer Elicitations, not a multi-answer contract.

2. **Run `buf generate` + `check:vectors` after the proto change** to regen both targets and confirm no protocol-vector drift. Note: `check:drift` is a **pre-existing broken repo gap** (needs `protoc-gen-prost` installed + has a Rust-gen divergence vs the committed bindings — see commit `9a2854f`/the protocol-idl arc). It is NOT run in CI and is not this feature's concern, but a proto change will touch the generated Rust bindings — be aware the drift check will report diffs regardless of correctness. The generated TS in `contracts/ts/src/gen/` and Rust in `contracts/rust/src/gen/` both need regen.

3. **Then implement the cockpit** (`feature-v0-web-cockpit`, 5 units in the feature body) via `/agile-workflow:implement-orchestrator feature-v0-web-cockpit` or `/agile-workflow:implement`. The cockpit consumes: the web-server's Connect-Web bridge (`web-server/src/routes/rpc.ts` proxies ControlService Submit/Subscribe/LoadSnapshot), the contracts TS bindings, and the now-done component layer's `tokens.css`+`components.css`. The cockpit's 5 units: protocol client + cursor-reconcile, presentation model fold, markdown rendering (the mobile-readability differentiator), elicitation handling (the three EC shapes + mobile sheet — now buildable against the typed proto), shell + list + detail.

### Two things to watch for in the cockpit

- **Reconnect correctness (Unit 1) is load-bearing** — the snapshot-correctness rule means an unreconciled snapshot must never render as live. The feature body says property-test the reconcile path.
- **The markdown renderer choice (Unit 3)** should be spiked early — must be small + safe + streaming-friendly. The feature body suggests `marked` + `DOMPurify`.
- The component layer's `check-presentation.mjs` is now CI-gated; the cockpit should consume `tokens.css`+`components.css` directly (not re-bind protocol states). If the cockpit introduces new state-bearing CSS that bypasses the component layer, that's a conformance-floor violation — the layer exists to prevent it.

## Notes for the next session

- The component-layer arc's lesson, applied forward: **a claimed-but-not-enforced conformance surface is a liability.** The cockpit is a consumer of the component layer and the protocol; both are now machine-checked. Hold the cockpit to the same standard — if it claims a conformance property (identity-before-submission, retry-safety derivation, stale-never-live, first-answer-wins), that property should be checkable, not just asserted in prose.
- The harness crashed mid-session (lost the pass-2 review agent); recovery was clean because the fix commit had landed before the crash. The thorough loop's commits are the durable state — each pass's fixes are committed before the next pass dispatches.
- `.work/bin/work-view` has a pre-existing uncommitted binary modification (not this session's; appears throughout the git log). Leave it.
- The component layer's accepted v0.1.0 limitations (documented in its review record + the check script's source comments): contrast checks co-located pairs not full cascade; `LOCKED_PRIMITIVES` hardcoded but design-sourced; CI doesn't run `check:drift`. Do not treat these as the cockpit's problem to solve unless they actively block it.
