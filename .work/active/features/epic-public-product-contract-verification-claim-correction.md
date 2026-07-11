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
updated: 2026-07-11
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

## Other agent review

- Invoked because: fresh-context adversarial design review before implementation; the design makes demotion/retention claims that needed independent verification against current HEAD.
- Reviewer (Phase 1 — completeness/advisory): `openai-codex/gpt-5.6-sol` (xhigh), fresh-context
  - Missing overclaims at HEAD the design failed to address:
    - `RetryReusesIdAndKey` — only proves the command→key map never changes; never observes an attempted retry's actual id/key (driver only calls `retry("c1", "k1")`).
    - `RetryAfterTerminalReturnsExisting` — formula is identical to `TerminalFinality` (terminal stasis); no returned-record or retry-attempt discriminator.
    - `SessionIdentityTuple` — adapter/deployment/runtime ids are constants, not per-session state; formula checks generation mirroring and singleton set sizes.
    - `LabelsCannotOverrideIdentity` — no routing/target-selection path where a label could override identity.
    - `ElicitationTimeoutNeitherSuccessNorDenial` — claims timeout never implies "grant" but model has no grant state.
    - `LateGenerationInert` — claims `stale_event` audit record but no audit state modeled.
    - `authority.qnt` has the same `= true` stub problem (`NoCommandWithoutGrant = true`, `CompoundIssuer = true`, `GrantAuthorityIsCommandKinds = true`, `RevocationPreventsFuture = always(true)`).
    - `docs/VERIFICATION.md:116-133` contradicts itself about authority properties (says draft, then lists as checked-model).
    - `specs/seed/patchbay-invariants.als` is another superseded toy artifact with the same fact-consequence ActorIds check.
    - `docs/PROTOCOL.md:140` says Operations inherit the three demoted properties as checked — not addressed by the original design.
    - `docs/VERIFICATION.md:545` seed-model summary omits `NoAcceptedToCompleted`.
  - Strengthening alternatives: remove `val` definitions entirely for stubbed properties (not `true`, which passes vacuously); encode story sequencing in `depends_on`; fix checker command order (`check-vectors` before `check-models`); update skill references before relocating toy files.
- Reviewer (Phase 2 — adversarial): `openai-codex/gpt-5.6-sol` (xhigh), fresh-context
  - BLOCKERS: 2 retained command properties also overclaim; re-inventory incomplete (session-identity, labels, elicitation-timeout, late-generation, authority stubs); `true` stubs pass vacuously; final counts wrong (28/16 not 29/15); PROTOCOL.md:140 drift; authority-tier contradiction; stories not sequenced; checker order wrong; toy-file references in skills.
  - IMPORTANT: the 3 headline demotions are substantively correct; `ActorIdsUnique` demotion is correct but the `check` has regression-test value worth preserving as non-promoted; vector references remain mechanically valid; emitted-TLA+ prose already honest.
- Accepted:
  - Demote `RetryReusesIdAndKey` and `RetryAfterTerminalReturnsExisting` alongside the original three (5 demoted from `command_lifecycle.qnt`, leaving only `TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted`).
  - Demote `SessionIdentityTuple`, `LabelsCannotOverrideIdentity`, `ElicitationTimeoutNeitherSuccessNorDenial`, `LateGenerationInert`.
  - Demote `SpawnCreatesDescendantGrant` (found in re-review: uses invented kind names `reboot`/`snapshot`/`stop_session` that contradict the canonical set in PROTOCOL.md:181; allowed-kind set is a hard-coded pure function, not action-created state).
  - Remove `val` definitions entirely for stubbed properties (not `true`) so `quint verify --invariant <name>` fails because the invariant doesn't exist.
  - Fix the `authority.qnt` `= true` stubs the same way (remove the `val` definitions).
  - Fix the VERIFICATION.md authority-tier contradiction (lines 116-133).
  - Fix PROTOCOL.md:140 (refinement table lists demoted properties as checked).
  - Fix additional foundation prose drift: VERIFICATION.md:213 (retry checked claim), VERIFICATION.md:43 (descendant-grant checked claim), VERIFICATION.md:93 (ElicitationFirstAnswerWins overclaim), PROTOCOL.md:75 (stale response-Operation correlation claim), PROTOCOL.md:270 (Elicitation lifecycle stated-normative), PROTOCOL.md:380 (RetryAfterTerminalReturnsExisting checked claim), PROTOCOL.md:568 (stale Elicitation classification), PROTOCOL.md:603 (stale model classification), ADAPTER-PI.md:75-79 and :147 (demoted property refs).
  - Narrow the `@promotion` semantics text of seven retained promoted properties whose formulas are genuine but whose descriptions overclaim: `NoAcceptedToCompleted` (metadata says "must pass through `delivered`" but formula permits `delivered` OR `running`), `FleetAuthorityForSpawn` and `SubscriptionGrantChecked` (claim "authenticated actor" but no authentication evidence modeled), `ElicitationResponderAuthority` (claims endpoint authentication but the model only checks the modeled endpoint-to-actor binding), `browser_local_state_not_authority` (claims grant-check protection but no grant state), `ElicitationStaleTargetInert` (semantics say "do not mutate live Elicitation state" but formula only excludes `answered` and answer data; no response-attempt discriminator or next-state equality), and `SpawnRevocationDoesNotCascade` (when `gDescOs3Live != "yes"`, the descendant condition becomes `true`, so a cascade that deletes the descendant grant passes).
  - Fix stale model header comments: `command_lifecycle.qnt:3-6` (claims durability, terminal-race coverage, 7 promoted properties), `patchbay-relational.als:31-32` (calls ActorIdsUnique promoted and genuine), `snapshot_recovery.qnt:15,21-23` (says removed properties typecheck and are exercised).
  - Relocate `patchbay-invariants.als` alongside `Counter.*`.
  - Encode story sequencing in `depends_on`.
  - Fix checker command order in stories: `check-vectors` exits 0 on regeneration (only exits 1 for validation failures); `check-models` exits 1 on table regeneration. Correct sequence: `check-vectors` (exits 0), then `check-models` (exits 1, regenerates), then `check-models` again (exits 0).
  - Update skill references before relocating toy files.
  - Fix the seed-model summary table to include `NoAcceptedToCompleted`.
  - Fix Unit 2 property-to-file allocation: `LateGenerationInert` is in `session_generation.qnt`, not `elicitation_lifecycle.qnt`. The real split is 3 from `session_generation.qnt` and 1 from `elicitation_lifecycle.qnt`.
- Rejected:
  - Keep the `ActorIdsUnique` `check` command as a non-promoted structural regression test (reviewer suggested this; accepted — the check guards against future fact weakening even though it is not independent assurance). The `@promotion` block is demoted, but the `check` line stays with a comment clarifying it is not promoted assurance.

## Design decisions

