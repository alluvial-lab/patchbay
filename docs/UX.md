# Patchbay UX

Patchbay UX is human-control-surface first. The operator experience must make remote/headless agent work feel trustworthy, inspectable, and recoverable across phone, laptop, desktop, and CLI.

## UX benchmark

Patchbay targets the confidence and continuity of a mature first-party remote agent app while keeping the infrastructure self-hosted and adapter-neutral.

Remote Pi compatibility is the immediate migration floor. Claude-app-style continuity, delivery clarity, and mobile ergonomics are the quality benchmark.

## First surface

The first full control surface is a responsive web cockpit with mobile-first layout. It uses the shared TypeScript operator domain so the future Expo app can reuse the same delivery, reconnect, and session-state logic.

The web cockpit must work well from:

- phone browser;
- laptop browser;
- desktop browser;
- constrained remote/network environments.

The CLI provides setup, administration, debugging, and scriptable access.

## Core operator flows

### Discover sessions

The operator can see available sessions and understand:

- machine/deployment;
- adapter;
- project or working context when available;
- session label;
- model/runtime metadata when available;
- protocol-derived connectivity/activity status;
- last authoritative update time.

### Send intent

The operator can submit Operations to a selected target: spawn or attach where supported, instruct a turn with prompt payload, cancel or interrupt active work, answer approvals or Elicitations, query status/snapshots, reconfigure adapter-declared settings, or perform session-management actions.

The UI displays local submission state and durable command state using the canonical registries in `docs/PROTOCOL.md`. Accepted does not mean completed; delivered does not mean completed. Cancellation is presented as a request into a moving system: if a command completed before cancellation arrived, the UI preserves the completed command state and explains the late cancellation rather than rewriting the outcome.

### Answer Elicitations

V0 Elicitations target the operator actor (not a specific endpoint) and fan out to all subscribed operator surfaces. The UI surfaces pending Elicitations (approvals and questions) as attention-required state. The operator may answer from any authenticated operator endpoint; the first valid answer clears the Elicitation everywhere. The endpoint that actually answered is captured in the response Operation audit. Tighter binding (endpoint class, fallback chain) is reserved.

### Recover after disconnect

When the control surface reconnects, it requests authoritative snapshots and reconciles local state. Stale data is shown as stale until corrected.

Reconnect does not rely on wall-clock freshness alone. The control surface submits its last-known cursor and the core returns newer events and/or a snapshot materialized at a later log sequence number. The UI keeps a view marked stale until a newer authoritative snapshot or live event stream confirms it; an older snapshot is never rendered as live.

### Continue across devices

A command sent from phone is visible from laptop. A session inspected from desktop reflects accepted commands and authoritative replies from other control surfaces.

### Handle failure

Failures are explicit and use the layer-aware vocabulary in `docs/PROTOCOL.md`:

- authorization denied;
- target offline or not found;
- adapter unavailable;
- command unsupported;
- command expired, cancelled, or superseded;
- delivery or submission unknown;
- execution failed.

The operator sees what is safe to retry.

## Presentation states

Patchbay UI presentation derives from the canonical protocol registries in `docs/PROTOCOL.md` rather than redefining state machines locally.

- Command display composes protocol-defined local submission state with durable `CommandState` once a command id exists.
- Session display composes `SessionConnectivityState` with `SessionActivityState`. Labels such as **Live idle**, **Working**, **Stale working**, **Offline**, **Unknown**, or **Failed** are UI labels over those protocol axes, not additional protocol states.
- Pending Elicitations (approvals, questions) and Observations (output, lifecycle facts, status emissions) are presented from subscription streams but never treated as authoritative alone; snapshots and core records reconcile.
- Failure text maps to the protocol failure/outcome vocabulary so timeout, denial, rejection, expiration, cancellation, supersession, and execution failure remain distinct.
- Command timelines can explain terminal races without adding protocol states, following `docs/PROTOCOL.md` "Cancellation, expiration, supersession, and race semantics"; examples include **Completed before cancellation arrived**, **Cancelled before completion**, or **Expired before adapter completion**.

Stale or unknown state must not be styled as live.

## Mobile-first web expectations

The responsive web cockpit prioritizes:

- readable session list on phone;
- clear target identity before sending;
- composer ergonomics for prompts and commands;
- visible protocol-derived pending, delivery, and failure states;
- rich message rendering where safe;
- low-friction reconnect;
- minimal reliance on continuous foreground connection;
- fast switching among sessions.

## Future Expo app

The Expo app is a later control surface using the same TypeScript operator domain. It adds native affordances when they become load-bearing:

- push notifications;
- biometric/local unlock;
- richer offline local cache;
- share sheet / attachments;
- app-specific notification routing;
- native background limitations handled explicitly rather than hidden.

The Expo app must not fork protocol semantics from the web cockpit.

## Anti-patterns

- Treating optimistic UI state as authoritative.
- Hiding accepted/delivered/completed distinctions.
- Letting a stale working indicator look live.
- Showing human-readable labels without stable identity context.
- Retrying commands without showing idempotency behavior.
- Building mobile-only assumptions into the shared operator domain.
- Making Pi-specific concepts mandatory in the core UI model.
