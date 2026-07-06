# Pi Adapter — v0 Parity Checklist

## 1. Purpose and scope

This document is the v0 parity checklist and migration floor for the Pi adapter — the first Patchbay adapter, targeting migration from the operator's current Remote Pi workflow.

It defines:

- the current Remote Pi workflow being migrated *from*;
- the required Pi adapter capabilities for v0, mapped onto the canonical Patchbay `OperationKind` registry;
- the mapping from Pi session metadata to Patchbay session identity;
- the discovery, send, stream, reconnect, and status parity surface;
- unsupported or deferred Remote Pi features;
- runnable criteria for deciding when the operator can switch from Remote Pi to Patchbay.

This document is **not** a core protocol document. It does not make Pi concepts part of the Patchbay core ontology. Pi-specific operations are represented as **adapter capabilities**, not core protocol states. The canonical registries — `OperationKind`, the adapter capability manifest shape, session identity, snapshot tiers — live in `docs/PROTOCOL.md` and are authoritative there; this checklist consumes them and maps the Pi adapter's behavior onto them. Where this doc and `docs/PROTOCOL.md` appear to diverge, `docs/PROTOCOL.md` is correct and this doc has a bug.

The high-level Pi-first migration positioning lives in `docs/ARCHITECTURE.md` ("Pi-first migration path"); this doc holds the detail. This is an adapter-specific reference, not a core foundation document, and is intentionally not listed in `AGENTS.md`'s orientation set.

The Pi action surface documented here is grounded in `.research/attestation/pi-extension.md`, verified against the remote_pi source.

## 2. Current Remote Pi workflow inventory

The migration *from* state is the operator's current Remote Pi control surface. The inbound (operator→agent) and outbound (agent→operator) action surfaces are inventoried below; both are grounded in `.research/attestation/pi-extension.md`.

### Inbound — operator→agent control actions

| Pi wire action | Semantics |
|---|---|
| `session_sync` | Sync/refresh session state (status, recent transcript); the reconnect/snapshot read path. |
| `ping` | Liveness query. |
| `user_message` | Drive the session — send prompt/content that begins a turn. |
| `approve_tool` | Approve a pending tool call (the approval gate). |
| `cancel` | Interrupt a running turn. |
| `model_set` | Reconfigure — set the model. |
| `thinking_set` | Reconfigure — set the thinking level. |
| `list_models` | Query available models. |
| `session_new` | Reset the session's conversation (session replacement; see §3, §6). |
| `session_compact` | Compact the session (in-place summarization; no generation change). |

`pair_request`, `queued_message_set`, and `queued_message_clear` are transport/pairing primitives, not agent-control actions; see §7.

### Outbound — agent→operator event hooks

| Pi event hook | Semantics |
|---|---|
| `turn_start` / `turn_end` | Session working/idle transitions. |
| `message_update` / `message_end` | Streaming and final message content. |
| `tool_call` | Tool-call request; the approval surface. |
| `tool_execution_start` / `tool_execution_end` | Tool-call lifecycle. |
| `model_select` / `thinking_level_select` | Reconfiguration events (model/thinking-level changes). |
| `session_before_compact` / `session_compact` | Compaction lifecycle. |
| `agent_end` | Agent/turn completion. |
| `input` | Input elicitation. |
| `resources_discover` | Resource discovery. |

### Provisioning

`pi-supervisord` is a long-running daemon supervisor that spawns `pi --mode rpc` children, managed by systemd/launchd. Provisioning is **out-of-band sysadmin**: the supervisor and its service templates are installed and operated outside the operator control surface. The setup wizard explicitly excludes daemon mode from the operator surface. See §7 for the v0 classification.

## 3. Pi session metadata → Patchbay session identity mapping

Patchbay session identity is the tuple `(adapter_id, deployment_scope, runtime_session_id, session_generation)`, defined in `docs/PROTOCOL.md` (Sessions). The mapping below records how the Pi adapter reports each field. It is the **Pi adapter's reported behavior**, not a core rule; a future adapter with different "new session" semantics would report differently.

