---
id: story-connect-node-tonic-interop-spike
kind: story
stage: drafting
tags: [protocol, verification, adapter, foundation]
parent: null
depends_on: []
created: 2026-07-07
updated: 2026-07-07
gate_origin: null
release_binding: null
research_origin: v0-stack-tooling
---

# Spike: @connectrpc/connect-node ↔ tonic interop validation

Retire the one residual operational caveat from the `v0-stack-tooling` research engagement's headline finding before the web↔core topology is committed in code.

## Origin

Surfaced as an enriching acquisition candidate by the `v0-stack-tooling` campaign (`internal-seam-connect` facet). Recategorized on operator direction (2026-07-07) from a research acquisition to a **pre-implementation validation story** — it is a validation activity (run the library pair and observe), not a source to acquire.

## Why this exists

The engagement's load-bearing claim — *the v0 internal seam is Connect-ES Node client over gRPC/HTTP2 to a tonic Rust server, and this does not reopen `feature-web-core-protocol-seam`* — rests on **generic** protocol-compatibility evidence:

- Connect clients can call any gRPC server (Connect-ES introduction) [connect-introduction-multiprotocol]{3}.
- `@connectrpc/connect-node` provides `createGrpcTransport()` over HTTP/2 [connect-node-client-transports]{6}.
- tonic is a documented Rust gRPC-over-HTTP/2 server [tonic-docs-current]{2}.

No fetched source records this **exact library pair** interoperating under realistic service shapes. The synthesis's own `## Revisit if` names this as a live trigger:

> A spike shows `@connectrpc/connect-node` `createGrpcTransport()` cannot interoperate cleanly with generated tonic services.

This spike either confirms the claim (removes the last caveat from the engagement's headline finding) or fires the trigger early, before `feature-web-core-protocol-seam` and the Rust-core implementation commit to the gRPC/HTTP2 topology.

## Scope

Build a minimal, throwaway interop harness proving the Connect-ES-Node-client → tonic-server pair under conditions `feature-web-core-protocol-seam` will actually exercise:

1. **Unary RPC** — a `tonic` server exposing one unary method; a `@connectrpc/connect-node` client (`createGrpcTransport()`, HTTP/2) calling it. Verify request/response round-trip with a generated Protobuf contract (reuse the existing `contracts/` `.proto` package — pick one small service or define a one-method spike service).
2. **Server-streaming RPC** — model the `Subscribe(cursor) returns (stream CoreEvent)` shape the synthesis recommends; verify the client receives an async iterable of events over HTTP/2.
3. **Error mapping** — return a tonic error with status/details; verify `@connectrpc/connect-node` surfaces it as a structured Connect error (not an opaque transport failure). This is the most likely interop seam to break.
4. **Metadata propagation** — pass gRPC metadata (the operator-session / CSRF-evidence-forwarding shape `feature-web-core-protocol-seam` will need) from client to server; verify it arrives.
5. **TLS** — verify the pair works over TLS, since the internal channel in a non-localhost deployment is TLS-terminated.

### Out of scope

- Full `feature-web-core-protocol-seam` design (RPC surface inventory, streaming taxonomy, web-server-as-principal auth). This spike only validates the transport interop; the seam *design* is the feature's job.
- Browser-facing transport (Connect-Web → web server). That's a separate surface; this spike is internal-seam only.
- Performance/load testing. A spike confirms interop, not throughput.

## Acceptance criteria

- [ ] Unary RPC round-trips: client `createGrpcTransport()` + `createClient()` calls a tonic unary method, receives the generated response.
- [ ] Server-streaming works: `Subscribe`-style streaming RPC delivers an async iterable of events to the Node client over HTTP/2.
- [ ] Error mapping confirmed: tonic errors with details surface as structured Connect errors on the client side (document the shape).
- [ ] Metadata propagation confirmed: client-set metadata arrives at the tonic handler (document what survives).
- [ ] TLS interop confirmed (or, if not feasible in the spike, an explicit documented gap + revisit trigger).
- [ ] Spike report committed to `.research/notes/` (or `.research/precis/`) recording: library versions used, what worked, any interop gap found, and an explicit verdict on whether the synthesis's revisit-trigger is **retired** or **fired**.
- [ ] If fired: file the follow-up (the synthesis's revisit condition becomes a live design problem for `feature-web-core-protocol-seam`).

## Relationship to the research engagement

- `research_origin: v0-stack-tooling` — this story discharges the engagement's residual acquisition candidate and its #1 revisit-trigger.
- `research_refs: [v0-stack-tooling]` (implicit via `research_origin`).
- The engagement's `parent.md` synthesis and `specialists/internal-seam-connect.md` carry the attested source ground for the generic claims; this spike validates the pair-specific claim that no source could attest.

## Routing

This is a `story` (behavior-validating spike), not a `[research]` engagement — it does not re-enter the research-orchestrator. It routes through `implement` (small, self-contained, ~50-150 LoC throwaway harness) when picked up. No design gate needed; the acceptance criteria are the spec.

Pick up when `feature-web-core-protocol-seam` moves from backlog to active design (it should run *before* or *early in* that feature's implementation, since the topology commitment depends on its outcome). It can also run earlier as a standalone spike if the operator wants the caveat retired now.
