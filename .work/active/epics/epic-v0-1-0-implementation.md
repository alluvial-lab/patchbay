---
id: epic-v0-1-0-implementation
kind: epic
stage: implementing
tags: [foundation, protocol, verification]
depends_on: [epic-foundation-hardening]
parent: null
created: 2026-07-11
updated: 2026-07-15
gate_origin: null
release_binding: null
---

# Epic: v0.1.0 implementation

## Brief

The v0.1.0 walking skeleton is fully designed but entirely unbuilt. The `epic-foundation-hardening` design arc produced the foundation docs (VISION, SPEC, ARCHITECTURE, PROTOCOL, SECURITY, VERIFICATION, UX, GLOSSARY, ADAPTER-PI), generated Protobuf contracts (Rust + TS bindings), formal models (8 promoted / 39 stated-normative properties), and conformance vectors. No application code exists yet — the only Rust and TypeScript in the repo are generated protobuf bindings.

This epic implements the first executable Patchbay milestone: one operator controls Pi-backed sessions through a responsive web cockpit and diagnostic CLI, proving the durable control loop and getting the initial operator operational. The foundation docs, generated contracts, and formal models are the inputs; running code is the output.

The v0.1.0 scope is defined in `docs/SPEC.md` § "v0.1.0 walking skeleton": one operator, one authoritative coordination core, local durable persistence behind ports, Pi adapter first, responsive web cockpit + CLI, no native mobile / HA / multi-operator / leases. The architecture is defined in `docs/ARCHITECTURE.md` § "v0.1.0 component slice" and "v0.1.0 process topology": a Rust coordination core (single authoritative writer) plus a TypeScript web server (HTTP-terminating control surface), with the Pi adapter as the only required runtime adapter.

## Why this is epic-sized

This epic turns a complete design into a running system across six layers (coordination core, internal protocol seam, Pi adapter, web server, web cockpit, CLI). It spans two languages (Rust, TypeScript), introduces the first application code in the repo, and must satisfy the formal-model-backed safety properties while remaining usable enough to replace the operator's current remote-pi workflow. The coordination core alone is the largest piece and may warrant its own decomposition during feature-design.

## Critical path

```
epic-v0-core  (root — nothing starts until the core exists)
    │
    ├── feature-v0-protocol-seam  [depends: core]
    │       │
    │       ├── feature-v0-web-server  [depends: seam]
    │       │       │
    │       │       └── feature-v0-web-cockpit  [depends: web-server]
    │       │
    │       └── feature-v0-cli  [depends: seam]
    │
    └── feature-v0-pi-adapter  [depends: core]
```

- **Phone-usable path:** core → protocol-seam → web-server → web-cockpit
- **Agent-control path:** core → pi-adapter (parallel with the web chain after core lands)
- **CLI:** side branch off the protocol seam

Parallel work within a layer and across independent branches is handled by `implement-orchestrator` wave dispatch; the depends_on graph above is the structural ordering, not a serialization constraint.

## Relationship to v1.0.0 work

`epic-public-product-contract` (v1.0.0 public-product design) is sidelined pending this epic. Its remaining child features carry `epic-v0-1-0-implementation` in their `depends_on` so the substrate honestly reflects that the v1.0.0 public product cannot ship without v0.1.0 built. The v1.0.0 design work that already landed (verification-claim-correction) is preserved; only the unbuilt v1.0.0 features are blocked.

## Foundation references

- `docs/SPEC.md` — v0.1.0 walking skeleton scope and exclusions
- `docs/ARCHITECTURE.md` — v0.1.0 component slice, process topology, persistence topology
- `docs/PROTOCOL.md` — canonical state registries, acceptance semantics, idempotency, snapshots, authority
- `docs/SECURITY.md` — threat model, grants, audit
- `docs/VERIFICATION.md` — property-graded assurance, 8 promoted / 39 stated-normative
- `docs/UX.md` — surface-neutral conformance floor, v0 web cockpit instance
- `docs/ADAPTER-PI.md` — Pi parity checklist, session_new = generation bump, snapshot tier = partial
- `contracts/proto/patchbay/*.proto` — generated contract source (7 proto packages)
- `contracts/rust/`, `contracts/ts/` — generated Rust + TS bindings
- Formal models in `contracts/` — `command_lifecycle.qnt`, `session_generation.qnt`, `csrf_browser.qnt`, `elicitation_lifecycle.qnt`, `authority.qnt`, `patchbay-relational.als`

## Decomposition

Six child features, one per architectural layer. The coordination core is the largest and may decompose further during `feature-design`; the others are feature-sized.

### Child features

- `epic-v0-core` — Rust coordination core: durable event log, storage port, operation acceptance + idempotency, authority checks, snapshots, crash recovery — depends on: `[]` (epic-sized; decomposes into child features via `epic-design`)
- `feature-v0-protocol-seam` — web↔core internal protocol seam: internal RPC, streaming channel, auth boundary — depends on: `[epic-v0-core]`
- `feature-v0-pi-adapter` — Pi adapter: session discovery, prompt delivery, cancel/interrupt, replies/events/snapshots — depends on: `[epic-v0-core]`
- `feature-v0-web-server` — TS web server: HTTP termination, operator sessions, CSRF, speaks Connect to core — depends on: `[feature-v0-protocol-seam]`
- `feature-v0-web-cockpit` — responsive web cockpit: session list, composer, delivery states, reconnect — depends on: `[feature-v0-web-server]`
- `feature-v0-cli` — CLI: setup, admin, debug, scripted access — depends on: `[feature-v0-protocol-seam]`

## Stage correction (2026-07-14)

Advanced `drafting → implementing`. The decomposition was settled (complete child list + critical-path diagram + depends_on graph) but the `epic-design` Phase 8 stage advance was never applied — a stale stage, not a deliberate hold. Per the `epic-design` skill, an epic advances to `implementing` once its decomposition is done, not once all children are done.

### Child status
- `epic-v0-core` — **done** (root of the critical path; completed this session)
- `feature-v0-protocol-seam` — **done** (the web↔core gRPC seam; transport caveat retired by interop spike, then designed + implemented + reviewed this arc)
- `feature-v0-pi-adapter` — drafting (parallel branch off core)
- `feature-v0-web-server` — drafting (now unblocked: depends on the done seam)
- `feature-v0-web-cockpit` — drafting (depends on web-server)
- `feature-v0-cli` — drafting (now unblocked: depends on the done seam)

The epic is ~2/6 built by layer (core + seam). The web-server (root of the remaining phone-usable path) and the CLI are both now unblocked; the pi-adapter remains the parallel branch.
