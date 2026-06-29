# Patchbay Protocol

Patchbay protocol semantics are defined around durable operator intent, explicit authority, unambiguous target identity, and recoverable state.

This document defines concepts and required behavior, not a final wire encoding. Until generated schemas or IDL exist, this document is the canonical source of truth for command state, session state, failure vocabulary, and transition semantics. Future TypeScript/Rust enums, TLA+/Quint variables, conformance vectors, and UI presentation labels derive from these registries rather than redefining them.

## Actors and endpoints

An **actor** is any represented participant: operator, agent, adapter, daemon, service, or control surface.

A **device** is a physical or virtual host that can run one or more endpoints, such as a browser on a laptop, a CLI on a VM, or an adapter process near a runtime.

An **endpoint** is a concrete connection or addressable runtime instance for an actor on a device. An actor may have multiple endpoints across devices or deployments.

Actors, devices, and endpoints have stable identifiers assigned by Patchbay or verified through an adapter-specific trust root. Human-readable labels are metadata, not routing authority.

An **operator session** is an authenticated browser or CLI session for the human operator. Operator sessions are endpoint-bound server-side records, not bearer authority stored in UI state. V0 has one human operator, but commands still name and validate the issuing actor, device, endpoint, and operator session so future multi-operator authority domains can extend the model without changing command semantics.

## Sessions

A **session** is an adapter-reported runtime/control target. A session identity binds enough information to prevent wrong-target mutation:

- adapter id;
- deployment or machine scope;
- runtime session id where available;
- optional project/cwd/name metadata;
- adapter-specific generation or epoch when session replacement occurs.

Late replies or events must bind to the session generation they describe. A reply for an old generation cannot mutate a new generation without an explicit adapter rule.

## Messages, commands, and replies

### Message

A message carries information. It may ask for a reply but does not itself grant authority to act.

### Command

A command is operator intent that may cause external action. Commands require:

- command id;
- target session or actor;
- authority grant;
- idempotency key;
- declared command kind;
- payload validated at the boundary;
- expiration or cancellation semantics where applicable.

### Reply

A reply references a prior message or command. A reply is valid only when its correlation id refers to a known prior event in the same authority/session context.

## Canonical state registries

These registries are committed v0 protocol behavior unless marked as an extension seam. Implementations may add display labels, colors, or adapter-specific metadata, but they must not add protocol states outside the registry without updating this document, contracts, models, and conformance vectors together.

### Command lifecycle state

`CommandState` is durable core state for an accepted command. Control-surface-local states such as `draft` and `submitting` are intentionally excluded from this registry.

| State | Terminal? | Meaning |
|---|---:|---|
| `accepted` | no | Patchbay validated the command, checked authority, deduplicated the idempotency key, and durably recorded the command before delivery. Delivery may not have been attempted yet. |
| `delivered` | no | The target adapter accepted delivery responsibility for the command. This does not imply execution started or completed. |
| `running` | no | The target adapter or runtime reports active execution for the command. |
| `completed` | yes | The command reached a successful semantic completion reported by the authoritative target context. |
| `rejected` | yes | Patchbay or the target adapter refused an already-recorded command before execution as a semantic/policy decision, such as unsupported command, invalid target, or delivery refusal. Pre-acceptance submission refusal is a `SubmissionOutcome`, not `CommandState = rejected`. |
| `failed` | yes | Delivery or execution reached a non-policy error after the command was accepted, such as adapter crash, transport failure after acceptance, runtime error, or unknown execution failure. |
| `expired` | yes | The command exceeded its validity window before reaching a later non-expired terminal state. |
| `cancelled` | yes | Operator or policy cancellation became the authoritative terminal outcome. |
| `superseded` | yes | A newer accepted command or policy explicitly replaced this command, and the old command must no longer be executed or presented as pending work. |

Allowed transitions:

```text
accepted  -> delivered | rejected | failed | expired | cancelled | superseded
delivered -> running | completed | rejected | failed | expired | cancelled | superseded
running   -> completed | failed | expired | cancelled | superseded

completed -> <terminal>
rejected  -> <terminal>
failed    -> <terminal>
expired   -> <terminal>
cancelled -> <terminal>
superseded -> <terminal>
```

Boundary rules:

- `accepted` is the only initial durable `CommandState` for a newly accepted command.
- Terminal states are final for that command id. Late adapter events are recorded as events for audit/reconciliation but do not mutate the command state.
- A duplicate submission with the same command id or idempotency key returns the existing command record and state; it does not create a new state transition.
- `rejected` means a known actor refused the command by semantics or policy. `failed` means an accepted attempt encountered an error. `expired`, `cancelled`, and `superseded` are distinct terminal outcomes and must not be collapsed into `failed`.

### Submission outcome and local submission state

A submission is the request to create or retrieve a command record. Not every submission creates a durable command. Pre-acceptance refusal is represented as `SubmissionOutcome = rejected`; it is not `CommandState = rejected` unless an explicit audit policy creates a separate non-command audit record.

`SubmissionOutcome` is the boundary result returned by Patchbay for a submission attempt:

| Outcome | Meaning |
|---|---|
| `accepted` | Patchbay created or found a durable command record. The returned command id has `CommandState = accepted` or the existing deduplicated command state. |
| `rejected` | Patchbay refused the submission before creating a command record, such as validation failure, authorization denial, unsupported command at the core boundary, or invalid target known before acceptance. |
| `failed` | Patchbay could not complete the submission attempt due to service or transport failure. The client must not infer acceptance. |
| `unknown` | The client cannot determine whether Patchbay accepted the submission and must reconcile by idempotency key or snapshot. |

`LocalSubmissionState` exists only inside a control surface before or while it reconciles with Patchbay. It is not persisted as durable command state.

| State | Meaning |
|---|---|
| `draft` | Local-only operator input that has not been submitted to Patchbay. It may be edited or discarded without protocol history. |
| `submitting` | The control surface sent a submission and is waiting for a `SubmissionOutcome`. |
| `submit_failed` | The control surface received or inferred `SubmissionOutcome = failed`. The operator may retry with the same idempotency key. |
| `unknown` | The control surface received or inferred `SubmissionOutcome = unknown` and must reconcile by querying Patchbay before claiming success or failure. |

Allowed local transitions:

```text
draft        -> submitting
submitting   -> draft | submit_failed | unknown | <reconciled command id> | <rejected submission>
submit_failed -> submitting | draft
unknown      -> submitting | submit_failed | <reconciled command id> | <rejected submission>
```

`<reconciled command id>` and `<rejected submission>` are exits from local submission state, not additional enum members. Once Patchbay returns or snapshots a command id, the UI derives command display from `CommandState`. A UI may still show local transport decoration, but durable truth comes from the core command record.

### Session state axes

Session presentation is the composition of two protocol axes. This avoids treating “live”, “idle”, “working”, “stale”, and “unknown” as one overloaded enum.

#### `SessionConnectivityState`

| State | Meaning |
|---|---|
| `live` | Patchbay has a sufficiently fresh authoritative signal that the adapter/session endpoint is reachable. |
| `stale` | Cached state exists, but Patchbay lacks a sufficiently fresh authoritative signal. Stale data must not be rendered as live. |
| `offline` | Patchbay has authoritative evidence that the adapter/session endpoint is unavailable. |
| `unknown` | Patchbay lacks enough information to classify the session as live, stale, or offline. |
| `failed` | Patchbay has an explicit adapter/session error that prevents reliable control or observation. |

#### `SessionActivityState`

| State | Meaning |
|---|---|
| `idle` | The session is not reporting active work. |
| `working` | The session reports active work, command execution, or adapter-known runtime activity. |
| `unknown` | Patchbay lacks a current authoritative activity report. |

Allowed connectivity observations:

```text
unknown -> live | stale | offline | failed
live    -> stale | offline | failed
stale   -> live | offline | unknown | failed
offline -> live | stale | unknown | failed
failed  -> live | stale | offline | unknown
```

Allowed activity observations:

```text
unknown -> idle | working
idle    -> working | unknown
working -> idle | unknown
```

Session transitions are driven by authoritative adapter events, timeout/staleness policy, and snapshots. Snapshots may move an axis to any state allowed above when they carry fresher authority than cached UI state.

Derived UI labels such as “Live idle”, “Working”, “Stale working”, or “Offline” are presentation labels over these axes, not protocol states. A stale or unknown connectivity value dominates presentation: stale working is not live working.

