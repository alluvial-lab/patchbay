---
id: feature-pi-parity-checklist
kind: feature
stage: implementing
tags: [adapter, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-session-identity-adapter-contract, feature-operator-presence-and-action-inventory]
created: 2026-06-28
updated: 2026-07-06
gate_origin: null
release_binding: null
---

# Feature: Define Pi migration and parity checklist

Pi is the first adapter because it lets the operator migrate from current Remote Pi-style workflows, but parity is not yet itemized. Define the migration floor without making Pi the core ontology.

## Scope

- Current Remote Pi workflow inventory.
- Required Pi adapter capabilities for v0.
- Session discovery, send prompt, stream/read replies, reconnect recovery, working/idle/stale/offline status.
- Commands such as cancel, compact, new/resume only as adapter-declared capabilities.
- Unsupported or deferred Remote Pi features.
- Mapping from Pi session metadata to Patchbay session identity.

## Acceptance criteria

- Add a Pi parity checklist to `docs/SPEC.md`, `docs/ARCHITECTURE.md`, or a dedicated adapter doc.
- The checklist is sufficient to decide when the operator can switch workflows.
- Pi-specific operations are represented as adapter capabilities, not core protocol states.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.

## Misroute note (2026-07-06)

Misrouted to prose-author; the work has a real design surface — retagged for feature-design. The `prose` tag was removed.

The prose-author black-box test was applied and initially passed (the `OperationKind` registry, adapter capability manifest shape, session identity tuple, and Pi action surface are all settled in `done` dependencies). But on second application the test fails: the scope line "Mapping from Pi session metadata to Patchbay session identity" contains a genuine semantic classification that cannot be made silently through prose, with real verification consequences.

The load-bearing design questions a `feature-design` pass must resolve:

1. **`session_new` classification.** Pi's `session_new` resets the attached session's conversation via `ctx.newSession()` *without spawning a new process* (grounded in `.research/attestation/pi-extension.md`). Is that a session *replacement* that bumps `session_generation` and tombstones the prior generation (triggering `GenerationMonotonic` and `LateGenerationInert`), or a same-generation `session-management` clear? The choice has durable correlation and audit consequences and cannot be made inside a prose checklist.
2. **Pi snapshot tier.** The adapter must declare a snapshot tier (`authoritative` / `partial` / `none` per `docs/PROTOCOL.md`). Pi's `session_sync` evidence suggests at least `partial`, but the tier drives the core's reconnect reconciliation contract; the checklist must not pin it in a foundation doc — the design pass must decide how the Pi adapter declares and the checklist records it.
3. **Provisioning seam.** `pi-supervisord` is out-of-band sysadmin (not an operator Operation in v0). The design must classify this as reserved/adapter-external without foreclosing a future supervisor OperationKind, and say so explicitly rather than as a prose aside.
4. **Pairing/queue messages.** `pair_request`, `queued_message_set`, `queued_message_clear` are transport/pairing, not agent-control Operations. The design must decide whether they map to the reserved Subscription/transport seam or are purely out-of-adapter-scope.

These are choosing-between-approaches / semantic-model-pinning commitments, not collapsed prose authoring. This is the same misroute pattern that hit `feature-session-identity-adapter-contract` (retagged `[prose]` → design on 2026-06-28) and that prompted the project-wide 2026-07-06 codification of the prose black-box test. Route to `feature-design`; do not advance the stage.

## Design decisions (feature-design, 2026-07-06)

Resolved interactively against the remote_pi source (`/home/agent/projects/remote_pi/pi-extension/`) rather than the attestation summary alone. The attestation summarized the Pi action surface; the source surfaced that the `session_new` classification was the load-bearing question and inverted the initial `/clear` intuition.