| Pi concept | Patchbay identity field | Mapping |
|---|---|---|
| Pi daemon/agent slot (the registered `remote-pi` daemon identity) | `runtime_session_id` | **Stable** across `session_new`, `--continue` restarts, and conversation resets. The daemon slot is the durable Patchbay target. |
| Pi SDK internal session id (changes on `newSession`) | *(adapter-internal, not exposed)* | The SDK's internal session id is an adapter implementation detail; it is mapped to a `session_generation` bump, not exposed as a new `runtime_session_id`. |
| `project`, `cwd`, `name` | **metadata, not identity** | These are display/routing metadata; they update independently of the identity tuple. A `cwd` change does not create a new session target. |
| `session_new` (conversation reset) | `session_generation` | **Bump + tombstone.** `session_new` is a session replacement (see §6): the prior generation is tombstoned and late events/replies binding to it become `stale_event` audit records. |
| Fresh-session restart (supervisor restart *without* `--continue`) | `session_generation` | **Bump + tombstone.** This is the `EXIT_DAEMON_FRESH_SESSION` path; it realizes a session replacement on the same daemon slot. |
| `session_compact` | *(no change)* | Compaction is in-place; it does not bump the generation. |
| `--continue` restart (supervisor restart *with* `--continue`) | *(no session_generation change)* | This is an adapter reconnect that reuses the one session; at most an `adapter_generation` bump, not a session replacement. |

This mapping must satisfy the checked properties documented in `docs/VERIFICATION.md` against `specs/seed/session_generation.qnt`:

- `LabelsCannotOverrideIdentity` — `project`/`cwd`/`name` cannot override the identity tuple.
- `GenerationMonotonic` — session supersession (`session_new` / fresh-session restart) requires a strictly-greater generation; a lower report is rejected; an equal report is a no-op.
- `LateGenerationInert` — events/replies binding to a tombstoned generation are `stale_event` audit records and do not mutate the live generation.

> **`session_new` is a session replacement, not a `/clear`.** remote_pi's own code groups `session_new` with `fork`/`switch`/`reload` as "session replacement" and marks the pre-replacement SDK context permanently stale. A `/clear` on other harnesses preserves the session handle and wipes the transcript in place; Pi does the opposite — the transcript event log is rotated and the old context becomes permanently unusable. This is why the mapping bumps `session_generation` rather than treating it as a same-generation clear.

## 4. Required Pi adapter capabilities for v0

This is the core of the checklist. It maps each committed v0 `OperationKind` (the registry is authoritative in `docs/PROTOCOL.md`, `### OperationKind registry`) to the Pi wire action(s) that satisfy it, and records the capability-manifest declarations the Pi adapter makes. Per-Operation delivery mapping and manifest-field declarations are kept in separate columns: manifest fields come from the manifest shape in `docs/PROTOCOL.md` (Adapter capabilities) and correspond to the generated `AdapterCapability` fields in `contracts/rust/src/gen/patchbay/patchbay.rs` — `supported_operation_kinds`, `supported_target_spec_shapes`, `streaming_support`, `snapshot_support`, `cancellation_support`, `session_replacement_support`, `idempotency_strength`, `attachment_method`, `known_failure_modes`. The names below use the manifest dimensions as prose shorthand (e.g. `streaming` for the `streaming_support` field); a delivery outcome such as `unsupported_command` is an adapter-reported delivery result, not a manifest field.

| `OperationKind` | Pi wire action(s) | Manifest declaration (actual fields) | v0 disposition |
|---|---|---|---|
| `attach` | `session_sync` | `supported_operation_kinds` includes `attach`; `streaming=true`; `snapshot=partial`; `cancellation=true`; `session_replacement=true` | committed v0 |
| `instruct` | `user_message` | `supported_operation_kinds` includes `instruct` | committed v0 |
| `cancel` / `interrupt` | `cancel` | `cancellation=true` (`interrupt` aliased to `cancel` or unsupported-by-adapter at delivery) | committed v0 |
| `approval-response` | `approve_tool` (approval Elicitation opened via the `tool_call` hook) | `supported_operation_kinds` includes `approval-response` | committed v0 |
| `query` | `session_sync` / `list_models` / `ping` | `supported_operation_kinds` includes `query` | committed v0 |
| `reconfigure` | `model_set` / `thinking_set` | `supported_operation_kinds` includes `reconfigure` | committed v0 |
| `session-management` | `session_new` / `session_compact` | `session_replacement=true` (`session_new` bumps `session_generation`; `session_compact` does not) | committed v0 |
| `spawn` | none | `supported_operation_kinds` excludes `spawn`; delivery of a `spawn` Operation returns `unsupported_command` | committed kind; Pi-adapter-unsupported in v0 (reserved seam) |
| `elicitation-response` | no distinct Pi non-approval question wire type; the `tool_call` approval gate is the closest | `supported_operation_kinds` includes `elicitation-response`; non-approval `question` Elicitations unsupported by the Pi adapter at delivery (`unsupported_command`) until a Pi question surface is promoted | committed core kind + committed `question` contract; Pi-adapter support for non-approval question Elicitations is a reserved adapter-level seam |
| `agent-send` *(reserved)* | n/a | excluded; submission rejects with `validation_failed` in v0 | reserved seam |
| `adapter-utility-exec` *(reserved)* | n/a | excluded; submission rejects with `validation_failed` in v0 | reserved seam |

