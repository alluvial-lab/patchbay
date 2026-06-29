---
id: feature-session-identity-adapter-contract
kind: feature
stage: done
tags: [protocol, adapter, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-command-state-ssot]
created: 2026-06-28
updated: 2026-06-29
gate_origin: null
release_binding: null
---

# Feature: Define session identity and adapter capability contract

Wrong-session prevention cannot rely on optional adapter metadata. Patchbay needs a normative session identity and adapter capability contract before Pi or other adapters are implemented.

## Retag note (2026-06-28)

Retagged from `[prose]` to a design feature. The `prose` tag was removed because the scope includes genuine design choices (adapter capability tier model, session generation semantics, capability manifest schema) that need a `feature-design` pass, not collapsed prose authoring. This is the misroute the prose-author black-box test should have caught originally.

A specific item carried over from `feature-persistence-snapshot-model`: the three-tier adapter snapshot model (authoritative / partial / none) currently committed in `docs/PROTOCOL.md` was invented during a prose feature without a design pass. It belongs in this feature's adapter-capability-tier design work and should be ratified or revised here.

## Scope

- Canonical session identity tuple and mandatory fields.
- Session generation/epoch semantics.
- Session replacement, tombstone, and reuse rules.
- Message/command/reply/event id spaces and correlation rules.
- Adapter registration/authentication to the core.
- Adapter capability manifest schema.
- Capability tiers for streaming, snapshots, cancellation, idempotency, and session replacement.

## Acceptance criteria

- `docs/PROTOCOL.md` defines session identity and correlation precisely.
- `docs/ARCHITECTURE.md` describes adapter registration and lifecycle.
- `docs/GLOSSARY.md` defines generation, endpoint, adapter capability, and correlation context.
- The Pi adapter can map its capabilities without redefining Patchbay core identity.

## Design decisions

- **Session identity (A1)**: The session identity tuple is `adapter id` + `deployment scope` + `runtime session id` + `session generation`. Project, cwd, and name are **metadata**, not identity — they update independently without changing the session target. This matches the wrong-session-prevention verification obligation, which binds commands to target session identity and generation, not to project/cwd/name.
- **Session generation (B1)**: The adapter reports a session generation when a runtime session is replaced. The core **tombstones** the prior generation (marks it superseded, retains it for audit/late-event correlation) and treats the new generation as the live target. Late events/replies binding to a tombstoned generation are `stale_event` audit records, not mutations — consistent with the ratified terminal-commit rule.
  - **Retention (1c)**: The tombstone fact ("generation N existed, superseded at LSN X") is an audit record retained indefinitely (cheap). Per-generation detail (full command/event/reply state) is bounded and reclaimable by log compaction. After compaction, an operator querying an aged-out generation gets the tombstone plus any not-yet-compacted detail, with a note that older detail was compacted.
  - **Monotonicity (2a)**: Supersession requires a strictly-greater generation. An equal report is a no-op (possibly a capability redeclaration, but generation unchanged). A lower report is rejected as an audit record, with the live generation unchanged. First registration (no live generation exists) is accepted; monotonicity has nothing to check against.
- **Correlation ids (C1)**: Four separate id spaces, each with a defined assigner. Command and message ids are client-generated (operator domain), assigned before submission (this is what makes idempotency work without a round-trip). Reply ids are adapter/core-generated and carry a **typed correlation reference** to a known command/message id (separate space — a reply id can never be mistaken for a command id, which structurally prevents forgery). Event ids are core-assigned (LSN). Command id and idempotency key are **separate fields**: command id is identity; idempotency key is the dedup handle. A retry reuses both; an intentional duplicate uses a new command id and a new idempotency key.
- **Adapter registration (D1)**: An adapter is a **principal** with an explicit registration lifecycle. At attach time it submits (a) attachment evidence verified by an adapter-specific trust root (Pi uses configured material; future adapters may use mTLS/OAuth — the mechanism is adapter-specific, not mandated by the core), and (b) a capability manifest. The core records adapter id, capability manifest, attach LSN, and adapter generation. Attach, detach, failure, and capability redeclaration are audit events. Capability redeclaration is allowed with audit; the core degrades affected sessions when capabilities are lost. Sessions discovered/reported by the adapter inherit the adapter's authenticated channel.
- **Capability manifest shape (E1a + E2a)**: Ratify the three-tier snapshot model (authoritative / partial / none) — tiering is warranted for snapshots because the core's reconciliation contract branches on the tier. Other capabilities are shaped by where the core's behavior branches: snapshot = 3-tier; idempotency = enum (none / at-Patchbay-boundary / end-to-end) because retry behavior depends on it; streaming, cancellation, and session replacement = boolean (the core does the same thing regardless of value beyond display); attachment = adapter-specific descriptor; known failure modes = advisory list mapping to the failure vocabulary.
- **Naming (γ)**: Keep **generation** as the consistent base term across all three scopes, qualified by scope: **core generation** (core-assigned on restart), **session generation** (adapter-reported on replacement), **adapter generation** (adapter-reported on re-attach). "Generation" is already deployed in the glossary and snapshot rules, readers understand it, and it is not wrong. The collision-protection discipline that matters is the qualifier + a glossary entry that **foregrounds the assigner** per scope, because the assigner is the structurally important fact and what the verification properties check. No cross-doc rename is performed.