- **Q1 — Pi `session_new` classification -> B (session replacement, generation bump + tombstone).** Pi's `session_new` is **not** a `/clear`. remote_pi's own code groups it with `fork`/`switch`/`reload` as "session replacement" (`index.ts:1148` "session replacement (newSession/fork/switch/reload)"; `index.ts:2232` "newSession marks every pre-replacement SDK context stale"; `peer_channel.ts:80` "session_new/session-replacement teardown"; `handlers.ts:209` `handleSessionNew`). The SDK tears down the old `ExtensionContext` and marks it stale — any later use throws "stale after session replacement or reload" via `assertActive()`. The `story-session-replacement-harness` work (done 2026-07-05) treats `/new`, `/resume`, `/fork` as one replacement bug-class and proves old-context-stale is the safety property. A `/clear` on other harnesses preserves the session handle and wipes the transcript in place; Pi does the opposite (transcript event log rotated, old context permanently unusable). That is the session replacement that `session_generation.qnt` models with a generation bump + tombstone, so late events/replies binding to the pre-`new` context become `stale_event` audit records (`LateGenerationInert`) rather than polluting the new conversation. The operator's `/clear` intuition was the dangerous one; flagged and corrected.
  - **Derived mapping:** the Pi adapter reports a **stable `runtime_session_id` for the daemon/agent slot** (the registered `remote-pi` daemon identity; `rpc_child.ts:178` notes `--name` pins the session's display name to the daemon's identity and that a `--continue` restart "REUSES the one session") and **bumps `session_generation` on `session_new`**. The Pi SDK's internal session id changes on `newSession` (`issuer.capture` of the fresh ctx), but that is an adapter-internal detail mapped to a generation bump, not exposed as a new `runtime_session_id`. A supervisor restart **with** `--continue` is *not* a replacement (same runtime_session_id, same session_generation; it is an adapter reconnect, bumping adapter_generation at most). A restart **without** `--continue` (fresh session, the `EXIT_DAEMON_FRESH_SESSION` path at `index.ts:2216`) *is* a session replacement -> session_generation bump.
  - **`session_compact` does not bump generation.** Compaction is in-place (summarizes history, preserves the session); `session_before_compact`/`session_compact` events, `handleSessionCompact`. It maps to `session-management` with no generation consequence.
  - **`/fork` is out of v0 Pi adapter surface.** remote_pi groups fork with the replacement bug-class internally, but fork is not an operator wire action in the surveyed inbound `CLIENT_MESSAGE_TYPES`; it is SDK-internal. v0 checklist covers `session_new` and `session_compact` only.
- **Q2 — `spawn` in v0 -> A (not implemented; out-of-band provisioning; reserved seam).** remote_pi has no operator `spawn` action — its inbound actions are the 10 already inventoried; none provisions a runtime. `pi-supervisord` (`pi-extension/src/bin/supervisord.ts`) is the supervisor, and the setup wizard explicitly excludes it from the operator surface ("Daemon mode ... is intentionally NOT in the wizard — it's an explicit, separate opt-in via `/remote-pi install`"). Provisioning is out-of-band sysadmin. The Pi adapter declares `spawn` unsupported at delivery (`unsupported_command`); the operator provisions runtimes out-of-band and Patchbay `attach`es. `spawn` stays committed in the `OperationKind` registry; a follow-on feature may promote supervisord-control spawn (start/stop/restart a registered daemon — small surface) once the supervisor RPC is stable. Full cold-start of arbitrary `target_spec.shape` variants is **not** v0 and remains a reserved seam.
- **Q3 — Pi snapshot tier -> A (`partial`; truthful declaration required).** remote_pi does not provide an authoritative snapshot. It has a **transcript event log** replayed via `session_sync` -> `session_history` (`sdk_session_projection.ts` `buildSessionHistoryMessage` / `resetSessionForNew`), which gives recent/current state, not arbitrary historical reconstruction -> at least `partial`, not `authoritative`. The checklist records `partial` and requires reconnect parity to be verified against that tier; the core's degraded-behavior rules handle the rest honestly. Live evidence that `authoritative` would be an overclaim: the in-flight `story-mobile-cross-session-history-leak` (remote_pi, drafting 2026-07-06) shows the phone rendering transcript content from 3-7 distinct Pi sessions because the session gate accepts `session_history` frames for non-active sessions — i.e. remote_pi's snapshot/replay path is demonstrably buggy under session replacement. (Per operator direction 2026-07-06: this bug is **not** bound to the feature as a `research_refs:` cross-reference now; a harvest pass on remote_pi real-life behavior will fold debug evidence in once the bugs close. Noted here only as the evidence that drove the tier choice.)
- **Q4 — checklist location -> A (dedicated `docs/ADAPTER-PI.md` + forward-reference).** Keeps Pi-specific capability detail out of core ontology docs (adapter-neutrality, repeated across VISION/SPEC/ARCHITECTURE/PROTOCOL). `docs/ARCHITECTURE.md` "Pi-first migration path" gains a one-line forward-reference. The new doc is adapter-specific, not a core foundation doc, so it is **not** added to the `AGENTS.md` orientation list.

## Architectural choice

