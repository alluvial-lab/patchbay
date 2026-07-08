---
id: feature-research-v0-stack-tooling
kind: feature
stage: done
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

## Engagement record (2026-07-07, closed)

**Walk:** multi-specialist decomposed (4 facets), `scope_authority: in-engagement-judgment`, `verification_rigor: full` (operator-confirmed). Decomposition: 3 candidates drafted, Candidate B (by-decision, 4 specialists) chosen — isolates the flagged weak layer (Connect-ES seam) as its own facet. Decomposition rationale persisted above per ARD §10.6.

**Fan-out (4 parallel specialists, `openai-codex/gpt-5.5`, discipline bundle inlined per dispatch):**
- `internal-seam-connect` — Connect-ES Node client fit + tonic + browser-facing transport. 14 attestations.
- `rust-core-primitives` — SQLite/sled/libsql/cqrs-es + statig/smlang + tokio/proptest/tonic. 13 attestations.
- `ts-web-and-browser` — Fastify vs Hono/Elysia/Oak + session/CSRF hardening + XState/TanStack/Connect-Web. 16 attestations (3 prior attestations extended).
- `reference-control-planes` — no forkable precedent; partial analogues. 13 attestations.

**Gate outcomes (`full` rigor stack):**
- `lint` (hard floor): clean — attestation-tier audit zero findings; 118 resolved/non-broken citations in parent; 1 genuine unreachable (`gnu.org`, minor analog).
- `adversarial-read`: NEEDS-REVISION → revision pass 1 → NEEDS-REVISION (4 residuals) → revision pass 2 → re-verify pass 3 APPROVED.
- `evaluate` (isolated context): NEEDS-REVISION (4 findings — no Contradictions section, thin Hono/Elysia/Oak rationale, under-grounded novelty likelihood, TanStack not tied to reconnect) → revision → re-evaluation APPROVED (5/5/5/4/5).
- `spot-check` (lead): clean — citation-chain, lens-not-substrate, contradictions structural, composed-claim markers present.

**Key finding:** the v0 stack story is coherent and does NOT reopen any committed architectural decision. Connect-ES is usable as a Node client library (disconfirms the seed's TS-as-client worry); the internal seam is gRPC/HTTP2 to tonic. SQLite+WAL+`synchronous=FULL` fits the durable-log requirement (Patchbay owns the LSN invariant; `synchronous=NORMAL` is not durable enough). No forkable precedent exists for a deployment-neutral human control plane (Patchbay is genuinely novel in this niche).

**Side issue found and fixed:** the ARD linter's `url_alive()` used urllib's default `Python-urllib` User-Agent, which many docs hosts 403 → false-positive `unreachable-source` flags. Filed upstream as `fix/lint-url-alive-user-agent` (projects/skills worktree, off the ongoing `feat/pi-sandbox-first-party-bwrap` branch) and applied locally to the `.pi` install clone (UA fallback chain: browser UA first, fall back to python UA). Recovery: `cd /home/agent/.pi/agent/git/github.com/nklisch/skills && git reset --hard origin/main` once the upstream fix lands.

**Output paths:**
- Parent synthesis: `.research/analysis/campaigns/v0-stack-tooling/parent.md`
- Specialist briefs: `.research/analysis/campaigns/v0-stack-tooling/specialists/*.md`
- Acquisition manifest: `.research/analysis/campaigns/v0-stack-tooling/acquisitions.md` (3 enriching, 0 blocking)
- Verification checklist: `.research/analysis/campaigns/v0-stack-tooling/verification-checklist.md`
- Attestations: `.research/attestation/*.md` (~45 source-direct)

**Acquisition-offgas (operator-confirmed promotion gate):** 3 enriching candidates surfaced (connect-node/tonic interop spike; Codex generated app-server JSON schema; OpenCode `/doc` OpenAPI doc). NOT auto-promoted to the `.work/` acquisition queue — operator decides via `/agentic-research:research-handoff v0-stack-tooling`.

**Carry-forward (non-blocking):** re-verify the two unreachable sources at implementation time; run the connect-node/tonic interop spike before relying on the internal seam operationally. Both captured in the synthesis's Verification notes and Revisit-if.
