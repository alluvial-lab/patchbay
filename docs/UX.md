# Patchbay UX

## Purpose and scope

This document defines Patchbay's **surface-neutral UX conformance floor** — the minimum obligations any conformant human control surface must meet — and the **v0 web cockpit as the first conformant instance** of that floor.

**Surface-neutrality** is a principle symmetric to adapter-neutrality: just as adapter-specific capabilities are adapter-declared features rather than core protocol primitives, surface-specific presentation is a surface-declared feature, not a core UX primitive. A surface is conformant when it meets the floor; skins, layouts, and surface-native affordances are surface-declared above the floor.

The floor is **registry-derived**: it references the canonical registries in `docs/PROTOCOL.md` (state machines, failure vocabulary, authority, snapshot/reconnect rules) and the operator-domain positioning in `docs/ARCHITECTURE.md`. It does not re-declare them. Where this document and `docs/PROTOCOL.md` appear to diverge, `docs/PROTOCOL.md` is authoritative and this document has a bug. (`feature-command-state-ssot` requires `docs/UX.md` to reference rather than redefine protocol state machines.)

UI labels such as "Live idle", "Stale working", or "Completed before cancellation arrived" are **non-authoritative presentation labels** over the protocol axes — examples of how a surface may render canonical states, not a restatement of the registry.

## Surface-neutral conformance floor

Any conformant control surface must meet these obligations. Each references the canonical source rather than re-listing the registry members.

- **State presentation honesty.** Present every canonical member of `CommandState`, `SessionConnectivityState`, `SessionActivityState`, and `ElicitationState` **as defined in `docs/PROTOCOL.md`** (Command lifecycle state; Session state axes; ElicitationState lifecycle) without inventing divergent states. Stale or unknown states must not be styled as live. Session display composes connectivity × activity; a stale or unknown connectivity value dominates presentation.
- **Liveness vs delivery separation.** Separate session liveness (connectivity × activity) from command delivery (`CommandState`). Accepted does not mean completed; delivered does not mean completed. Cancellation is presented as a request into a moving system: if a command completed before cancellation arrived, the UI preserves the completed command state and explains the late cancellation rather than rewriting the outcome.
- **Identity-before-intent.** Show stable target identity (adapter id, deployment scope, runtime session id, session generation) before the operator can submit an Operation. Human-readable labels (project, cwd, name) are metadata, not identity — they must not override verified target identity.
- **Authority/grant visibility.** The control surface must answer the operator's question "Who is allowed to control this session or resource?" (`docs/VISION.md`). Action availability is derived from grants and adapter capabilities, but **UI availability is never authority** (`docs/PROTOCOL.md`, Authority grants: control surfaces may hide unavailable actions, but UI availability is never authority). A surface must distinguish denial (`authorization_denied`) from unsupported (`unsupported_command`) from revoked, and surface operator-visible grant and audit context where needed (current-session/endpoint/adapter revocation, security lockdown). Revocation prevents future authority; already-accepted commands follow the policy attached to their grant and OperationKind.
- **Operation affordance coverage.** Every committed v0 `OperationKind` (`spawn`, `attach`, `instruct`, `cancel`, `interrupt`, `query`, `approval-response`, `elicitation-response`, `reconfigure`, `session-management` per `docs/PROTOCOL.md`, OperationKind registry) is either actionable through an appropriate surface flow or visibly presented as unavailable/unsupported with a canonical reason. Reserved kinds (`agent-send`, `adapter-utility-exec`) are not presented as committed v0 actions. The composer need not surface every kind — `spawn` and `attach` may be entry-point actions rather than composer actions — but the surface as a whole must cover them.
- **Failure vocabulary mapping.** Map failure text to the protocol failure/outcome vocabulary in `docs/PROTOCOL.md` so timeout, denial, rejection, expiration, cancellation, supersession, and execution failure remain distinct. Show what is safe to retry.
- **Reconnect reconciliation.** On reconnect, the surface submits its last-known cursor and the core returns newer events and/or a snapshot materialized at a later log sequence number. An older snapshot is never rendered as live; the view stays marked stale until a newer authoritative snapshot or live event stream confirms it. Reconnect does not rely on wall-clock freshness alone.
- **Elicitation presentation.** Surface pending Elicitations (approvals and questions) as attention-required state. V0 Elicitations target the operator actor (not a specific endpoint) and fan out to all subscribed operator surfaces; any authenticated endpoint may answer, and the first valid answer clears the Elicitation everywhere. The endpoint that actually answered is captured in the response Operation audit. Tighter binding (endpoint class, fallback chain) is reserved.
- **Observation/subscription-stream honesty.** Present Observations (output, lifecycle facts, status emissions) from subscription streams but never as authoritative alone; snapshots and core records reconcile. Streams are delivery optimizations.
- **Terminal-race explanation.** Command timelines can explain terminal races — for example "Completed before cancellation arrived", "Cancelled before completion", or "Expired before adapter completion" — without adding protocol states, following `docs/PROTOCOL.md` (Cancellation, expiration, supersession, and race semantics). These are UI labels, not protocol states.
- **No optimistic-state authority.** Optimistic UI state is never authority for command submission, grant status, or session liveness.

