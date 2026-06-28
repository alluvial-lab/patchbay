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
- live/working/idle/stale/offline/unknown status;
- last authoritative update time.

### Send intent

The operator can send a prompt, command, approval, cancel, or other adapter-supported action to a selected session.

The UI displays command state from draft through acceptance and completion. Accepted does not mean completed; delivered does not mean completed.

### Recover after disconnect

When the control surface reconnects, it requests authoritative snapshots and reconciles local state. Stale data is shown as stale until corrected.

### Continue across devices

A command sent from phone is visible from laptop. A session inspected from desktop reflects accepted commands and authoritative replies from other control surfaces.

### Handle failure

Failures are explicit:

- not authorized;
- target offline;
- adapter unavailable;
- command unsupported;
- command expired;
- delivery unknown;
- execution failed.

The operator sees what is safe to retry.

## Presentation states

Patchbay uses explicit UI states:

- **Live idle** — target is reachable and no work is reported.
- **Working** — target reports active work.
- **Submitting** — local control surface is sending intent.
- **Accepted** — Patchbay durably recorded intent.
- **Delivered** — adapter accepted the command.
- **Stale** — cached state exists but lacks fresh confirmation.
- **Offline** — target is known unavailable.
- **Unknown** — current state cannot be determined.
- **Failed** — a known error occurred.

Stale or unknown state must not be styled as live.

## Mobile-first web expectations

The responsive web cockpit prioritizes:

- readable session list on phone;
- clear target identity before sending;
- composer ergonomics for prompts and commands;
- visible pending/sent/failed states;
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