### Failure and outcome vocabulary

Failures are layer-aware. Use the narrowest term that matches the authoritative event.

| Term | Layer | Meaning | Typical command effect |
|---|---|---|---|
| `validation_failed` | submission | Patchbay rejected the payload shape, command kind, target shape, or required field before acceptance. | `SubmissionOutcome = rejected`; no `CommandState` |
| `authorization_denied` | submission | The actor/endpoint lacks a valid grant for the command. | `SubmissionOutcome = rejected`; no `CommandState` |
| `target_not_found` | submission/delivery | The addressed actor/session/resource does not exist in the relevant authority/session context. | submission `rejected` before acceptance, or command `rejected`/`failed` after acceptance by policy |
| `unsupported_command` | submission/delivery | The core or adapter does not support the declared command kind/capability. | submission `rejected` before acceptance, or command `rejected` after acceptance |
| `target_offline` | delivery | The target is known unavailable. | `failed` or `expired`, depending on command policy |
| `adapter_unavailable` | delivery | The adapter required for delivery is unavailable. | `failed` or remains `accepted` until retry/expiration policy resolves |
| `transport_timeout` | submission/delivery | A transport layer did not answer within its timeout. Timeout never implies success or denial. | local `unknown`/`submit_failed`, or durable `failed`/continued `accepted` by policy |
| `delivery_rejected` | delivery | The adapter received the command but refused delivery responsibility. | `rejected` |
| `execution_failed` | execution | The target began or accepted execution and reported failure. | `failed` |
| `expired` | policy/time | The command validity window closed. | `expired` |
| `cancelled` | policy/operator | Cancellation became the authoritative result. | `cancelled` |
| `superseded` | policy/operator | A newer command or policy replaced this command. | `superseded` |
| `stale_event` | reconciliation | A late event refers to an old command/session generation or terminal command. | audit record only; no state mutation |

Extension seam: future adapters may attach adapter-specific diagnostic codes, but those codes map onto this vocabulary at the Patchbay boundary.

## Acceptance semantics

Patchbay distinguishes acceptance from delivery and completion.

A command accepted by Patchbay is durably recorded before delivery. After acceptance, it remains visible as a `CommandState` until and after it reaches a terminal state. An accepted command cannot disappear silently.

Acceptance creates a command record only after boundary validation, authority checking, idempotency reconciliation, and target identity binding. Invalid submissions that fail before acceptance return `SubmissionOutcome = rejected` without creating durable command state. Audit policy may record rejected attempts, but those audit records are not command records and do not use `CommandState`.

## Idempotency and retry

Commands are idempotent by default at the Patchbay boundary. Retrying the same command id or idempotency key does not apply the command twice.

A duplicate command returns the existing command state unless the operator explicitly creates a new command with a new command id and idempotency key.

Adapters that cannot guarantee idempotent external execution must report that limitation as a capability constraint. Patchbay still deduplicates at the coordination boundary and exposes the adapter limitation to control surfaces.

## Cancellation, expiration, supersession, and race semantics

- Cancellation is a command or policy request that races with execution. If `completed`, `failed`, `expired`, or another terminal state is committed first, later cancellation cannot mutate the command and is recorded only as a late event or separate cancellation failure.
- Expiration is evaluated against the command validity window. If expiration wins before a later terminal outcome is committed, the command becomes `expired`; if a terminal outcome wins first, expiration does not rewrite history.
- Supersession requires an explicit replacement relationship to a newer accepted command or policy decision. Supersession is not a synonym for cancellation or failure.
- Running is non-terminal. A running command remains observable until one terminal state wins.
- First durable terminal commit wins. The core assigns a total order to accepted state-transition events in the durable event log; the earliest committed valid terminal transition becomes authoritative.
- If two terminal candidates are truly concurrent before persistence, models may treat the winner as nondeterministic, but implementations must persist one total order and expose the chosen terminal state consistently in snapshots and conformance traces.
- Later conflicting events are audit/reconciliation events, not state rewrites.

## Snapshots and streams

Event streams are useful but not authoritative by themselves.

A snapshot is an authoritative state view for an actor, session, command, lease, or resource. Control surfaces reconcile against snapshots after reconnect, resume, tab restore, app restart, or suspected drift.

