# Patchbay

Patchbay is a **deployment-neutral human control plane for operating agent sessions across machines**.

It is designed for operators who run coding agents, shells, jobs, and other runtimes on VMs, containers, laptops, desktops, home servers, or cloud hosts — then need one trustworthy cockpit for discovering sessions, sending intent, receiving replies, approving or interrupting work, and recovering state after disconnection.

Patchbay starts with a **responsive web cockpit** and a **Pi-first adapter target** so it can become useful as a replacement for current Remote Pi-style workflows. The core model remains adapter-neutral: Pi is the first plug in the patchbay, not the architecture.

## Current status

Patchbay now contains the implemented **`v0.1.0` walking skeleton**: a Rust coordination core with durable local state, a TypeScript web server and responsive web cockpit, a Pi adapter, a diagnostic/scriptable CLI, generated Protobuf contracts, the browser/core trust boundary, and the registry-derived presentation conformance floor. Together these components exercise the single-operator durable control loop; deployment and operation are documented in [`docs/RUNBOOK.md`](docs/RUNBOOK.md).

This is an executable internal milestone rather than a finished public distribution. Versioned SQLite migrations (schema v2 via `PRAGMA user_version`) exist; packaging, supported public upgrade/rollback policy, public compatibility guarantees, broader adapter proof, and independent-operator deployment hardening remain work for the `v0.x` line.

`v0.1.0` gets the initial operator operational; it is a personal/internal milestone rather than a public distribution milestone and does not require completed publication legal review. It is not the product ceiling. The `v0.x` line hardens deployment, migrations, public contracts, executable assurance, and adapter portability. `v1.0.0` is the reliable self-hosted public-product threshold: independent operators can deploy Patchbay through a supported reference path, designated public contracts carry SemVer compatibility, and Pi plus a credible second or materially distinct reference adapter proves the adapter boundary.

## Why Patchbay exists

Remote/headless agent operation fails when the control surface is treated as “just chat.” Operators need to know:

- Which sessions exist, and where are they running?
- What is the session's authoritative connectivity and activity status?
- Was my command accepted?
- Did it reach the intended session?
- Can I safely retry after a timeout or reconnect?
- What state is authoritative after my phone, laptop, or browser disconnects?
- Who is allowed to control which session or resource?

Patchbay exists so **accepted operator intent cannot disappear silently or mutate the wrong session**.

## Product direction

Patchbay leads with the human control surface:

```text
human control surfaces
  web cockpit
  CLI
  future Expo app
      │
      ▼
generated Protobuf contracts + protocol semantics
(each surface implements its own client domain)
      │
      ▼
Patchbay coordination core
      │
      ├── Pi adapter
      ├── shell/job adapters
      ├── future harness adapters
      └── future tool/project adapters
```

The first useful milestone is a responsive web cockpit backed by durable command/message semantics and a Pi adapter good enough to migrate existing Remote Pi workflows. v0.1.0 is single-operator and single-core: no native mobile app, no high availability, no multi-human coordination, and no arbitrary adapter ecosystem yet. The UX quality bar is closer to a mature first-party remote agent app: clear session identity, visible delivery state, recoverable history, stale-state honesty, and multi-device continuity. Canonical command, session, and failure state names live in [`docs/PROTOCOL.md`](docs/PROTOCOL.md).

## Core ideas

Patchbay separates examples from architecture through explicit planes:

- **Human control surface plane** — web, CLI, future mobile, notifications, approvals.
- **Operation plane** — operator-originated Operations: prompts, commands, cancels, approvals, resumes.
- **Runtime/session plane** — agents, shells, jobs, harness sessions, containers, worktrees.
- **Adapter plane** — Pi first; other harnesses/tools later.
- **Operation / Observation / Elicitation plane** — delivery, replies, idempotent retry, failure vocabulary, and operator elicitations.
- **State and snapshot plane** — authoritative snapshots, stale/offline/unknown recovery.
- **Authority and identity plane** — grants, revocation, identity, anti-spoofing.
- **Coordination plane** — leases, ownership claims, locks, handoffs.
- **Deployment plane** — daemon, container, VM, local service, sidecar, split deployment.
- **Verification plane** — formal specs, contracts, conformance vectors, property tests.

