---
id: feature-formal-model-seed
kind: feature
stage: done
tags: [verification, protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-command-state-ssot, feature-verification-contract-authority, feature-research-formal-methods-tooling]
created: 2026-06-28
updated: 2026-07-02
gate_origin: null
release_binding: null
---

# Feature: Author seed formal models

Patchbay's verification posture requires checked models before implementation treats coordination semantics as product behavior. This feature creates the first normative model artifacts after the prose state machines and verification authority order are defined.

## Scope

- Author the first TLA+/Quint model for operator-intent delivery.
- Model accepted command durability, visible terminal/continuing states, timeout semantics, and retry/deduplication at the Patchbay boundary.
- Author initial Alloy relational invariants for identity uniqueness, authority graph constraints, and any lease properties that remain in v0 scope.
- Record model-promotion metadata: property checked, finite bounds/constants, tool invocation, expected pass/fail status, and product-semantics note.
- Document how model variables trace to protocol state-machine terms and future contract fields.

## Acceptance criteria

- `specs/` contains the seed TLA+/Quint model and Alloy model, or docs explicitly record why one of the two is deferred from v0.
- `docs/VERIFICATION.md` references the seed models and their promotion status.
- The models check the v0 command/session semantics defined by `feature-command-state-ssot` rather than inventing new terminology.
- A future implementation item can derive property/conformance-test obligations from the model artifacts.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.

## Design decisions (interactive, 2026-07-01)

- **Q1 — Seed coverage ambition: focused cluster checked-to-pass + vocabulary-for-all (choice B).** Check-to-pass only the checked-normative properties whose semantics are fully pinned by `done` dependencies: operator intent delivery (TerminalFinality, PreAppendTerminalChoice, LsnDeterminesTerminalWinner, accepted-command durability) — pinned by `feature-command-state-ssot` + `feature-design-terminal-commit-race`; wrong-session prevention (session identity tuple, GenerationMonotonic, LateGenerationInert) — pinned by `feature-session-identity-adapter-contract`; idempotency at the Patchbay boundary (no-double-apply, retry reuses id+key, retry-after-terminal returns existing record) — boundary semantics pinned, `maybe_executed` adapter ambiguity excluded (blocked on the still-drafting `feature-idempotency-ambiguous-execution`). Plus TypedCorrelation (pinned by PROTOCOL reply-correlation section) and the CSRF spine (pinned by SECURITY.md §CSRF) pulled into checked-to-pass as small models (choice B-wide, Q2). The rest of the checked-normative set — snapshot core safety, crash recovery, authority-safety (CompoundIssuer etc.), audit integrity, adapter-failure visibility — are carried as **draft models that compile, each with a reserved property-id**, with a one-line deferral reason. The full property-id vocabulary for the entire checked-normative set is established here, even for not-yet-promoted properties, so `feature-protocol-idl-and-conformance` has the complete set of ids to wire conformance vectors to. Leases are out of scope (VERIFICATION makes lease safety a precondition, not a v0 baseline; `feature-lease-scope-decision` is still drafting with no decision) — deferred with a documented reason, satisfying the feature's own acceptance criterion.
- **Q2 — Decomposition shape: clustered by shared state + Alloy (choice B-wide).** One Quint model per cohesive state machine, with cross-cluster references modeled as minimal *projected variables* (a terminal flag, a tombstone LSN, a committed-prefix abstraction) rather than imports, so each model stays independently checkable. Idempotency is folded into `command_lifecycle.qnt` (not separate) because retry-after-terminal genuinely needs to observe terminal state — the tightest coupling in the set; splitting it would force an artificial projection of the very thing the property is about. Everything else projects cleanly. The concrete set: `command_lifecycle.qnt`, `session_generation.qnt`, `reply_correlation.qnt`, `csrf_browser.qnt` (checked); `snapshot_recovery.qnt`, `authority.qnt` (draft); `patchbay-relational.als` (checked, Alloy).
- **Q3 — Liveness/temporal in the seed: mixed backends per property shape (choice C).** TLC (`quint verify --backend tlc`) for the two-state properties — TerminalFinality, PreAppendTerminalChoice, LsnDeterminesTerminalWinner, retry-after-terminal, GenerationMonotonic, LateGenerationInert — because they are inherently temporal (`always(...)` next-state properties) and cannot be honestly encoded as non-temporal invariants without falling into the self-defining trap (encoding the property *into* the transition guard, then checking a tautology). Apalache (`quint verify`) for the one-state invariants — no-double-apply, TypedCorrelation, CSRF spine, identity-tuple. Alloy CLI for the relational shapes. Honest clarification: all checked properties here are `always(...)` safety properties, not `eventually` liveness; the routing to TLC is driven by two-state shape (the skill conservatively routes temporal→TLC), not by liveness. Genuine checking preserved (permissive transitions + property checked against them, not self-defining).
- **Q4 — Authoring language / TLA+ baseline: Quint-primary; emitted TLA+ is a generated inspection artifact (choice A).** Author in Quint; check via `quint verify` (Apalache) and `quint verify --backend tlc` (TLC). Commit a `quint compile --target tlaplus` emission as a checked-in, never-hand-edited artifact alongside each checked `.qnt` source. The emitted `.tla` is a durable, human-inspectable, version-pinned baseline form — but it is NOT an independent verification lane: standalone `tla2tools-1.7.4.jar` cannot parse emitted TLA+ alone (`EXTENDS ... Apalache, Variants` requires the Apalache jar), and even with the Apalache jar on the classpath it is the same toolchain (Apalache+TLC) reached via Quint. So there is one verification lane, not two; the emitted `.tla` is an inspection artifact, not a second-check path. This avoids the drift surface of a hand-maintained parallel TLA+ copy (choice B) and respects the research's confirmed Q1 verdict (Quint-primary-checked-via-TLC viable, so no parallel-TLA+ maintenance burden is needed).
- **Q5 — Promotion-metadata home: structured `@promotion` comment block in each model file, inline next to its property (choice 1).** A fenced structured comment block per checked property, placed inline next to the `temporal`/`val`/`assert` it describes, not a 60-line preamble. A CI script greps the fenced `@promotion` blocks and generates the traceability table in `docs/VERIFICATION.md` as a checked-in artifact (extending the mechanism the authority feature already committed for conformance vectors). This is the only option where the metadata physically cannot separate from the model (it's in the same file) — drift is structurally impossible; the only failure is omission, caught by the CI coverage check ("fails if a checked-normative property lacks a promoted model"). Rejected: YAML frontmatter at the top of `.qnt`/`.als` files (infeasible — tool parsers own line 1: Quint expects `module`, Alloy expects `sig`/`module`, TLA+ expects `---- MODULE`); sidecar `.yaml` per model (native YAML but a drift surface); central `promotion.yaml` (drift-prone, rejected on symmetry with the authority feature's rejection of a central vector registry). The `status` field reuses the vector vocabulary (`draft` | `promoted`), so a property is "checked-normative" exactly when its model block is `status: promoted` AND a promoted vector traces to it — composing the model-promotion rule with the vector-promotion rule and making "checked-normative" a derivable, CI-verifiable property.

## Architectural choice

Quint-primary authoring with mixed backends (TLC for two-state temporal, Apalache for one-state invariants, Alloy CLI for relational), decomposed by shared-state clusters with projection seams, checked-to-pass for a focused cluster of pinned properties (command delivery + wrong-session + idempotency-boundary + reply-correlation + CSRF spine), draft models carrying reserved property-ids for the rest, emitted TLA+ committed as generated inspection artifacts, and structured `@promotion` comment blocks in each model file as the machine-readable source for a generated `docs/VERIFICATION.md` traceability table.