**Snapshot-tier declaration:** the Pi adapter declares `snapshot = partial`. The transcript event log replayed via `session_sync` → `session_history` provides recent/current state, not arbitrary historical reconstruction. The core reconciles reconnects against this tier per the degraded-behavior rules in `docs/PROTOCOL.md` (Adapter snapshot capability tiers); it never fabricates a snapshot from cached state.

> `pair_request` is transport/pairing, **not** an `attach` wire action. It is classified in §7.

## 5. Discovery, send, stream, reconnect, and status parity

This section covers the specific parity surface the migration floor requires.

- **Discover / attach.** The operator discovers available Pi daemon slots and attaches via `session_sync`-backed reconciliation. `attach` establishes or refreshes endpoint availability and triggers cursor/snapshot reconciliation.
- **Send prompt.** `user_message` → `instruct` Operation carrying prompt payload. Slash-commands are payload, not separate protocol kinds.
- **Stream / read replies.** Pi event hooks (`message_update`/`message_end`, `tool_call`, `tool_execution_*`, `model_select`/`thinking_level_select`, `session_before_compact`/`session_compact`, `agent_end`, `turn_*`) map to `Observation`s — source-authenticated output, lifecycle facts, and status emissions (including reconfiguration-status facts). Observation streams are delivery optimizations; the durable core record and snapshots remain authoritative.
- **Reconnect recovery.** On reconnect the control surface submits its cursor and reconciles against the `partial` snapshot (transcript event log replay) and newer events. The snapshot tier is adapter-declared (`partial`); the core reconciles per the degraded-behavior rules and marks unreconciled axes `unknown` or `stale` rather than synthesizing live state.
- **Working / idle / stale / offline status.** `turn_start`/`turn_end` → `SessionActivityState`; endpoint connectivity → `SessionConnectivityState` (`live` / `stale` / `offline` / `unknown` / `failed`). Status is protocol-derived, not invented by the UI.

The snapshot tier is adapter-declared (`partial`) and recorded here as the Pi adapter's declaration. This checklist does not pin the tier in a foundation document; if the Pi adapter's live behavior changes (e.g. as bugs close), the declaration is revised here.

## 6. Commands as adapter-declared capabilities, not core states

Pi-specific operations are adapter capabilities over committed `OperationKind`s, not core protocol states. The capability-not-authority and capability-not-delivery-gate rules in `docs/PROTOCOL.md` (Adapter capabilities) apply: adapter capability declarations are advisory for control-surface UX only — they are not an authority gate and not a delivery gate. The core delivers the `OperationKind` to the adapter, and the adapter accepts or rejects based on its own support at delivery time.

`session_new` and `session_compact` are `session-management` Operations whose adapter-side effects are adapter-reported, not core protocol states:

- **`session_new`** — the adapter reports a `session_generation` bump; the core tombstones the prior generation and treats the new generation as the live target. The generation bump is the Pi adapter's reported behavior (see §3).
- **`session_compact`** — in-place summarization; no generation change.

> **`session_new` ≠ `spawn`.** `session_new` resets the conversation on the **same Patchbay runtime/daemon slot** (`runtime_session_id` is stable) and does not provision a new runtime. In daemon mode remote_pi may intentionally restart the Pi RPC child without `--continue` to obtain the fresh session (the `EXIT_DAEMON_FRESH_SESSION` path) — that process restart is the mechanism by which the same daemon slot realizes a session replacement, not a `spawn` of a new Patchbay target. `spawn` provisions a new runtime/session/daemon slot; `session_new` does not.

