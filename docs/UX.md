# Patchbay UX

## Purpose and scope

This document defines Patchbay's **surface-neutral UX conformance floor** — the minimum obligations any conformant human control surface must meet — and the **v0.1.0 web cockpit as the first conformant instance** of that floor.

**Surface-neutrality** is a principle symmetric to adapter-neutrality: just as adapter-specific capabilities are adapter-declared features rather than core protocol primitives, surface-specific presentation is a surface-declared feature, not a core UX primitive. A surface is conformant when it meets the floor; skins, layouts, and surface-native affordances are surface-declared above the floor.

The floor is **registry-derived**: it references the canonical registries in `docs/PROTOCOL.md` (state machines, failure vocabulary, authority, snapshot/reconnect rules) and the operator-domain positioning in `docs/ARCHITECTURE.md`. It does not re-declare them. Where this document and `docs/PROTOCOL.md` appear to diverge, `docs/PROTOCOL.md` is authoritative and this document has a bug. (`feature-command-state-ssot` requires `docs/UX.md` to reference rather than redefine protocol state machines.)

UI labels such as "Live idle", "Stale working", or "Completed before cancellation arrived" are **non-authoritative presentation labels** over the protocol axes — examples of how a surface may render canonical states, not a restatement of the registry.

## Surface-neutral conformance floor

Any conformant control surface must meet these obligations. Each references the canonical source rather than re-listing the registry members.

