---
id: feature-pi-parity-checklist
kind: feature
stage: implementing
tags: [prose, adapter, foundation]
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

## Outline (prose-author, 2026-07-06)

**Routing note.** This is a `[prose]` deliverable. Black-box test applied: every semantic model the checklist depends on is already settled in `done` dependencies — the `OperationKind` registry and per-action Pi→OperationKind mapping (`feature-operator-presence-and-action-inventory`), the adapter capability manifest shape and session identity tuple (`feature-session-identity-adapter-contract`), the v0 scope (`feature-v0-walking-skeleton`), and the Pi action surface (`.research/attestation/pi-extension.md`). The checklist consumes those registries; it pins no new semantic model, exposes no code surface, and makes no architectural choice between approaches. No design pass, pre-mortem, or alternatives evaluation is needed. Large enough to use the staged draft→write→revise rhythm, so this outline is the structural draft; `implement` writes the doc; `review` is a coherence pass.

**Target path.** New dedicated adapter doc `docs/ADAPTER-PI.md`. Rationale: the adapter-neutrality principle (repeated across `docs/VISION.md`, `docs/SPEC.md`, `docs/ARCHITECTURE.md`, `docs/PROTOCOL.md`) argues for keeping Pi-specific capability detail out of the core ontology docs; `docs/ARCHITECTURE.md`'s existing "Pi-first migration path" section stays as the high-level pointer and gains a one-line forward-reference to the new doc. This is an adapter-specific reference doc, not a core foundation doc, so it is **not** added to the `AGENTS.md` orientation list.

### Sections

1. **Purpose and scope** — what this doc is (the Pi adapter v0 parity checklist and migration floor), what it is not (not a core protocol doc; does not make Pi the ontology), and its relationship to `docs/ARCHITECTURE.md` "Pi-first migration path" (pointer) and `docs/PROTOCOL.md` (canonical registries it consumes).

2. **Current Remote Pi workflow inventory** — the migration *from* state, grounded in `.research/attestation/pi-extension.md`. Lists the operator's current Remote Pi inbound actions (`session_sync`, `ping`, `user_message`, `approve_tool`, `cancel`, `model_set`, `thinking_set`, `list_models`, `session_new`, `session_compact`) and the agent→operator outbound event hooks (`turn_start`/`turn_end`, `message_update`/`message_end`, `tool_call`, `tool_execution_start`/`tool_execution_end`, `session_before_compact`/`session_compact`, `agent_end`, `input`, `resources_discover`), plus the out-of-band `pi-supervisord` provisioning.

3. **Pi session metadata → Patchbay session identity mapping** — field-by-field table over the settled identity tuple `(adapter_id, deployment_scope, runtime_session_id, session_generation)` from `feature-session-identity-adapter-contract`. Establishes that Pi's `project`/`cwd`/`name` are **metadata**, not identity; Pi's runtime session id → `runtime_session_id`; Pi session replacement → `session_generation` bump. Cites the `LabelsCannotOverrideIdentity` and `GenerationMonotonic` checked properties.

4. **Required Pi adapter capabilities for v0** — the core of the checklist. A table mapping each committed v0 `OperationKind` to the Pi wire action(s) that satisfy it and the capability-manifest declaration the Pi adapter must make (per the manifest shape in `docs/PROTOCOL.md` Adapter capabilities). Derived from the per-action Pi evidence in `feature-operator-presence-and-action-inventory`. Columns: `OperationKind` → Pi wire action → manifest capability → v0 disposition.

5. **Discovery, send, stream, reconnect, and status parity** — the specific surface the brief calls out: discover/attach (`session_sync`/`pair_request` → `attach`/`query`), send prompt (`user_message` → `instruct`), stream/read replies (Pi event hooks → Observations), reconnect recovery (cursor + snapshot; Pi's snapshot tier from `session_sync`), and working/idle/stale/offline status (Pi `turn_start`/`turn_end` → `SessionActivityState`; connectivity → `SessionConnectivityState`). Notes that Pi's snapshot tier is an **adapter-declared** capability — `session_sync` evidence suggests at least `partial`; the checklist does not pin the tier in a foundation doc, it requires the adapter to declare it.

6. **Commands as adapter-declared capabilities, not core states** — explicit restatement: `cancel`, `session_compact`, `session_new`/resume are adapter capabilities over committed OperationKinds (`cancel`/`interrupt`, `session-management`), not core protocol states. Cites the capability-not-authority and capability-not-delivery-gate rules. Maps Pi's `session_new` carefully (it resets the attached session's conversation via `ctx.newSession()`, it does **not** spawn a new process) so it is not confused with `spawn`.

7. **Unsupported or deferred Remote Pi features** — committed/reserved/rejected classification per Pi feature: `pi-supervisord` provisioning = reserved/adapter-external (out-of-band sysadmin, not an operator Operation in v0); `pair_request`/`queued_message_set`/`queued_message_clear` = transport/pairing, not agent-control Operations; agent→operator free-form question/Elicitation beyond the `tool_call` approval gate = reserved (Pi has no distinct free-form question wire type in the surveyed surface). Each row tagged committed-v0 / reserved-seam / rejected.

8. **Migration-decision criteria** — satisfies the acceptance criterion "sufficient to decide when the operator can switch workflows." A runnable checklist: the operator can switch from Remote Pi to Patchbay when (a) every committed-v0 Pi capability in section 4 is implemented by the Pi adapter, (b) the session identity mapping in section 3 is verified, (c) reconnect/snapshot parity in section 5 holds, (d) deferred features in section 7 are consciously accepted as gaps, and (e) the UX acceptance criteria in `feature-ux-v0-acceptance` are met.

9. **Extension pressure classification** — local committed-v0 / reserved-seam / rejected classification consistent with `feature-extension-seams-non-foreclosure`'s discipline and its ordering note that local per-feature classification suffices until the central sweep runs. Notes that the central extension-seams sweep will consolidate this into the project-wide registry when it executes.

### Acceptance criteria mapping

- "Add a Pi parity checklist to `docs/SPEC.md`, `docs/ARCHITECTURE.md`, or a dedicated adapter doc" → new `docs/ADAPTER-PI.md` (sections 1–9) plus a forward-reference in `docs/ARCHITECTURE.md`.
- "The checklist is sufficient to decide when the operator can switch workflows" → section 8.
- "Pi-specific operations are represented as adapter capabilities, not core protocol states" → sections 4 and 6.
- Extension pressure test → section 9.

### Implementation notes

- Implement stride: write `docs/ADAPTER-PI.md` per this outline; add a one-line forward-reference from `docs/ARCHITECTURE.md` "Pi-first migration path" to the new doc. Do **not** edit core registries in `docs/PROTOCOL.md` or `docs/SPEC.md` — the checklist only consumes them.
- No child stories — one inline authoring stride covers it.
- No formal-model or conformance-vector changes — this is adapter documentation, not protocol semantics.
- No `AGENTS.md` orientation-list change — the new doc is adapter-specific, not a core foundation doc.