- **Q1 — Overclaiming `command_lifecycle.qnt` properties: demote, don't rewrite or rename.** `CommandDurability`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, and `RetryAfterTerminalReturnsExisting` have formulas too narrow to support their product-claim names. Rewriting them to model the real failure boundary (crash/restart, competing pre-append candidates, retry-input identity, returned-record identity) is v1 formal-gate work owned by `epic-public-product-contract-executable-release-assurance`. Renaming preserves checked-model properties so narrow they aren't worth the traceability overhead. Demote cleanly: `status: promoted → draft`, move from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`, update `@promotion` blocks and VERIFICATION.md tables. The property ids survive as stated-normative obligations for the v1 gate to fulfill with real formulas. The 3 genuine properties in that model (`TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted`) stay promoted.
- **Q2 — Overclaiming `session_generation.qnt` and `elicitation_lifecycle.qnt` properties: demote.** `SessionIdentityTuple`, `LabelsCannotOverrideIdentity`, and `LateGenerationInert` (from `session_generation.qnt`) and `ElicitationTimeoutNeitherSuccessNorDenial` (from `elicitation_lifecycle.qnt`) all overclaim. Demote to draft. The genuine properties in those models (`GenerationMonotonic`, `TypedCorrelation`, `ElicitationCorrelationTyped`, `ElicitationPendingFinality`, `ElicitationFirstAnswerWins`, `ElicitationInvalidResponseRejected`, `ElicitationStaleTargetInert`, `ElicitationWithdrawalFinality`) stay promoted.
- **Q3 — `snapshot_recovery.qnt` and `authority.qnt` draft formulas: remove the `val` definitions entirely, not `true`.** The properties are already `status: draft`, so no tier change is needed. But the formulas themselves mislead: they look like real checks but don't model the claimed behavior. Replacing with `true` makes `quint verify --invariant <name>` pass vacuously — reproduating the authority-stub defect rather than correcting it. Instead, remove the `val` definitions entirely so the invariant doesn't exist to verify. The `@promotion` blocks stay (status: draft, invocation: `<TBD>`), preserving the property ids as stated-normative obligations. The real crash/replay/snapshot and authority modeling is v1 gate work.
- **Q3.5 — `SpawnCreatesDescendantGrant` overclaims: demote.** The model uses invented kind names (`reboot`, `snapshot`, `stop_session`) that contradict the canonical descendant-grant allowed-kind set in `docs/PROTOCOL.md:181` (`instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, `session-management`). The allowed-kind set is a hard-coded pure function (`grantAllows`), not action-created state, so a mutation that writes incorrect kinds cannot be represented and the claimed allowed-kind guarantee is not mutation-survivable. Demote to draft; the v1 formal gate owns the real property with the canonical kind set.
- **Q4 — `ActorIdsUnique` demotion: demote the `@promotion` block, keep the `check` as a non-promoted structural regression test.** The assert is a fact-consequence check, not independent assurance — demote it from promoted to draft. But the `check` command has regression-test value: it guards against future accidental weakening of the `ActorIdsUnique` fact. Keep the `check` line with a comment clarifying it is not promoted assurance evidence. Actor uniqueness as product assurance belongs in generated/database constraints plus executable negative tests. Alloy remains the reserved relational tool for real delegation/authority-graph/lease problems.
- **No UI surface.** This feature changes formal-model files, CI scripts, and verification prose. No net-new screen, flow, or component. Mockup-first convention does not apply.

## Architectural choice

The correction is a demotion-and-honesty pass, not a rewrite. It preserves the property-graded program, the genuine-checking mutation discipline, the property-id vocabulary as the SSOT, and the future-useful seams (Alloy for delegation/authority-graph/lease problems; TLA+ as a semantic baseline if independently authored; Quint as the primary authoring language). It removes only artifacts whose claims exceed their evidence at every version, and it demotes overclaiming properties to stated-normative so the v1 formal gate can fulfill them with real formulas rather than inheriting misleading checked-model status.

The work is organized into six child stories by artifact surface, sequenced via `depends_on` so the traceability machinery stays green after each story lands. Each story is independently reviewable and independently gated by the `[verification]` deep-review lane.

## Implementation Units

### Unit 1: Demote overclaiming `command_lifecycle.qnt` properties
**Story**: `story-verification-correction-command-lifecycle`
**File**: `specs/seed/command_lifecycle.qnt`, `contracts/scripts/check-vectors.mjs`, `docs/VERIFICATION.md`, `docs/PROTOCOL.md`

Demote `CommandDurability`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, and `RetryAfterTerminalReturnsExisting` from `status: promoted` to `status: draft`. In each `@promotion` block: change `status: promoted → draft`, replace the concrete `invocation` with `<TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property>`, and add a `demotion_reason` field explaining the gap. Rename the property-section heading at `command_lifecycle.qnt:165` so it no longer labels the now-mixed promoted/draft blocks as all promoted. Remove the five property ids from `CHECKED_MODEL_PROPERTIES` and add them to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`. Update VERIFICATION.md prose: the checked-model property list (the line listing `command_lifecycle.qnt` properties), the `OperationState` ⇿ `CommandState` refinement table (rows referencing `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting`), the property definitions (`LsnDeterminesTerminalWinner`, `PreAppendTerminalChoice`), the idempotent-retry section (~line 213, which calls `RetryReusesIdAndKey` and `RetryAfterTerminalReturnsExisting` checked), the seed-model summary table (move the five to the draft column; add `NoAcceptedToCompleted` which is currently missing), and the summary line. Update PROTOCOL.md: the `OperationState` ⇿ `CommandState` refinement equivalence section (~line 140) lists `CommandDurability`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting` as checked — mark these as stated-normative. Also fix PROTOCOL.md ~line 380, which calls `RetryAfterTerminalReturnsExisting` a checked guarantee. The draft vectors (`command-acceptance.json`, `late-terminal-candidate-audit-only.json`, `terminal-cancellation-before-completion.json`, `terminal-completion-before-cancellation.json`) remain draft — they reference property ids that are now stated-normative, which is valid (vectors may reference stated-normative properties as draft per `check-vectors.mjs` `validatePropertyReferences`). Run `node contracts/scripts/check-vectors.mjs` (exits 0 and regenerates the conformance table), then `node contracts/scripts/check-models.mjs` (exits 1 and regenerates the model table), then run `node contracts/scripts/check-models.mjs` again (exits 0 and confirms the table is current). `check-vectors.mjs` exits 1 only for validation failures; only `check-models.mjs` treats generated-table drift as a failing first pass.

**Demotion reasons**:
- `CommandDurability`: formula is `CMD_IDS.forall(c => state.keys().contains(c))` — commands are pre-installed at init and no action removes keys. Proves map-domain persistence inside the abstraction, not durable acceptance across a failure boundary. No crash, restart, torn commit, or reconstruction is modeled.
- `PreAppendTerminalChoice`: only proves that a transition into terminal assigns a positive terminalLsn. Does not model two pre-append candidates competing before durable append, or relate the selected state to the chosen candidate. Does not even constrain post-assignment stability despite its comment.
- `LsnDeterminesTerminalWinner`: effectively `terminal state implies terminalLsn > 0`. Does not retain competing candidates, compare their LSNs, or establish minimum-LSN selection. The `lateTerminalCandidate` action is a no-op, so the second candidate cannot even commit.
- `RetryReusesIdAndKey`: only proves the command→key map never changes after init. Never observes an attempted retry's actual command id or key — the driver only calls `retry("c1", "k1")`, and the action rejects mismatches structurally. Does not check the named retry-input obligation.
- `RetryAfterTerminalReturnsExisting`: formula is identical to `TerminalFinality` (terminal stasis). No returned-record identity, record count, retry-attempt discriminator, or candidate-creation state. Does not check the named return-existing-record behavior.