This composes with the authority order (`feature-verification-contract-authority`): the property-id vocabulary established here is the Single Source of Truth that models, `.proto`, conformance vectors, and implementation all derive from; the `@promotion` blocks are the model-layer analog of the vector frontmatter the authority feature committed; the generated traceability table extends the one the authority feature already specified.

## Property-id vocabulary (established here as SSOT)

All checked-normative property-ids are named here, even those whose models are draft. Downstream work (`feature-protocol-idl-and-conformance`) wires conformance vectors to these ids; a property is "checked-normative" iff its model block is `status: promoted` AND ≥1 promoted vector traces to it.

| Property-id | Area | Tier (this feature) | Model | Backend |
|---|---|---|---|---|
| `CommandDurability` | operator intent delivery | checked | `command_lifecycle.qnt` | apalache |
| `TerminalFinality` | operator intent delivery | checked | `command_lifecycle.qnt` | apalache-temporal |
| `PreAppendTerminalChoice` | operator intent delivery | checked | `command_lifecycle.qnt` | apalache-temporal |
| `LsnDeterminesTerminalWinner` | operator intent delivery | checked | `command_lifecycle.qnt` | apalache-temporal |
| `TimeoutNeitherSuccessNorDenial` | operator intent delivery | stated (draft) | `<transport model (future)>` | apalache |
| `BoundaryDedup` | idempotent retry | checked | `command_lifecycle.qnt` | apalache |
| `RetryReusesIdAndKey` | idempotent retry | checked | `command_lifecycle.qnt` | apalache-temporal |
| `RetryAfterTerminalReturnsExisting` | idempotent retry | checked | `command_lifecycle.qnt` | apalache-temporal |
| `SessionIdentityTuple` | wrong-session prevention | checked | `session_generation.qnt` | apalache |
| `GenerationMonotonic` | wrong-session prevention | checked | `session_generation.qnt` | apalache-temporal |
| `LateGenerationInert` | wrong-session prevention | checked | `session_generation.qnt` | apalache-temporal |
| `LabelsCannotOverrideIdentity` | wrong-session prevention | checked | `session_generation.qnt` | apalache |
| `TypedCorrelation` | reply correlation | checked | `reply_correlation.qnt` | apalache |
| `CsrfRejectsUnauthenticated` | browser session/CSRF | checked | `csrf_browser.qnt` | apalache |
| `CsrfRejectsMissingProof` | browser session/CSRF | checked | `csrf_browser.qnt` | apalache |
| `RevokedSessionCannotCommand` | browser session/CSRF | checked | `csrf_browser.qnt` | apalache |
| `ActorIdsUnique` | relational (identity) | checked | `patchbay-relational.als` | alloy-cli |
| `AuthorityGraphAcyclic` | relational (authority) | stated (draft) | `patchbay-relational.als` | alloy-cli |
| `SenderMatchesClaim` | relational (anti-spoofing) | stated (draft) | `patchbay-relational.als` | alloy-cli |
| `SnapshotStaleRejected` | snapshot convergence (core safety) | stated (draft) | `snapshot_recovery.qnt` | tlc |
| `SnapshotCrossDomainRejected` | snapshot convergence (core safety) | stated (draft) | `snapshot_recovery.qnt` | tlc |
| `SnapshotConsistentPrefix` | snapshot convergence (core safety) | stated (draft) | `snapshot_recovery.qnt` | tlc |
| `LateEventNoRewrite` | snapshot convergence (core safety) | stated (draft) | `snapshot_recovery.qnt` | tlc |
| `CrashNoAcceptedLost` | crash recovery | stated (draft) | `snapshot_recovery.qnt` | tlc |
| `IdempotentLogReplay` | crash recovery | stated (draft) | `snapshot_recovery.qnt` | tlc |
| `NoCommandWithoutGrant` | authority safety | stated (draft) | `authority.qnt` | apalache |
| `CompoundIssuer` | authority safety | stated (draft) | `authority.qnt` | apalache |
| `GrantAuthorityIsCommandKinds` | authority safety | stated (draft) | `authority.qnt` | apalache |
| `RevocationPreventsFuture` | authority safety | stated (draft) | `authority.qnt` | apalache-temporal |

Note: `TimeoutNeitherSuccessNorDenial` is listed as stated (draft) rather than checked because it concerns the *transport/submission* layer ("timeout implies neither success nor denial" — a failure-vocabulary property), not the command-lifecycle state machine. It belongs in a future transport/failure-vocabulary model, not `command_lifecycle.qnt`. Discovered during Unit 1 implementation when the story's 7-property scope did not include it; the vocabulary table retains the reserved property-id for the downstream model.

## Implementation Units

All model source lives under `specs/seed/`. Existing hello-world artifacts (`Counter.qnt`, `Counter.tla`, `Counter.cfg`, `patchbay-invariants.als`) are retained as toolchain-validation references; the Patchbay models are new files. The `_apalache-out/` directory is gitignored tool output.

### `@promotion` block format (applies to every unit below)

```text
// @promotion {
//   property:    <property-id>
//   tier:        checked-normative | stated-normative
//   status:      draft | promoted
//   model:       specs/seed/<file>
//   language:    quint | tla | alloy
//   backend:     apalache | tlc | alloy-cli
//   invocation:  <exact CLI incl. jar-path classpath>
//   bounds:      { <finite bounds/constants> }
//   expected:    pass | fail
//   proto_fields: [ <proto field/path> ... | none ]   # reserved for feature-protocol-idl-and-conformance
//   semantics:   <one-line connection to product behavior>
// }
```

Each block sits inline immediately above the `temporal` / `val` / `assert` it describes. A model with N checked properties has N blocks. The `proto_fields` field is reserved (populated by the downstream IDL feature); the seed sets it to `none` and the generator treats `none` as "not yet wired."

---

### Unit 1: `command_lifecycle.qnt` (trickiest unit — designed first)

**File**: `specs/seed/command_lifecycle.qnt`
**Story**: spawned as `story-formal-model-command-lifecycle` (the terminal-race nondeterminism + dedup map fusion makes this the highest-unknown unit)

Models accepted-command durability, the terminal-race (first-durable-terminal-commit-wins), and idempotency-boundary dedup in one fused state machine. Folds idempotency in because `RetryAfterTerminalReturnsExisting` must observe terminal state.

**State variables** (trace to PROTOCOL: `CommandState`, `CommandId`, idempotency key, `LSN`):

```quint
module command_lifecycle {
  // CommandState registry from docs/PROTOCOL.md (terminal set derived from the table)
  pure val TERMINAL = Set("completed", "rejected", "failed", "expired", "cancelled", "superseded")
  pure val NON_TERMINAL = Set("accepted", "delivered", "running")
  pure val ALL_STATES = TERMINAL.union(NON_TERMINAL)

  // finite command-id space (bound: 3 command ids)
  pure val CMD_IDS = Set("c1", "c2", "c3")
  // finite idempotency-key space (bound: 3 keys)
  pure val IDEMPOTENCY_KEYS = Set("k1", "k2", "k3")

  var state: str -> str            // CommandId -> CommandState (map; str -> str)
  var idemKey: str -> str          // CommandId -> idempotency key (the dedup handle)
  var appliedKeys: Set[str]        // idempotency keys already applied at the boundary
  var lsn: int                     // monotonic gap-free log sequence number (the durable order)
  var terminalLsn: str -> int     // CommandId -> LSN at which it went terminal (0 = not yet terminal)
}
```

**Actions** — permissive transitions (the model *allows* terminal races and late events; the properties are checked *against* these, not baked into guards):

