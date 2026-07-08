---
id: story-formal-model-realignment-traceability
kind: story
stage: implementing
tags: [verification, protocol, foundation]
parent: feature-formal-model-realignment
depends_on: []
created: 2026-07-08
updated: 2026-07-08
gate_origin: null
release_binding: null
---

# Story: Traceability script + VR2 metadata realignment + VR4 promotion (Unit TR + Unit M)

Implements Unit TR (`contracts/scripts/check-models.mjs` — the tier authority) and Unit M (VR2 drop `tier` field + VR4 promote `browser_local_state_not_authority` with verification stride) from `feature-formal-model-realignment`. These two units are one stride because the check validates the edit.

## Scope

### Unit TR: `contracts/scripts/check-models.mjs` (the tier authority)

Parses every `@promotion { ... }` block in `specs/seed/*.qnt` and `specs/seed/*.als`, extracts fields (`property`, `status`, `model`, `backend`, `invocation`, `bounds`, `expected`, `proto_fields`, `semantics` — NO `tier` field under Q1), computes product tier from `status` + vector coverage, cross-checks against `docs/VERIFICATION.md` and `contracts/scripts/check-vectors.mjs`.

**Checks (exit 1 on any failure):**
1. Coverage — distinguish modeled_properties (have a block) from reserved_unmodeled_properties (no block, expected stated-normative). `TypedCorrelation` is ONE id whose coverage expands; no distinct response-correlation id.
2. Tier derivation — `status: promoted` + ≥1 promoted vector → checked-normative; + 0 vectors → checked-model; `status: draft` → stated-normative; no block → stated-normative. Computed tier must agree with VERIFICATION.md.
3. Status-vs-vector consistency — derived checked-normative requires ≥1 promoted vector.
4. Invocation well-formedness — promoted blocks have non-empty invocation naming the tool.
5. Drift detection vs hardcoded arrays — derived map must agree with check-vectors.mjs arrays.

**Registry composition (I3 fix):** `check-vectors.mjs` runs `main()` on load — importing it mutates docs. Read the arrays via source-parsing (regex-extract from source text) OR refactor to export a side-effect-free `property-registry.mjs` that both scripts import. Implementer picks the cleaner option.

Writes a generated `## Generated model-promotion traceability` section into `docs/VERIFICATION.md` (parallel to the conformance-vector table). CI fails if the generated block drifts. Wired into `contracts/ts/package.json` as `check:models`.

### Unit M: VR2 drop `tier` field + VR4 promotion

**VR2:** remove the `tier:` line from ALL `@promotion` blocks (16 promoted, 12 draft). Preserve `status:`. Update stale header comments that say "checked-normative"/"checked-model" to "promoted model"/"draft model".

**VR4:** add `@promotion` block for `browser_local_state_not_authority` (`status: promoted`, no `tier`) in `csrf_browser.qnt` (~line 207). **Then run the verification stride (B6):** `quint verify csrf_browser.qnt --invariant browser_local_state_not_authority --max-steps 12` exit 0; mutation test (break `serverAccepts` to accept without valid proof; confirm `browser_local_state_not_authority` fails `[violation]`).

Update `check-vectors.mjs` arrays: move `browser_local_state_not_authority` from `STATED_NORMATIVE_PROPERTIES` to `CHECKED_MODEL_PROPERTIES` (17 checked-model properties).

## Acceptance Criteria

- [ ] `contracts/scripts/check-models.mjs` exists and `node contracts/scripts/check-models.mjs` exits 0 on the post-Unit-M model set.
- [ ] All `@promotion` blocks no longer carry a `tier` field (Q1 derive-tier).
- [ ] `browser_local_state_not_authority` has a complete `@promotion` block with `status: promoted`.
- [ ] **VR4 verification:** `quint verify csrf_browser.qnt --invariant browser_local_state_not_authority --max-steps 12` exits 0.
- [ ] **VR4 mutation test:** breaking `serverAccepts` causes `browser_local_state_not_authority` to fail `[violation]`.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0 (arrays updated).
- [ ] All 4 checked Quint models still `quint parse` + `quint compile` exit 0.
- [ ] Deliberately removing a `@promotion` block for a modeled property causes `check-models.mjs` exit 1.
- [ ] Generated traceability section appears in `docs/VERIFICATION.md`; CI fails if it drifts.
- [ ] `check:models` wired into `contracts/ts/package.json`.

## Key files

- New: `contracts/scripts/check-models.mjs`
- Edit (metadata): `specs/seed/command_lifecycle.qnt`, `session_generation.qnt`, `reply_correlation.qnt`, `csrf_browser.qnt`, `patchbay-relational.als`, `snapshot_recovery.qnt`, `authority.qnt`
- Edit (arrays): `contracts/scripts/check-vectors.mjs`
- Edit (generated table): `docs/VERIFICATION.md`
- Design reference: `.work/active/features/feature-formal-model-realignment.md` Units TR + M

## Environment note (mechanical — resolve in-stride)

`quint` and `buf` are installed as npm globals but their bin dir (`/home/agent/.npm-global/bin`) is not on PATH in this harness. The implementer must prefix invocations with that PATH: `export PATH="/home/agent/.npm-global/bin:$PATH"` at the start of the verification run, or invoke via the full path `/home/agent/.npm-global/bin/quint`. Version: Quint 0.32.0. This is a mechanical environment detail, not a semantic question — resolve it in-stride and log under Implementation notes. The `@promotion` block `invocation` fields use bare `quint verify ...`; that's the canonical form for docs — the implementer applies the PATH fix at run time, it does not change the recorded invocation.