The deliverable is a dedicated adapter reference document that **consumes** the settled registries (`OperationKind` registry, adapter capability manifest shape, session identity tuple, v0 scope) and maps Pi's grounded action surface (`.research/attestation/pi-extension.md`, verified against remote_pi source) onto them. It pins no new core protocol semantics; the one semantic classification it needed (`session_new` = replacement) is resolved above and recorded as an adapter-level mapping decision, not a core-doc edit.

Approaches considered:

1. **Dedicated adapter doc consuming registries (chosen).** A new `docs/ADAPTER-PI.md` that references rather than duplicates the canonical registries in `docs/PROTOCOL.md`. Optimizes for adapter-neutrality (Pi detail stays out of core docs) and single-source-of-truth (the registry is authoritative; the checklist only maps). Sacrifices single-doc discoverability (a reader must know `docs/ADAPTER-PI.md` exists) — mitigated by the `docs/ARCHITECTURE.md` forward-reference.
2. **Inline sections in `docs/SPEC.md` / `docs/ARCHITECTURE.md` (rejected).** Keeps everything in core docs but grows them with adapter-specific capability detail and Pi wire-action tables, eroding the adapter-neutrality discipline those docs assert. This is the same pressure the design principles guard against.
3. **A generated artifact from `.proto` (rejected).** The checklist is product/migration prose (a human-readable parity floor with migration-decision criteria), not a wire shape. `.proto` is authority for wire shape and enum encoding only; the checklist's content (migration-decision criteria, deferred-feature classification) is not derivable from the schema.

## Implementation Units

The deliverable is one new doc plus a one-line forward-reference. Units are the doc's sections; each specifies exact content and acceptance criteria so the implement stride is mechanical.

### Unit 1: `docs/ADAPTER-PI.md` — Pi adapter v0 parity checklist

**File**: `docs/ADAPTER-PI.md` (new)

**Sections** (each must be present and internally consistent with the cited canonical source):

1. **Purpose and scope** — what this doc is (the Pi adapter v0 parity checklist and migration floor), what it is not (not a core protocol doc; does not make Pi the ontology), and its relationship to `docs/ARCHITECTURE.md` "Pi-first migration path" (pointer) and `docs/PROTOCOL.md` (canonical registries it consumes). State explicitly that Pi-specific operations are adapter capabilities, not core protocol states.
2. **Current Remote Pi workflow inventory** — the migration *from* state, grounded in `.research/attestation/pi-extension.md`. Two tables: (a) operator->agent inbound actions (`session_sync`, `ping`, `user_message`, `approve_tool`, `cancel`, `model_set`, `thinking_set`, `list_models`, `session_new`, `session_compact`) with one-line semantics each; (b) agent->operator outbound event hooks (`turn_start`/`turn_end`, `message_update`/`message_end`, `tool_call`, `tool_execution_start`/`tool_execution_end`, `session_before_compact`/`session_compact`, `agent_end`, `input`, `resources_discover`). Note `pi-supervisord` provisioning as out-of-band sysadmin.
3. **Pi session metadata -> Patchbay session identity mapping** — field-by-field table over the identity tuple `(adapter_id, deployment_scope, runtime_session_id, session_generation)` from `docs/PROTOCOL.md`. Rows: Pi daemon/agent slot -> `runtime_session_id` (stable across `session_new`, `--continue` restart, conversation resets); Pi SDK internal session id -> adapter-internal, not exposed; Pi `project`/`cwd`/`name` -> **metadata, not identity**; `session_new` / fresh-session restart -> `session_generation` bump + tombstone; `session_compact` -> no generation change; `--continue` restart -> adapter reconnect (adapter_generation at most, no session_generation bump). Cite `LabelsCannotOverrideIdentity`, `GenerationMonotonic`, `LateGenerationInert` as the checked properties this mapping must satisfy.
4. **Required Pi adapter capabilities for v0** — the core checklist. Table mapping each committed v0 `OperationKind` to the Pi wire action(s) that satisfy it and the capability-manifest declaration (per the manifest shape in `docs/PROTOCOL.md` Adapter capabilities). Columns: `OperationKind` | Pi wire action(s) | manifest capability declaration | v0 disposition. Rows derived from the per-action Pi evidence in `feature-operator-presence-and-action-inventory`:
   - `attach` <- `session_sync`/`pair_request` | streaming=bool, snapshot=`partial`, cancellation=bool, session_replacement=bool | committed
   - `instruct` <- `user_message` | (delivery) | committed
   - `cancel`/`interrupt` <- `cancel` | cancellation=bool | committed (interrupt aliased or unsupported-by-adapter)
   - `approval-response` <- `approve_tool` | (approval Elicitation opened via `tool_call`) | committed
   - `query` <- `session_sync`/`list_models`/`ping` | (read lifecycle) | committed
   - `reconfigure` <- `model_set`/`thinking_set` | (payload schema) | committed
   - `session-management` <- `session_new`/`session_compact` | session_replacement=bool (true: `session_new` bumps generation) | committed
   - `spawn` <- none | declared unsupported at delivery (`unsupported_command`) | committed kind, Pi-adapter-unsupported in v0 (reserved seam)
   - `elicitation-response` <- (no distinct Pi wire type; the `tool_call` gate is the closest) | committed OperationKind; `question` contract reserved pending promotion | committed kind, Pi question-Elicitation surface reserved
   - Reserved `agent-send`/`adapter-utility-exec` <- n/a | rejected with `validation_failed` in v0 | reserved
   Include the manifest snapshot-tier declaration: Pi declares `partial`.