- **State presentation honesty.** Present every canonical member of `CommandState`, `SessionConnectivityState`, `SessionActivityState`, and `ElicitationState` **as defined in `docs/PROTOCOL.md`** (Command lifecycle state; Session state axes; ElicitationState lifecycle) without inventing divergent states. Stale or unknown states must not be styled as live. Session display composes connectivity × activity; a stale or unknown connectivity value dominates presentation.
- **Liveness vs delivery separation.** Separate session liveness (connectivity × activity) from command delivery (`CommandState`). Accepted does not mean completed; delivered does not mean completed. Cancellation is presented as a request into a moving system: if a command completed before cancellation arrived, the UI preserves the completed command state and explains the late cancellation rather than rewriting the outcome.
- **Identity-before-intent.** Show stable target identity (adapter id, deployment scope, runtime session id, session generation) before the operator can submit an Operation. Human-readable labels (project, cwd, name) are metadata, not identity — they must not override verified target identity.
- **Authority/grant visibility.** The control surface must answer the operator's question "Who is allowed to control this session or resource?" (`docs/VISION.md`). Action availability is derived from grants and adapter capabilities, but **UI availability is never authority** (`docs/PROTOCOL.md`, Authority grants: control surfaces may hide unavailable actions, but UI availability is never authority). A surface must distinguish denial (`authorization_denied`) from unsupported (`unsupported_command`) from revoked, and surface operator-visible grant and audit context where needed (current-session/endpoint/adapter revocation, security lockdown). Revocation prevents future authority; already-accepted commands follow the policy attached to their grant and OperationKind.
- **Operation affordance coverage.** Every committed v0.1.0 `OperationKind` (`spawn`, `attach`, `instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, `session-management` per `docs/PROTOCOL.md`, OperationKind registry) is either actionable through an appropriate surface flow or visibly presented as unavailable/unsupported with a canonical reason. Reserved kinds (`agent-send`, `adapter-utility-exec`) are not presented as committed v0.1.0 actions. The composer need not surface every kind — `spawn` and `attach` may be entry-point actions rather than composer actions — but the surface as a whole must cover them.
- **Failure vocabulary mapping.** Map failure text to the protocol failure/outcome vocabulary in `docs/PROTOCOL.md` so timeout, denial, rejection, expiration, cancellation, supersession, and execution failure remain distinct. Show what is safe to retry. **Retry safety is derived from the specific failure term plus the adapter's declared `idempotency_strength`, never from `CommandState` alone.** `failed` includes `execution_failed` (the target began or accepted execution and reported failure) and `execution_outcome_unknown` (execution may have occurred); neither is unconditionally safe to retry. The surface presents retry safety by combining the failure term with the capability:

  | Failure term | `idempotency_strength` | Retry safety |
  |---|---|---|
  | `execution_outcome_unknown` | `end-to-end` | safe to retry (adapter dedups externally) |
  | `execution_outcome_unknown` | `at-Patchbay-boundary` | retry may double-execute |
  | `execution_outcome_unknown` | `none` | retry will double-execute if the original executed |
  | `execution_failed` | any | not unconditionally safe — the target began executing; evaluate against the capability as above |
  | pre-execution failures (`target_offline`, `adapter_unavailable`, `delivery_rejected`) | any | safe to retry (execution did not start) |

  The surface must never present a retry as unconditionally safe without these signals. An **intentional duplicate** (a distinct new action, not a retry) is presented as a new action requiring a new command id and a new idempotency key, never as a retry of the original.
- **Reconnect reconciliation.** On reconnect, the surface submits its last-known cursor and the core returns newer events and/or a snapshot materialized at a later log sequence number. An older snapshot is never rendered as live; the view stays marked stale until a newer authoritative snapshot or live event stream confirms it. Reconnect does not rely on wall-clock freshness alone.
- **Elicitation presentation.** Surface pending Elicitations (approvals and questions) as attention-required state. v0.1.0 Elicitations target the operator actor (not a specific endpoint) and fan out to all subscribed operator surfaces; any authenticated endpoint may answer, and the first valid answer clears the Elicitation everywhere. The endpoint that actually answered is captured in the response Operation audit. Tighter binding (endpoint class, fallback chain) is reserved.
- **Observation/subscription-stream honesty.** Present Observations (output, lifecycle facts, status emissions) from subscription streams but never as authoritative alone; snapshots and core records reconcile. Streams are delivery optimizations.
- **Terminal-race explanation.** Command timelines can explain terminal races — for example "Completed before cancellation arrived", "Cancelled before completion", or "Expired before adapter completion" — without adding protocol states, following `docs/PROTOCOL.md` (Cancellation, expiration, supersession, and race semantics). These are UI labels, not protocol states.
- **No optimistic-state authority.** Optimistic UI state is never authority for command submission, grant status, or session liveness.

## Shared presentation-component layer (architectural seam)

The floor is enforced structurally by a **shared presentation-component layer** — the layer that binds canonical protocol states to skin-able presentable primitives (`StateBadge`, `CommandTimeline`, `Composer`, `ElicitationCard`, and similar) that any conformant surface composes. This refines the "presentation model" already named as part of the shared TypeScript operator domain in `docs/ARCHITECTURE.md`.

The layer's obligations:

- bind canonical protocol states to presentable primitives (present the registry; never invent divergent state names);
- be skin-able via design tokens, so an operator can customize the visual language without forking protocol semantics;
- be composable by any conformant surface (web, CLI, future Expo).

The layer is implemented as the registry-derived static check and skin-able CSS/showcase artifacts in `.mockups/design-system/`. The check is run with `node contracts/scripts/check-presentation.mjs` (or `npm run check:presentation` from `contracts/ts/`) and regenerates the traceability block below. The `ux-ui-design` `components` skill remains the mockup-time analog of this layer. Consumer guarantees and obligations are documented in the `Runtime conformance contract` section of `.work/active/features/feature-v0-presentation-component-layer.md`; the cockpit must satisfy that contract before it submits Operations.


<!-- BEGIN GENERATED PRESENTATION CONFORMANCE TRACEABILITY -->
<!-- Generated by `node contracts/scripts/check-presentation.mjs`; do not edit this block by hand. -->

### Generated presentation conformance traceability

Source registries: `.proto` enum declarations. CI check: `node contracts/scripts/check-presentation.mjs` (or `npm run check:presentation` from `contracts/ts/`).

| Registry | Members bound | CSS | Showcase | Accessibility |
|---|---|---|---|---|
| `OperationState` | accepted, delivered, running, completed, rejected, failed, expired, cancelled, superseded | all CSS bindings present | all showcase bindings present | pass |
| `SessionConnectivityState` | live, stale, offline, unknown, failed | all CSS bindings present | all showcase bindings present | pass |
| `SessionActivityState` | idle, working, unknown | all CSS bindings present | all showcase bindings present | pass |
| `ElicitationState` | answered, declined, expired, cancelled, withdrawn, superseded, stale, opened (base .elicitation-card), pending (base .elicitation-card) | all CSS bindings present | all showcase bindings present | pass |

Retry-safety matrix: all five `docs/UX.md` rows cross-reference `FailureCode` and `IdempotencyStrength` and are documented in the showcase.
Accessibility: WCAG contrast pairs and axe-core scan of `.mockups/design-system/components.html` pass.

<!-- END GENERATED PRESENTATION CONFORMANCE TRACEABILITY -->

## v0.1.0 web cockpit — first conformant instance

The first full control surface is a responsive web cockpit with mobile-first layout. It uses the shared TypeScript operator domain so the future Expo app can reuse the same delivery, reconnect, and session-state logic. The CLI provides setup, administration, debugging, and scriptable access.

### UX benchmark

Patchbay targets the confidence and continuity of a mature first-party remote agent app while keeping the infrastructure self-hosted and adapter-neutral. Remote Pi compatibility is the immediate migration floor (see `docs/ADAPTER-PI.md`); Claude-app-style continuity, delivery clarity, and mobile ergonomics are the quality benchmark.

### Required v0.1.0 screens

- **Session list.**
- **Session detail** — message timeline + command delivery timeline.
- **Composer.**
- **Elicitation/attention surface.** — pending approvals and questions.

The navigation pattern between these screens is an instance decision, deferred to the v0.1.0 surface-design mockup follow-on (see Reserved follow-up); the floor requires the screens exist, not their layout.

### Session list visible fields

- machine/deployment;
- adapter;
- project or working context when available;
- session label;
- model/runtime metadata when available;
- protocol-derived connectivity/activity status;
- last authoritative update time.

### Session detail / message timeline behavior

Render Observations (assistant messages, tool calls and results, lifecycle facts) with source authentication and correlation context. Render command delivery states (`CommandState`) distinctly from message content.

### Composer requirements

The composer surfaces the in-session OperationKinds: `instruct` with prompt payload, `cancel`/`interrupt`, `approval-response`/`elicitation-response`, `query`, `reconfigure`, and `session-management`. `spawn` and `attach` are surfaced as entry-point actions elsewhere (per the Operation affordance coverage obligation). The composer displays local submission state and durable `CommandState` using the canonical registries in `docs/PROTOCOL.md`, and shows idempotency behavior on retry.

### Reconnect/stale/offline banners

Visible connectivity-state banners; a stale view stays marked stale until a newer authoritative snapshot or live event stream confirms it.

### Multi-device continuity

A command sent from phone is visible from laptop. A session inspected from desktop reflects accepted commands and authoritative replies from other control surfaces.

### Empty/error/loading states

Explicit states for no sessions, no messages, target-not-found, adapter-unavailable, and failure cases — all using the failure vocabulary in `docs/PROTOCOL.md`.

### Mobile-first responsive

The responsive web cockpit prioritizes: a readable session list on phone; clear target identity before sending; composer ergonomics for prompts and commands; visible protocol-derived pending, delivery, and failure states; rich message rendering where safe; low-friction reconnect; minimal reliance on continuous foreground connection; and fast switching among sessions. It must work well from phone, laptop, desktop, and constrained remote/network environments.

### CLI

The CLI provides setup, administration, debugging, and scripted access — not a second independent product surface with divergent semantics.

v0.1.0 CLI diagnostic commands are read-only projections of existing durable state. They introduce no new storage path and no new write path; they filter and present what the durable event log, audit records, and already-modeled session/adapter state already hold. Each is issued as a `query` Operation against the core (`docs/PROTOCOL.md` OperationKind registry: `query` reads status, snapshot, capabilities, lists, history, metadata, or diagnostics) — the CLI is a control surface and never touches persistence directly; the core owns storage reads through the storage port (`docs/ARCHITECTURE.md` Storage port; `docs/PROTOCOL.md` Persistence and recovery). A no-lifecycle bypass read of the audit log (CLI-local, not routed as a `query` Operation) is a reserved seam, not v0.1.0 behavior.

| Command | Surfaces | Data source |
|---|---|---|
| `audit-query` | Filter audit records by actor / command / target / time / outcome. | audit log, read via a `query` Operation against the core (the audit log is durable and queryable per `docs/SECURITY.md` Audit events) |
| `inspect-command <id>` | Full lifecycle + audit trail for one command: accepted → routed → delivered → running → terminal, with timestamps + LSNs. Answers "why didn't my command deliver?" without a separate trace store. (`routed` is a routing audit entry, not a canonical `CommandState` — see `docs/PROTOCOL.md` CommandState registry.) | event log + audit records, read via a `query` Operation filtered by command id / correlation id |
| `session-health` | Session connectivity × activity axes — the full canonical registries: `SessionConnectivityState` (`live`/`stale`/`offline`/`unknown`/`failed`) × `SessionActivityState` (`idle`/`working`/`unknown`) — for one or all sessions. | session state axes (`docs/PROTOCOL.md` Session state axes) |
| `adapter-status` | Attached adapters, capability manifests (excluding raw `attachment_method.descriptor` — see SECURITY redaction), attach LSN, adapter generation. | adapter registry (`docs/ARCHITECTURE.md` Adapter plane; adapter lifecycle is audited per PROTOCOL.md) |

The delivery trace surfaced by `inspect-command` is a **projection**, not an authoritative command state; canonical `CommandState` remains as defined in `docs/PROTOCOL.md`. The trace is consistent with the snapshot-correctness rule that UI/presentation state is never authoritative (`docs/ARCHITECTURE.md` State and snapshot plane).

Deferred to post-v0.1.0: `event-inspect <lsn>` (raw event at LSN), metrics (counters/histograms/throughput), a dedicated health/status dashboard, SIEM export. These are reserved seams (`docs/SPEC.md` v0.1.0 observability scope), not silently absent.

## Reserved seams

- **Operator-customizable skins/layouts** — e.g. "Codex-style vs Claude-style vs CLI" presentations; an operator may customize the visual language without forking protocol semantics.
- **Design tokens / visual language** — the token vocabulary (colors, type, spacing, motion) a skin consumes.
- **Executable consumer conformance contract** — runtime assertions that cockpit/CLI/Expo consumers import and enforce directly remain reserved. The v0.1.0 registry-derived static check and skin-able presentation primitives are implemented above.
- **Native/mobile/Expo affordances** — push notifications, biometric/local unlock, richer offline local cache, share sheet/attachments, app-specific notification routing, native-background handling. The Expo app must not fork protocol semantics from the web cockpit.
- **Multi-surface presence-leak prevention** — filter-scoped subscriptions for multi-operator presence; reserved until multi-operator work arrives.

## Rejected directions

- Pi-specific concepts mandatory in the core UI model.
- Mobile-only assumptions built into the shared operator domain.
- Treating optimistic UI state as authoritative.
- Hiding accepted/delivered/completed distinctions.
- A pinned single visual design as the floor (the floor is behavioral + state-binding; visual design is surface-declared).
- Collapsing failure outcomes into a generic "failed".

## Anti-patterns

- Treating optimistic UI state as authoritative.
- Hiding accepted/delivered/completed distinctions.
- Letting a stale working indicator look live.
- Showing human-readable labels without stable identity context.
- Retrying commands without showing idempotency behavior or retry safety (per the `idempotency_strength` + `execution_outcome_unknown` matrix).
- Building mobile-only assumptions into the shared operator domain.
- Making Pi-specific concepts mandatory in the core UI model.
- Inventing divergent state names.
- Rendering a stale snapshot as live.
- Binding Elicitation responses to a specific endpoint rather than the operator actor.
