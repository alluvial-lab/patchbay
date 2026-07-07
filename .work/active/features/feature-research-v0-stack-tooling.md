---
id: feature-research-v0-stack-tooling
kind: feature
stage: drafting
tags: [research, protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-research-contract-tooling]
created: 2026-06-28
updated: 2026-07-07
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

## Registration (dispatch-time, ARD SPEC §9)

The commissioning subset rode this item at scope time (2026-06-28). The remaining fields settled at dispatch (2026-07-07 kickoff, operator-confirmed):

```yaml
intent: inform-architecture-decision
output_kind: synthesis-brief
consumer: future-agent           # feature-web-core-protocol-seam + implementation features
verification_rigor: full         # operator-confirmed at kickoff (two facets can reopen architecture)
temporal_contract: write-once-on-converge
primitives_extends: []
primitives_opts_out: []
decision_relevance: |
  If this finds Connect-ES server-side does not fit a TS process that is a client of a
  Rust core, the internal seam in feature-web-core-protocol-seam changes (possibly to a
  non-Connect internal protocol). If it finds the Rust durable-log story is weak, the v0
  persistence assumption in feature-persistence-snapshot-model reopens. If it finds a
  reference control-plane project worth forking, v0 walking-skeleton scope changes.
scope_authority: in-engagement-judgment
analytical_artifact_type: per-campaign-brief
```

## Substrate-check (decompose prelude)

Three prior `.research/` artifacts overlap this engagement's seed:

- `.research/analysis/briefs/protocol-contract-tooling.md` — grounded Protobuf/Buf/prost/Protobuf-ES/Connect-ES. The contract layer is a **refresh** of this prior ground, not a fresh acquisition. The prior artifact is a **lens, not a substrate**: the new engagement builds on its settled ground but must not cite it as a `[handle]{N}` source.
- `.research/analysis/briefs/web-control-security.md` — grounded server-side sessions, `__Host-` cookies, SameSite=Strict, CSRF. Relevant to the TS web-framework + session/CSRF library questions; same lens discipline.
- `.research/analysis/campaigns/harness-action-surfaces/` — mapped 6 harness action surfaces. Relevant context for the reference-projects facet.

No refresh re-engagement registration (supersedes-prior) is opened: the prior artifacts are loaded as lenses framing what to re-engage, not superseded. The contract-layer facet's specialist observes the lens-not-substrate guard from the discipline bundle.

## Decomposition (Checkpoint A — operator-confirmed 2026-07-07)

`scope_authority: in-engagement-judgment` → decomposition is emergent; the chosen rationale persists back onto this item per ARD SPEC §10.6.

Three candidate decompositions drafted against the seed (4 named layers + reference-projects facet):

- **Candidate A — by-layer (5 specialists mirroring the seed's named layers).** Each layer its own facet. Rejected: Connect-ES appears in 3 facets (web, rust, browser) → risk of fragmented/contradictory assessment of the same library across specialists; the reference-projects facet is speculative and may return thin as a standalone.
- **Candidate B —-decision (4 specialists organized around the decisions that can reopen architecture).** (1) Internal seam: Connect-ES fit for TS-as-client-of-Rust — the flagged weak layer, refreshes contract-tooling; (2) Rust core durability primitives (log/state-machine — can reopen persistence-snapshot-model); (3) TS web + browser operator-domain (framework + session/CSRF + XState/TanStack — refreshes web-control-security); (4) reference control-plane projects. **Chosen.**
- **Candidate C — by-domain (3 specialists: Rust, TS, external-landscape).** Rejected: the TS domain specialist would conflate two distinct deployment contexts (server framework vs browser state-management) — shallow treatment of one; the critical Connect-ES seam question gets split across two domains rather than isolated.

**Choice: B.** Facets align with the engagement's decision-relevance. The seed itself flags the Connect-ES/seam question as the weak layer that can reopen `feature-web-core-protocol-seam`; isolating it as its own facet gives the highest-risk decision focused depth and lets adversarial-read + evaluator target it. The TS-web + browser grouping under B is the one weak spot; mitigated by scoping that specialist's brief to treat them as two sub-questions sharing security constraints, not one merged question. 4 specialists keeps fan-out tractable while preserving decision-aligned boundaries.

### Facet dispatch (4 parallel specialists, `full` rigor)

| # | facet | seed scope | lens (prior art) |
|---|---|---|---|
| 1 | `internal-seam-connect` | Connect-ES server-side fit for TS-as-client-of-Rust; Connect-over-HTTP vs gRPC; streaming channel for events/replies; the flagged weak layer | `protocol-contract-tooling.md` (lens) |
| 2 | `rust-core-primitives` | Durable log/event-sourcing (SQLite/LibSQL/sled/custom WAL); state-machine crates (statig/smlang) vs hand-rolled; tokio; proptest; tonic server-side | — |
| 3 | `ts-web-and-browser` | TS web framework (Hono/Fastify/Elysia); session/CSRF/cookie libs; WS/SSE; browser: XState, TanStack Query, Connect-ES client; two sub-questions sharing security constraints | `web-control-security.md` (lens) |
| 4 | `reference-control-planes` | Does any project model a human control plane for agent sessions? Patterns from process supervisors, remote-control tools, harness-management | `harness-action-surfaces/` (context) |

## Relationship to committed decisions

- The v0 two-process topology (`docs/ARCHITECTURE.md` "V0 process topology") is settled; this research grounds its implementation choices, not the topology itself.
- The contract layer (Protobuf+Buf) was grounded by `feature-research-contract-tooling`; this engagement deepens the Connect-ES and prost server-side questions that the two-process topology raises.
- Findings feed `feature-web-core-protocol-seam` (backlog) and the implementation features that follow.