## Shared presentation-component layer (architectural seam)

The floor is enforced structurally by a **shared presentation-component layer** — the layer that binds canonical protocol states to skin-able presentable primitives (`StateBadge`, `CommandTimeline`, `Composer`, `ElicitationCard`, and similar) that any conformant surface composes. This refines the "presentation model" already named as part of the shared TypeScript operator domain in `docs/ARCHITECTURE.md`.

The layer's obligations:

- bind canonical protocol states to presentable primitives (present the registry; never invent divergent state names);
- be skin-able via design tokens, so an operator can customize the visual language without forking protocol semantics;
- be composable by any conformant surface (web, CLI, future Expo).

Implementation is **deferred**. The named layer is the future structural enforcement mechanism that makes the floor machine-checkable and skins possible; until it is implemented, conformance is enforced by the UX acceptance criteria in this document, protocol references, and later tests/vectors. The `ux-ui-design` `components` skill is the mockup-time analog of this layer. **The first real web cockpit must not proceed without either this component layer or an explicit conformance-test substitute** (a UX conformance vector/checklist that gates the web cockpit) — see Reserved follow-up.

## v0 web cockpit — first conformant instance

The first full control surface is a responsive web cockpit with mobile-first layout. It uses the shared TypeScript operator domain so the future Expo app can reuse the same delivery, reconnect, and session-state logic. The CLI provides setup, administration, debugging, and scriptable access.

### UX benchmark

Patchbay targets the confidence and continuity of a mature first-party remote agent app while keeping the infrastructure self-hosted and adapter-neutral. Remote Pi compatibility is the immediate migration floor (see `docs/ADAPTER-PI.md`); Claude-app-style continuity, delivery clarity, and mobile ergonomics are the quality benchmark.

### Required v0 screens

- **Session list.**
- **Session detail** — message timeline + command delivery timeline.
- **Composer.**
- **Elicitation/attention surface.** — pending approvals and questions.

The navigation pattern between these screens is an instance decision, deferred to the v0 surface-design mockup follow-on (see Reserved follow-up); the floor requires the screens exist, not their layout.

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

## Reserved seams

- **Operator-customizable skins/layouts** — e.g. "Codex-style vs Claude-style vs CLI" presentations; an operator may customize the visual language without forking protocol semantics.
- **Design tokens / visual language** — the token vocabulary (colors, type, spacing, motion) a skin consumes.
- **Shared presentation-component layer implementation** — the component library that binds canonical states to skin-able primitives. Named above; build is deferred.
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
- Retrying commands without showing idempotency behavior.
- Building mobile-only assumptions into the shared operator domain.
- Making Pi-specific concepts mandatory in the core UI model.
- Inventing divergent state names.
- Rendering a stale snapshot as live.
- Binding Elicitation responses to a specific endpoint rather than the operator actor.