Snapshots expose the canonical state axes above. Stale cached state must not render as live state.

### Revisions and cursors

The coordination core owns a single totally-ordered durable event log per authority domain. Every accepted state-transition event is assigned a monotonic, gap-free **log sequence number** (`LSN`) at durable-commit time. The `LSN` is the canonical ordering for first-terminal-commit-wins and for snapshot reconciliation.

A **revision** is the `LSN` at which a specific view (command, session, actor, grant, audit record) was last durably updated. A **cursor** is an `LSN` a control surface or adapter holds to express "I have authoritative knowledge up to here."

V0 revision/cursor rules:

- Every snapshot carries the `LSN` it was materialized at and the per-view revisions it reflects.
- A control surface reconciles by submitting its cursor; the core returns events with `LSN > cursor` and/or a snapshot materialized at a later `LSN`.
- A snapshot with an `LSN` strictly less than the core's current state for that view is **older** and is rejected as an authority source; the core returns the current view instead.
- A snapshot from a different authority domain or a different core generation is rejected outright.
- Late events carry the `LSN` at which they were committed; an event whose `LSN` is older than the view it would mutate is recorded as an audit/reconciliation event and does not rewrite the current view.
- The core may serve a compressed snapshot at any `LSN`; cursors remain valid across compaction because revisions are monotonic.

### Atomicity between events and snapshots

V0 requires the following atomicity guarantees at the persistence boundary:

- A command is durably recorded (`accepted`) before delivery is attempted. Delivery never relies on in-memory state.
- A terminal transition is committed to the log before it is reflected in snapshots or returned to control surfaces.
- A snapshot materialization reads a consistent log prefix: it reflects every event with `LSN <= snapshot_LSN` and no event with `LSN > snapshot_LSN`.
- Snapshot writes do not reorder the log. A snapshot is a derived artifact keyed by `LSN`; it never becomes a second source of ordering.

If the persistence backend cannot provide these atomicity guarantees, the core must treat the write as failed (`SubmissionOutcome = failed` for submissions, or `failed`/continued `accepted` per policy for delivery) rather than expose an inconsistent view.

## Persistence and recovery

The coordination core owns durable command state, the event log, snapshots, and audit records through a storage port. V0 persistence assumptions:

- **Single-writer**: one authoritative core process writes to the log. There is no multi-writer, HA, or split-brain recovery in v0.
- **Local-first**: the default backend is embedded and local to the core process. Domain semantics must not depend on a specific storage engine.
- **Port-isolated**: the core reads/writes through a storage port; adapters and control surfaces never touch persistence directly.
- **Crash recovery**: on restart, the core replays the durable log to reconstruct in-memory state up to the last committed `LSN`. Accepted-but-not-yet-terminal commands are restored as `accepted` (or a later committed state) and continue through their lifecycle. No accepted command disappears silently after a crash.
- **Idempotent reprocessing**: replaying the log produces identical state. Re-delivery to adapters after recovery is governed by adapter capability and command policy, not by log replay.
- **Snapshot checkpointing**: snapshots are periodic materializations used to bound replay cost on recovery; they are derived artifacts, never an alternate source of truth. A recovery may load the latest snapshot then replay events with `LSN > snapshot_LSN`.

V0 does not require WAL replication, remote replication, point-in-time cloning, or storage-engine hot swap. Those are reserved seams.

## Authority grants

A grant authorizes an actor or endpoint to perform a set of command kinds against a target scope. Grants are explicit, revocable, and evaluated inside one authority domain.

A v0 grant records:

- grant id;
- authority domain id;
- subject actor id;
- optional subject device id;
- optional subject endpoint id or endpoint class;
- target scope, such as actor, adapter, runtime session, project/session group, or other modeled resource;
- allowed command kinds or adapter capability set;
- creation time and provenance;
- optional expiration;
- revocation generation or revoked time;
- revocation policy for already accepted commands;
- optional parent grant id / delegated-by field reserved for future delegation.

Grant checks happen before command acceptance. A submission without a live matching grant is rejected before delivery with `SubmissionOutcome = rejected` and `authorization_denied` or a narrower applicable failure term.

