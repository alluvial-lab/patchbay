---
provenance: agent-synthesis
updated: 2026-07-07
campaign: v0-stack-tooling
engagement: feature-research-v0-stack-tooling
rigor: full
---

# V0 stack and tooling for Patchbay

Cross-specialist synthesis over four within-facet briefs (`internal-seam-connect`, `rust-core-primitives`, `ts-web-and-browser`, `reference-control-planes`). Citations resolve against `.research/attestation/`. This brief informs `feature-web-core-protocol-seam` (backlog) and the implementation features that follow the foundation-hardening arc.

## Bottom line

The v0 stack story is **coherent and does not reopen any committed architectural decision**. Each layer has a credible off-the-shelf primitive; none of the load-bearing picks is weak enough to revisit the topology, persistence, or walking-skeleton decisions. Patchbay's semantic core (LSN invariant, registry-driven state machines, authority) remains self-owned — the libraries are durable substrates, not protocol sources.

The recommended v0 stack, per layer, with the decision each grounds:

| Layer | Pick | Grounds |
|---|---|---|
| TS web framework | **Fastify** | First-party session/cookie/CSRF/WebSocket plugins + first-party Connect-ES Fastify server plugin path [fastify-session]{3} [fastify-csrf]{4} [fastify-websocket]{1} [connect-es-node]{6} |
| Browser→web-server RPC | **Connect-Web** (`@connectrpc/connect-web`) | Typed client, fetch transport, server-streaming async iterables [connect-es-web]{1} [connect-es-web]{5} |
| Web-server→Rust-core seam | **Connect-ES Node client over gRPC/HTTP2 to tonic** | Connect-ES is a first-class Node client library (not server-only); tonic is mature Rust gRPC [connect-node-client-transports]{1} [connect-node-client-transports]{6} [tonic-docs-current]{2} |
| Rust durable log | **SQLite (WAL, `synchronous=FULL`) via rusqlite** | Single-writer-friendly durable substrate; Patchbay owns the LSN invariant [sqlite-wal]{2} [sqlite-isolation]{2} [rusqlite]{3} [rusqlite]{4} |
| Rust async runtime | **tokio** | Non-blocking runtime covering timers, signals, fs, process [tokio]{4} [tokio]{5} |
| Rust gRPC server | **tonic** | Documented Rust gRPC-over-HTTP/2 implementation with streaming request/response types, built on hyper/tower/tokio [tonic-docs-current]{1} [tonic-docs-current]{5} |
| Rust property testing | **proptest** | Generate/shrink inputs, minimal failing-case shrinking, per-value strategy composition, state-machine testing in scope [proptest]{1} [proptest]{2} [proptest]{3} [proptest]{8} |
| Browser state machines | **Generated reducer first; XState only if generated/conformance-tested** | XState must not duplicate protocol registries; persistence caveats matter for long-lived sessions [xstate-docs]{3} [xstate-docs]{6} [xstate-docs]{12} |
| Browser server-state cache | **TanStack Query** (cache plumbing only) | `stale`/`paused` are library states, NOT authoritative protocol state [tanstack-query-docs]{1} [tanstack-query-docs]{3} |

Current fetched versions (refresh before implementation): `@connectrpc/connect`/`connect-node` 2.1.2, `@connectrpc/connect-web` 2.1.2, Fastify 5.10.0, `@fastify/session` 11.1.1, `@fastify/cookie` 11.0.2, `@fastify/csrf-protection` 8.0.0, `@fastify/websocket` 11.2.0, Hono 4.12.28, Elysia 1.4.29, XState 5.32.4, TanStack React Query 5.101.2, rusqlite 0.40.1, sqlx 0.9.0, libsql 0.9.30, sled 0.34.7, statig 0.4.1, smlang 0.8.0, tokio 1.52.3, proptest 1.11.0, tonic 0.14.6 [connect-npm-current]{1} [connect-node-npm-current]{1} [npm-registry-web-stack]{1} [npm-registry-web-stack]{2} [rusqlite]{10} [sqlx-sqlite]{9} [libsql-rust]{9} [sled]{11} [statig]{9} [smlang]{9} [tokio]{10} [proptest-docs]{1} [tonic]{13}.