## Architectural choice

Ratify a session-identity, generation, correlation, and adapter-contract model against the five design decisions above.

The rejected alternatives were:

1. **Project/cwd/name in the identity tuple (A2)** — rejected because it breaks the stable-identity guarantee when any of those fields changes (a cwd change would look like a new session, breaking late-reply correlation) and conflates routing authority with display metadata, inverting the rule that human-readable labels cannot override verified target identity.
2. **Reuse-allowed without tombstone (B3)** — rejected because it collides with at least four already-committed goals: wrong-session prevention, audit integrity, the ratified terminal-commit rule, and snapshot reconciliation. Without tombstoned generations, late events would be dropped silently (violating audit) or could be attributed to the wrong generation (violating wrong-session prevention). The tombstone is the mechanism that makes "late events are audit-only" enforceable.
3. **Core-assigned session generation (B2/B4)** — rejected because the core cannot observe external session replacement; it would introduce a window where the adapter knows a session is replaced but the core hasn't yet assigned a new generation, during which late replies for the old generation would be accepted for mutation.
4. **Shared id space (C2/C3)** — rejected because it creates forgery vectors (a reply could masquerade as a command) and clashes with LSN-bound event ids.
5. **Implicit adapter presence (D2)** — rejected because it provides no clean attach/detach/fail semantics, no place for audited capability changes, and no adapter-generation concept to reject stale-adapter events. Contradicts the degraded-behavior rules.
6. **mTLS-mandated trust root (D3)** — rejected because it over-specifies the mechanism and violates adapter-neutrality (excluding adapters that use configured-local material like Pi). The requirement is adapter-proves-identity; the mechanism is deferred.
7. **Boolean snapshot capability (E1b)** — rejected because it forces adapters with partial state into either lying (authoritative) or over-degrading (none), losing the honest middle ground the degraded-behavior rules depend on.
8. **Tiering all capabilities (E2b)** — rejected as speculative over-engineering; no core logic branches on streaming/cancellation tiers.

## Implementation Units

### Unit 1: Revise session identity and generation semantics in protocol prose

**File**: `docs/PROTOCOL.md` (Sessions section)

```text
SessionIdentity {
  adapter_id
  deployment_scope
  runtime_session_id   // adapter-reported, stable per generation
  session_generation   // adapter-reported, monotonic per session
}
// project/cwd/name are metadata, not identity

SessionGenerationBump(session_id, new_generation):
  if new_generation > current_live_generation(session_id):
    tombstone(current_live_generation) at next LSN
    set live generation = new_generation
  elif new_generation == current_live_generation:
    no-op (capability redeclaration may proceed)
  else:  // lower
    reject as stale_event audit; live generation unchanged
```

**Implementation Notes**:
- Replace "adapter-specific generation or epoch" with "session generation (adapter-reported, monotonic per session)."
- Clarify that project/cwd/name are metadata, not identity; they update independently of the identity tuple.
- Add tombstone semantics: prior generation marked superseded, retained as audit; late events for tombstoned generations are `stale_event` records.
- Add monotonicity rule: strictly-greater required for supersession; equal = no-op; lower = rejected as audit.
- Add bounded-detail retention note: tombstone fact indefinite; per-generation detail reclaimable by log compaction.

**Acceptance Criteria**:
- [ ] Session identity tuple lists adapter id, deployment scope, runtime session id, session generation — and no project/cwd/name as identity.
- [ ] Project/cwd/name described as metadata.
- [ ] Tombstone and monotonicity rules present.

---

### Unit 2: Define correlation id spaces in protocol prose

**File**: `docs/PROTOCOL.md` (Messages, commands, and replies section)

```text
CommandId        // client-generated, identity
IdempotencyKey   // client-generated, dedup handle (separate from command id)
MessageId        // client-generated, identity
ReplyId          // adapter/core-generated, identity
CorrelationRef   // typed reference: { kind: command|message, id: CommandId|MessageId }
EventId = LSN    // core-assigned
```

**Implementation Notes**:
- Clarify four separate id spaces with assigners: command/message = client; reply = adapter/core with typed correlation; event = core/LSN.
- Clarify command id and idempotency key are separate fields; retry reuses both, intentional duplicate uses new command id + new idempotency key.
- State the forgery-prevention property: a reply correlates by typed reference, so it cannot masquerade as a command.