Authorization is deny-by-default. Control surfaces may hide unavailable actions, but UI availability is never authority. Sender identity is derived from the verified connection/session context, not from self-asserted payload fields, display names, project labels, cwd metadata, or adapter-reported friendly names.

Revocation prevents future authority. Already accepted commands follow the policy attached to their grant and command kind: continue, cancel where supported, or require reauthorization. Revocation does not delete command history; late events after revocation are audit/reconciliation events unless they are valid transitions for commands already accepted under the relevant policy.

V0 revocation actions include current-session revocation, all-session revocation, endpoint/device revocation, adapter/session grant revocation, and security lockdown. A lockdown rejects new commands, marks affected runtime sessions stale, requires fresh login, and records the reason.

## Leases

A lease is a time-bounded exclusive claim over a resource or coordination role. A lease has:

- resource id;
- holder actor;
- scope;
- expiration;
- renewal rules;
- release rules.

Within one modeled Patchbay authority domain, two live leases cannot grant exclusive ownership of the same resource and scope at the same time.

V0 reserves leases as an extension seam. Lease-backed behavior must define its own lifecycle registry before shipping; it must not overload `CommandState` or session state.

## Adapter capabilities

Adapters declare supported commands and guarantees:

- command kinds;
- streaming support;
- snapshot support;
- cancellation support;
- session replacement support;
- idempotency strength;
- attachment/authorization method;
- known failure modes.

Control surfaces render unsupported actions as unavailable rather than attempting best-effort hidden behavior.

### Adapter snapshot capability tiers

Adapter snapshot support is not boolean. V0 recognizes three tiers:

- **Authoritative snapshot** — the adapter can return a complete, authoritative view of the session at a generation the core can reconcile. The core treats this as a valid snapshot source and may use it to repair missed events.
- **Partial snapshot** — the adapter can return some state (e.g. command history or last-known status) but cannot fully reconstruct the session view. The core marks the unreconciled axes `unknown` or `stale` per `SessionConnectivityState`/`SessionActivityState` rather than synthesizing live state.
- **No snapshot** — the adapter cannot snapshot. The core holds the last-known cached view marked `stale` (or `unknown` if no cached view exists) and does not present it as live. Reconnect after missed events cannot be repaired by a snapshot; the control surface must reconcile against command/event records it can still query, and present unreconciled session state honestly.

Degraded behavior rules:

- The core never fabricates a snapshot from optimistic UI or cached state when an adapter reports no or partial snapshot capability.
- A `partial` or `no snapshot` adapter does not weaken durable command state: accepted commands and their `CommandState` remain authoritative from the core's log.
- If an adapter loses the ability to snapshot it previously had, the core records the capability change as an audit record and moves affected sessions to `stale` or `unknown` until a fresh authoritative signal arrives.

## Extension pressure classification

- **Committed v0 behavior:** `SubmissionOutcome`, `CommandState`, `LocalSubmissionState`, `SessionConnectivityState`, `SessionActivityState`, failure vocabulary, idempotent retry at the Patchbay boundary, and stale/unknown presentation honesty.
- **Reserved extension seams:** adapter-specific diagnostics, future command kinds, richer activity details, multi-operator authority domains, lease lifecycle, native/mobile-specific local cache states, and additional control surfaces.
- **Rejected direction:** Pi-specific state names, UI-only optimistic states, transport-specific errors, or adapter-specific lifecycle variants becoming core protocol states without registry updates.

## Security and trust boundary

Patchbay protocol assumes cryptographic primitives work as specified by their libraries and deployments. Formal models cover authority and identity relationships, not primitive cryptographic correctness.

Browser control uses server-side operator sessions with hardened cookies and CSRF protection for state-changing requests. Browser-local UI state is never authority for command submission, grant status, or session liveness.

Sender identity is derived from verified connection/authentication context, not from self-asserted display names or payload fields. External actor identities remain claims until verified by an adapter-specific trust root or deployment policy.

Security audit records are durable protocol-adjacent records for authentication, authorization, session management, command lifecycle, revocation, adapter attach/detach/failure, and stale-event rejection. Audit records are distinct from durable command/session state-transition events: they may record rejected attempts and failed checks that do not create command records. Audit records must not directly store raw session cookies, CSRF tokens, access tokens, passwords, bootstrap secrets, encryption keys, command prompt bodies by default, or sensitive attachments.
