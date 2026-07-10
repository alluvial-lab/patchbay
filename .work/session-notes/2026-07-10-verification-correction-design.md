# Session Note — Verification Claim Correction Design

## Context

We decomposed `epic-public-product-contract` (6 child features) and are designing the first one: `epic-public-product-contract-verification-claim-correction`. The feature demotes overclaiming formal-model properties and fixes verification prose drift. It has been through **5 rounds of adversarial review** on `openai-codex/gpt-5.6-sol` (xhigh), each returning REQUEST CHANGES with progressively narrower findings. The 11 demotion decisions have been confirmed correct since round 2; later rounds found additional retained-property semantics narrowings and prose drift.

## Current State

- **Feature**: `.work/active/features/epic-public-product-contract-verification-claim-correction.md` at `stage: implementing`
- **6 child stories** at `stage: implementing` in `.work/active/stories/story-verification-correction-*.md`
- **Latest commit**: `e39590c` (addressed fifth review)
- **Working tree**: clean

## Commit Chain

```
aedfdf7 — initial design (4 stories)
d9557ed — revise after 1st review (5 stories)
b7a8b33 — address 2nd review (SpawnCreatesDescendantGrant, file allocation, checker sequence, prose drift)
8469507 — address 3rd review (retained-property semantics, model headers, checker sequence, prose drift)
ce38095 — address 4th review (ElicitationResponderAuthority, ActorIdsUnique semantics, prose coverage, section headings)
e39590c — address 5th review (ElicitationStaleTargetInert, SpawnRevocationDoesNotCascade, prose drift, Alloy note)
```

## The Design (Settled)

### 11 Demotions (promoted → draft)

All confirmed correct across all 5 reviews:

**`command_lifecycle.qnt`** (5):
- `CommandDurability` — proves map-domain persistence, not durability across failure boundary
- `PreAppendTerminalChoice` — only proves terminal transition assigns positive LSN; no competing candidates
- `LsnDeterminesTerminalWinner` — effectively `terminal → terminalLsn > 0`; no candidate comparison
- `RetryReusesIdAndKey` — only proves command→key map never changes; never observes retry input
- `RetryAfterTerminalReturnsExisting` — formula identical to `TerminalFinality`; no returned-record discriminator

**`session_generation.qnt`** (3):
- `SessionIdentityTuple` — identity components are constants, not per-session state
- `LabelsCannotOverrideIdentity` — no routing/target-selection path
- `LateGenerationInert` — claims audit records but no audit state modeled

**`elicitation_lifecycle.qnt`** (1):
- `ElicitationTimeoutNeitherSuccessNorDenial` — claims about grant state not modeled

**`patchbay-relational.als`** (1):
- `ActorIdsUnique` — fact-consequence check; assertion checks same constraint as fact

**`authority.qnt`** (1):
- `SpawnCreatesDescendantGrant` — uses invented kind names (`reboot`/`snapshot`/`stop_session`) contradicting PROTOCOL.md:181; allowed-kind set is hard-coded pure function, not mutation-survivable

### 7 Retained-Property Semantics Narrowings (Unit 6)

Formulas are genuine (mutation-survivable, independent oracle) but `@promotion` semantics text overclaims:

1. `NoAcceptedToCompleted` — says "must pass through `delivered`" but formula permits `delivered` OR `running`
2. `FleetAuthorityForSpawn` — claims "authenticated actor" but no authentication evidence
3. `SubscriptionGrantChecked` — same authentication overclaim
4. `ElicitationResponderAuthority` — claims endpoint authentication but model only checks endpoint-to-actor mapping
5. `browser_local_state_not_authority` — claims grant-check protection but no grant state
6. `ElicitationStaleTargetInert` — says "do not mutate live state" but formula only excludes `answered`
7. `SpawnRevocationDoesNotCascade` — when `gDescOs3Live != "yes"`, descendant condition becomes `true`

### 10 Misleading Draft Formula Removals (Unit 4)

Remove `val` definitions entirely (not `true`) so `quint verify --invariant <name>` fails because the invariant doesn't exist:

**`snapshot_recovery.qnt`** (6): `SnapshotStaleRejected`, `SnapshotCrossDomainRejected`, `SnapshotConsistentPrefix`, `LateEventNoRewrite`, `CrashNoAcceptedLost`, `IdempotentLogReplay`

**`authority.qnt`** (4): `NoCommandWithoutGrant`, `CompoundIssuer`, `GrantAuthorityIsCommandKinds`, `RevocationPreventsFuture`

### Toy Relocation (Unit 3)