## v0.1.0 walking skeleton

v0.1.0 proves the smallest useful control loop:

- one human operator;
- responsive web cockpit as the primary surface;
- CLI for setup, administration, debugging, and scripted access;
- one authoritative coordination core;
- local durable event-log persistence behind ports (snapshot-capable store with on-demand snapshot materialization);
- Pi adapter as the first runtime integration;
- initial commands for message/prompt delivery, cancel/interrupt where supported, status/snapshot refresh, and correlated replies/events.

v0.1.0 intentionally defers native mobile, HA or replicated cores, multi-human authority workflows, arbitrary adapters, project-management features, and lease-backed coordination unless later foundation work explicitly promotes a specific lease-backed workflow.

## Design commitments

Patchbay is:

- **human-control-surface first** — the operator experience is the lead product value;
- **deployment-neutral** — the operator decides where core and adapters run;
- **adapter-neutral** — Pi, Claude, Codex, shell, CI, and project tools are integrations, not primitives;
- **snapshot-driven** — live streams are useful, but snapshots repair missed events and reconnect gaps;
- **authority-aware** — commands require grants and target identity;
- **idempotent by default** — retries should not accidentally double-apply dangerous intent;
- **formally specified** — delivery, authority, identity, snapshots, and leases are modeled before being treated as product semantics.

## Formal verification posture

Patchbay uses formal methods for coordination semantics, not for everything.

- **TLA+** is the long-lived semantic baseline for dynamic state-machine models.
- **Quint** is an ergonomic authoring candidate and may be used where it improves readability and iteration.
- **Alloy** is used for relational invariants such as identity, authority graphs, routing legality, and lease exclusivity.

Verification focuses on properties like:

- accepted commands cannot vanish silently;
- commands cannot hit the wrong session;
- retries are idempotent at the coordination boundary;
- replies correlate only to known prior messages or commands;
- snapshots correct stale control-surface state;
- unauthorized commands are rejected before delivery;
- exclusive leases cannot have two live owners in one authority domain.

Patchbay does **not** claim to formally verify LLM reasoning quality, OS scheduling, UI rendering, cryptographic primitive correctness, third-party harness internals, or real-world network latency bounds.

## Repository layout

```text
README.md

docs/
  VISION.md        project purpose and boundaries
  SPEC.md         starting scope, stack posture, and core concepts
  ARCHITECTURE.md planes, components, and boundaries
  PROTOCOL.md     protocol concepts and required behavior
  VERIFICATION.md formal verification scope and tool posture
  UX.md           human control surface expectations
  GLOSSARY.md     terminology
```

v0.1.0 repository layout:

```text
specs/        TLA+/Quint and Alloy models
contracts/    Protobuf contracts, generated bindings, and conformance vectors
core/         Rust coordination-core domain and storage code
server/       Rust coordination-core server
web-server/   TypeScript HTTP/HTTPS control-surface server
web-cockpit/  TypeScript browser cockpit and shared operator-domain code
pi-adapter/   TypeScript Pi runtime adapter
cli/          TypeScript setup, administration, and diagnostic CLI
```

Additional adapters and control surfaces, including a native Expo app, are
reserved seams; v0.1.0 does not represent them as current repository paths.

## Reading guide

Start here:

1. [`docs/VISION.md`](docs/VISION.md) — what Patchbay is and why it exists.
2. [`docs/SPEC.md`](docs/SPEC.md) — starting scope, stack decisions, and non-goals.
3. [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — conceptual planes and component boundaries.
4. [`docs/PROTOCOL.md`](docs/PROTOCOL.md) — durable operator intent, commands, replies, grants, snapshots, and leases.
5. [`docs/VERIFICATION.md`](docs/VERIFICATION.md) — what formal verification must cover.
6. [`docs/UX.md`](docs/UX.md) — first control-surface expectations.
7. [`docs/GLOSSARY.md`](docs/GLOSSARY.md) — shared terminology.

## Non-goals

Patchbay is not:

- a Pi-specific remote app;
- a mobile-only client;
- a replacement for every agent harness;
- an autonomous LLM orchestrator;
- a project-management system;
- a workflow substrate tied to one repository convention;
- a dashboard that hides best-effort delivery behind optimistic UI.

## License

No license has been selected yet.