## Decision-relevance: does this reopen any committed decision?

This was the engagement's depth gate (`decision_relevance`). Three decisions were at risk:

### `feature-web-core-protocol-seam` — NOT reopened (with one narrowing)

Connect-ES is usable as a **Node.js client library**, not only as a TypeScript server stack [connect-node-client-transports]{1} [connect-node-client-transports]{2}. This disconfirms the seed's worry that the TS-as-client-of-Rust topology might not fit Connect-ES. The cleanest internal topology is Connect-ES client over gRPC/HTTP2 to a tonic Rust server [connect-introduction-multiprotocol]{3} [connect-node-client-transports]{6} [tonic-docs-current]{2} {inferred: combines}.

The decision **reopens only if** it was understood as "the Rust core must speak the Connect protocol natively" — the fetched Connect implementation list does not name a Rust server [connect-introduction-multiprotocol]{6}, and tonic is gRPC, not Connect-protocol. If `feature-web-core-protocol-seam` meant "generated Connect-ES clients against a Rust gRPC server," it holds as stated. The one item to carry into that feature: the web↔core transport is **gRPC/HTTP2** (not native Connect-protocol-over-HTTP), which makes HTTP/2 a v0 deployment requirement for the internal channel [connect-node-client-transports]{4}.

### `feature-persistence-snapshot-model` — NOT reopened

SQLite with WAL mode and `synchronous=FULL` provides a single-writer, crash-recoverable, durable append substrate {inferred: fits-the-LSN-ordered-log-contract} [sqlite-wal]{2} [sqlite-wal]{6} [sqlite-isolation]{2} [sqlite-isolation]{3}. Patchbay owns the LSN-ordered log contract on top of that substrate (see caveat below); SQLite is the durable append layer, not the protocol semantics source. The storage-port abstraction (ARCHITECTURE Ports & Adapters) holds: SQLite sits behind the port, core domain logic does not depend on it [patchbay-architecture-v0-topology]{1}.

**Sharp caveat that must travel into implementation:** none of the fetched storage options provides Patchbay's `(authority_domain_id, LSN)` gap-free monotonic contract directly {inferred: fit}. The LSN invariant is a Patchbay-owned responsibility enforced at the storage-port boundary, not a database feature. Specifically:
- `synchronous=NORMAL` is **not** durable enough for the "no accepted command disappears silently" claim — it omits the per-commit sync and may roll back transactions after power loss [sqlite-wal]{6} [sqlite-wal]{8}. v0 must pin `synchronous=FULL` and test the acceptance boundary.
- sled's generated IDs are monotonic-but-not-contiguous → disqualified as an LSN allocator [sled]{7}; sled's durability is only up to the last `flush` [sled]{6}.
- `cqrs-es` event envelopes are aggregate-scoped (`aggregate_type + aggregate_id + sequence`) → cannot back a total authority-domain order [cqrs-es]{8} [cqrs-es]{9}; it is a domain-pattern candidate, not the storage primitive.

### `feature-v0-walking-skeleton` — NOT reopened

No reference project models a deployment-neutral human control plane for agent sessions with Patchbay's full semantic bundle (durable command acceptance/delivery/execution state, reconnectable snapshots, typed response slots, explicit authority across adapters) [claude-code-remote-control-current]{2} [cursor-cloud-agents-current]{3} [codex-appserver-current]{2} [opencode-server-docs-current]{4} {inferred: cross-source comparison}. The high-overlap precedents (Claude Code Remote Control, Cursor Cloud Agents, Codex app-server, OpenCode) are each product/vendor-bound control surfaces, not neutral authority-bearing cores. The walking-skeleton scope does not change: Patchbay remains self-grounded, not pattern-borrowed.