```quint
  action init = all {
    state' = CMD_IDS.mapBy(c => "accepted"),
    idemKey' = Set("c1" -> "k1", "c2" -> "k2", "c3" -> "k3"),
    appliedKeys' = Set("k1", "k2", "k3"),
    lsn' = 3,                       // three accepts already committed at LSN 1,2,3
    terminalLsn' = CMD_IDS.mapBy(c => 0),
  }

  // a non-terminal command races toward any terminal candidate; LSN assigned on commit
  action commitTerminal(cmd, candidate) = all {
    state.get(cmd).in(NON_TERMINAL),
    candidate.in(TERMINAL),
    state' = state.set(cmd, candidate),
    lsn' = lsn + 1,
    terminalLsn' = terminalLsn.set(cmd, lsn + 1),
    idemKey' = idemKey,
    appliedKeys' = appliedKeys,
  }

  // a late conflicting terminal candidate after one is already terminal: audit only, no rewrite
  action lateTerminalCandidate(cmd, candidate) = all {
    state.get(cmd).in(TERMINAL),
    state' = state,                 // NO mutation — this is the TerminalFinality guarantee
    lsn' = lsn,
    terminalLsn' = terminalLsn,
    idemKey' = idemKey,
    appliedKeys' = appliedKeys,
  }

  // retry with same command id + idempotency key: returns existing record, no double-apply
  action retry(cmd, key) = all {
    idemKey.get(cmd) == key,        // same key
    key.in(appliedKeys),           // already applied at boundary
    state' = state,                // existing record returned, no new transition
    appliedKeys' = appliedKeys,
    lsn' = lsn,
    terminalLsn' = terminalLsn,
    idemKey' = idemKey,
  }

  action step = any {
    nondet cmd = CMD_IDS.oneOf(); nondet cand = TERMINAL.oneOf(); commitTerminal(cmd, cand),
    nondet cmd = CMD_IDS.oneOf(); nondet cand = TERMINAL.oneOf(); lateTerminalCandidate(cmd, cand),
    nondet cmd = CMD_IDS.oneOf(); nondet key = IDEMPOTENCY_KEYS.oneOf(); retry(cmd, key),
  }
```

**Checked properties** (each with its inline `@promotion` block):

```quint
  // @promotion { property: CommandDurability, tier: checked-normative, status: promoted,
  //   model: specs/seed/command_lifecycle.qnt, language: quint, backend: tlc,
  //   invocation: quint verify command_lifecycle.qnt --backend tlc --invariant command_durability --max-steps 12,
  //   bounds: { cmd_ids: 3, idempotency_keys: 3, max_steps: 12 },
  //   expected: pass, proto_fields: [none],
  //   semantics: an accepted command is durably recorded before delivery and cannot vanish silently }
  val command_durability = all { c in CMD_IDS => state.contains(c) }   // every command id has a state

  // @promotion { property: TerminalFinality, tier: checked-normative, status: promoted,
  //   model: specs/seed/command_lifecycle.qnt, language: quint, backend: tlc,
  //   invocation: quint verify command_lifecycle.qnt --backend tlc --temporal terminal_finality,
  //   bounds: { cmd_ids: 3, idempotency_keys: 3 },
  //   expected: pass, proto_fields: [none],
  //   semantics: once a command reaches a terminal CommandState, later events do not mutate it }
  temporal terminal_finality =
    always(all { cmd in CMD_IDS =>
      state.get(cmd).in(TERMINAL) implies (next(state.get(cmd)) == state.get(cmd)) })

  // @promotion { property: PreAppendTerminalChoice, tier: checked-normative, status: promoted, ... }
  temporal pre_append_terminal_choice =
    always(all { cmd in CMD_IDS, cand in TERMINAL =>
      // before an LSN is assigned (terminalLsn=0), the winner may be chosen nondeterministically;
      // after assignment, the LSN order is stable
      (terminalLsn.get(cmd) == 0 and next(state.get(cmd)).in(TERMINAL)) implies next(terminalLsn.get(cmd)) > 0 })

  // @promotion { property: LsnDeterminesTerminalWinner, tier: checked-normative, status: promoted, ... }
  temporal lsn_determines_terminal_winner =
    always(all { cmd in CMD_IDS =>
      // two terminal candidates cannot both commit; the lowest committed LSN wins
      (state.get(cmd).in(TERMINAL)) implies terminalLsn.get(cmd) > 0 })

  // @promotion { property: BoundaryDedup, tier: checked-normative, status: promoted, backend: apalache, ... }
  val boundary_dedup = appliedKeys.size() <= IDEMPOTENCY_KEYS.size()   // no key double-applied

  // @promotion { property: RetryReusesIdAndKey, tier: checked-normative, status: promoted, backend: apalache, ... }
  val retry_reuses_id_and_key = all { cmd in CMD_IDS, key in IDEMPOTENCY_KEYS =>
    (idemKey.get(cmd) == key and key.in(appliedKeys)) }

  // @promotion { property: RetryAfterTerminalReturnsExisting, tier: checked-normative, status: promoted, backend: tlc, ... }
  temporal retry_after_terminal_returns_existing =
    always(all { cmd in CMD_IDS =>
      state.get(cmd).in(TERMINAL) implies next(state.get(cmd)) == state.get(cmd))
}
```

**Implementation Notes**:
- Bounds: 3 command ids × 3 idempotency keys, `--max-steps 12`. Small enough for TLC exhaustive search; large enough to exercise the race (≥2 terminal candidates competing for one command).
- The `lateTerminalCandidate` action is deliberately a no-op on state — that's the *permissive* modeling (it's *allowed* to happen); `terminal_finality` then proves the no-op holds. This is the genuine-checking discipline: the property is checked *against* permissive transitions, not baked into a guard.
- `lsn` is monotonic and gap-free (always `+1` on commit) — models the durable log's total order.
- Tracing: `state` ↔ `CommandState`; `lsn` ↔ `LSN`; `idemKey`/`appliedKeys` ↔ command-id/idempotency-key separation (PROTOCOL "Messages, commands, and replies").
- **Implementation target, not verified claim**: this composed model has not been parse-checked yet. The skill idioms parse individually, but a fused model with maps + multiple `temporal` blocks + nondeterministic `step` must be validated in the implementation stride (see Risks).

**Acceptance Criteria**:
- [ ] `quint parse command_lifecycle.qnt` exits 0 (parse + the typed-action-parameter pitfall).
- [ ] `quint compile command_lifecycle.qnt` exits 0 (typecheck).
- [ ] `quint verify --backend tlc --temporal terminal_finality` exits 0 (no counterexample).
- [ ] `quint verify --invariant boundary_dedup --max-steps 12` exits 0 (Apalache, no double-apply).
- [ ] `command_lifecycle.emitted.tla` committed (generated, never hand-edited).
- [ ] All 7 `@promotion` blocks present and grep-able.

---

### Unit 2: `session_generation.qnt`

**File**: `specs/seed/session_generation.qnt`

Models session identity tuple + generation supersession + tombstoning. Projects an `lsn` from the command log (for tombstone-commit ordering) without importing the command model.

**State variables**:

```quint
module session_generation {
  pure val SESSION_IDS = Set("s1", "s2")          // bound: 2 sessions
  pure val ADAPTER_IDS = Set("a1")                 // bound: 1 adapter
  pure val DEPLOY_SCOPES = Set("d1")               // bound: 1 deployment scope
  pure val RUNTIME_IDS = Set("r1")                // bound: 1 runtime session id
  pure val LABELS = Set("proj-A", "proj-B")       // metadata, not identity

  var generation: str -> int        // SessionId -> live generation (monotonic)
  var tombstoned: Set[str]          // tombstoned generation keys ("s1:gen")
  var tombstoneLsn: str -> int      // tombstone key -> LSN at which superseded
  var lsn: int                       // projected from command log (monotonic; not the command model's LSN)
  var label: str -> str             // SessionId -> human-readable label (metadata; cannot override identity)
}
```