**Acceptance Criteria**:
- [ ] Five `@promotion` blocks in `command_lifecycle.qnt` changed to `status: draft` with `demotion_reason` and `<TBD>` invocation.
- [ ] The mixed-status property-section heading in `command_lifecycle.qnt` no longer labels every following block as promoted.
- [ ] Five ids moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0 on its regeneration run; `node contracts/scripts/check-models.mjs` exits 1 on the regeneration run and 0 on the confirming second run; generated tables reflect the demotion.
- [ ] VERIFICATION.md prose updated: checked-model property list, refinement table, property definitions, idempotent-retry section (~line 213), seed-model summary (including adding `NoAcceptedToCompleted`).
- [ ] PROTOCOL.md `OperationState` ⇿ `CommandState` refinement section (~line 140) updated: the five demoted properties marked stated-normative.
- [ ] PROTOCOL.md ~line 380 updated: `RetryAfterTerminalReturnsExisting` no longer called checked.
- [ ] The 3 genuine promoted properties in `command_lifecycle.qnt` remain `status: promoted`: `TerminalFinality`, `BoundaryDedup`, `NoAcceptedToCompleted`.
- [ ] `quint parse specs/seed/command_lifecycle.qnt` exits 0.

---

### Unit 2: Demote overclaiming `session_generation.qnt` and `elicitation_lifecycle.qnt` properties
**Story**: `story-verification-correction-session-elicitation`
**Depends on**: `story-verification-correction-command-lifecycle` (shares `check-vectors.mjs` and generated tables)
**File**: `specs/seed/session_generation.qnt`, `specs/seed/elicitation_lifecycle.qnt`, `contracts/scripts/check-vectors.mjs`, `docs/VERIFICATION.md`

Demote `SessionIdentityTuple`, `LabelsCannotOverrideIdentity`, and `LateGenerationInert` (from `session_generation.qnt`) and `ElicitationTimeoutNeitherSuccessNorDenial` (from `elicitation_lifecycle.qnt`) from `status: promoted` to `status: draft`. In each `@promotion` block: change `status: promoted → draft`, replace the `invocation` with `<TBD — demoted; formula does not model the claimed failure boundary; v1 formal gate owns the real property>`, add `demotion_reason`. Rename the property-section headings at `session_generation.qnt:108` and `elicitation_lifecycle.qnt:538` so they no longer label mixed promoted/draft blocks as all promoted. Remove the four ids from `CHECKED_MODEL_PROPERTIES` and add them to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`. Update VERIFICATION.md prose: the checked-model property lists, the seed-model summary tables for both models, the summary line, and the stale sentence near line 566 that calls `ElicitationTimeoutNeitherSuccessNorDenial` a checked-model analog. Also fix `docs/ADAPTER-PI.md:75-79` which references `LabelsCannotOverrideIdentity` and `LateGenerationInert` as checked properties — mark these as stated-normative. Run `check-vectors` (exits 0 and regenerates), then `check-models` (exits 1 and regenerates), then `check-models` again to confirm exit 0.

**Demotion reasons**:
- `SessionIdentityTuple`: adapter id, deployment scope, and runtime id are constants (`ADAPTER_IDS = Set("a1")`, etc.), not per-session identity state. The formula checks generation mirroring and that three singleton sets have size one, not the four-field identity tuple named in the metadata.
- `LabelsCannotOverrideIdentity`: proves labels use strings disjoint from three constant singleton sets and that generation mirrors remain equal. Models no routing/target-selection path in which a label could override identity.
- `LateGenerationInert` (in `session_generation.qnt`): the formula proves only that generation and `identityGeneration` do not change. Its promoted semantics additionally claim a `stale_event` audit record, but no audit state is modeled.
- `ElicitationTimeoutNeitherSuccessNorDenial` (in `elicitation_lifecycle.qnt`): metadata and VERIFICATION.md claim timeout never implies "grant," but the model has no grant state. The formula checks only answer/decline fields.

**Acceptance Criteria**:
- [ ] Four `@promotion` blocks changed to `status: draft` with `demotion_reason` and `<TBD>` invocation.
- [ ] Four ids moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0 on its regeneration run; `node contracts/scripts/check-models.mjs` exits 1 on the regeneration run and 0 on the confirming second run.
- [ ] VERIFICATION.md prose updated: checked-model property lists, seed-model summary tables, and the stale `ElicitationTimeoutNeitherSuccessNorDenial` “checked-model analog” sentence (~line 566).
- [ ] The mixed-status property-section headings in `session_generation.qnt` and `elicitation_lifecycle.qnt` no longer label every following block as promoted.
- [ ] `docs/ADAPTER-PI.md:75-79` updated: `LabelsCannotOverrideIdentity` and `LateGenerationInert` marked stated-normative.
- [ ] The genuine promoted properties in those models remain promoted: `GenerationMonotonic`, `TypedCorrelation`, `ElicitationCorrelationTyped`, `ElicitationPendingFinality`, `ElicitationFirstAnswerWins`, `ElicitationInvalidResponseRejected`, `ElicitationStaleTargetInert`, `ElicitationWithdrawalFinality`.
- [ ] `quint parse specs/seed/session_generation.qnt` and `quint parse specs/seed/elicitation_lifecycle.qnt` exit 0.

---

### Unit 3: Demote tautological Alloy `ActorIdsUnique` and relocate superseded toy artifacts
**Story**: `story-verification-correction-alloy-and-toys`
**Depends on**: `story-verification-correction-session-elicitation` (shares `check-vectors.mjs` and generated tables)
**Files**: `specs/seed/patchbay-relational.als`, `specs/seed/patchbay-invariants.als`, `contracts/scripts/check-vectors.mjs`, `docs/VERIFICATION.md`, `.agents/skills/alloy/SKILL.md`, `.agents/skills/quint/SKILL.md`, `.agents/skills/tla-plus/SKILL.md`

Demote `ActorIdsUnique` from `status: promoted` to `status: draft`. In the `@promotion` block: change `status: promoted → draft`, replace the `invocation` with `<TBD — demoted; assertion checks a constraint already imposed by the ActorIdsUnique fact; actor uniqueness belongs in generated/database constraints plus executable negative tests>`, add `demotion_reason`, and rewrite `semantics` so it does not claim the assertion proves non-vacuity: actor-id injectivity remains the stated obligation, while the retained fact-consequence check is only a structural regression test against accidental fact weakening. Keep the `check ActorIdsUniqueAssert for 5` command line but add a comment: `// structural regression test — NOT promoted assurance; guards against accidental fact weakening`. Move `ActorIdsUnique` from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`. Relocate `specs/seed/patchbay-invariants.als` (superseded by `patchbay-relational.als`, same fact-consequence ActorIds check) and `specs/seed/Counter.qnt`, `Counter.tla`, `Counter.cfg` (hello-world tooling examples) out of `specs/seed/`. These files are referenced by `.agents/skills/alloy/SKILL.md:106`, `.agents/skills/quint/SKILL.md:152`, and `.agents/skills/tla-plus/SKILL.md:104` as "hello-world artifact" pointers. Relocate the files to the skill directories (e.g., `.agents/skills/alloy/examples/`, `.agents/skills/quint/examples/`, `.agents/skills/tla-plus/examples/`) and update the skill references to point at the new locations. Update VERIFICATION.md: the checked-model property list, the generated tables, the seed-model summary table, and the summary line. Run `check-vectors` (exits 0 and regenerates), then `check-models` (exits 1 and regenerates), then `check-models` again (exits 0).