Partial analogues worth borrowing patterns from (not forking): process supervisors for state-axis separation and restart/backoff vocabulary [systemd-systemctl]{2} [systemd-service-semantics]{3} [supervisor-process-states]{1}; tmux/screen for attach/reconnect persistence [tmux-session-persistence]{3} [gnu-screen-session-persistence]{4}; Jupyter for `parent_header` correlation [jupyter-kernel-messaging]{2}; the vendor agent APIs for generated-contract and event-stream shapes [codex-appserver-current]{5} [opencode-server-docs-current]{8}.

## Open decision carried to `feature-web-core-protocol-seam`

The **browser-facing** transport (web server → browser) is left open by both relevant specialists and is correctly a decision for that feature, not this brief:

- **Connect server-streaming** preferred for typed, protocol-derived streams (keeps browser in the generated Connect-Web client stack) [connect-es-web]{2} [connect-es-web]{5}.
- **SSE** is a credible one-way fallback, but Fastify lacks a fetched first-party SSE helper → custom streaming or a separately-researched plugin {confidence: fetched-docs-only} [hono-docs]{9} [oak-docs]{9}.
- **WebSocket** only if bidirectional browser messaging is required; Fastify's `@fastify/websocket` is first-party but post-upgrade messages exit the HTTP response lifecycle (needs message-level authority) [fastify-websocket]{1} [fastify-websocket]{6}.
- Browser Connect-Web clients support **server-streaming only** (no client/bidi browser streaming) [connect-es-web]{2} — so a bidirectional browser channel forces WebSocket, not Connect.

This is distinct from the internal core↔web-server seam, which the evidence firmly fixes at gRPC/HTTP2.

## Cross-specialist convergence (no contradictions)

No direct source contradiction surfaced across or within the four briefs. The independent specialists converge on the same principles from different angles, which strengthens the recommendations:

- **tonic as Rust server half** — independently attested by `internal-seam-connect` (gRPC/HTTP2 fit) and `rust-core-primitives` (maturity + streaming) [tonic-docs-current]{2} [tonic]{1}.
- **Registry-as-SSOT for state machines** — `rust-core-primitives` (Rust side: hand-roll or generate registry-driven tables; statig/smlang only downstream [statig]{3} [smlang]{3}) and `ts-web-and-browser` (browser side: generated reducer first; XState must not duplicate protocol registries [xstate-docs]{3} [xstate-docs]{12}) reach the identical conclusion.
- **Connect-Web browser streaming limit** — independently confirmed by `internal-seam-connect` and `ts-web-and-browser` [connect-web-client-streaming]{2} [connect-es-web]{2}.

Within-specialist architectural tensions were surfaced honestly as `qualifies`/`incommensurable` (e.g. Connect-Web typed streaming vs WebSocket bidirectional messaging are incommensurable — adjacent but different problems [connect-es-web]{2} [fastify-websocket]{6}).

## Security wiring (consolidated, from `ts-web-and-browser`)

Fastify's security plugins are **not secure-by-default** — this is the single most important implementation caveat:

- `@fastify/cookie` does not default `HttpOnly` or `Secure` [fastify-cookie]{7} [fastify-cookie]{9}.
- `@fastify/session` defaults the cookie name to `sessionId` and the default in-memory store is not production-safe [fastify-session]{5} [fastify-session]{8}.
- `@fastify/csrf-protection` explicitly says CSRF security remains the developer's responsibility [fastify-csrf]{2}.

The hardened v0 config: cookie name `__Host-patchbay_session`, no `Domain`, `Path=/`, `Secure`, `HttpOnly`, `SameSite=Strict`, a production session store, `saveUninitialized: false` {extends: inferred from the default-true `saveUninitialized` to avoid persisting empty unauthenticated sessions}, a session-stored CSRF secret, a custom CSRF header extractor [fastify-csrf]{9}, and **separate** Origin/Fetch-Metadata checks (the CSRF package is a token utility, not a complete Origin/Fetch-Metadata policy) [fastify-session]{6} [fastify-csrf]{4} [owasp-csrf]{4} [owasp-csrf]{9} [mdn-set-cookie]{4} [mdn-set-cookie]{5} [owasp-session-management]{6} [owasp-session-management]{7}. OWASP requires ≥64-bit-entropy CSPRNG session IDs with application meaning stored server-side [owasp-session-management]{1} [owasp-session-management]{2}.