**Acceptance Criteria**:
- [ ] Four id spaces defined with assigners.
- [ ] Typed correlation reference specified.
- [ ] Command id / idempotency key separation stated.

---

### Unit 3: Add adapter registration lifecycle to architecture and protocol

**File**: `docs/ARCHITECTURE.md` (Adapter plane / V0 component slice) and `docs/PROTOCOL.md` (Adapter capabilities section)

```text
AdapterRegistration {
  adapter_id
  attachment_evidence   // trust-root-verified, adapter-specific
  capability_manifest
  attach_lsn
  adapter_generation    // adapter-reported, monotonic per adapter
}
// attach/detach/failure/redeclare are audit events
// capability redeclaration allowed with audit; degraded sessions on loss
```

**Implementation Notes**:
- In ARCHITECTURE.md, describe adapter registration: attach (identity + manifest), detach (sessions marked stale/offline), failure (detected via timeout, degraded honestly), capability redeclaration (audited).
- In PROTOCOL.md, add adapter-as-principal language and the registration lifecycle.
- Keep trust-root mechanism adapter-specific (not mandated).
- Add adapter generation concept for rejecting stale-adapter events.

**Acceptance Criteria**:
- [ ] `docs/ARCHITECTURE.md` describes adapter registration and lifecycle.
- [ ] `docs/PROTOCOL.md` treats the adapter as a principal with a generation.
- [ ] Trust-root mechanism left adapter-specific.

---

### Unit 4: Ratify capability manifest shape and snapshot tiers

**File**: `docs/PROTOCOL.md` (Adapter capabilities / Adapter snapshot capability tiers sections)

```text
CapabilityManifest {
  command_kinds: [CommandKind]
  streaming: bool
  snapshot: authoritative | partial | none
  cancellation: bool
  session_replacement: bool
  idempotency_strength: none | at_patchbay_boundary | end_to_end
  attachment_method: descriptor   // adapter-specific
  known_failure_modes: [FailureTerm]
}
```

**Implementation Notes**:
- Remove the "Under design review" note from the Adapter snapshot capability tiers section.
- Ratify the three-tier snapshot model as committed v0 behavior.
- Shape each capability per E2a: snapshot=3-tier, idempotency=enum, streaming/cancellation/replacement=boolean, attachment=descriptor, failure modes=advisory list.
- Preserve the degraded-behavior rules unchanged.

**Acceptance Criteria**:
- [ ] Snapshot tier model ratified; under-design-review marker removed.
- [ ] Capability manifest shape documented per E2a.
- [ ] Degraded-behavior rules intact.

---

### Unit 5: Align verification obligations

**File**: `docs/VERIFICATION.md` (Wrong-session prevention + Reply correlation sections)

```text
Invariant LateGenerationInert:
  events/replies binding to a tombstoned session generation
  are stale_event audit records; they do not mutate the live generation

Invariant GenerationMonotonic:
  session supersession requires strictly-greater generation;
  lower reports are rejected; equal is a no-op

Invariant TypedCorrelation:
  a reply correlates by typed reference to a known command/message id;
  it cannot forge correlation across id spaces or session/authority contexts
```

**Implementation Notes**:
- Add generation-tombstone and monotonicity properties under wrong-session prevention.
- Add typed-correlation forgery-prevention property under reply correlation.
- Confirm normative variables cover `SessionGeneration`, `AdapterGeneration`, `CorrelationRef`.
- Ensure any "generation" normative variable is qualified by scope (core / session / adapter).

**Acceptance Criteria**:
- [ ] Tombstone/monotonicity/typed-correlation properties present.
- [ ] Normative variables include generation terms, scope-qualified.

---

### Unit 6: Ensure consistent qualified-generation terminology and glossary entry

**File**: `docs/GLOSSARY.md` (and spot-check `docs/PROTOCOL.md`, `docs/VERIFICATION.md`, `docs/SECURITY.md`, `docs/ARCHITECTURE.md`)

**Implementation Notes**:
- Keep "generation" as the base term (no cross-doc rename — it is already deployed and not wrong).
- Ensure every use of "generation" is qualified by scope: **core generation**, **session generation**, or **adapter generation**. Remove any bare unqualified "generation" that could read as ambiguous across scopes.
- Add a glossary entry for **generation** that foregrounds the assigner per scope: core generation (core-assigned on restart), session generation (adapter-reported on replacement), adapter generation (adapter-reported on re-attach). The qualifier + assigner is the collision-protection discipline.
- Confirm the existing snapshot-domain-rejection wording ("different core generation is rejected outright") is consistent with the qualified terminology.

**Acceptance Criteria**:
- [ ] No bare unqualified "generation" remains in normative contexts.
- [ ] Glossary defines generation with the three scopes and their assigners.

## Implementation Order

