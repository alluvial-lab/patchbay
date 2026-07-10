---
id: epic-public-product-contract-verification-claim-correction
kind: feature
stage: implementing
tags: [verification, protocol, foundation]
parent: epic-public-product-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Verification claim correction

## Brief

Make every verification artifact claim only what its formula, modeled failure boundary, and independent evidence support. Re-inventory current HEAD using the epic's three-way work classification: remove or correct artifacts valueless at every scale, preserve useful seams while deferring their implementation, and keep machinery that serves the committed product. Initial review candidates are evidence to investigate, not a stale deletion checklist, because completed foundation work has already corrected some findings.

The feature is the home for rewriting, renaming, demoting, relocating, or removing remaining overclaims: lifecycle properties that do not represent the durability or race boundary named; weak draft crash/replay/snapshot formulas; fact-consequence Alloy checks; generated TLA+ presented anywhere as independent evidence; toy models appearing in the product inventory; stale semantic traceability; and metadata/process machinery described as behavioral assurance. It preserves the property-graded program, genuine-checking mutation discipline, independently useful Alloy/TLA+/Quint roles, and future-useful authority and protocol seams. It creates honest inputs for executable release assurance; it does not substitute metadata validation for running implementation evidence.

## Epic context

- Parent epic: `epic-public-product-contract`
- Position in epic: independent correction arc; executable release assurance depends on its reconciled property identities and claims.
- Reuse completed `feature-formal-model-realignment` work rather than replaying it.

## Foundation references

- `docs/VERIFICATION.md` — v1 release assurance policy; promotion and genuine-checking rules
- `docs/PROTOCOL.md` — canonical semantics the models claim to represent
- `specs/seed/` — current model inventory
- `contracts/scripts/check-models.mjs` — metadata/traceability check, not a model runner
- `contracts/scripts/check-vectors.mjs` — vector metadata check, not an implementation executor

## Design decisions

- **Q1 — Overclaiming `command_lifecycle.qnt` properties: demote, don't rewrite or rename.** `CommandDurability`, `PreAppendTerminalChoice`, and `LsnDeterminesTerminalWinner` have formulas too narrow to support any product claim worth making under an honest name. Rewriting them to model the real failure boundary (crash/restart, competing pre-append candidates) is v1 formal-gate work owned by `epic-public-product-contract-executable-release-assurance`. Renaming preserves a checked-model property so narrow it isn't worth the traceability overhead. Demote cleanly: `status: promoted → draft`, move from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`, update `@promotion` blocks and VERIFICATION.md tables. The property ids survive as stated-normative obligations for the v1 gate to fulfill with real formulas. The other 5 properties in that model (`TerminalFinality`, `BoundaryDedup`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting`, `NoAcceptedToCompleted`) are genuine and stay promoted.
- **Q2 — `snapshot_recovery.qnt` draft formulas: replace misleading formulas with honest stubs.** The properties are already `status: draft`, so they're not overclaiming tier. But the formulas themselves mislead: `LateEventNoRewrite` checks key existence, `IdempotentLogReplay` checks numeric bounds, `CrashNoAcceptedLost` copies `PreCrashRecoveredState` instead of deriving from the log. Replace each formula with `true` plus a comment: `// formula deferred to promotion; current placeholder is not a behavioral check`. The real crash/replay/snapshot convergence modeling is v1 gate work. An honest stub is better than a misleading formula that looks like a check.
- **No UI surface.** This feature changes formal-model files, CI scripts, and verification prose. No net-new screen, flow, or component. Mockup-first convention does not apply.

## Architectural choice

The correction is a demotion-and-honesty pass, not a rewrite. It preserves the property-graded program, the genuine-checking mutation discipline, the property-id vocabulary as the SSOT, and the future-useful seams (Alloy for delegation/authority-graph/lease problems; TLA+ as a semantic baseline if independently authored; Quint as the primary authoring language). It removes only artifacts whose claims exceed their evidence at every version, and it demotes overclaiming properties to stated-normative so the v1 formal gate can fulfill them with real formulas rather than inheriting misleading checked-model status.

The work is organized into four child stories by artifact surface, sequenced so the traceability machinery stays green after each story lands. Each story is independently reviewable and independently gated by the `[verification]` deep-review lane.

## Implementation Units

### Unit 1: Demote overclaiming `command_lifecycle.qnt` properties
**Story**: `story-verification-correction-command-lifecycle`
**File**: `specs/seed/command_lifecycle.qnt`, `contracts/scripts/check-vectors.mjs`, `docs/VERIFICATION.md`

