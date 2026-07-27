# Patchbay Vision

Patchbay is a deployment-neutral human control plane for operating agent sessions and the operational resources that govern their availability, capability, and safe control across machines.

Patchbay gives an operator a reliable cockpit for discovering sessions and operational resources, spawning or attaching to runtime sessions, submitting authorized Operations, receiving source-authenticated Observations, answering Elicitations, and recovering state after disconnection. Sessions remain the product center; resources enter the control plane only when their state materially changes what the operator can ask an agent to do or requires human action to keep agent work operating. The coordination core is a network-reachable fixed point: operator surfaces, agent/harness machines, and resource adapters are reconnecting clients of it, and neither side is load-bearing for the other. Patchbay starts with Pi as its first workflow adapter and uses token-commune as its second, materially non-session reference adapter; neither system defines the core architecture.

## Audience and release horizon

Patchbay is intended to become a publishable, reliable self-hosted product that independent operators can deploy for themselves. The first executable milestone, `v0.1.0`, gets the initial operator operational; it is a personal/internal milestone rather than a public distribution milestone, does not require completed publication legal review, and is not the product ceiling.

The `v0.x` line hardens deployment, migrations, public contracts, adapter boundaries, the operational-resource plane, and implementation-backed assurance while retaining pre-1.0 freedom to make explicit breaking changes. `v1.0.0` is the public-product threshold: one human operator per deployment, one supported reference deployment path, Pi plus the materially distinct token-commune resource adapter, and stable designated public contracts. Multi-human shared deployments, federation, HA, and broader provider/adopter integrations remain explicit post-v1 seams rather than hidden assumptions or `v0.1.0` obligations.

## Why Patchbay exists

Headless and remote agent work needs more than chat transport. An operator often works from several human surfaces — phone, laptop, desktop, web, CLI — while agents and runtimes live wherever the operator chooses: a VM, container, local workstation, home server, cloud host, or future deployment target.

The core must remain reachable independently of any one operator device or harness host. A colocated v0.1.0 deployment is a convenience for installation and testing; it is not the architectural model. The architectural model is a durable coordination core that reconnecting surfaces and adapters can independently join.

The control surface must answer these questions reliably:

- Which sessions and agent-operational resources exist?
- Which machine, project, adapter, and runtime does each session belong to?
- Which resources currently constrain or enable those sessions?
- What is each session's authoritative connectivity/activity state and each resource's authoritative domain state?
- Was my command accepted?
- Did it reach the intended session or resource?
- Can I retry safely?
- What state is authoritative after reconnect?
- Who is allowed to control this session or resource?

Patchbay exists so accepted operator intent cannot disappear silently or mutate the wrong session or resource.

## Product center

Patchbay leads with the human control surface.

The first operator experience is a responsive web cockpit backed by a shared TypeScript operator domain. The same domain supports an Expo mobile app later without changing protocol semantics.

```text
human control surfaces
  web cockpit
  CLI
  future Expo app
      │
      ▼
shared operator domain + protocol client
      │
      ▼
Patchbay coordination core
      │
      ├── Pi session adapter
      ├── token-commune resource adapter
      ├── shell/job adapters
      └── future harness/project/tool adapters
```

## What Patchbay is

Patchbay is:

- a publishable, reliable self-hosted product for independent operators;
- a personal human-operated control plane for headless and distributed agent sessions plus the operational resources that govern their capability and availability;
- a durable Operation, Observation, Elicitation, snapshot, and authority layer;
- an adapter-neutral protocol and daemon model;
- a web-first cockpit with mobile-quality ergonomics;
- a formally specified coordination system where safety properties are modeled before they are treated as product semantics.

## What Patchbay is not

Patchbay is not:

- a Pi-specific mobile remote app;
- a replacement for any one harness;
- an LLM orchestrator that decides what work should happen;
- a project-management system;
- a workflow substrate tied to one repo convention;
- a UI-only dashboard without durable delivery semantics.

Adapters may integrate with Pi, Claude, Codex, shell jobs, project trackers, or workflow substrates. Those integrations remain edges around a neutral core.

## Success criteria

Patchbay is successful when an operator can move among phone, laptop, desktop, and CLI while controlling remote/headless agent sessions and the resources that materially govern them, with clear delivery state, durable history, actionable attention, and recoverable snapshots. Resource-specific views may be richer than the generic conformance floor, but they must preserve authority, delivery, and stale-state honesty. At the public-product threshold, an additional operator can independently install, secure, upgrade, back up, restore, diagnose, and operate their own Patchbay deployment through the supported reference path.

A useful Patchbay session has these properties:

- accepted Operations are durable and visible through the `CommandState`-equivalent lifecycle until terminal outcome;
- retries are idempotent unless the operator explicitly duplicates an action;
- replies and response Operations correlate through typed references to the command/message/elicitation they answer;
- session identity is stable enough that late replies cannot affect the wrong session;
- stale state is displayed as stale rather than live;
- authority grants are checked before Operations execute;
- adapters can fail without hiding the failure from the operator.

## Quality benchmark

Patchbay targets the confidence and continuity of a mature first-party remote agent app while preserving self-hosted, adapter-neutral, formally specified infrastructure.

Remote Pi is the immediate migration bridge. Claude-app-style remote control is the UX quality benchmark.