5. **Discovery, send, stream, reconnect, and status parity** — the specific surface the brief calls out: discover/attach, send prompt, stream/read replies, reconnect recovery, working/idle/stale/offline status. Map Pi event hooks -> `Observation`s; `turn_start`/`turn_end` -> `SessionActivityState`; connectivity -> `SessionConnectivityState`; reconnect = cursor + `partial` snapshot reconciliation (the transcript event log replay). State that the snapshot tier is adapter-declared (`partial`) and the core reconciles per the degraded-behavior rules; the checklist does not pin the tier in a foundation doc, it records the adapter's declaration.
6. **Commands as adapter-declared capabilities, not core states** — explicit restatement of the capability-not-authority and capability-not-delivery-gate rules from `docs/PROTOCOL.md`. Map `session_new`/`session_compact` as `session-management` Operations whose adapter-side effect (generation bump for `session_new`; in-place summarize for `session_compact`) is adapter-reported, not a core protocol state. Call out the `session_new` != `spawn` distinction (resets the conversation on the same daemon slot; does not provision a new runtime).
7. **Unsupported or deferred Remote Pi features** — committed/reserved/rejected classification per Pi feature: `pi-supervisord` provisioning = reserved/adapter-external (out-of-band sysadmin, not an operator Operation in v0; a follow-on feature may promote supervisord-control spawn); `pair_request`/`queued_message_set`/`queued_message_clear` = transport/pairing, out of adapter Operation scope (web/transport layer); agent->operator free-form question Elicitation beyond the `tool_call` approval gate = reserved (Pi has no distinct free-form question wire type in the surveyed surface); `/fork` = SDK-internal, out of v0 Pi adapter surface. Each row tagged committed-v0 / reserved-seam / rejected.
8. **Migration-decision criteria** — satisfies the acceptance criterion "sufficient to decide when the operator can switch workflows." A runnable checklist: the operator can switch from Remote Pi to Patchbay when (a) every committed-v0 Pi capability in section 4 is implemented by the Pi adapter, (b) the session identity mapping in section 3 is verified (incl. `session_new` generation bump), (c) reconnect/snapshot parity in section 5 holds against the `partial` tier, (d) deferred features in section 7 are consciously accepted as gaps, and (e) the UX acceptance criteria in `feature-ux-v0-acceptance` are met.
9. **Extension pressure classification** — local committed-v0 / reserved-seam / rejected classification consistent with `feature-extension-seams-non-foreclosure`'s discipline and its ordering note that local per-feature classification suffices until the central sweep runs. Note that the central extension-seams sweep will consolidate this into the project-wide registry when it executes. List: committed (the OperationKind mappings in section 4; `partial` snapshot tier; `session_new` generation-bump mapping); reserved (supervisord-control spawn; free-form question Elicitation; `/fork`; tighter responder binding); rejected (Pi-specific state names in core; treating `session_new` as a same-generation clear; treating `session_new` as `spawn`).

**Implementation Notes**:
- Consume, do not duplicate: every registry value (`OperationKind`, capability manifest shape, identity tuple, snapshot tiers) must reference `docs/PROTOCOL.md` as authoritative, not re-declare it. If a value diverges, that is a bug in the canonical doc, not a license for the checklist to invent a parallel registry.
- Cite `.research/attestation/pi-extension.md` as the grounding source for the Pi action surface; do not re-derive Pi wire types from memory.
- Current-state oriented prose. No history narratives.
- The `session_new` generation-bump mapping (Unit 3 + Unit 4 + Unit 6) is the one non-obvious, verification-consequential claim — state it precisely and consistently in all three sections.

