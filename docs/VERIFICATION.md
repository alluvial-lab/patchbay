# Patchbay Verification

Patchbay treats coordination semantics as specification-first. Formal models define the behavior the implementation must preserve.

## Artifact authority order

Authority is question-type-layered, not a single ranked list. Each artifact type owns one class of question and is authority only for that class:

| Question type | Authority | Not authority for |
|---|---|---|
| Invariants, dynamic/relational properties | Formal models (TLA+/Quint/Alloy), once promoted | wire shape, product intent naming |
| Wire shape, field identity, enum wire encoding, payload envelopes | `.proto` (Protobuf+Buf) | invariants, product intent, enum variant naming |
| Product intent, vocabulary naming, registry names | Prose (`docs/PROTOCOL.md`, `docs/SPEC.md`, `docs/SECURITY.md`, `docs/ARCHITECTURE.md`) | invariants, wire shape |
| Expected executable examples for a specific scenario | Conformance vectors, once promoted | invariants, wire shape, product intent |
| Anything | Implementation (never authority) | — |

Disagreements route to the artifact that owns the question type. "The higher artifact wins" is not a global rule. A contradiction between two promoted artifacts that each own their question type — for example a promoted conformance vector and the formal model whose property it exercises — is a **surfaced reconciliation event**, not a silent override by whichever artifact is ranked higher: either the model is wrong (update the model, then re-check every vector exercising that property) or the vector is wrong (demote, fix, re-promote). Implementation is never authority; if running code disagrees with a normative artifact, the code is the bug-fix target.

This layered order takes effect once generated schemas or IDL exist. `docs/PROTOCOL.md` remains the canonical source of truth for product intent and vocabulary naming both before and after `.proto` exists; it is the *provisional* wire reference only until `.proto` takes over wire-shape authority.

## v0 normative baseline (property-graded)

Each required model area below is obligated at v0. Properties within each area are tiered by risk, not all-or-nothing per area. This reconciles `docs/SPEC.md`'s verification-floor seed ("at least seed formal/property checks for command acceptance, idempotent retry, session identity, snapshots, and authority") with this document's required-areas list: the checked set is the seed done right, not a different program.

**checked-normative** — must clear the model-promotion rule **and** have ≥1 promoted conformance vector tracing to the property before v0 treats the behavior as product. Covers safety/security-critical properties:

- Operator intent delivery: `TerminalFinality`, `LsnDeterminesTerminalWinner`, `PreAppendTerminalChoice`; accepted-command durability (an accepted command cannot vanish silently); timeout implies neither success nor denial.
- Wrong-session prevention: session identity tuple (adapter id + deployment scope + runtime session id + session generation); `LateGenerationInert`; `GenerationMonotonic`; human-readable labels cannot override verified target identity.
- Idempotent retry: boundary dedup (retrying the same idempotency key cannot double-apply); retry reuses both command id and idempotency key; retry after terminal returns the existing terminal record rather than creating a later terminal candidate.
- Authority safety: no-command-without-grant rejection before acceptance and delivery; `CompoundIssuer`; `GrantAuthorityIsCommandKinds`; revocation prevents future command acceptance under the revoked grant.
- Crash recovery: no accepted command disappears silently after an ungraceful restart; idempotent log replay (replaying the same committed prefix produces identical state).
- Browser session and CSRF boundary: a state-changing request without an authenticated operator session is rejected before command acceptance; a state-changing request without a valid session-bound CSRF proof is rejected before command acceptance; revoked or expired operator sessions cannot issue new commands.
- Snapshot convergence (core safety): a snapshot with an LSN strictly less than the core's current revision for that view is rejected as an authority source and replaced by the current view; a snapshot from a different authority domain or core generation is rejected outright; snapshot materialization reads a consistent log prefix (every event with `LSN <= snapshot_LSN` and no event with `LSN > snapshot_LSN`); a late event whose LSN is older than the view it would mutate is recorded as an audit/reconciliation event and does not rewrite the current view.
- Reply correlation (core safety): `TypedCorrelation` — a reply correlates by typed reference to a known command or message id in the same authority/session context and cannot forge correlation across id spaces (a reply id cannot masquerade as a command id) or across session/authority contexts.