**Actions** (permissive — lower/equal reports are *allowed* but become no-ops or audit; the properties prove the no-op):

```quint
  action init = all {
    generation' = SESSION_IDS.mapBy(s => 0),
    tombstoned' = Set(),
    tombstoneLsn' = Set(),
    lsn' = 0,
    label' = SESSION_IDS.mapBy(s => "proj-A"),
  }

  // strictly-greater generation: supersede (tombstone the old, advance the new)
  action supersede(sid, newGen) = all {
    newGen > generation.get(sid),
    tombstoned' = tombstoned.union(Set(sid ++ ":" ++ intToString(generation.get(sid)))),
    lsn' = lsn + 1,
    tombstoneLsn' = tombstoneLsn.set(sid ++ ":" ++ intToString(generation.get(sid)), lsn + 1),
    generation' = generation.set(sid, newGen),
    label' = label,
  }

  // equal generation: no-op (capability redeclaration may proceed, generation unchanged)
  action equalReport(sid, gen) = all {
    gen == generation.get(sid),
    generation' = generation,   tombstoned' = tombstoned,   tombstoneLsn' = tombstoneLsn,
    lsn' = lsn,   label' = label,
  }

  // lower generation: rejected as audit, live generation unchanged
  action lowerReport(sid, gen) = all {
    gen < generation.get(sid),
    generation' = generation,   tombstoned' = tombstoned,   tombstoneLsn' = tombstoneLsn,
    lsn' = lsn,   label' = label,
  }

  // late reply binding to a tombstoned generation: stale_event audit, no mutation
  action lateReplyToTombstoned(sid, gen) = all {
    tombstoned.contains(sid ++ ":" ++ intToString(gen)),
    generation' = generation,   tombstoned' = tombstoned,   tombstoneLsn' = tombstoneLsn,
    lsn' = lsn,   label' = label,
  }

  action step = any {
    nondet sid = SESSION_IDS.oneOf(); nondet g = 1.to(3).oneOf(); supersede(sid, g),
    nondet sid = SESSION_IDS.oneOf(); equalReport(sid, generation.get(sid)),
    nondet sid = SESSION_IDS.oneOf(); nondet g = 0.to(0).oneOf(); lowerReport(sid, g),
    nondet sid = SESSION_IDS.oneOf(); nondet g = 0.to(0).oneOf(); lateReplyToTombstoned(sid, g),
  }
```