## 7. Unsupported or deferred Remote Pi features

| Remote Pi feature | Classification | Note |
|---|---|---|
| `pi-supervisord` provisioning | reserved / adapter-external | Out-of-band sysadmin, not an operator Operation in v0. A follow-on feature may promote supervisord-control `spawn` (start/stop/restart a registered daemon). |
| `pair_request` | transport/pairing | Out of adapter Operation scope (web/transport layer); not an `attach` wire action. |
| `queued_message_set` / `queued_message_clear` | transport/pairing | Out of adapter Operation scope (web/transport layer). Current operator-visible queued-message behavior being left outside adapter Operation scope; the switch-decision checklist (§8) requires an explicit accept-or-replace decision. |
| Agent→operator free-form question Elicitation beyond the `tool_call` approval gate | reserved / adapter-level | The core `question` `response_contract` is committed v0 (`docs/PROTOCOL.md` `response_contract` registry). The Pi adapter has no distinct non-approval question wire type in the surveyed surface, so it declares non-approval `question` Elicitations unsupported at delivery (`unsupported_command`) until a Pi question surface is promoted. This is an adapter-support limitation, not a reclassification of the core `question` contract. |
| `/fork` | reserved / SDK-internal | remote_pi groups fork with the replacement bug-class internally, but fork is not an operator wire action in the surveyed inbound actions; it is SDK-internal. Out of v0 Pi adapter surface. |
| `/reload` | reserved / out-of-scope-unless-verified | Same-session `session_start:reload` against a re-`require`d module; a Pi-process concern the session-replacement harness explicitly leaves out of scope. Out of v0 Pi adapter parity scope unless a follow-on verifies it. |

## 8. Migration-decision criteria

The operator can switch from Remote Pi to Patchbay when **all** of the following hold:

- [ ] **(a) Capability coverage.** Every committed-v0 Pi capability in §4 is implemented by the Pi adapter.
- [ ] **(b) Identity mapping verified.** The session identity mapping in §3 is verified, including the `session_new` generation-bump rule (and the fresh-session-restart bump, and the no-bump rules for `session_compact` and `--continue` restarts).
- [ ] **(c) Reconnect/snapshot parity.** Reconnect and snapshot parity in §5 holds against the `partial` tier; unreconciled state is shown as `stale`/`unknown`, never as live.
- [ ] **(d) Deferred features accepted.** The deferred features in §7 are consciously accepted as gaps — including an explicit accept-or-replace decision for the Remote Pi queued-message behavior (`queued_message_set`/`queued_message_clear`), since that is current operator-visible behavior being left outside adapter Operation scope.
- [ ] **(e) Replacement-window safety verified.** All subscribed surfaces reconcile the `session_new` generation bump consistently (not just the sender); late old-generation frames become `stale_event` audit-only and do not mutate the live generation (`LateGenerationInert`).
- [ ] **(f) UX acceptance met.** The UX acceptance criteria in `feature-ux-v0-acceptance` are met.

## 9. Extension pressure classification

This is the local committed-v0 / reserved-seam / rejected classification for the Pi adapter, consistent with the non-foreclosure discipline in `feature-extension-seams-non-foreclosure` and its ordering note (local per-feature classification suffices until the central extension-seams sweep runs). The central sweep will consolidate this into the project-wide registry when it executes.

- **Committed v0:**
  - the `OperationKind` mappings in §4;
  - the `partial` snapshot tier;
  - the `session_new` generation-bump mapping (session replacement, not a same-generation clear).
- **Reserved seams:**
  - supervisord-control `spawn` (follow-on promotion);
  - free-form question Elicitation support in the Pi adapter (the core `question` contract is committed; Pi-adapter support is the reserved seam);
  - `/fork` (SDK-internal);
  - tighter Elicitation responder binding (endpoint class / fallback chain).
- **Rejected for v0:**
  - Pi-specific state names in core protocol;
  - treating `session_new` as a same-generation clear;
  - treating `session_new` as `spawn`.