- `Counter.qnt`, `Counter.tla`, `Counter.cfg` → skill example directories
- `patchbay-invariants.als` → `.agents/skills/alloy/examples/`
- Update skill references in `.agents/skills/{alloy,quint,tla-plus}/SKILL.md`

### Prose Drift (Unit 5)

Fix stale checked-model claims in: PROTOCOL.md (lines 75, 94, 140, 142, 270, 380, 568, 603), VERIFICATION.md (lines 43, 93, 127, 139, 213, 298, 566), ADAPTER-PI.md (lines 75-79, 147), SPEC.md (line 70), GLOSSARY.md (lines 63, 71)

### Final Counts

32 promoted / 12 draft → **21 promoted / 23 draft** (11 demoted)

## 6 Stories (dependency chain)

1. `story-verification-correction-command-lifecycle` — `depends_on: []`
2. `story-verification-correction-session-elicitation` — `depends_on: [1]`
3. `story-verification-correction-alloy-and-toys` — `depends_on: [2]`
4. `story-verification-correction-draft-formulas` — `depends_on: [3]`
5. `story-verification-correction-prose` — `depends_on: [4]`
6. `story-verification-correction-retained-semantics` — `depends_on: [5]`

## Critical Implementation Details

### Checker Sequence

`check-vectors.mjs` exits 0 on regeneration (only exits 1 for validation failures). `check-models.mjs` exits 1 when its generated table changes, then exits 0 on second run. Correct sequence:
```
node contracts/scripts/check-vectors.mjs   # exits 0, regenerates conformance table
node contracts/scripts/check-models.mjs   # exits 1, regenerates model table
node contracts/scripts/check-models.mjs   # exits 0, confirms current
```

### ActorIdsUnique Approach

Demote `@promotion` block to draft. Keep the `check ActorIdsUniqueAssert for 5` command as a non-promoted structural regression test with comment. Rewrite semantics to not claim non-vacuity. Fix the adjacent NOTE at lines 50-52.

### Mixed-Status Section Headings

Rename section headings at `command_lifecycle.qnt:165`, `session_generation.qnt:108`, `elicitation_lifecycle.qnt:538` so they no longer label mixed promoted/draft blocks as all promoted.

## Review History

| Round | Verdict | Key Findings |
|-------|---------|-------------|
| 1 | REQUEST CHANGES | 2 more retained overclaims; re-inventory incomplete; `true` stubs pass vacuously; wrong counts; PROTOCOL.md:140 drift; `patchbay-invariants.als` toy; stories not sequenced; checker order wrong; toy references in skills |
| 2 | REQUEST CHANGES | `SpawnCreatesDescendantGrant` overclaims; `LateGenerationInert` in wrong file; checker needs two passes; additional prose drift |
| 3 | REQUEST CHANGES | 4 retained properties need semantics narrowing; additional prose drift; stale model header comments |
| 4 | REQUEST CHANGES | `ElicitationResponderAuthority` overclaims; `ActorIdsUnique` semantics says non-vacuity; VERIFICATION.md:566; mixed-status headings; feature says "five" not "six" |
| 5 | REQUEST CHANGES | `ElicitationStaleTargetInert` and `SpawnRevocationDoesNotCascade` overclaim; more prose drift (VERIFICATION.md:139/298/568, SPEC.md:70, GLOSSARY.md:63/71); Alloy note incomplete |

## Next Steps

The design is ready for implementation. The 11 demotions are settled. The 7 retained-property narrowings are settled. The prose drift inventory is comprehensive. Options:

1. **Proceed to implementation** — run `/agile-workflow:implement story-verification-correction-command-lifecycle` (sequential). The `[verification]` deep-review lane during story review will catch any remaining issues.
2. **One more review round** — if desired, but diminishing returns. Each round finds 2-3 more narrowings in the 21 retained properties.

## Broader Epic Context

`epic-public-product-contract` has 6 child features:
- `...-public-compatibility` — `depends_on: []` (ready for design)
- `...-self-hosted-operations` — `depends_on: [public-compatibility]`
- `...-adapter-portability-proof` — `depends_on: [public-compatibility]`
- `...-verification-claim-correction` — `depends_on: []` (IN PROGRESS — designing)
- `...-executable-release-assurance` — `depends_on: [all 4 above]`
- `...-publication-governance` — `depends_on: []` (ready for design)

Key decisions locked in the epic:
- v0.1.0 is personal/internal, not public distribution; legal review does NOT block it
- AGPL-3.0-or-later (application) / Apache-2.0 (interoperability) licensing, subject to legal review
- `Patchbay` is a provisional name; rename before public release
- No outside contributions until contributor terms settled (deferred decision — option C)
- Full v1-readiness program (not just contract definition)