**Checked properties**: `GenerationMonotonic` (temporal, TLC), `LateGenerationInert` (temporal, TLC), `SessionIdentityTuple` (invariant, Apalache), `LabelsCannotOverrideIdentity` (invariant, Apalache — changing `label` doesn't change `generation`). Each with inline `@promotion` block (elided for brevity; same shape as Unit 1).

**Acceptance Criteria**:
- [ ] `quint parse` + `quint compile` exit 0.
- [ ] `quint verify --backend tlc --temporal generation_monotonic` exits 0.
- [ ] `session_generation.emitted.tla` committed.

---

### Unit 3: `reply_correlation.qnt`

**File**: `specs/seed/reply_correlation.qnt`

Models the four separate id spaces (command id, message id, reply id, event id) and typed correlation. Small state space; one-state invariant.

**State variables**: `commandIds`, `messageIds`, `replyIds` (Sets[str]); `replyCorrelatesTo: str -> str` (ReplyId -> correlated command/message id); `replyCorrelationType: str -> str` (ReplyId -> "command" | "message").

**Checked property**: `TypedCorrelation` (Apalache invariant) — a reply's correlation ref resolves to a known prior id *of the correct type* in the same context; cannot forge correlation across id spaces. Inline `@promotion` block.

**Acceptance Criteria**: parse + compile exit 0; `quint verify --invariant typed_correlation` exits 0; emitted TLA+ committed.

---

### Unit 4: `csrf_browser.qnt`

**File**: `specs/seed/csrf_browser.qnt`

Models the server-side effects of browser session/CSRF evidence (per VERIFICATION: "formal models do not prove browser cookie mechanics... they model the server-side effects"). Grounded in SECURITY.md §CSRF ("Every state-changing web route must require an authenticated operator session cookie [and] a CSRF token tied to that operator session").

**State variables**: `operatorSessions: Set[str]`; `csrfProofs: str -> str` (session -> proof); `sessionStatus: str -> str` (active | revoked | expired).

**Actions**: `submitStateChangingRequest(session, proof)` — permissive (accepts any session/proof); the invariant rejects when session missing/revoked/expired or proof invalid.

**Checked properties**: `CsrfRejectsUnauthenticated`, `CsrfRejectsMissingProof`, `RevokedSessionCannotCommand` (all Apalache invariants). Inline `@promotion` blocks.

**Acceptance Criteria**: parse + compile exit 0; `quint verify --invariant <each>` exits 0; emitted TLA+ committed.

---

### Unit 5: `snapshot_recovery.qnt` (draft — stated-normative)

**File**: `specs/seed/snapshot_recovery.qnt`

Draft model: compiles and checks, but `status: draft` (not promoted). Carries reserved property-ids: `SnapshotStaleRejected`, `SnapshotCrossDomainRejected`, `SnapshotConsistentPrefix`, `LateEventNoRewrite`, `CrashNoAcceptedLost`, `IdempotentLogReplay`.

**State variables** (per VERIFICATION's normative-variable list): `LSN`, `Cursor`, `SnapshotRevision`, `AuthorityDomain`, `CoreGeneration`, `CommittedPrefixLSN`, `CheckpointSnapshotLSN`, `RecoveredCommandState`, `RecoveryPhase`.

**Deferral reason**: the snapshot/recovery state machine is large (8+ model variables, recovery phases, crash/restart triggers) and warrants its own implementation item rather than inflating the seed stride. The property-ids are reserved here so downstream work can promote them one at a time.

**Acceptance Criteria**: `quint parse` + `quint compile` exit 0 (compiles); `@promotion` blocks present with `status: draft`; deferral reason recorded.

---

### Unit 6: `authority.qnt` (draft — stated-normative)

**File**: `specs/seed/authority.qnt`

Draft model: compiles, `status: draft`. Carries reserved property-ids: `NoCommandWithoutGrant`, `CompoundIssuer`, `GrantAuthorityIsCommandKinds`, `RevocationPreventsFuture`.

**State variables** (per VERIFICATION's authority list): `Actor`, `Device`, `Endpoint`, `OperatorSession`, `Grant`, `GrantScope`, `CommandKind`, `Target`, `TargetGeneration`, `RevocationGeneration`, `CommandIssuer`, `AuthorityDomain`. Projects `SessionGeneration` and acceptance-gate from the session/command models.

**Deferral reason**: `CompoundIssuer` involves the transport-endpoint-vs-operator-actor verification interaction, which is the most complex authority property; warrants its own item. The `Anti-spoofing caveat` from the Alloy brief (the *binding* of authenticated identity to transport/session is a dynamic property, not a relational shape) means part of authority-safety lives in this dynamic model, not the Alloy model.

**Acceptance Criteria**: `quint parse` + `quint compile` exit 0; `@promotion` blocks present with `status: draft`; deferral reason recorded.

---

### Unit 7: `patchbay-relational.als` (checked — Alloy, relational-only)

**File**: `specs/seed/patchbay-relational.als`

Replaces the current hello-world `patchbay-invariants.als` (which only models `ActorIdsUnique`) with the three Patchbay v0 relational shapes. Relational-only (no temporal operators, no NuSMV — per the Alloy skill's v0 scope).

```alloy
sig Identity {}
sig Actor { id: one Identity }

fact ActorIdsUnique { id in Actor lone -> Identity }
assert ActorIdsUniqueAssert { all disj a, b: Actor | a.id != b.id }

// Authority-graph acyclicity (trivially true in v0 since delegation is removed, but the shape is reserved)
sig Grant { issuer: one Actor, subject: one Actor }
fact NoGrantCycles { no g: Grant | g.subject = g.issuer or g.subject in g.issuer.^issuer }
// (delegation removed → acyclicity is structural; the check guards against a future re-introduction)
assert AuthorityGraphAcyclicAssert { no g: Grant | g.subject in g.issuer.^issuer }

// Anti-spoofing (consistency shape only — the binding is dynamic, modeled in authority.qnt)
sig Message { sender: one Actor, claimedSender: one Actor }
fact SenderMatchesClaim { all m: Message | m.sender = m.claimedSender }
assert SenderMatchesClaimAssert { all m: Message | m.sender = m.claimedSender }

check ActorIdsUniqueAssert for 5
check AuthorityGraphAcyclicAssert for 5
check SenderMatchesClaimAssert for 5
```

**Implementation Notes**:
- `lone ->` forces injectivity (no two actors share an identity).
- `^issuer` is transitive closure (grant-chain reachability).
- **Anti-spoofing caveat** (load-bearing, from the Alloy brief): this models the *consistency shape* (sender ≠ self-asserted). The *binding* of an authenticated identity to a transport/session is a dynamic verification action (`CompoundIssuer`-style) — that belongs in `authority.qnt`, not here. The `@promotion` block for `SenderMatchesClaim` records this boundary.
- Scope `for 5` (5 atoms per top-level sig) — sufficient for small-counterexample relational checks.

**Acceptance Criteria**:
- [ ] `java -jar org.alloytools.alloy.dist.jar exec --command ActorIdsUniqueAssert --type json --output - patchbay-relational.als` → `UNSAT` (no counterexample).
- [ ] Same for `AuthorityGraphAcyclicAssert` and `SenderMatchesClaimAssert`.
- [ ] 3 `@promotion` blocks present.
- [ ] Old `patchbay-invariants.als` retained (not deleted — preserve-only per substrate discipline) but superseded by a note pointing to `patchbay-relational.als`.

---

## Implementation Order

1. **`command_lifecycle.qnt`** (story, trickiest unit — validates the fused terminal-race + dedup design and the TLC temporal path on the hardest model first).
2. **`session_generation.qnt`** (parallelizable with nothing below it; exercises the second temporal model).
3. **`reply_correlation.qnt`** + **`csrf_browser.qnt`** (parallelizable — small Apalache-invariant models, independent of each other).
4. **`patchbay-relational.als`** (independent — Alloy, no Quint dependency).
5. **`snapshot_recovery.qnt`** + **`authority.qnt`** (draft — compile-only; can run in parallel with the checked units since they don't block).
6. **Emit + commit `.emitted.tla`** for each checked Quint model; record the Q4-honest standalone-check note.
7. **Update `docs/VERIFICATION.md`** — reference the seed models and their promotion status (the feature's acceptance criterion).

## When stories vs. single-feature

Spawn **one child story**: `story-formal-model-command-lifecycle` (Unit 1). Rationale: it's the trickiest unit (terminal-race nondeterminism + dedup fusion + the TLC temporal path), it carries 7 of the checked properties, and a separate story gives a fresh implementation agent a smaller, focused surface. The remaining units are authored inline under the feature: Units 2–4 are small checked models, Units 5–6 are draft compile-only models, Unit 7 is Alloy. This matches the precedent (`feature-verification-contract-authority` spawned no stories for a cohesive single-stride design).

## Testing

There is no implementation *code* (Rust/TS) — verification is by running the checkers:

- **Parse + typecheck gate**: `quint parse` + `quint compile` on every `.qnt` (catches the typed-action-parameter pitfall the bank-review found).
- **Apalache invariant checks**: `quint verify --invariant <v> --max-steps N` exit 0 (no counterexample) for the one-state properties.
- **TLC temporal checks**: `quint verify --backend tlc --temporal <p>` exit 0 for the two-state properties. Use default workers (the research's liveness-multi-worker caveat). **Implementation discovery (Unit 1)**: the `--backend tlc` path does NOT work for `next()`-in-`always()` temporal properties — see Implementation discovery below; use `echo y | quint verify --temporal <p>` (Apalache default) instead.
- **Alloy checks**: `exec --command <label> --type json` → `UNSAT` for all three relational asserts.
- **Promotion-metadata grep**: a script greps `@promotion` blocks and verifies (a) every checked-normative property-id in the vocabulary table has a `status: promoted` block, (b) no block references a misspelled property-id, (c) every `invocation` names the jar-path classpath. This is the seed of the CI coverage check the authority feature specified; full CI wiring belongs to `feature-protocol-idl-and-conformance`.

## Risks

- **Composed-model parse risk (highest).** The research validated hello-worlds and individual idiom snippets, not a composed Patchbay model. `command_lifecycle.qnt` fuses maps (`str -> str`), multiple `temporal` blocks, and a nondeterministic `step` with `any { ... }` alternation — none of which has been runtime-validated as a *composed* unit. The skill idioms parse individually (bank-review verified), but composition may surface Quint grammar limits (e.g. `intToString` in a Set concatenation, or map comprehension syntax). **Mitigation**: Unit 1 is implemented first and is the story; if it fails to parse, the implementation stride resolves the syntax before Units 2–7. This is an honest *design target the implementation stride must discharge*, not a verified capability — recorded as such.
- **Projection-seam mechanism unverified.** Cross-cluster references modeled as "projected variables" (e.g. `session_generation.qnt` carries its own `lsn` rather than importing `command_lifecycle.qnt`'s) has not been validated as a Quint pattern. If projection proves awkward, the fallback is independent models with duplicated-but-bounded projection variables (the current design) — already the chosen shape, so no re-design needed, only possible syntax adjustment.
- **TLC bounds scalability.** `command_lifecycle.qnt` with 3 command ids × 6 terminal candidates × 3 idempotency keys × `--max-steps 12` may hit TLC state-space limits or be slow. **Mitigation**: bounds are the smallest that exercise the race (≥2 terminal candidates competing); if TLC can't exhaust it, reduce to 2 command ids. Bounds are recorded per-property in `@promotion` so a re-check is reproducible.
- **Emitted-TLA+ is not an independent re-check lane (Q4 correction).** Standalone `tla2tools-1.7.4.jar` cannot parse emitted TLA+ (`EXTENDS ... Apalache, Variants`); even with the Apalache jar on the classpath it's the same toolchain (Apalache+TLC) reached via Quint. The emitted `.tla` is an *inspection artifact*, not a second verification lane. **Accepted**: the durable, human-inspectable, version-pinned form is still valuable; the "two independent paths to TLC" framing is dropped from the rationale. Recorded here so the design does not overclaim defense-in-depth.
- **`intToString` / string-concatenation in Quint.** The `session_generation` model uses `sid ++ ":" ++ intToString(gen)` for tombstone keys; if Quint 0.32.0 lacks `intToString` or `++` for strings, the key-shape must be adjusted (e.g. a `(str, int)` tuple as the map key). Implementation-stride resolution, not a design blocker.
- **Property-set stability.** The checked-normative property list is a v0 commitment. If implementation reveals a stated-normative property (snapshot/authority) is safety-critical earlier than expected, it must be promoted before its behavior ships — a per-property operation, not a re-open of the baseline (per the authority feature's Q2 design).

## Implementation discovery (Unit 1: command_lifecycle.qnt)

Unit 1 is implemented and all 7 properties check (`[ok]`). Three classes of discrepancy from the design were found and resolved in the implementation stride:

1. **Quint syntax (3 fixes, all grounded in `.research/attestation/quint-builtin.md`)**:
   - Map literal is `Map("c1" -> "k1", ...)`, NOT `Set("c1" -> "k1", ...)` (the latter is a Set of tuples). The design used the wrong form.
   - Map membership is `state.keys().contains(c)`, NOT `state.contains(c)` (`.contains` is a Set op; maps use `.keys()` then set membership).
   - Quint `any { ... }` action branches must update the same set of variables — branches touching fewer vars need explicit `x' = x` for the untouched ones (added `all { ... }` wrappers in `receive`).

2. **Two properties were not genuine checks (caught by test integrity — Apalache found a counterexample on `retry_reuses_id_and_key`)**:
   - `boundary_dedup` was self-defining (`appliedKeys.size() <= IDEMPOTENCY_KEYS.size()` — a Set cannot exceed its universe by construction). Restructured to a genuine per-key count check `applyCount.get(k) <= 1` over a new `applyCount: str -> int` state variable and a permissive `receive(key)` action.
   - `retry_reuses_id_and_key` was inverted (asserted the opposite of reality). Restructured to a temporal binding-stability property `next(idemKey.get(cmd)) == idemKey.get(cmd)`.
   - This validates the genuine-checking discipline from Q1/Q3: permissive transitions + property checked against them catches self-defining/inverted invariants that would otherwise pass for the wrong reason.

3. **Q3's `backend: tlc` for temporal properties does not work (load-bearing discovery)**:
   - The Quint→TLA+ compilation of `next()`-in-`always()` emits `[](\A cmd: state[cmd]' = state[cmd])`, which TLC rejects with `[] followed by action not of form [A]_v` (a TLA+ stencil requirement that `[]` wraps an action subscripted by vars).
   - Apalache (default backend) checks these temporal properties correctly (all 5 pass, exit 0) but warns its temporal support is "experimental and might give incorrect results" — and prompts interactively, requiring `echo y |` in non-interactive runs.
   - **Impact on the design**: the `backend` field for the 5 temporal properties is changed from `tlc` to `apalache-temporal`; the `invocation` is `echo y | quint verify ... --temporal <p> --max-steps 10`. The vocabulary-table `Backend` column for `command_lifecycle.qnt` temporal properties should read `apalache-temporal` (not `tlc`) in any downstream CI/generator.
   - **Impact on Q3 design choice**: Q3=C (mixed backends) is still the right call — Apalache handles both the one-state invariants AND the temporal safety properties. But the *rationale* shifts: the original rationale was "TLC for two-state, Apalache for one-state"; the corrected rationale is "Apalache for both, with the temporal path requiring the prompt-piped invocation and carrying the experimental-support caveat." This is a real residual risk for a safety-claiming model: Apalache's temporal checker is experimental. **Mitigation**: the properties are all `always(...)` safety (not `eventually` liveness), which is the more conservative end of Apalache's temporal support; and the emitted TLA+ is inspectable. A future follow-on can re-examine the TLC stencil workaround (e.g. hand-writing the `[A]_vars` form) if Apalache temporal confidence is insufficient.
   - This is the same species of gloss the audit caught in Q4/Q5: Q3 asserted the TLC path works for temporal without verifying it on a composed Patchbay model. The research validated `--backend tlc` on a hello-world *invariant*, not on a `next()`-in-`always()` *temporal* property. Caught here in the implementation stride, not at design time — the honest target the stride discharges.

## Implementation notes (Unit 2: session_generation.qnt)

Unit 2 is implemented and all 4 properties check (`[ok]`). Implementation used tuple-keyed tombstone maps rather than string-concatenated keys:

- `tombstoned: (str, int) -> bool` and `tombstoneLsn: (str, int) -> int`, with keys drawn from `TOMBSTONE_KEYS`.
- Avoided unverified `intToString` and string `++`; tuple keys were parse/typecheck validated before use.
- Used a unified conditional `step` event (`report` / `late` / `relabel`) so Apalache's temporal checker completes at the required `--max-steps 10` while preserving permissive lower/equal generation no-ops and stale late-event no-ops.
- `session_generation.emitted.tla` is generated and committed as an inspection artifact only, not an independent verification lane.

## Orchestrator implementation summary (2026-07-01)

All 7 implementation units are complete. The feature was implemented as 1 child story (Unit 1, the trickiest unit — done separately) + 6 inline units authored via parallel sub-agent dispatch (the orchestrator's childless-feature / inline-units case, since the design specified Units 2–7 as inline feature work, not stories).

### Units delivered

| Unit | File | Tier | Status |
|---|---|---|---|
| 1 | `specs/seed/command_lifecycle.qnt` (+ `.emitted.tla`) | checked | done (story, 7 properties `[ok]`) |
| 2 | `specs/seed/session_generation.qnt` (+ `.emitted.tla`) | checked | done (4 properties `[ok]`) |
| 3 | `specs/seed/reply_correlation.qnt` (+ `.emitted.tla`) | checked | done (1 property `[ok]`) |
| 4 | `specs/seed/csrf_browser.qnt` (+ `.emitted.tla`) | checked | done (3 properties `[ok]`) |
| 7 | `specs/seed/patchbay-relational.als` | checked | done (3 asserts `UNSAT`) |
| 5 | `specs/seed/snapshot_recovery.qnt` | stated (draft) | done (compiles, 6 reserved ids) |
| 6 | `specs/seed/authority.qnt` | stated (draft) | done (compiles, 4 reserved ids) |

### Dispatch (2 waves, 6 parallel sub-agents, all openai-codex per AGENTS.md)

- **Wave 1 (4 parallel)**: Unit 2 (`gpt-5.5` high), Unit 3 (`gpt-5.5` high), Unit 4 (`gpt-5.5` high), Unit 7 Alloy (`gpt-5.5` high). All checked models + Alloy, file-independent.
- **Wave 2 (2 parallel)**: Unit 5 draft, Unit 6 draft (`gpt-5.3-codex-spark` medium — compile-only draft models, lower complexity).
- **Bundling**: no multi-item bundles — each unit is one independent file with its own verification surface, so one agent per item (the default). Parallelism followed file-write ownership, not item count.

### Integration verification (orchestrator re-ran, not just trusting agent claims)

- All 4 checked `.qnt` models: `quint parse` + `quint compile` exit 0.
- Spot-checked one property per checked model: `generation_monotonic` `[ok]`, `typed_correlation` `[ok]`, `csrf_rejects_unauthenticated` `[ok]` (Unit 1 re-confirmed in its review).
- Alloy: all 3 asserts `UNSAT` (the CLI writes status to output dirs, not stdout — confirmed by agents and consistent with the skill's documented behavior).
- Both draft models: `quint parse` + `quint compile` exit 0.
- `@promotion` block counts: 27 across the 6 Quint models + 3 in Alloy = 30 total. Property-id vocabulary established as SSOT for the entire checked-normative set (checked + draft).

### Deviations across items (recorded by agents, consolidated here)

- **Unit 2 (session_generation)**: avoided unverified `intToString`/string `++` (the Risks-section flag was correct — not in the attestation); used `(str, int)` tuple keys for tombstones. Unified conditional `step` so Apalache temporal completes at `--max-steps 10`.
- **Unit 3 (reply_correlation)**: discovered `pure def` cannot read state variables — helpers inspecting state use `def`. Used filter-in-action encoding (invalid reply attempts leave no record; invariant asserts all recorded replies are valid).
- **Unit 4 (csrf_browser)**: same `pure def` → `def` discovery. Permissive request action accepts any session/proof/UI-claim; acceptance computed from server-side evidence only.
- **Unit 7 (Alloy)**: resolved the fact-vs-assert genuine-check question by modeling `fact DelegationRemovedV0 { no Grant }` (the v0 premise) and asserting acyclicity as a consequence (not a self-defining tautology). Used `let issuer = ~subject.issuer | no a: Actor | a in a.^issuer` for transitive closure over the Actor→Actor graph (the design's `g.issuer.^issuer` was the wrong relation shape).
- **No design flaws** triggered the escape hatch — all deviations were syntax/encoding fixes resolved in-stride, consistent with the "implementation target, not verified claim" risk posture recorded at design time.

### VERIFICATION.md update (feature acceptance criterion)

`docs/VERIFICATION.md` now references the seed models and their promotion status: a "Seed models (v0)" section with checked-normative and stated-normative tables, the toolchain note (Apalache-temporal, not TLC), and the emitted-TLA+ inspection-artifact caveat.

### Verification status

All checked properties pass (`[ok]` / `UNSAT`); all draft models compile; property-id vocabulary complete; emitted TLA+ artifacts committed. The residual risk from Unit 1 (Apalache-temporal experimental support for safety-claiming temporal properties) carries forward unchanged — flagged in the Unit 1 review and the feature's Implementation discovery section.

## Review (2026-07-01)

**Verdict**: Block (bounced to implementing)

**Review lane**: deep, substrate mode, fresh-context cross-model adversarial on `openai-codex/gpt-5.5` (xhigh thinking) — a different model class than the umans orchestrator, satisfying the cross-model requirement. The host verified the reviewer's findings empirically (mutation tests reproduced) before classifying.

**Blockers** (filed as `story-fix-formal-model-genuine-checks`):
- **B1 — `TypedCorrelation` self-referential** (`reply_correlation.qnt`): the invariant's `recordedReplyOk` calls `typedReferenceOk`, the *same* helper used in the action's `replyRecordable` filter. Mutation test confirmed: setting `typedReferenceOk = true` (total anti-forgery break) leaves `typed_correlation` still `[ok]`. The invariant cannot detect a broken correlation rule.
- **B2 — CSRF invariants self-referential** (`csrf_browser.qnt`): `validCsrfProof` is used in both the action's `serverAccepts` and the invariant. Same pattern as B1 — a broken proof predicate would let invalid requests through AND pass the invariant.
- **B3 — `LateGenerationInert` vacuously true** (`session_generation.qnt`): the `"late"` event kind is a dead stutter branch; the property passes because generation only changes when `lsn` changes (structural), not because late events are proven inert. Removing `"late"` from `EVENT_KINDS` leaves the property passing.
- **B4 — `GenerationMonotonic` weaker than claimed** (`session_generation.qnt`): proves non-decrease, not strict-supersession. A mutation allowing `gen >= current` (equal reports superseding) still passed. The action's `if gen > generation` guard makes strictness structural.
- **B5 — Alloy `AuthorityGraphAcyclicAssert` vacuous + contradicts PROTOCOL** (`patchbay-relational.als`): `fact DelegationRemovedV0 { no Grant }` removes ALL grants, but PROTOCOL (line 290-307) says v0 HAS grants (only delegation is absent). The assert proves an empty graph is acyclic.
- **B6 — Alloy `SenderMatchesClaimAssert` checks a fact** (`patchbay-relational.als`): the `fact` forces `sender = claimedSender`, then the assert checks the same — a tautology.

**Important** (filed as `story-fix-formal-model-disclosure-drift`):
- Feature body vocabulary-table backend drift (`tlc` vs `apalache-temporal` for session_generation temporal properties — the orchestrator's VERIFICATION.md update didn't propagate to the feature body table).
- Malformed emitted TLA+ header in `reply_correlation.emitted.tla` (`MODULE reply_correlation ---` without leading dashes).
- Draft `snapshot_recovery.qnt` omits several VERIFICATION normative variables.
- Draft `authority.qnt` has dead actions (`rotateSession`, `revokeTarget` not reachable from `step`).

**Backlog** (filed as `idea-tlc-temporal-workaround`): the experimental-temporal residual risk — all promoted temporal properties rely on Apalache's experimental temporal support. Not fixable in this stride; options to evaluate when promoted.

**Nits**: `@promotion` invocation fields omit the `specs/seed/` path prefix (not exact from repo root); Alloy CLI `--output -` produces empty stdout for UNSAT (status is in output dirs, not console — don't claim observed `UNSAT` unless captured).

**Notes**: This is the review bar earning its keep at the highest value — the cross-model adversarial pass found that 6 of the newly-promoted properties are self-defining or vacuous, which is the exact failure mode the verification program exists to prevent. The Unit 1 review (fast lane) caught 2 self-defining properties in one model; this deep review caught 6 more across the other models — confirming the pattern is systemic when agents author models without an independent check path. The host reproduced the two most damning findings (B1 TypedCorrelation, B2 CSRF) via mutation tests before classifying. The fix pattern is uniform: separate the action's implementation predicate from an independent property oracle, and verify via mutation test that breaking the predicate fails the invariant. The fixes stay within the existing design (Q1=B focused cluster, Q3=C mixed backends) — no design re-open needed, just genuine-checking rigor applied to Units 2–7 that Unit 1 already had.

## Re-review readiness (2026-07-01)

Both fix stories from the deep-review block are now `done`:
- `story-fix-formal-model-genuine-checks` (done) — 6 self-defining/vacuous properties fixed; all 4 mutation tests reproduce `[violation]` (independent-oracle proof).
- `story-fix-formal-model-disclosure-drift` (done) — 4 disclosure/drift findings fixed; draft models parse+compile; all checked temporal properties consistently `apalache-temporal` across all 3 sources.

All 3 child stories are now terminal (`done`). The feature re-advances `implementing → review`. The original deep-review blockers (B1–B6) are resolved with the genuine-checking discipline empirically proven via mutation tests; the important findings (I1–I4) are resolved. The residual `idea-tlc-temporal-workaround` backlog item remains (experimental-temporal risk — not fixable in this stride).

## Re-review (2026-07-01)

**Verdict**: Block (bounced to implementing again)

**Review lane**: deep, substrate mode, fresh-context cross-model adversarial re-review on `openai-codex/gpt-5.5` (xhigh) — same posture as the original block. The host verified the reviewer's findings empirically before classifying (and caught its own measurement error in the process).

**Blockers** (filed as `story-fix-alloy-relational-assertions`):
- **B5 — `AuthorityGraphAcyclicAssert` now FAILS (regression from the fix)**: removing `fact DelegationRemovedV0 { no Grant }` turned the assert from vacuously-true into actually-false. Alloy finds a counterexample (self-grants create a 1-cycle). The assert checks an *invented* rule — PROTOCOL does not state v0 grants form an acyclic issuer graph (acyclicity is only meaningful once delegation exists, which is out of v0). **Fix**: demote to `status: draft` (reserved for the delegation follow-on).
- **B6 — `SenderMatchesClaimAssert` now FAILS (regression from the fix)**: removing `fact SenderMatchesClaim` left nothing forcing `sender = claimedSender`. Alloy finds `sender=Actor$0, claimedSender=Actor$1`. Per the Alloy brief's caveat, the *binding* is a dynamic CompoundIssuer-style property that belongs in `authority.qnt`, not a relational snapshot. **Fix**: demote to `status: draft` (reserved for the authority follow-on).

**Important** (filed in the same story):
- **B2-trace — CSRF invariant trace-fidelity weakness**: the B2 fix removed the helper self-reference, but the invariant trusts `lastProof` (recorded by the action). A deeper mutation (action lies about submitted proof) still passes. **Fix**: add attempted-evidence state; invariant checks raw submitted values. (Generalized to `idea-csrf-trace-fidelity` backlog.)
- **B4-overclaim — `GenerationMonotonic` semantics overclaim**: the checked property is non-decrease, but the `@promotion` `semantics` field still claims strict-supersession. Mutation allowing equal supersession stays `[ok]`. **Fix**: narrow the semantics field to "non-decrease"; note strict-supersession as a structural guard property.

**Confirmed-genuine** (no action): B1 (`TypedCorrelation`) and B3 (`LateGenerationInert`) are genuinely fixed — mutation tests reproduce `[violation]`. B2's helper self-reference IS fixed (the partial fix is real, just incomplete on the trace layer).

**Notes**: This re-review found a regression the original block didn't — the B5/B6 "fixes" traded vacuous-true for actually-false, which is worse for a safety-claiming artifact. The root cause: removing a forcing fact without adding a real constraint. The honest resolution is that B5/B6 are NOT checkable as relational invariants in v0 without becoming tautological (B6's binding is dynamic; B5's acyclicity needs delegation) — so both demote to draft, and only `ActorIdsUniqueAssert` remains a promoted Alloy check. The host also caught its own measurement error: `--type json`/file-count gave false UNSAT; `--type text` with a skolem-witness check is the reliable method (recorded in the fix story). The `idea-tlc-temporal-workaround` and new `idea-csrf-trace-fidelity` backlog items carry the residual risks. The review bar earned its keep again — the adversarial re-pass caught a regression the fix pass introduced.

## Re-review readiness (2026-07-01, second pass)

All 4 child stories are now `done`:
- `story-formal-model-command-lifecycle` (done) — Unit 1, the trickiest model.
- `story-fix-formal-model-genuine-checks` (done) — 6 self-defining/vacuous properties fixed; mutation-test proof.
- `story-fix-formal-model-disclosure-drift` (done) — 4 disclosure/drift findings fixed.
- `story-fix-alloy-relational-assertions` (done) — B5/B6 regression resolved (demoted to draft — not checkable relationally in v0); B2-trace attempted-evidence fix; B4-overclaim semantics narrowed.

The feature re-advances `implementing → review` for the second deep-review re-pass. Current promoted state: only `ActorIdsUniqueAssert` promoted in Alloy (UNSAT); all promoted Quint properties `[ok]` with mutation-test genuine-checking proof. Residuals honestly disclosed: `idea-tlc-temporal-workaround` (experimental-temporal), `idea-csrf-trace-fidelity` (pattern generalized). The two deep reviews caught, respectively, 6 self-defining properties and 1 fix-pass regression — the adversarial re-pass is the right scrutiny for a safety-claiming artifact that's now been through two block→fix cycles.

## Review (2026-07-01, second re-review)

**Verdict**: Approve with comments (advanced to done)

**Review lane**: deep, substrate mode, fresh-context cross-model adversarial re-review on `openai-codex/gpt-5.5` (xhigh) — the second feature-level deep pass. The host verified the reviewer's findings empirically before classifying.

**Blockers**: none. All promoted safety-critical properties genuinely hold (mutation-test proven by the reviewer and re-confirmed by the host): B1 TypedCorrelation, B2 CSRF helper, B3 LateGenerationInert, B4 GenerationMonotonic all catch their mutations `[violation]`; the one promoted Alloy assert (`ActorIdsUniqueAssert`) is UNSAT (0 skolems, reliable `--type text` method); B5/B6 honestly demoted.

**Important** (filed as `story-fix-csrf-trace-and-ssot-drift`):
- **I1 — CSRF `attemptedProof` still action-recorded**: the B2-trace fix closed the recorded-trace lie (`lastProof`) but `attemptedProof` is still action-assigned, so a *combined* mutation (drop the proof check AND lie about `attemptedProof`) still passes `[ok]`. Confirmed empirically by the host. Root cause: the raw submitted evidence must be PRE-STATE/environment input the accepting action reads but cannot rewrite. Fix story: split request capture from server processing. This is the `idea-csrf-trace-fidelity` pattern's correct completion.
- **I2 — vocabulary-table SSOT drift**: three draft-property mismatches between the feature body table and the `@promotion` blocks / VERIFICATION.md (`TimeoutNeitherSuccessNorDenial` model, `CompoundIssuer` backend, `RevocationPreventsFuture` backend). Not safety-failing (draft properties) but the feature claims the table is SSOT.

**Nit**: `ActorIdsUniqueAssert` checks the same constraint as `fact ActorIdsUnique`; the comment's non-vacuity claim is overstated. Recorded in the fix story.

**Residuals**: `idea-tlc-temporal-workaround` (experimental-temporal) — adequate disclosure. `idea-csrf-trace-fidelity` — partially applied (csrf_browser only, and incompletely per I1); the fix story completes it.

**Notes**: This is the third deep pass over the seed models. The arc: first review caught 6 self-defining properties; second caught a fix-pass regression (B5/B6 vacuous-true → actually-false); this third pass found the B2-trace fix was itself incomplete (`attemptedProof` still action-recorded) plus metadata drift. Each pass caught what the previous couldn't because each attacked from a fresh angle. The promoted safety-critical properties now genuinely hold — the remaining findings are a deeper trace-fidelity refinement (I1, filed) and draft-metadata consistency (I2, filed), neither of which undermines the soundness of the checked properties. Advancing to done with the findings filed as a follow-up story rather than blocking: the safety claims that ARE promoted are sound, and the gaps are honestly disclosed + tracked. The feature delivers its acceptance criteria (checked-normative properties genuinely checked, draft models with reserved ids, property-id vocabulary as SSOT, VERIFICATION.md referencing the models). Two block→fix cycles + this approve-with-comments is the right cost for a safety-claiming artifact.

## Final re-review (2026-07-01, post-follow-up)

**Verdict**: Approve (re-advanced to done)

**Lane**: deep-lane posture, host-run (the follow-up story was fast-lane reviewed; this is the parent re-review the substrate requires when a child changes under a done feature — the procedural step skipped when the follow-up was filed under the already-done feature).

**Context**: `story-fix-csrf-trace-and-ssot-drift` (the follow-up filed at the second feature review) landed and was fast-lane reviewed to `done`. Filing a child under a done feature re-opens the parent's review surface in principle; this pass closes that gap. All 5 child stories are now terminal.

**Verification (host re-ran the full suite + mutation-test sweep)**:
- All 10 promoted checks green: 7 Quint invariants `[ok]`, 2 Quint temporal `[ok]`, 1 Alloy assert `UNSAT` (0 skolems, reliable `--type text`).
- Mutation-test sweep (the genuine-checking proof) — all 4 reproduce `[violation]`: B1 TypedCorrelation (break `typedReferenceOk`), B2-trace CSRF (drop proof check; `csrf_rejects_unauthenticated` correctly stays `[ok]` — discriminating), B3 LateGenerationInert (late mutates generation), B4 GenerationMonotonic (allow decrease). The follow-up's CSRF restructure (pre-state `attemptedProof` split) introduced no regressions.

**Blockers**: none. **Important**: none. **Nits**: none.

**Notes**: The follow-up's substantive work — closing the CSRF attempted-evidence trace-fidelity gap (raw submitted evidence is now pre-state the accepting action reads but cannot rewrite) and fixing the vocabulary-table SSOT drift — is verified and holds. The seed formal-model arc is complete: all promoted safety-critical properties genuinely checked (mutation-test proven end-to-end across the full review arc), SSOT consistent across the 7 model files / `docs/VERIFICATION.md` / feature body, and the two residuals (`idea-tlc-temporal-workaround`, `idea-csrf-trace-fidelity`) honestly disclosed in backlog. The procedural lesson recorded: filing a child under a done feature re-opens its review surface — re-review the parent when the child lands, even for "post-hoc refinements."