Demote `CommandDurability`, `PreAppendTerminalChoice`, and `LsnDeterminesTerminalWinner` from `status: promoted` to `status: draft`. In each `@promotion` block: change `status: promoted → draft`, replace the concrete `invocation` with `<TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property>`, and add a `demotion_reason` field explaining the gap. Remove the three property ids from `CHECKED_MODEL_PROPERTIES` and add them to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`. Update VERIFICATION.md prose: the checked-model property list (line 29), the refinement table (line 82), the property definitions (lines 189–190), the generated model-promotion table, the generated conformance-vector table, the seed-model summary table (line 545), and the summary line ("32 promoted" → "29 promoted, 15 draft"). The three draft vectors (`command-acceptance.json`, `late-terminal-candidate-audit-only.json`, `terminal-cancellation-before-completion.json`, `terminal-completion-before-cancellation.json`) remain draft — they reference property ids that are now stated-normative, which is valid (vectors may reference stated-normative properties as draft). Run `node contracts/scripts/check-models.mjs` and `node contracts/scripts/check-vectors.mjs` to confirm both exit 0 and the generated tables reflect the demotion.

**Acceptance Criteria**:
- [ ] Three `@promotion` blocks in `command_lifecycle.qnt` changed to `status: draft` with `demotion_reason` and `<TBD>` invocation.
- [ ] Three ids moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `node contracts/scripts/check-models.mjs` exits 0; generated table shows 29 promoted / 15 draft.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0; generated table shows the three as stated-normative.
- [ ] VERIFICATION.md prose lists updated: line 29 list, line 82 refinement row, lines 189–190 definitions, seed-model summary.
- [ ] The 5 genuine promoted properties in `command_lifecycle.qnt` remain `status: promoted`.

---

### Unit 2: Demote tautological Alloy `ActorIdsUniqueAssert`
**Story**: `story-verification-correction-alloy-actor-ids`
**File**: `specs/seed/patchbay-relational.als`, `contracts/scripts/check-vectors.mjs`, `docs/VERIFICATION.md`

Demote `ActorIdsUnique` from `status: promoted` to `status: draft`. In the `@promotion` block: change `status: promoted → draft`, replace the `invocation` with `<TBD — demoted; assertion checks a constraint already imposed by the ActorIdsUnique fact; actor uniqueness belongs in generated/database constraints plus executable negative tests>`, add `demotion_reason`. Remove the `check ActorIdsUniqueAssert for 5` command line (the assert is a fact-consequence check — removing the fact to make it "genuine" turns vacuous-true into actually-false, so the check is removed, not the fact). Move `ActorIdsUnique` from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`. Update VERIFICATION.md: the checked-model property list (line 36), the generated model-promotion table, the generated conformance-vector table, the seed-model summary table (line 552), and the summary line. Preserve the Alloy file itself and the two already-draft reserved properties (`AuthorityGraphAcyclic`, `SenderMatchesClaim`) — Alloy remains the reserved relational tool for real delegation/authority-graph/lease problems.

**Acceptance Criteria**:
- [ ] `ActorIdsUnique` `@promotion` block changed to `status: draft` with `demotion_reason` and `<TBD>` invocation.
- [ ] `check ActorIdsUniqueAssert for 5` line removed from `patchbay-relational.als`.
- [ ] `ActorIdsUnique` moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `node contracts/scripts/check-models.mjs` exits 0; generated table shows `ActorIdsUnique` as stated-normative.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0.
- [ ] VERIFICATION.md prose lists updated: line 36, seed-model summary.
- [ ] The two already-draft Alloy properties remain draft; the Alloy file and its sigs/facts are preserved.

---

### Unit 3: Replace misleading `snapshot_recovery.qnt` draft formulas with honest stubs
**Story**: `story-verification-correction-snapshot-stubs`
**File**: `specs/seed/snapshot_recovery.qnt`

The six `snapshot_recovery.qnt` properties are already `status: draft`, so no tier change is needed. But their formulas mislead: `LateEventNoRewrite` checks `RecoveredCommandState.keys().contains(cmd)` (key existence, not no-rewrite); `IdempotentLogReplay` checks numeric bounds (`CommittedPrefixLSN >= 0 and Cursor <= CommittedPrefixLSN`), not state equality; `CrashNoAcceptedLost` copies `PreCrashRecoveredState` into `RecoveredCommandState` during replay rather than deriving state from log entries — it assumes the answer. Replace each of the six draft property formulas with `true` plus a comment: `// formula deferred to promotion; current placeholder is not a behavioral check`. Do not change `@promotion` status (stays draft), invocation (stays `<TBD>`), or the model's actions/state — only the property `val` definitions. The real crash/replay/snapshot convergence modeling is v1 gate work owned by `epic-public-product-contract-executable-release-assurance`.

