---
id: feature-research-v0-stack-tooling
kind: feature
stage: drafting
tags: [research, protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-research-contract-tooling]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
research_dials:
  scope_authority: in-engagement-judgment
  verification_rigor: standard
  intent: inform-architecture-decision
  output_kind: synthesis-brief
---

# Research: v0 stack and tooling picks for Patchbay implementation

Before implementation locks in concrete library/framework/backend choices, ground the v0 stack picks with attested sources. The contract layer was partially grounded by `feature-research-contract-tooling`; this engagement covers the remaining layers and deepens the contract-layer questions that the v0 two-process topology raises.

## Seed questions

### TS web-server stack
- Which TS web framework (Hono, Fastify, Elysia, Deno-native) best fits a control-surface process that terminates HTTP, owns cookies/session/CSRF, and speaks Connect/Protobuf to a Rust core?
- Which session/CSRF/cookie libraries match `docs/SECURITY.md` v0 requirements (server-side sessions, `__Host-` cookies, SameSite=Strict, CSRF tokens tied to sessions)?
- How well does Connect-ES server-side runtime fit a TS process that is a *client* of a Rust core (not a Rust server)? Is the internal seam naturally Connect-over-HTTP, Connect-over-gRPC, or something else?
- WebSocket / SSE support for streaming events/replies from core through the web server to the browser.

### Rust core primitives
- Durable log / event-sourcing options in Rust: SQLite/LibSQL, sled, custom WAL, or event-sourcing crates. Which fit a single-writer, local-first, LSN-ordered log?
- State-machine crates (`statig`, `smlang`) for `CommandState` / session-axis machines — or is hand-rolled better for the protocol registries?
- Async runtime (tokio) and property-test tooling (proptest) fit.
- Connect server-side in Rust: maturity, streaming support, fit for the internal protocol seam.

### Browser operator-domain primitives
- State-machine libraries (XState) and reconnect/cache patterns (TanStack Query) — reusable for delivery/reconnect/session-presentation state machines, or better hand-rolled?
- Connect-ES client fit for the browser talking to the TS web server.
- What is genuinely custom (Patchbay-specific) vs. reusable primitives.

### Reference control-plane / agent-session projects (high-value unknown)
- Does any existing project model a "human control plane for agent sessions" that Patchbay could learn from or fork? (Not an LLM orchestrator, not a remote-desktop tool, not a generic process supervisor.)
- Are there relevant patterns from process supervisors, remote-control tools, or harness-management projects that inform the architecture?

## Expected output

A `.research/analysis/briefs/` synthesis brief with source-grounded recommendations per layer, identifying what is off-the-shelf vs. custom, and flagging any layer where the tooling story is weak enough to revisit the architectural choice (e.g., if Connect-ES server-side doesn't fit the two-process topology, that affects `feature-web-core-protocol-seam`).

Follow-up work items may be emitted only after operator confirmation.

## Relationship to committed decisions

- The v0 two-process topology (`docs/ARCHITECTURE.md` "V0 process topology") is settled; this research grounds its implementation choices, not the topology itself.
- The contract layer (Protobuf+Buf) was grounded by `feature-research-contract-tooling`; this engagement deepens the Connect-ES and prost server-side questions that the two-process topology raises.
- Findings feed `feature-web-core-protocol-seam` (backlog) and the implementation features that follow.