**stated-normative** — documented v0 obligation with a *draft* model, not yet checked-to-pass; scheduled for promotion post-v0. Covers liveness/cosmetic/operational properties:

- Snapshot convergence (refinements): compaction and cursor validity nuances; "event streams not required for correctness when snapshots exist" as an operational property.
- Audit integrity: completeness of audit records and correlation coverage.
- Adapter failure visibility: failure-vocabulary distinguishability refinements.
- Reply correlation (refinements): duplicate-reply idempotency/rejection and reference-resolution edge cases beyond the checked `TypedCorrelation` core.

This composes with the model-promotion rule: a property promotes its model **and** its vectors together. "checked-normative" = model promoted + ≥1 promoted vector; "stated-normative" = draft model, no promoted vector yet. Promotion is a per-property operation; if implementation reveals a safety-critical property classified stated-normative, it must be promoted before its behavior ships.

The delegation precondition and lease safety sections below are preconditions for future behavior and are **not** part of the v0 normative baseline.

## TLA+ and Quint position

TLA+ and Quint are compatible at the architecture level because both model state machines in the TLA tradition. Patchbay does not need to choose a permanent winner before design starts.

Patchbay uses this policy:

- **TLA+ is the semantic baseline** for durable, long-lived protocol models because it is established and has mature TLC tooling.
- **Quint is the ergonomic authoring candidate** for models where readability, type checking, and developer/agent editing speed matter.
- Models may begin in Quint and be checked through available backends, including TLC where appropriate.
- A model promoted to normative status must have a stable checked artifact and documented tool invocation, regardless of whether it is authored in TLA+ or Quint.
- The repository keeps model intent portable: no product decision depends on a tool-specific trick when the underlying property can be stated plainly.

This means Patchbay can start with Quint for approachability and keep TLA+ as the reference-compatible foundation.

## Alloy position

Alloy is complementary rather than competing with TLA+/Quint.

Patchbay uses Alloy for bounded relational invariants:

- actor identity uniqueness;
- endpoint/address ambiguity;
- authority graph constraints;
- revocation relationships;
- lease exclusivity;
- routing legality;
- anti-spoofing relationships.

TLA+/Quint models dynamic histories. Alloy models relational shapes and small counterexamples.

## Required model areas

### Operator intent delivery

Properties:

- An accepted command is durably recorded before delivery.
- An accepted command cannot vanish silently.
- Every accepted command remains observable in exactly one canonical `CommandState` from `docs/PROTOCOL.md` until and after it reaches a terminal state.
- **TerminalFinality**: once a command reaches a terminal `CommandState`, later events for that command do not mutate the command state.
- **LsnDeterminesTerminalWinner**: for competing valid terminal candidates, the terminal winner is the candidate with the lowest committed log sequence number in the authority domain.
- **PreAppendTerminalChoice**: if terminal candidates are truly concurrent before durable append, the model may choose the appended winner nondeterministically; after an `LSN` is assigned, that order determines all later snapshots, replay, conformance traces, and UI reconciliation.
- Timeout does not imply success or denial.

### Wrong-session prevention

Properties:

- Commands bind to target session identity and generation. Session identity is the tuple adapter id + deployment scope + runtime session id + session generation; project, cwd, and name are metadata, not identity.
- **LateGenerationInert**: events/replies binding to a tombstoned session generation are `stale_event` audit records; they do not mutate the live generation.
- **GenerationMonotonic**: session supersession requires a strictly-greater generation; lower reports are rejected as audit and the live generation is unchanged; equal reports are a no-op.
- Human-readable labels cannot override verified target identity.

### Reply correlation

Properties:

- A reply references a known prior message or command by typed correlation.
- **TypedCorrelation**: a reply correlates by typed reference to a known command or message id in the same authority/session context; it cannot forge correlation across id spaces (a reply id cannot masquerade as a command id) or across session/authority contexts.
- Duplicate replies are either idempotent or visibly rejected.

### Idempotent retry

Properties:

- Retrying the same idempotency key cannot double-apply a command at the Patchbay boundary.
- A retry reuses both the command id and the idempotency key; an intentional duplicate action uses a new command id and a new idempotency key.
- Duplicate submission returns existing command state.
- Retrying after a command is terminal returns the existing terminal command record rather than creating a later terminal candidate.
- Explicit duplicate action requires a new command id/key.

### Snapshot convergence

Properties:

- A reconnecting control surface can recover authoritative state from snapshots.
- Stale cached live/working state is corrected by a newer authoritative snapshot.
- Event streams are not required for correctness when snapshots exist.
- A snapshot with a log sequence number strictly less than the core's current revision for that view is rejected as an authority source and replaced by the current view.
- A snapshot from a different authority domain or core generation is rejected outright.
- A late event whose log sequence number is older than the view it would mutate is recorded as an audit/reconciliation event and does not rewrite the current view.
- Terminal outcomes are deterministic after durable append: replay, snapshots, conformance traces, and UI reconciliation expose the terminal state chosen by committed log order.
- Snapshot materialization reads a consistent log prefix: it reflects every event with `LSN <= snapshot_LSN` and no event with `LSN > snapshot_LSN`.

Normative model variables should include at least `LSN`, `Cursor`, `SnapshotRevision`, `AuthorityDomain`, `CoreGeneration` (the core's own incarnation, core-assigned on restart), `SessionGeneration`, `AdapterGeneration`, and the view variables (`CommandId`, `MessageId`, `ReplyId`, `CorrelationRef`, `SessionId`, `ActorId`) the snapshot reconciles.

### Crash recovery

Properties:

- After an ungraceful core restart, replay of the durable log reconstructs in-memory state up to the last committed `LSN`.
- Accepted commands are restored as `accepted` (or a later committed state) and continue through their lifecycle; no accepted command disappears silently.
- Log replay is idempotent: replaying the same committed prefix produces identical state.
- Snapshot checkpointing bounds recovery replay cost without becoming an alternate ordering authority.

Normative model variables should include at least `CommittedPrefixLSN` (the last durably committed log prefix), `CheckpointSnapshotLSN` (the latest snapshot loaded before tail replay), `RecoveredCommandState` (the command-id to `CommandState` map reconstructed from the log), `RecoveredInbox` (delivery/inbox queue state reconstructed from the log), `RecoveredSessionView` (session connectivity/activity axes reconstructed from the log), and `RecoveryPhase` (initial load vs tail-replay vs live). `Crash` and `Restart` are the transition triggers.

V0 models do not need to prove remote replication, HA failover, or split-brain resolution. Those are out of formal scope.

### Authority safety

Properties:

- Commands without grants are rejected before durable acceptance and before delivery.
- Grant matching checks issuer actor, optional endpoint, target scope, command kind, expiration, and revocation generation. Device is not a grant-matching field; it is an identity and revocation-grouping variable.
- Revocation prevents future command acceptance under the revoked grant.
- Already accepted commands follow the grant's revocation policy: continue, cancel where supported, or require reauthorization.
- Lockdown rejects new commands and marks affected runtime sessions stale until fresh authentication or operator action clears the condition.
- **CompoundIssuer**: when a command arrives through a control surface, the core verifies the transport endpoint (e.g. the web server, or a CLI endpoint) as a principal and independently verifies the operator actor against operator-session evidence. The core must not trust a self-asserted operator identity.
- **GrantAuthorityIsCommandKinds**: grant authority is expressed only in canonical Patchbay command kinds. Adapter capability declarations are advisory UX state and are not an authority or delivery gate; the adapter accepts or rejects at delivery time.

Normative model variables should include at least `Actor`, `Device`, `Endpoint`, `OperatorSession`, `Grant`, `GrantScope`, `CommandKind`, `Target`, `TargetGeneration` (the session generation bound to a command target), `RevocationGeneration`, `CommandIssuer`, `AuthorityDomain`, `SessionGeneration`, `AdapterGeneration`, and `CorrelationRef`. `Device` is included as an identity, audit, and revocation-grouping variable even though it is not a grant-matching field.

### Delegation precondition

Delegation is not part of v0. The following property is a precondition that must be satisfied before any delegation-backed behavior ships; it is not a required v0 authority-safety obligation:

- Delegation cannot create authority beyond its parent grant.

### Lease safety

Lease safety remains a required model area before any lease-backed product behavior ships. It is not part of the v0 executable walking skeleton unless later foundation work explicitly promotes a specific lease-backed workflow.

Properties:

- Two actors cannot simultaneously hold the same exclusive live lease in one authority domain.
- Expired leases do not authorize new exclusive action.
- Lease renewal respects holder identity and scope.

### Adapter failure visibility

Properties:

- Adapter disconnect, crash, rejection, unsupported command, target offline, timeout, expiration, cancellation, and supersession remain distinguishable using the failure/outcome vocabulary in `docs/PROTOCOL.md`.
- Adapter failure cannot appear as command completion.
- A `partial` or `no snapshot` adapter cannot cause the core to fabricate a live snapshot from cached or optimistic state; affected session axes move to `stale` or `unknown`.

### Browser session and CSRF boundary

Properties:

- A state-changing browser request without an authenticated operator session is rejected before command acceptance.
- A state-changing browser request without a valid session-bound CSRF proof is rejected before command acceptance.
- Revoked or expired operator sessions cannot issue new commands.
- Browser-local state cannot grant authority or override core grant checks.

Formal models do not prove browser cookie mechanics or cryptographic token strength; they model the server-side effects of valid, missing, expired, and revoked session/CSRF evidence.

### Audit integrity

Properties:

- Security-relevant decisions produce audit records: authentication success/failure, session revocation, failed authorization, command acceptance/rejection, grant changes, lockdown, adapter failure, and stale-event rejection.
- Audit records correlate to actor, device, endpoint/session when known, target, command, outcome, and reason without requiring secret material in the model.
- Rejected attempts and failed checks can produce audit records without creating command records.
- Revocation and terminal command outcomes remain visible in audit history; they are not deleted by later state changes.

## Out of formal scope

Patchbay formal models do not prove:

- LLM output quality;
- correctness of cryptographic primitives;
- operating-system scheduling or mobile background behavior;
- UI rendering correctness;
- third-party harness internals;
- real-world network latency bounds;
- adapter-specific behavior beyond declared adapter contracts.

Those areas require tests, monitoring, adapter documentation, and operational discipline.

## Conformance testing

Formal models produce implementation obligations. The implementation uses:

- protocol golden vectors shared across languages and derived from the canonical state/failure registries in `docs/PROTOCOL.md`;
- terminal-commit race vectors covering completion before cancellation, cancellation before completion, expiration before late completion, retry after terminal, late terminal candidate as audit/reconciliation only, and replay of the same committed prefix;
- property tests for Rust core behavior;
- property tests for TypeScript operator-domain behavior;
- adapter conformance tests for declared capabilities;
- replay tests for event logs and snapshots;
- reconnect tests for stale control surfaces.

A protocol semantic change updates `docs/PROTOCOL.md`, the model, generated contract, conformance vectors, and implementation together.

### Conformance-vector promotion and traceability

Conformance vectors are draft/derived until explicitly promoted, mirroring the model-promotion rule. A promoted vector is a peer authority for expected executable examples (see Artifact authority order). Vectors are never authority for invariants (that is the formal models) or for wire shape (that is `.proto`).

Vector promotion requires:

- a named model property the vector exercises (property id);
- the `.proto` fields/enums the vector constrains (or `none` for pure state-transition vectors);
- an expected outcome matching the referenced model property's invariant;
- a reviewed status (a vector is promoted by review, not automatically).

Each conformance vector file carries structured frontmatter naming these fields:

```yaml
property: <property-id>
status: draft | promoted
proto_fields: [field/path, ...]   # or [none]
expected: <outcome>
```

A CI script reads all vectors and:

- fails if a checked-normative property lacks a promoted vector;
- fails if a vector references a missing or misspelled property id;
- fails if a promoted vector's expected outcome contradicts its referenced model property's invariant (a surfaced contradiction, per the authority order);
- generates the traceability table in this document as a checked-in artifact, so the human-readable mapping from property → `.proto` fields → vectors never drifts.

A promoted vector that later contradicts its model is a reconciliation event: either the model is wrong (update the model, re-check every vector exercising it) or the vector is wrong (demote, fix, re-promote). It is never a silent override.

## Model promotion rule

A model becomes normative only when it includes:

- the property being checked;
- finite bounds or constants used for checking;
- command/tool invocation;
- expected pass/fail status;
- a short explanation connecting the model to product semantics.

Draft models may explore ideas without becoming product commitments.

## Seed models (v0)

The v0 seed formal models live under `specs/seed/`. Each model carries its promotion metadata as inline `@promotion` comment blocks (one per checked/draft property) — the machine-readable source a future CI script reads to generate the traceability table above. The property-id vocabulary established by the seed is the Single Source of Truth that `.proto` contracts, conformance vectors, and implementation all derive from.

### Checked-normative (model promoted; awaiting conformance vectors)

| Model | Language | Properties checked | Backend |
|---|---|---|---|
| `specs/seed/command_lifecycle.qnt` | Quint | `CommandDurability`, `BoundaryDedup` (invariants); `TerminalFinality`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `RetryReusesIdAndKey`, `RetryAfterTerminalReturnsExisting` (temporal) | Apalache + Apalache-temporal |
| `specs/seed/session_generation.qnt` | Quint | `SessionIdentityTuple`, `LabelsCannotOverrideIdentity` (invariants); `GenerationMonotonic`, `LateGenerationInert` (temporal) | Apalache + Apalache-temporal |
| `specs/seed/reply_correlation.qnt` | Quint | `TypedCorrelation` (invariant) | Apalache |
| `specs/seed/csrf_browser.qnt` | Quint | `CsrfRejectsUnauthenticated`, `CsrfRejectsMissingProof`, `RevokedSessionCannotCommand` (invariants) | Apalache |
| `specs/seed/patchbay-relational.als` | Alloy | `ActorIdsUnique` | Alloy CLI |

Each checked Quint model also commits a generated `*.emitted.tla` inspection artifact (via `quint compile --target tlaplus`); these are generated, never hand-edited, and are NOT an independent re-check lane (they `EXTENDS ... Apalache, Variants` and need the Apalache jar on the classpath — same toolchain reached via Quint).

### Stated-normative (draft models; property-ids reserved)

| Model | Language | Reserved property-ids |
|---|---|---|
| `specs/seed/snapshot_recovery.qnt` | Quint | `SnapshotStaleRejected`, `SnapshotCrossDomainRejected`, `SnapshotConsistentPrefix`, `LateEventNoRewrite`, `CrashNoAcceptedLost`, `IdempotentLogReplay` |
| `specs/seed/authority.qnt` | Quint | `NoCommandWithoutGrant`, `CompoundIssuer`, `GrantAuthorityIsCommandKinds`, `RevocationPreventsFuture` |
| `specs/seed/patchbay-relational.als` | Alloy | `AuthorityGraphAcyclic` (reserved — needs delegation, out of v0), `SenderMatchesClaim` (reserved — dynamic CompoundIssuer binding, belongs in authority.qnt) |

`TimeoutNeitherSuccessNorDenial` is a reserved property-id for a future transport/failure-vocabulary model (not in `command_lifecycle.qnt` — it concerns the submission/transport layer, not command-lifecycle state).

### Toolchain note (implementation discovery)

Quint temporal properties using `next()` inside `always()` are checked via the **Apalache default backend** (`echo y | quint verify --temporal <p> --max-steps 10`), not `--backend tlc`. The Quint→TLA+ compilation emits `[](...)` forms that TLC rejects with `[] followed by action not of form [A]_v`. Apalache checks these correctly but warns its temporal support is experimental; all checked temporal properties here are `always(...)` safety (not `eventually` liveness), the more conservative end of Apalache's temporal support. Tool versions: Quint 0.32.0, Apalache 0.56.1, Alloy 6.2.0, tla2tools 1.7.4. See `feature-formal-model-seed` Implementation discovery for detail.