**Acceptance Criteria**:
- [ ] `ActorIdsUnique` `@promotion` block changed to `status: draft` with `demotion_reason`, `<TBD>` invocation, and semantics that describe the stated injectivity obligation without falsely claiming the fact-consequence assert proves non-vacuity.
- [ ] `check ActorIdsUniqueAssert for 5` line kept with a comment clarifying it is not promoted assurance.
- [ ] `ActorIdsUnique` moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `patchbay-invariants.als`, `Counter.qnt`, `Counter.tla`, `Counter.cfg` relocated out of `specs/seed/` to skill example directories.
- [ ] `.agents/skills/alloy/SKILL.md`, `.agents/skills/quint/SKILL.md`, `.agents/skills/tla-plus/SKILL.md` references updated to new file locations.
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0; then `node contracts/scripts/check-models.mjs` exits 0.
- [ ] VERIFICATION.md prose updated: checked-model property list, seed-model summary.
- [ ] The two already-draft Alloy properties (`AuthorityGraphAcyclic`, `SenderMatchesClaim`) remain draft.
- [ ] The Alloy file's sigs, facts, and the `check` command are preserved.

---

### Unit 4: Remove misleading draft formulas from `snapshot_recovery.qnt` and `authority.qnt`
**Story**: `story-verification-correction-draft-formulas`
**Depends on**: `story-verification-correction-alloy-and-toys` (shares `check-vectors.mjs` and generated tables)
**Files**: `specs/seed/snapshot_recovery.qnt`, `specs/seed/authority.qnt`, `docs/VERIFICATION.md`

The six `snapshot_recovery.qnt` properties, four `authority.qnt` draft properties, and one `authority.qnt` promoted property (`SpawnCreatesDescendantGrant`) need correction. The draft properties are already `status: draft`, so no tier change is needed for them — but their formulas mislead. `SpawnCreatesDescendantGrant` is currently promoted and must be demoted. Remove the `val` definitions entirely for all eleven so `quint verify --invariant <name>` fails because the invariant doesn't exist, rather than passing vacuously with `true`. Keep the `@promotion` blocks (status: draft, invocation: `<TBD>`) so the property ids survive as stated-normative obligations. Do not change the model's actions/state.

**Properties to stub (remove `val` definition)**:
- `snapshot_recovery.qnt`: `SnapshotStaleRejected`, `SnapshotCrossDomainRejected`, `SnapshotConsistentPrefix`, `LateEventNoRewrite`, `CrashNoAcceptedLost`, `IdempotentLogReplay`
- `authority.qnt`: `NoCommandWithoutGrant`, `CompoundIssuer`, `GrantAuthorityIsCommandKinds`, `RevocationPreventsFuture` (already draft), and `SpawnCreatesDescendantGrant` (demote from promoted to draft first)

**Why each `snapshot_recovery.qnt` formula misleads**:
- `SnapshotStaleRejected`: checks `SnapshotRevision >= Cursor` (non-decreasing revision), not that stale snapshots are rejected as authority sources.
- `SnapshotCrossDomainRejected`: checks current snapshot origin matches core, not that cross-domain snapshots are rejected.
- `SnapshotConsistentPrefix`: checks lookup-table consistency, not that materialization reads a consistent log prefix.
- `LateEventNoRewrite`: checks key existence, not that late events don't rewrite state.
- `CrashNoAcceptedLost`: copies `PreCrashRecoveredState` into `RecoveredCommandState` during replay rather than deriving from log entries — assumes the answer.
- `IdempotentLogReplay`: checks numeric bounds, not that replay produces identical state.

**Why each `authority.qnt` formula misleads**:
- `NoCommandWithoutGrant = true`: literal placeholder, not a check.
- `CompoundIssuer = true`: literal placeholder.
- `GrantAuthorityIsCommandKinds = true`: literal placeholder.
- `RevocationPreventsFuture = always(true)`: literal placeholder.