1. Ensure consistent qualified-generation terminology and glossary entry (Unit 6 first, so the substantive edits use the settled terminology).
2. Revise session identity and generation semantics in PROTOCOL (Unit 1).
3. Define correlation id spaces in PROTOCOL (Unit 2).
4. Add adapter registration lifecycle to ARCHITECTURE and PROTOCOL (Unit 3).
5. Ratify capability manifest and snapshot tiers in PROTOCOL (Unit 4).
6. Align verification obligations (Unit 5).

No child stories are spawned. This is a single-stride documentation/verification design with tight cross-doc cohesion; stories would add overhead rather than useful parallelism.

## Testing

There is no implementation code yet. Verification for this design is by document consistency:

- confirm `docs/PROTOCOL.md`, `docs/ARCHITECTURE.md`, `docs/VERIFICATION.md`, `docs/SECURITY.md`, and `docs/GLOSSARY.md` use "generation" consistently and define the three scopes;
- confirm session identity tuple excludes project/cwd/name as identity;
- confirm tombstone and monotonicity rules are present;
- confirm four id spaces are defined with assigners and typed correlation;
- confirm the adapter-registration lifecycle is described;
- confirm the snapshot-tier model is ratified (marker removed) and capability shapes match E2a;
- confirm no "generation" term remains unqualified or in a live field name.

## Risks

- **Adapter-observed replacement accuracy**: B1 depends on the adapter reporting generation bumps accurately. A buggy adapter that never bumps would leave the core thinking an old generation is live, degrading to stale/unknown presentation honestly (not wrong-target mutation). Acceptable degradation.
- **Tombstone storage growth**: Mitigated by 1c split retention — tombstone fact is cheap and indefinite; detail is bounded and compacted.
- **Three-generation naming collision**: Mitigated by γ — one term ("generation") with scope qualifiers and glossary definitions foregrounding the assigner. The risk is readers conflating scopes; the glossary is the collision-detection surface.
- **Adapter trust-root mechanism variability**: D1 defers the mechanism to adapter-specific design. If a future adapter cannot provide attachment evidence, it cannot register — which is the correct fail-closed behavior, not a design flaw.
- **Correlation id-space complexity**: Four spaces is more than one, but each has a clear assigner and the typed-correlation reference structurally prevents forgery. The complexity is justified by the verification properties.

## Implementation notes

- Files changed: `docs/PROTOCOL.md`, `docs/ARCHITECTURE.md`, `docs/VERIFICATION.md`, `docs/GLOSSARY.md`, `.work/active/features/feature-session-identity-adapter-contract.md`. (`docs/SECURITY.md` was reviewed for consistency and needed no change — its existing target-session-and-generation and audit-record wording already align.)
- Tests added: none; this is foundation-doc implementation.
- Discrepancies from design: none. The design's Unit 6 "rename generation → incarnation" was walked back per operator direction before implementation; "generation" is retained as the base term, scope-qualified, with an assigner-foregrounded glossary entry. Unit 6 became a consistency/glossary pass instead of a rename.
- Adjacent issues parked: none.
- Verification: confirmed via `rg` that session identity excludes project/cwd/name; tombstone + monotonicity rules are present; four id spaces with assigners and typed correlation are defined; adapter registration lifecycle is described in both ARCHITECTURE and PROTOCOL; snapshot-tier marker is removed and capability shapes match E2a; the existing `Core generation` glossary entry now points to a unified `Generation` entry foregrounding the assigner per scope; `Adapter capability` and `Correlation context` glossary entries were added.

## Review (2026-06-29)

**Verdict**: Approve with comments

**Blockers**: none
**Important**: none
**Nits** (applied in stride):
- `docs/PROTOCOL.md` "at a generation the core can reconcile" tightened to "at a session generation" to preserve scope-qualified discipline.
- Feature acceptance checkbox grammar "with an generation" fixed to "with a generation".

**Notes**: Deep substrate feature review performed by one fresh-context cross-model reviewer on `openai-codex/gpt-5.5` per operator request (implementor was GLM 5.2). Reviewer confirmed all five design decisions are faithfully encoded across PROTOCOL/ARCHITECTURE/VERIFICATION/GLOSSARY (and SECURITY consistent without change), the snapshot-tier under-design-review marker is removed while the four `story-review-provisional-semantics` markers remain intact, session identity excludes project/cwd/name, the four id spaces are separate with typed correlation, the adapter registration lifecycle is present with adapter-specific (not mTLS-mandated) trust root, and the glossary keeps "generation" as the protocol term with assigner-foregrounded scope qualifications. Both nits were applied per the nit-triage convention; no follow-up items filed.

## Related parked ideas

- `idea-multi-human-coordination` — v0 remains single-operator unless this feature decides otherwise, but the foundation should not foreclose future multi-human authority domains, grants, audit, handoffs, or third-party coordination surfaces.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.