**Acceptance Criteria**:
- [ ] All nine sections present and internally consistent.
- [ ] Section 4 table covers all 10 committed `OperationKind`s plus the 2 reserved kinds, each with a Pi wire action or explicit unsupported/reserved marker.
- [ ] Section 3 identity mapping excludes project/cwd/name from identity and states the `session_new` generation-bump rule.
- [ ] Section 8 gives a runnable switch-decision checklist.
- [ ] No core registry value is re-declared; all reference `docs/PROTOCOL.md`.
- [ ] Snapshot tier recorded as `partial`, not pinned in a foundation doc.

### Unit 2: `docs/ARCHITECTURE.md` — forward-reference

**File**: `docs/ARCHITECTURE.md` (existing, "Pi-first migration path" section)

**Change**: append one sentence to the "Pi-first migration path" paragraph pointing to the new doc:

> The v0 Pi adapter parity checklist, capability mapping, and migration-decision criteria live in `docs/ADAPTER-PI.md`.

**Acceptance Criteria**:
- [ ] Forward-reference present; no other ARCHITECTURE.md content changed (the high-level positioning stays here; detail lives in the adapter doc).

## Implementation Order

1. Write `docs/ADAPTER-PI.md` (Unit 1, all nine sections).
2. Add the forward-reference to `docs/ARCHITECTURE.md` (Unit 2).

Single inline implement stride — no child stories. The deliverable is one cohesive doc with tight cross-section cohesion (the `session_new` mapping must be consistent across sections 3/4/6); splitting would add overhead, not parallelism. No code, no build, no coordination -> inline, not the orchestrator.

## Testing

No implementation code; verification is by document consistency, mirroring the sibling foundation features:

- confirm every `OperationKind` in the section 4 table matches the registry in `docs/PROTOCOL.md` (and the generated enum in `contracts/rust/src/gen/patchbay/patchbay.rs`);
- confirm the identity tuple in section 3 matches `docs/PROTOCOL.md` Sessions and the `session_generation.qnt` variables;
- confirm `session_new` is classified as a generation-bumping replacement consistently in sections 3, 4, and 6, and that `session_compact` is not;
- confirm the capability manifest fields in section 4 match the manifest shape in `docs/PROTOCOL.md` Adapter capabilities;
- confirm the snapshot tier is recorded as `partial` and not pinned in a foundation doc;
- confirm no canonical registry value is re-declared (only referenced);
- confirm the migration-decision criteria in section 8 are runnable and reference `feature-ux-v0-acceptance`.

## Risks

- **`session_new` mapping is adapter-level, not core-ratified.** The generation-bump mapping is a Pi-adapter decision grounded in remote_pi source, not a checked property of `session_generation.qnt` (which models generation abstractly). If a future adapter's "new session" semantics differ, the mapping is per-adapter — which is correct (adapter-neutrality), but the doc must frame it as the Pi adapter's mapping, not a core rule. Mitigation: section 3 states it as the Pi adapter's reported behavior.
- **Snapshot tier may change as bugs close.** The `partial` tier is grounded in current remote_pi behavior, which is buggy under session replacement (the cross-session leak). A harvest pass after bugs close may move the tier or sharpen reconnect parity. Mitigation: the doc records `partial` as the current declaration and notes that adapter snapshot behavior is verified against live behavior; a follow-on may revise. (Per operator direction: not bound as a `research_refs:` now.)
- **`/fork` and free-form Elicitation surfaces are thin.** Both are reserved on thin evidence (not operator wire actions in the surveyed surface). If later Pi versions expose them, the checklist needs a registry update. Mitigation: section 7 marks them reserved, not rejected, so promotion is a registry/classification update, not a reversal.
- **Checklist drift from canonical registries.** If `docs/PROTOCOL.md` registries change (e.g. a new committed `OperationKind`), the checklist must update. Mitigation: section 1 states the checklist consumes the registries and is authoritative only for the Pi mapping; the `feature-foundation-doc-completeness-gaps` / gate-docs discipline catches drift at release time.

## Reserved follow-up (not v0)

- Harvest pass on remote_pi real-life behavior: once the in-flight session-replacement / cross-session-leak / reconnect bugs close, fold the debug evidence into a revision of the snapshot-tier and reconnect-parity sections (and possibly promote `research_refs:` bindings). Not bound to this feature now per operator direction 2026-07-06.
- Supervisord-control `spawn` promotion: a follow-on feature may add a small supervisor-RPC-backed spawn capability to the Pi adapter. Out of v0 scope.