**Why `SpawnCreatesDescendantGrant` overclaims**:
- The model uses invented kind names (`reboot`, `snapshot`, `stop_session`) in `grantAllows` that contradict the canonical descendant-grant allowed-kind set in `docs/PROTOCOL.md:181` (`instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, `session-management`). The allowed-kind set is a hard-coded pure function, not action-created state, so a mutation that writes incorrect kinds cannot be represented and the claimed allowed-kind guarantee is not mutation-survivable.

Also fix the VERIFICATION.md authority-tier contradiction (lines ~116-133): the prose says the four general authority properties "remain draft/stated-normative" but then lists them under "Checked-model spawn properties." Correct the list to include only the genuinely promoted spawn properties (`FleetAuthorityForSpawn`, `SpawnRevocationDoesNotCascade`, `ElicitationResponderAuthority`) and explicitly state the five properties (`NoCommandWithoutGrant`, `CompoundIssuer`, `GrantAuthorityIsCommandKinds`, `RevocationPreventsFuture`, `SpawnCreatesDescendantGrant`) are stated-normative with no executable formula. Move `SpawnCreatesDescendantGrant` from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.

**Acceptance Criteria**:
- [ ] All eleven draft property `val` definitions removed from `snapshot_recovery.qnt` and `authority.qnt`.
- [ ] `SpawnCreatesDescendantGrant` `@promotion` block changed to `status: draft` with `demotion_reason` and `<TBD>` invocation.
- [ ] `SpawnCreatesDescendantGrant` moved from `CHECKED_MODEL_PROPERTIES` to `STATED_NORMATIVE_PROPERTIES` in `check-vectors.mjs`.
- [ ] `@promotion` blocks for the other ten unchanged (status stays draft, invocation stays `<TBD>`).
- [ ] `node contracts/scripts/check-vectors.mjs` exits 0 on its regeneration run; `node contracts/scripts/check-models.mjs` exits 1 on the regeneration run and 0 on the confirming second run.
- [ ] `quint parse specs/seed/snapshot_recovery.qnt` and `quint parse specs/seed/authority.qnt` exit 0.
- [ ] VERIFICATION.md authority-tier contradiction fixed: only the 3 genuinely promoted spawn properties (`FleetAuthorityForSpawn`, `SpawnRevocationDoesNotCascade`, `ElicitationResponderAuthority`) listed as checked-model; the 5 general/descendant properties explicitly stated-normative with no executable formula.

---

### Unit 5: Fix stale PROTOCOL.md prose and audit emitted TLA+
**Story**: `story-verification-correction-prose`
**Depends on**: `story-verification-correction-draft-formulas` (transitively includes the command/session demotions; this story must run after all tier changes so one doc does not simultaneously underclaim prose and retain generated checked-model rows)
**Files**: `docs/PROTOCOL.md`, `docs/VERIFICATION.md`, `docs/ADAPTER-PI.md`, `specs/seed/*.emitted.tla`

Fix stale PROTOCOL.md assertions that contradict current HEAD, and audit emitted TLA+ files for any prose presenting them as independent evidence. Also fix the stale model classification in the PROTOCOL.md extension seams registry and additional foundation prose drift points.

**Acceptance Criteria**:
- [ ] PROTOCOL.md `reply_correlation.qnt` coverage claim (~lines 75 and 94) corrected: `TypedCorrelation` now covers response Operation → Elicitation.
- [ ] PROTOCOL.md transition-adjacency claim (~line 142) corrected: `NoAcceptedToCompleted` is checked-model; `allowedTransition` enforces the exact table; full adjacency graph remains stated-normative.
- [ ] PROTOCOL.md extension seams registry (~line 603) corrected: Elicitation, spawn-authority, subscription, and response-correlation models no longer classified as purely "stated-normative, reserved model ids" — they have partial checked-model coverage.
- [ ] PROTOCOL.md `ElicitationState` lifecycle classification (~line 270) corrected: no longer purely "stated-normative until promoted" — partial checked-model coverage.
- [ ] PROTOCOL.md extension pressure classification (~line 568) corrected: stale Elicitation classification updated.
- [ ] VERIFICATION.md:43 corrected: descendant-grant behavior no longer called checked-model after `SpawnCreatesDescendantGrant` demotion.
- [ ] VERIFICATION.md:93 corrected: `ElicitationFirstAnswerWins` semantics narrowed to answer-terminal only (not decline-terminal).
- [ ] ADAPTER-PI.md:147 corrected: `LateGenerationInert` no longer described as "verified".
- [ ] `*.emitted.tla` files audited: no prose presents them as independent evidence.
- [ ] `node contracts/scripts/check-models.mjs` exits 0.

---

### Unit 6: Narrow retained promoted property semantics and fix stale model header comments
**Story**: `story-verification-correction-retained-semantics`
**Depends on**: `story-verification-correction-prose` (all demotions and prose-tier corrections must be final before the last retained-semantics/header pass)
**Files**: `specs/seed/command_lifecycle.qnt`, `specs/seed/authority.qnt`, `specs/seed/subscription_authority.qnt`, `specs/seed/csrf_browser.qnt`, `specs/seed/patchbay-relational.als`, `specs/seed/snapshot_recovery.qnt`, `docs/VERIFICATION.md`

Five Seven retained promoted properties have formulas that are genuine (mutation-survivable, independent oracle) but whose `@promotion` semantics text overclaims what the formula establishes. Narrow the semantics text to match the formula, not demote the property.

**Properties to narrow**:
- `NoAcceptedToCompleted` (`command_lifecycle.qnt`): metadata says the command "must pass through `delivered`" but the formula permits either `delivered` OR `running` immediately before completion. Narrow the semantics to: "a command cannot transition directly from `accepted` to `completed`; it must pass through `delivered` or `running`".
- `FleetAuthorityForSpawn` (`authority.qnt`): semantics claims "authenticated actor" but the model has no authentication evidence — it proves grant-subject matching for the modeled actor. Narrow to: "spawn acceptance requires a live fleet-scope spawn Grant whose subject matches the submitting actor; per-session grants alone cannot authorize spawning a not-yet-existing session".
- `SubscriptionGrantChecked` (`subscription_authority.qnt`): same issue — claims "authenticated actor" but no authentication evidence. Narrow to: "subscription establishment succeeds only with a live subscribe-kind Grant record whose subject matches the submitting actor and stream/filter scope".
- `ElicitationResponderAuthority` (`authority.qnt`): semantics claims endpoint authentication, but the model has no independent authentication evidence; it checks that the modeled submitting endpoint maps to the expected responder actor and that the claimed actor matches. Narrow to: "response Operations are accepted only when the modeled submitting endpoint maps to the expected responder actor and the claimed actor matches that responder". Apply the same narrowing to `docs/VERIFICATION.md:127`.
- `browser_local_state_not_authority` (`csrf_browser.qnt`): semantics claims protection of "grant checks" but the model has no grant state — it checks operator-session status and CSRF evidence. Narrow to: "browser-local UI claims cannot grant authority or override server-side session/CSRF checks". Apply the same narrowing to `docs/VERIFICATION.md:298`.
- `ElicitationStaleTargetInert` (`elicitation_lifecycle.qnt`): semantics says "do not mutate live Elicitation state" but the formula only excludes `answered` and answer data; it has no response-attempt discriminator or next-state equality, so a stale response mutation to another state could pass. Narrow to: "responses to stale target/session generations do not cause the Elicitation to become answered or record answer data".
- `SpawnRevocationDoesNotCascade` (`authority.qnt`): when `gDescOs3Live != "yes"`, the descendant condition becomes `true`, so a cascade that deletes the descendant grant passes. Narrow to: "revoking the fleet spawn grant blocks future spawns and, when a descendant grant exists, does not revoke it".

**Stale model header comments to fix**:
- `command_lifecycle.qnt:3-6`: says "models accepted-command durability, the first-durable-terminal-commit-wins race, and idempotency-boundary dedup" and "carries the 7 promoted model properties". After demotion, the model carries 3 promoted properties and does not model durability or the terminal-race boundary. Update to reflect the 3 retained promoted properties and the actual scope (terminal finality, boundary dedup, no-accepted-to-completed adjacency).
- `patchbay-relational.als:31-32`: says "Promoted model: the one relational invariant that is genuinely checkable". After demotion, no Alloy property is promoted. Update to reflect that the model contains draft/reserved properties only.
- `snapshot_recovery.qnt:15,21-23`: says draft properties "typecheck cleanly" and "are exercised against the LSN/cursor/revision/domain/generation core". After removing the `val` definitions, the properties don't typecheck (they don't exist). Update to reflect that the property ids are reserved stated-normative obligations with no executable formula.

**Acceptance Criteria**:
- [ ] Seven `@promotion` semantics fields narrowed to match their formulas.
- [ ] VERIFICATION.md:127 uses the same narrowed checked-model description for `ElicitationResponderAuthority`.
- [ ] VERIFICATION.md:298 uses the same narrowed checked-model description for `browser_local_state_not_authority`.
- [ ] `command_lifecycle.qnt` header comment updated to reflect 3 retained promoted properties and actual scope.
- [ ] `patchbay-relational.als` header comment and the adjacent NOTE at lines 50-52 updated: remove the claim that the check "verifies non-vacuity" and the claim that non-vacuity is "observed via the check finding a satisfying instance" — an UNSAT assertion check does not establish that.
- [ ] `snapshot_recovery.qnt` header comment updated to reflect reserved stated-normative obligations with no executable formula.
- [ ] `node contracts/scripts/check-models.mjs` exits 0 (semantics text changes don't affect tier derivation).
- [ ] `quint parse` exits 0 for all affected model files.

---

## Implementation Order

1. `story-verification-correction-command-lifecycle` (Unit 1) — the largest surface; lands the demotion pattern the other stories follow. `depends_on: []`.
2. `story-verification-correction-session-elicitation` (Unit 2) — same demotion pattern. `depends_on: [story-verification-correction-command-lifecycle]`.
3. `story-verification-correction-alloy-and-toys` (Unit 3) — demotion + toy relocation. `depends_on: [story-verification-correction-session-elicitation]`.
4. `story-verification-correction-draft-formulas` (Unit 4) — remove misleading `val` definitions. `depends_on: [story-verification-correction-alloy-and-toys]`.
5. `story-verification-correction-prose` (Unit 5) — prose fixes after all demotions. `depends_on: [story-verification-correction-draft-formulas]`.
6. `story-verification-correction-retained-semantics` (Unit 6) — narrow retained promoted property semantics and fix stale model header comments. `depends_on: [story-verification-correction-prose]`.

Units 1–4 are sequenced via `depends_on` because they all touch `check-vectors.mjs` and the generated VERIFICATION.md tables; landing them sequentially keeps the traceability machinery green after each commit. Unit 5 depends on Unit 4 so every tier demotion and generated-table update lands before the prose is reconciled; this prevents a single VERIFICATION.md revision from simultaneously describing a property as stated-normative in prose and checked-model in a generated row. Unit 6 depends on Unit 5 (and therefore transitively on Unit 4) so all model demotions, prose-tier corrections, and generated-table updates are final before retained semantics and header comments are updated; this also avoids parallel writes to `docs/VERIFICATION.md`.

**Checker command order (critical):** `check-vectors.mjs` exits 0 on regeneration (it only exits 1 for validation failures). `check-models.mjs` exits 1 when its generated table changes on the first run, then exits 0 on the second run. Correct sequence for stories that touch `check-vectors.mjs`: run `node contracts/scripts/check-vectors.mjs` (exits 0, regenerates conformance table), then `node contracts/scripts/check-models.mjs` (exits 1, regenerates model table), then `node contracts/scripts/check-models.mjs` again (exits 0, confirms current).

## Testing

- `node contracts/scripts/check-vectors.mjs` exits 0 (regenerates conformance table); then `node contracts/scripts/check-models.mjs` exits 0 on second run (first run exits 1, regenerates model table).
- `quint parse specs/seed/command_lifecycle.qnt`, `quint parse specs/seed/session_generation.qnt`, `quint parse specs/seed/elicitation_lifecycle.qnt`, `quint parse specs/seed/snapshot_recovery.qnt`, `quint parse specs/seed/authority.qnt` all exit 0.
- The generated VERIFICATION.md tables reflect the demotions: 21 promoted / 23 draft (down from 32 promoted / 12 draft). Demoted: 5 from `command_lifecycle.qnt` + 4 from `session_generation.qnt`/`elicitation_lifecycle.qnt` + 1 Alloy + 1 `SpawnCreatesDescendantGrant` = 11 demoted from promoted to draft.
- No promoted property loses its genuine-checking mutation proof (the genuine `command_lifecycle.qnt` properties, `GenerationMonotonic`, `TypedCorrelation`, all genuine Elicitation properties, the 3 genuine spawn-authority properties, all subscription properties, and all CSRF properties remain promoted).

## Risks

- **Demotion may feel like losing coverage.** The demoted properties were not providing the coverage their names claimed. Honest stated-normative obligations are more useful than misleading checked-model status. The v1 formal gate owns the real properties.
- **Vector references to demoted properties.** The draft vectors referencing demoted property ids remain valid as draft vectors against stated-normative properties per `check-vectors.mjs` `validatePropertyReferences`. No vector needs deletion.
- **Alloy file preservation.** The `check` command stays as a non-promoted structural regression test. The `ActorIdsUnique` fact and sigs are preserved — those are the relational vocabulary future delegation/authority-graph work needs.
- **Removed `val` definitions.** Removing the `val` definitions means `quint verify --invariant <name>` will fail because the invariant doesn't exist. This is the intended behavior — the properties are stated-normative obligations, not executable checks. The `@promotion` blocks preserve the property ids for the v1 gate.
- **Prose drift recurrence.** The PROTOCOL.md fixes are point corrections. The parked `idea-proto-prose-registry-consistency-check.md` is the long-term drift-detection mechanism; this feature does not build it (it's owned by `epic-public-product-contract-public-compatibility`).
- **Skill reference updates.** Relocating toy files requires updating skill references in the same story. Deletion without updating skills breaks current documentation.

## Implementation summary

All seven child stories implemented and advanced to `stage: review` via `implement-orchestrator` (sequential wave, width 1 — the `depends_on` chain is strictly linear because every unit touches `check-vectors.mjs` and the generated VERIFICATION.md tables):

| Unit | Story | Commit | Status |
|---|---|---|---|
| 1 | `story-verification-correction-command-lifecycle` | `eae524e` | review |
| 2 | `story-verification-correction-session-elicitation` | `9faa6c0` | review |
| 3 | `story-verification-correction-alloy-and-toys` | `6dd69ca` | review |
| 4 | `story-verification-correction-draft-formulas` | `c4e8279` | review |
| 5 | `story-verification-correction-prose` | `48fe336` | review |
| 6 | `story-verification-correction-retained-semantics` | `15276ce` | review |
| 7 | `story-verification-correction-trace-fidelity-demotion` | `19f54c4` | review |

Units 1–6 were the original design; Unit 7 was added by the round-2 deep review (4 promoted properties with trace-fidelity/mutation-fragility defects the original design missed).

**Net verification-claim result:** 15 properties demoted from promoted (checked-model) to draft (stated-normative): 5 from `command_lifecycle.qnt`, 4 from `session_generation.qnt`/`elicitation_lifecycle.qnt`, 1 Alloy (`ActorIdsUnique`), 1 from `authority.qnt` (`SpawnCreatesDescendantGrant`), and 4 trace-fidelity-defective (`FleetAuthorityForSpawn`, `ElicitationResponderAuthority`, `SpawnRevocationDoesNotCascade`, `SubscriptionGrantChecked`). 15 misleading/defective `val`/`temporal` formulas removed entirely (not stubbed to `true`) so `quint verify --invariant <name>` fails honestly rather than passing vacuously or on a defective formula. 7 retained promoted properties' `semantics:` text narrowed to match their formulas. `GenerationMonotonic` prose corrected (strict-supersession is action-enforced, not checked). 4 superseded toy artifacts relocated out of `specs/seed/` to skill example directories. 4 stale model header comments fixed. Stale prose reconciled across PROTOCOL.md, VERIFICATION.md, ADAPTER-PI.md, SPEC.md, GLOSSARY.md.

**Final property tier counts:** 17 promoted / 30 stated-normative (27 modeled-draft + 3 reserved-unmodeled) = 47 total. Down from 32 promoted / 15 stated-normative pre-correction.

**Cross-cutting deviations:**
- Unit 3 (alloy-and-toys): the toy files are referenced in `.research/analysis/briefs/*.md` (historical attestations) and `feature-formal-model-seed.md` (a done feature body) beyond the 3 in-scope skill SKILL.md files. The worker left both untouched — research attestations are historical records that remain true, and editing a done feature re-opens its review surface. Recorded as deferred references.
- Unit 5 (prose): the story's Files list omitted `docs/SPEC.md` and `docs/GLOSSARY.md` (acceptance items 10-12); the worker handled both. Also corrected a residual `SpawnCreatesDescendantGrant` checked-model reference left by Unit 4.
- Unit 6 (retained-semantics): the story body said "five" properties but the design narrowed seven (the review phase added `ElicitationStaleTargetInert` and `SpawnRevocationDoesNotCascade`). The worker corrected the count.

**Verification status:** `quint parse` exits 0 for all affected model files; `check-vectors.mjs` exits 0; `check-models.mjs` exits 0 (tables current). Generated VERIFICATION.md tables reflect the demotions. The 4 trace-fidelity demotions (Unit 7) were each verified by an independent mutation test before demotion; the round-3 review's broader claim that 9 *surviving* promoted properties share the defect is partially disputed by the host (the `ElicitationPendingFinality` mutation was caught by the property, not passed as the reviewer claimed) and routed to `executable-release-assurance` as a model-architecture question rather than absorbed as further demotions in this feature.

**Deferred to v1 formal gate (`epic-public-product-contract-executable-release-assurance`):** real formulas for the 15 demoted properties (crash/restart durability, competing pre-append candidates, retry-input identity, returned-record identity, per-session identity tuple, label-override routing, stale-event audit, elicitation-timeout grant boundary, descendant-grant allowed-kind set with action-created state, snapshot/recovery failure boundaries, general authority, independent-actor/endpoint/scope verification, genuine non-cascade formula with pre/post state). Also deferred: the systematic trace-fidelity question across the surviving promoted properties (see `idea-csrf-trace-fidelity` and the round-3 review record) — whether each surviving property's formula is an independent oracle against the model's own actions is a model-architecture determination the v1 gate makes when it builds the real formal checker.

## Review (2026-07-10)

**Verdict**: Approve with comments

**Lane**: deep (feature), substrate mode. Two-phase fresh-context review on `openai-codex` (different model class from the umans orchestrator):
- Phase 1 (completeness/complementary): `openai-codex/gpt-5.6-terra` (high)
- Phase 2 (adversarial): `openai-codex/gpt-5.6-sol` (xhigh)

**Blockers**: 4 — all hand-authored VERIFICATION.md prose that still used pre-narrowing wording for properties whose `@promotion` `semantics:` was narrowed in Unit 6 (generated table rows auto-regenerated, but the non-generated descriptions did not). All four fixed inline in this review stride:
- `ElicitationStaleTargetInert` (VERIFICATION.md:99) — said "do not mutate live state"; narrowed to "do not cause the Elicitation to become answered or record answer data".
- `SpawnRevocationDoesNotCascade` (VERIFICATION.md:123) — said "does not revoke already-created descendant grants unless those grants are separately revoked" (a qualifier the formula does not establish); narrowed to "revoking the fleet spawn grant blocks future spawns and, when a descendant grant exists, does not revoke it".
- `ElicitationResponderAuthority` (VERIFICATION.md:124) — claimed "the responding endpoint is audited" (not proven by the formula); narrowed to endpoint-mapping + claimed-actor matching, with auditing noted as stated-normative.
- `browser_local_state_not_authority` (VERIFICATION.md:303) — claimed "core grant checks" / "session/CSRF/grant evidence" but the model has no grant state; narrowed to "server-side session/CSRF checks" with explicit "model has no grant state" note.

**Important**: 2 — filed as backlog items:
- `idea-spawn-revocation-model-coverage.md` — `SpawnRevocationDoesNotCascade` formula is accurate over reachable traces (the `revokeSpawnGrant` action preserves the descendant), but the model does not explore the separate-revocation path (`revokeDescendantGrant` is not in the `step` relation) and the descendant clause is `true` when no descendant exists. Not a demotion candidate (genuine oracle for reachable traces); the v1 formal gate owns the strengthened formula.
- `idea-check-models-draft-discipline-enforcement.md` — `check-models.mjs` does not require `demotion_reason` for drafts, does not validate draft invocations are `<TBD>`, and does not verify executable definitions are absent for stated-normative properties. The misleading-formula defect this feature corrected can recur while CI stays green.

**Nits**: none recorded.

**Notes**: Both phases independently converged on the same 4 prose drift points (high confidence these are real, not reviewer noise). All 4 fixed inline in this review stride. All 11 demotion reasons verified accurate against their actual formulas. Array consistency clean (47 unique ids, no duplicates, sorted). Checker green after prose fixes.

## Review round 2 (2026-07-10)

**Verdict**: Request changes

**Lane**: deep, adversarial convergence pass, `openai-codex/gpt-5.6-sol` (xhigh), fresh-context. Given the round-1 fixes (applied by the host, not fresh-context-verified) and the standing evidence that silent overclaims survive review, a second convergence pass was run.

**Blockers** (3, all independently verified by the host against HEAD):
- `SpawnRevocationDoesNotCascade` (`authority.qnt`) is not a mutation-survivable oracle. Host ran the reviewer's mutation: setting `gDescOs3Live' = "no"` in `revokeSpawnGrant` (deleting the descendant during fleet revocation) still passes `quint verify --temporal spawn_revocation_does_not_cascade`. The round-1 backlog note (`idea-spawn-revocation-model-coverage`) incorrectly called the formula "a genuine mutation-survivable independent oracle" — that classification was wrong; the mutation test disproves it. Superseded by `story-verification-correction-trace-fidelity-demotion`.
- `FleetAuthorityForSpawn`, `ElicitationResponderAuthority`, `SubscriptionGrantChecked` (authority/subscription) have a trace-fidelity defect: their invariants inspect state (`LastSpawnActor`, `ResponseEndpoint*`/`ResponseClaimedActor*`, `LastSubscriptionActor`/`LastSubscriptionScope`) written by the *same action* that decides acceptance. A mutation that accepts arbitrary inputs while recording the expected values passes. The formulas prove recorded-state consistency, not independent submitting-actor/endpoint/scope authority. Verified via diff: this feature did not introduce the architecture (the vals/actions are untouched by Units 1-6), but the defect contradicts the feature's own honesty theme and the narrowed `semantics:` still overclaims what the formula independently establishes.
- `GenerationMonotonic` hand-authored prose (`VERIFICATION.md:204`, `ADAPTER-PI.md:78`) presents action-enforced strict-supersession as checked-model behavior. The `@promotion` `semantics:` (SSOT) explicitly says strict-supersession "is NOT a checked temporal property." This feature's Unit 2 touched the surrounding list line and left the overclaim.

**Important** (2):
- VERIFICATION.md:122,144 hand-authored `FleetAuthorityForSpawn`/`SubscriptionGrantChecked` descriptions don't match their generated `semantics:` rows. Addressed by the demotion (they leave the checked-model list).
- `idea-check-models-draft-discipline-enforcement.md` is overbroad (a blanket ban on draft executable definitions would reject intentionally-retained non-vacuous demoted formulas). Needs refinement before implementation.

**Nits**: the 2 round-1 backlog files use active-item-style frontmatter; canonical parked-item frontmatter is minimal.

**Disposition**: Bounced the feature `done → implementing`. Filed `story-verification-correction-trace-fidelity-demotion` (Unit 7, depends on Unit 6) to demote the 4 trace-fidelity-defective properties, fix the `GenerationMonotonic` prose, and reconcile the tier changes across docs. The round-1 backlog note `idea-spawn-revocation-model-coverage` is superseded (the property demotes rather than staying promoted-with-a-note). Operator chose Option 2 (demote) over Option 1 (narrow-and-keep) for the trace-fidelity properties, consistent with how the other 11 overclaims were handled.

**Notes**: Round 2 did not converge — it escalated, finding defects round 1 missed. The convergence loop continues after Unit 7 lands. The host verified each round-2 claim independently (ran the `SpawnRevocationDoesNotCascade` mutation test, confirmed the `GenerationMonotonic` `@promotion` semantics vs. prose mismatch, confirmed via diff that the trace-fidelity architecture is pre-existing not feature-introduced) rather than accepting the reviewer's assertions. `SubscriptionAudited` and `SubscriptionCursorReplayAuthorized` assessed as NOT sharing the trace-fidelity defect (structural invariants, not recorded-state inspection) — they stay promoted.

## Review round 3 (2026-07-11)

**Verdict**: Request changes (but host partially disputes scope — see Notes)

**Lane**: deep, adversarial convergence pass, `openai-codex/gpt-5.6-sol` (xhigh), fresh-context. Given round 2 found real blockers and Unit 7 landed, a third pass verified the fixes and hunted for remaining overclaims.

**Round-2 fixes: verified fixed.** All 4 trace-fidelity demotions complete (status draft, `<TBD>` invocation, accurate demotion_reason, vals removed, @promotion blocks preserved, moved to STATED_NORMATIVE). `GenerationMonotonic` prose narrowed correctly in both VERIFICATION.md and ADAPTER-PI.md.

**Round-3 claimed blockers (3 classes, 9 promoted properties)** — the reviewer ran mutation tests on the 6 surviving Elicitation properties, 2 surviving subscription properties, and `TypedCorrelation`, claiming all 9 pass when the accepting action is mutated to lie about recorded state.

**Host independent verification (PARTIALLY DISPUTES the reviewer):**
- `ElicitationPendingFinality`: the host ran the reviewer's described mutation (rewrite `state` + `firstTerminalState` together on re-terminalization) — the property CAUGHT it ("Found an issue" / Error). A fuller mutation rewriting all 4 checked fields was ALSO caught. The temporal property's `__saved_` baseline mechanism (Apalache compiles it to compare current state against the prior step's saved baseline) gives it more protection than the reviewer credited. The reviewer's "passed at max-steps 10" claim is NOT reproduced by the host for this property. The round-3 finding for the Elicitation properties is **not confirmed** by the host's mutation tests and is **disputed**.
- `SubscriptionAudited`: the host confirmed a *partial* defect. A "skip audit" mutation (attempts=1, audit=0) is CAUGHT. But a "lie about audit" mutation (`auditRecords' = SubscriptionEstablishAttempts + 1`, always claiming audit happened regardless) PASSES. So the property detects counter mismatch but not a coordinated lie. This is a genuine but narrower defect than the reviewer's sweeping claim.

**Host assessment of scope**: Round 3 escalates from "this feature's 4 properties are defective" to "9 more promoted properties across 3 model families share a pervasive trace-fidelity architecture defect." This is a materially different claim than what this feature was designed to correct. The defect class was already known and parked from the seed arc (`idea-csrf-trace-fidelity`, filed 2026-07-01, explicitly says "Applies to: authority.qnt ... and any future server-side-acceptance model"); the CSRF properties were deliberately fixed (`story-fix-csrf-trace-and-ssot-drift`) but the Elicitation/subscription/reply-correlation models were not given the same attempted-evidence treatment. A systematic re-architecture of 3 model families to introduce immutable attempted-evidence state is v1-formal-gate-scale work, not a verification-claim correction.

**Important (host-confirmed, not disputed)**:
- `authority.qnt:1-3` header comment still says "partially promoted" / "Unit SA promotes spawn-authority properties" but the file now has zero promoted properties. Fix needed.
- The feature's Implementation summary section (lines ~290-312) was not rolled through Unit 7: it still says six stories, 11 demotions, 11 removed formulas, 21/26 tiers, and "No promoted property lost its genuine-checking mutation proof." Current totals: seven stories, 15 demotions, 15 removals, 17/30. Fix needed.

**Disposition**: Filed two items for the confirmed important findings (header comment + summary roll-through). The disputed round-3 blockers (9 promoted properties) require operator judgment: whether to (a) accept the reviewer's sweeping claim and demote 9 more properties (v1-gate-scale scope expansion into model architecture), (b) treat the host's disputed mutation results as sufficient to retain the Elicitation properties and file only the confirmed `SubscriptionAudited` narrower defect, or (c) pause this feature's convergence loop and route the systematic trace-fidelity question to a dedicated design/research engagement given it spans the whole formal-model substrate.

**Notes**: Round 3 did not converge cleanly — the reviewer's findings are partially disputed by the host's independent mutation tests. This is the convergence loop's tension point: the reviewer is applying a maximally-strict independent-oracle standard that, if fully applied, would demote most of the seed model's promoted properties and defer nearly all formal verification to the v1 gate. The host verified the `SpawnRevocationDoesNotCascade` defect in round 2 (genuine) but could not reproduce the `ElicitationPendingFinality` defect in round 3 (the property caught the mutation). The two confirmed items (header comment, summary roll-through) are in-scope and will be fixed; the 9-property dispute needs an operator decision on how strict the independent-oracle bar should be for this feature's scope.

## Review round 4 (2026-07-11)

**Verdict**: Request changes (host accepts — round-3 dispute was wrong)

**Lane**: deep, adversarial convergence pass, `openai-codex/gpt-5.6-sol` (xhigh), fresh-context.

**Blockers** (2, both confirmed real by the host):
1. **9 demoted properties still have their `val`/`temporal` definitions** (Units 1-2 demoted `@promotion` status but left the formulas). `quint verify --invariant command_durability` still passes on a draft property — the exact vacuous-pass the feature exists to prevent. A Phase 7 verification miss across the original 6 waves.
2. **9 surviving promoted properties are mutation-fragile** — the host's round-3 dispute was **wrong**. The round-3 test mutated the wrong branch (already-terminal, which the `__saved_` baseline catches); the reviewer's mutation (first-terminal branch, re-terminalization from any state + lying about the baseline) passes. The host reproduced it. All 9 confirmed: 6 Elicitation, 2 subscription, `TypedCorrelation`.

**Host correction**: The round-3 scope-routing argument was correct for the *re-architecture* (attempted-evidence state is v1-gate work) but wrong for the *demotion* of properties with confirmed mutation failure — that is in-scope claim correction per the feature's brief. The operator confirmed absorbing both as Unit 8.

**Disposition**: Filed `story-verification-correction-mutation-fragility-demotion` (Unit 8, depends on Unit 7) to (a) remove the 9 leftover formulas and (b) demote the 9 mutation-fragile properties. The `idea-csrf-trace-fidelity` backlog item and `executable-release-assurance` design-input section remain accurate (they route the re-architecture, not the demotions). After Unit 8: 8 promoted / 39 stated-normative; the 8 survivors were confirmed clean (CSRF uses attempted evidence; the 3 lifecycle + GenerationMonotonic check transition/structural invariants not recorded by the accepting action).

**Notes**: Round 4 did not converge. The host's round-3 dispute is retracted — it was based on an incomplete mutation test. This is the convergence loop's hardest lesson: the host must run the *right* mutation (the one that breaks the property's actual claim), not just *a* mutation. The loop continues after Unit 8 lands.