## Verification notes (re-verify candidates)

Two attestation handles were flagged `unreachable-source` by the lint and are **genuine** (not lint false positives — the linter's UA-fallback fix was applied and verified before this synthesis):

- `cursor-cloud-agents-current` — `cursor.com/docs/cloud-agent/api/endpoints` returns 403 to both browser and python UAs on GET (Cloudflare bot-wall or auth-gated). The specialist attested it (substantive, 27 citations), so the attestation stands as a record-of-fetch, but the URL cannot be independently re-verified from this host right now. Re-verify at implementation time if Cursor's API becomes load-bearing for the Pi adapter.
- `gnu-screen-session-persistence` — `www.gnu.org` is network-unreachable from this host (errno 101). Screen is a minor analog (terminal persistence), not load-bearing for the novelty finding. Re-verify opportunistically.

A broad web search for "human control plane" + "agent sessions" was attempted by the reference-control-planes specialist but reported as hitting an anti-bot challenge on DuckDuckGo's HTML endpoint. Spot-check from this host found DDG returns a clean 302 redirect (not a block), so the search may not have been as blocked as reported — the novelty finding is therefore scoped to the fetched corpus (four vendor agent-API sources + supervisor/terminal/IDE analogues), with a residual `{confidence: fetched-corpus-limited}` marker on the "no precedent exists" claim. The four fetched sources do not disconfirm novelty; a wider search might surface a closer analogue but is unlikely to given the niche.

## Acquisition candidates

Consolidated in `acquisitions.md`. Three enriching candidates (no blocking gaps): a Patchbay-local connect-node/tonic interop spike; Codex generated app-server JSON schema; OpenCode `/doc` OpenAPI document. Promotion to the `.work/` acquisition queue is operator-confirmed at the handoff gate (not auto-fired).

## What is genuinely custom vs off-the-shelf

- **Off-the-shelf:** web framework, session/CSRF/cookie plugins, Connect-ES clients (node + web), tonic, SQLite/rusqlite, tokio, proptest, TanStack Query.
- **Patchbay-owned (custom):** the LSN invariant and gap-free monotonic log contract; the registry-driven protocol state machines (CommandState, ElicitationState, session axes) and their generation to Rust enums/transition predicates/conformance tests; the cursor/subscription reconciliation model; authority/grants; the adapter capability manifest; the storage port abstraction.
- **Downstream-of-registry (optional crate use):** statig/smlang for non-authoritative runtime/session machines; XState for complex generated/conformance-tested browser workflow orchestration.

## Revisit if

- `feature-web-core-protocol-seam` requires the Rust core to serve the Connect protocol specifically (not just gRPC reachable by Connect-ES clients) [connect-introduction-multiprotocol]{6}.
- SQLite with `synchronous=FULL` cannot meet v0 latency targets once acceptance/snapshot/replay tests run on realistic hardware [sqlite-wal]{6}.
- A spike shows `@connectrpc/connect-node` `createGrpcTransport()` cannot interoperate cleanly with generated tonic services.
- v0 deployment cannot reliably provide HTTP/2 between the TS web server and Rust core [connect-node-client-transports]{4}.
- A maintained Rust Connect-protocol implementation becomes available and is preferred over tonic gRPC after source-backed review.
- A new project appears that explicitly models human-operated, reconnectable, durable command control for heterogeneous autonomous agent sessions (would reopen the novelty finding).

## Specialist briefs

- `specialists/internal-seam-connect.md` — Connect-ES client fit + tonic + browser-facing transport alternatives.
- `specialists/rust-core-primitives.md` — SQLite/sled/libsql/cqrs-es + statig/smlang + tokio/proptest/tonic.
- `specialists/ts-web-and-browser.md` — Fastify vs Hono/Elysia/Oak + session/CSRF/cookie hardening + XState/TanStack Query/Connect-Web.
- `specialists/reference-control-planes.md` — no forkable precedent; partial analogues to borrow patterns from.