**Acceptance Criteria**:
- [ ] All six draft property formulas in `snapshot_recovery.qnt` replaced with `true` + deferred-to-promotion comment.
- [ ] `@promotion` blocks unchanged (status stays draft, invocation stays `<TBD>`).
- [ ] `node contracts/scripts/check-models.mjs` exits 0 (no metadata change).
- [ ] `quint parse snapshot_recovery.qnt` exits 0 (model still parses).
- [ ] No VERIFICATION.md change required (generated tables derive from `@promotion` metadata, which is unchanged).

---

### Unit 4: Fix stale PROTOCOL.md prose and relocate toy examples
**Story**: `story-verification-correction-prose-and-toys`
**Files**: `docs/PROTOCOL.md`, `specs/seed/Counter.qnt`, `specs/seed/Counter.tla`, `specs/seed/Counter.cfg`, `specs/seed/command_lifecycle.emitted.tla` (and other `*.emitted.tla` if referenced as independent evidence)

Fix two stale PROTOCOL.md assertions that contradict current HEAD: (1) line 94 says `reply_correlation.qnt` does not cover response Operation → Elicitation, but `TypedCorrelation` now covers both Reply → Command/Message and response Operation → Elicitation; (2) line 142 says the checked model permits any non-terminal-to-any-terminal and adjacency rules are stated-normative, but `NoAcceptedToCompleted` is now a checked-model property and `allowedTransition` enforces the exact PROTOCOL table. Update both to reflect current HEAD. Relocate `Counter.qnt`, `Counter.tla`, and `Counter.cfg` out of `specs/seed/` — they are hello-world tooling examples, not product verification. Move to `.agents/skills/quint/` or `.agents/skills/tla-plus/` tooling documentation if those skills reference them, or delete if unreferenced (verified: no references found outside the files themselves). Audit `*.emitted.tla` files: VERIFICATION.md already says they are not an independent lane, but confirm no prose anywhere presents them as independent evidence; if found, correct to "generated inspection artifact, not independently checked."

**Acceptance Criteria**:
- [ ] PROTOCOL.md line 94 corrected: `TypedCorrelation` now covers response Operation → Elicitation.
- [ ] PROTOCOL.md line 142 corrected: `NoAcceptedToCompleted` is checked-model; `allowedTransition` enforces the exact table; full adjacency graph remains stated-normative.
- [ ] `Counter.qnt`, `Counter.tla`, `Counter.cfg` removed from `specs/seed/` (relocated or deleted).
- [ ] `*.emitted.tla` files audited: no prose presents them as independent evidence.
- [ ] `node contracts/scripts/check-models.mjs` exits 0 (Counter files are not `@promotion`-bearing, so removal doesn't affect traceability).

---

## Implementation Order

1. `story-verification-correction-command-lifecycle` (Unit 1) — the largest surface; lands the demotion pattern the other stories follow.
2. `story-verification-correction-alloy-actor-ids` (Unit 2) — same demotion pattern, smaller surface.
3. `story-verification-correction-snapshot-stubs` (Unit 3) — formula-only change, no metadata change.
4. `story-verification-correction-prose-and-toys` (Unit 4) — prose + file relocation; independent of the model changes.

Units 1–3 are sequenced because they all touch `check-vectors.mjs` and the generated VERIFICATION.md tables; landing them sequentially keeps the traceability machinery green after each commit. Unit 4 is independent and could run in parallel, but sequencing it last keeps the model/prose changes in one review arc.

## Testing

- `node contracts/scripts/check-models.mjs` exits 0 after each story.
- `node contracts/scripts/check-vectors.mjs` exits 0 after each story.
- `quint parse specs/seed/command_lifecycle.qnt` exits 0 (model still parses after demotion).
- `quint parse specs/seed/snapshot_recovery.qnt` exits 0 (model still parses after formula stubbing).
- The generated VERIFICATION.md tables reflect the demotions: 29 promoted / 15 draft (down from 32 promoted / 12 draft).
- No promoted property loses its genuine-checking mutation proof (the 5 genuine `command_lifecycle.qnt` properties and all Elicitation/authority/subscription/CSRF properties remain promoted).

## Risks

- **Demotion may feel like losing coverage.** The demoted properties were not providing the coverage their names claimed. Honest stated-normative obligations are more useful than misleading checked-model status. The v1 formal gate owns the real properties.
- **Vector references to demoted properties.** The three draft vectors referencing `CommandDurability`/`LsnDeterminesTerminalWinner`/`PreAppendTerminalChoice` remain valid as draft vectors against stated-normative properties. No vector needs deletion.
- **Alloy file preservation.** Removing the `check` command must not remove the `ActorIdsUnique` fact or the sigs — those are the relational vocabulary future delegation/authority-graph work needs. Only the tautological assert check is removed.
- **Prose drift recurrence.** The PROTOCOL.md fixes are point corrections. The parked `idea-proto-prose-registry-consistency-check.md` is the long-term drift-detection mechanism; this feature does not build it (it's owned by `epic-public-product-contract-public-compatibility`).
