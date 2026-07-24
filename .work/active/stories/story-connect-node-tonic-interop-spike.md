---
id: story-connect-node-tonic-interop-spike
kind: story
stage: done
tags: [protocol, verification, adapter, foundation]
parent: null
depends_on: []
created: 2026-07-07
updated: 2026-07-15
gate_origin: null
release_binding: v0.1.0
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

- [x] Unary RPC round-trips: client `createGrpcTransport()` + `createClient()` calls a tonic unary method, receives the generated response.
- [x] Server-streaming works: `Subscribe`-style streaming RPC delivers an async iterable of events to the Node client over HTTP/2.
- [x] Error mapping confirmed: tonic errors with details surface as structured Connect errors on the client side (document the shape).
- [x] Metadata propagation confirmed: client-set metadata arrives at the tonic handler (document what survives).
- [x] TLS interop confirmed (or, if not feasible in the spike, an explicit documented gap + revisit trigger).
- [x] Spike report committed to `.research/notes/` (or `.research/precis/`) recording: library versions used, what worked, any interop gap found, and an explicit verdict on whether the synthesis's revisit-trigger is **retired** or **fired**.
- [ ] If fired: file the follow-up (the synthesis's revisit condition becomes a live design problem for `feature-web-core-protocol-seam`).
  — N/A: verdict is RETIRED, not fired. No follow-up filed.

## Relationship to the research engagement

- `research_origin: v0-stack-tooling` — this story discharges the engagement's residual acquisition candidate and its #1 revisit-trigger.
- `research_refs: [v0-stack-tooling]` (implicit via `research_origin`).
- The engagement's `parent.md` synthesis and `specialists/internal-seam-connect.md` carry the attested source ground for the generic claims; this spike validates the pair-specific claim that no source could attest.

## Routing

This is a `story` (behavior-validating spike), not a `[research]` engagement — it does not re-enter the research-orchestrator. It routes through `implement` (small, self-contained, ~50-150 LoC throwaway harness) when picked up. No design gate needed; the acceptance criteria are the spec.

Pick up when `feature-web-core-protocol-seam` moves from backlog to active design (it should run *before* or *early in* that feature's implementation, since the topology commitment depends on its outcome). It can also run earlier as a standalone spike if the operator wants the caveat retired now.

## Implementation notes

- Execution capability: inline host session (throwaway validation spike; one cohesive deliverable, no fan-out warranted). `~50-150 LoC` estimate in the brief was accurate — the harness is ~180 LoC Rust + ~170 LoC TS.
- Review weight: standalone-story lane (bounded inline review; no independent/cross-model reviewer per the implement skill's standalone-story rule).
- Files changed:
  - `spikes/connect-tonic-interop/proto/spike.proto` (minimal spike service: Submit unary, Subscribe server-streaming, SubmissionFailureDetail for error-detail condition)
  - `spikes/connect-tonic-interop/rust/` (Cargo.toml, build.rs, src/main.rs — tonic server, plain + tls modes)
  - `spikes/connect-tonic-interop/ts/` (package.json, tsconfig.json, buf.gen.yaml, src/run.ts — connect-node client harness)
  - `spikes/connect-tonic-interop/tls/{cert,key}.pem` (self-signed, for the TLS condition)
  - `.research/notes/2026-07-15-connect-node-tonic-interop-spike.md` (the spike report)
  - `.gitignore` (added throwaway spike build artifacts + writable cargo home)
- Tests added: none in the repo's test suite (this is a throwaway spike, not production code). The harness IS the test: it exits 0 only if all 5 conditions pass. Verified green in both h2c and TLS modes.
- Discrepancies from design: none. The acceptance criteria were the spec; all met.
- Adjacent issues parked: none. The one non-obvious finding (prost structs don't derive `prost::Name`, so `type_url()` was unavailable for the custom error detail — used a manual `type.googleapis.com/...` string instead) is recorded in the spike report as an implementation-ergonomics note for `feature-v0-protocol-seam`, not a backlog item. It is not an interop failure.

### Environment notes (for the next spike / the seam implementation)

- The prior session note's `CARGO_HOME=/tmp/cargo-home` is **stale**: that cache is now on a read-only layer (EROFS on write). It holds the 86 core-deps vendored, but cannot accept new crates. A project-local writable cargo home (`.cargo-home/`, gitignored) is required for any build that fetches new crates (tonic, tonic-prost, tonic-types, hyper, etc. were all fetched fresh here). `/tmp` is also read-only.
- npm similarly needs `--cache <project-local-dir>` because `~/.npm/_cacache` is read-only.
- Network is open: crates.io (200 with a UA), registry.npmjs.org (200), static.crates.io (200). Fetching works once the cache is writable.
- Only `/home/agent/projects/patchbay` is writable in this sandbox; `/tmp`, `/home/agent`, and `~/.cargo` are read-only layers.

### Library-version note for the seam

The spike confirmed `tonic-prost` (runtime codec) and `tonic-types` are real runtime deps the Rust core will need, not just `tonic-prost-build` for codegen. The seam/web-server implementation should declare `tonic-prost` and `tonic-types` explicitly.
