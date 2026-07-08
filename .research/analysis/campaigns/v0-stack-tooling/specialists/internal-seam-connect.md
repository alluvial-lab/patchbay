---
provenance: agent-synthesis
updated: 2026-07-07
campaign: v0-stack-tooling
facet: internal-seam-connect
---

# Internal seam: TypeScript web server as client of Rust coordination core

## Bottom line

Connect-ES fits the TypeScript side of the internal seam well **as a Node.js client library**, not only as a TypeScript server stack. Connect for Node explicitly reuses the same generated clients as Connect for Web, swaps in `@connectrpc/connect-node` transports, and constructs clients with `createClient(service, transport)` [connect-node-client-transports]{1} [connect-node-client-transports]{2}. The current npm latest versions fetched for this engagement are `@connectrpc/connect` 2.1.2 and `@connectrpc/connect-node` 2.1.2, with `connect-node` requiring Node `>=20` and a peer dependency on the matching `@connectrpc/connect` version [connect-npm-current]{1} [connect-node-npm-current]{1} [connect-node-npm-current]{3} [connect-node-npm-current]{4}.

The cleanest internal topology is **Connect-ES client API over gRPC/HTTP2 to a tonic Rust gRPC server**, not native Connect-protocol-over-HTTP to Rust. {inferred: combines} Connect's docs say Connect clients can call any gRPC server and clients can switch from Connect protocol to gRPC with a configuration toggle [connect-introduction-multiprotocol]{3} [connect-introduction-multiprotocol]{5}; Connect for Node documents `createGrpcTransport()` and says the gRPC transport requires HTTP/2 [connect-node-client-transports]{6}. Tonic, meanwhile, is a Rust gRPC-over-HTTP/2 implementation with server and channel transport pieces built on tokio, hyper, and tower [tonic-docs-current]{2} [tonic-docs-current]{4} [tonic-docs-current]{6}. That pairing avoids depending on an official Rust Connect-protocol server; the Connect introduction's current implementation list names Go, TypeScript/JavaScript, Swift/Kotlin, and Python, but not Rust [connect-introduction-multiprotocol]{6}.

This should not reopen `feature-web-core-protocol-seam` if that decision means “generated Protobuf contract with Connect-ES clients.” It should reopen or narrow the decision if it means “the Rust core must speak the Connect protocol natively.” {inferred: design implication} The evidence supports generated Connect-ES clients against a Rust gRPC server; it does not establish a first-party Rust Connect protocol server.

## Seam requirements from Patchbay sources

Patchbay v0 has a two-process topology: a Rust coordination core and a TypeScript web server [patchbay-architecture-v0-topology]{1}. The Rust core is the single authoritative process for the durable event log, Operation acceptance, authority checks, snapshots, and the storage port [patchbay-architecture-v0-topology]{2}. The TypeScript web server terminates browser HTTP/HTTPS, owns operator sessions/cookies/CSRF, and speaks the generated Protobuf/Connect contract to the Rust core [patchbay-architecture-v0-topology]{3}. The architecture explicitly reserves the web↔core protocol surface, streaming/event channel, operator-session/CSRF evidence crossing, and web-surface authentication to the core for follow-on design [patchbay-architecture-v0-topology]{5}.

The internal seam must carry Patchbay's cursor/subscription model. The protocol defines a single totally ordered durable event log per authority domain, with event/cursor identity shaped as `(authority_domain_id, LSN)` [patchbay-protocol-subscription-cursors]{1} [patchbay-protocol-subscription-cursors]{2}. Reconnecting clients submit a cursor, and the core returns events with `LSN > cursor` and/or a fresh snapshot [patchbay-protocol-subscription-cursors]{3} [patchbay-protocol-subscription-cursors]{6}. Subscriptions are long-lived, grant-checked transport establishments rather than lifecycle-bearing Operations [patchbay-protocol-subscription-cursors]{5}.

## Connect-ES client/server orientation

Connect-Node has a server-side story: its getting-started guide opens by describing Connect-Node as a library for serving Connect, gRPC, and gRPC-Web-compatible HTTP APIs in Node.js, and the guide builds a Fastify server using `@connectrpc/connect-node` and `@connectrpc/connect-fastify` [connect-node-server-getting-started]{1} [connect-node-server-getting-started]{3} [connect-node-server-getting-started]{4}. That server-oriented documentation is real, but it is not the whole library posture: the same guide later consumes the service with a Node Connect client using `createClient` and `createConnectTransport` [connect-node-server-getting-started]{5}, and the dedicated Node client page is explicitly about using generated clients on Node with `@connectrpc/connect-node` transports [connect-node-client-transports]{1}.

For the Patchbay internal seam, the relevant Connect-ES feature is therefore the transport abstraction. The Node client docs state that with HTTP/2, clients can use the Connect, gRPC, or gRPC-Web protocol and call all RPC types; with HTTP/1.1, gRPC and bidirectional streaming are not supported [connect-node-client-transports]{4}. That means a TS web-server process can be a first-class client of a Rust server as long as the deployment accepts HTTP/2 for the internal channel.

## Recommended internal protocol shape

Use one `.proto` service family for the Rust core boundary, generate TypeScript clients, and run the TypeScript web server as a Connect-ES client using `createGrpcTransport()` over HTTP/2 to tonic. {inferred: design recommendation} This aligns the TypeScript side with Connect-ES's client API [connect-node-client-transports]{1} [connect-node-client-transports]{6} and the Rust side with tonic's released gRPC/HTTP2 server implementation [tonic-docs-current]{1} [tonic-docs-current]{2} [tonic-docs-current]{6}.

Do **not** make native Connect protocol support a v0 requirement for the Rust core unless a Rust Connect server implementation is deliberately selected and re-attested. {inferred: negative recommendation} Connect's own introduction says Connect protocol is one of three supported protocols [connect-introduction-multiprotocol]{2}, but its implementation list fetched here does not name Rust [connect-introduction-multiprotocol]{6}; tonic is attested as gRPC over HTTP/2, not as a Connect protocol server [tonic-docs-current]{2}.

For unary command/query RPCs, either Connect protocol or gRPC could satisfy the API shape, but the Rust server evidence points to gRPC. For subscription delivery, model the main stream as a server-streaming or bidirectional gRPC method rather than a separate hand-rolled event socket. {inferred: maps protocol to seam} Connect protocol itself supports unary, client-streaming, server-streaming, and bidirectional-streaming RPCs over Protobuf/JSON, with bidirectional streaming requiring HTTP/2 [connect-protocol-reference]{4} [connect-protocol-reference]{5}; Node Connect clients can call all RPC types over HTTP/2 [connect-node-client-transports]{4}; tonic lists streaming requests/responses and its README feature list includes bidirectional streaming [tonic-docs-current]{7} [tonic-readme-current]{7}.

A natural v0 subscription RPC is:

```proto
rpc Subscribe(SubscribeRequest) returns (stream CoreEvent);
```

where `SubscribeRequest` carries the authority domain and cursor. {inferred: applies source semantics} This matches Patchbay's rule that reconnecting clients submit a cursor and receive `LSN > cursor` events or a fresh snapshot [patchbay-protocol-subscription-cursors]{3} [patchbay-protocol-subscription-cursors]{6}. If the web server needs to send flow-control acknowledgements, dynamic filter changes, or heartbeat/control messages inside the same long-lived call, use a bidirectional streaming method over HTTP/2 instead; the fetched sources establish that Node Connect over HTTP/2 and tonic both cover bidirectional streaming [connect-node-client-transports]{4} [tonic-readme-current]{7}.

## Browser-facing delivery is a separate seam

The internal core→web-server stream should not be constrained by browser transport limits. The TypeScript web server can bridge durable core events to the browser using a browser-appropriate transport while still using gRPC/HTTP2 internally. Connect for Web supports generated clients and server-streaming methods as async iterables for promise clients [connect-web-client-streaming]{1} [connect-web-client-streaming]{3}. If browser delivery is only core-to-browser notifications, SSE is a credible browser-facing option because MDN describes it as server-to-front-end streaming via `EventSource`, but it is one-way and cannot carry client-to-server events on the same channel [mdn-sse-browser-events]{1} [mdn-sse-browser-events]{2} [mdn-sse-browser-events]{3}. If browser delivery needs symmetrical low-latency client/server messages, WebSocket is credible because MDN describes it as a two-way browser/server session [mdn-websocket-browser-events]{1} [mdn-websocket-browser-events]{2}, but the stable `WebSocket` interface lacks backpressure [mdn-websocket-browser-events]{4} [mdn-websocket-browser-events]{5} [mdn-websocket-browser-events]{6}.

Raw gRPC-Web is less attractive as the primary internal seam because it is browser-oriented and currently supports only unary and server-side streaming, with no client-side or bidirectional streaming [grpc-web-readme-streaming]{1} [grpc-web-readme-streaming]{3} [grpc-web-readme-streaming]{4} [grpc-web-readme-streaming]{6}. It remains useful as an ingress/browser compatibility protocol where a Connect server supports it out of the box [connect-introduction-multiprotocol]{2} [connect-introduction-multiprotocol]{5}, but Patchbay's Node-to-Rust internal channel has a cleaner gRPC/HTTP2 path.

## Alternatives if Connect-ES does not fit

1. **Plain tonic gRPC client in Node without Connect-ES** — not the preferred path because Connect-ES already provides generated TypeScript clients and a gRPC transport [connect-node-client-transports]{1} [connect-node-client-transports]{6}. Use only if a spike finds Connect-ES transport/runtime issues against tonic. {inferred: contingent alternative}
2. **JSON-over-HTTP plus SSE** — credible for a minimal operational seam, especially if the browser and web server want human-debuggable JSON, since Connect unary RPCs themselves are HTTP/JSON-friendly [connect-protocol-reference]{7} and SSE is simple one-way event delivery [mdn-sse-browser-events]{2} [mdn-sse-browser-events]{7}. The cost is hand-owning schema/versioning, framing, errors, and streaming semantics that Connect/gRPC otherwise provide. {inferred: tradeoff}
3. **WebSocket with hand-rolled framed JSON or Protobuf** — credible if bidirectional browser and server behavior dominates; WebSocket supplies two-way browser/server communication [mdn-websocket-browser-events]{1} [mdn-websocket-browser-events]{2}. The cost is custom flow/error/reconnect/cursor semantics, and stable browser WebSocket lacks backpressure [mdn-websocket-browser-events]{5} [mdn-websocket-browser-events]{6}. {inferred: tradeoff}
4. **Native Connect protocol in Rust** — not established by fetched current Connect implementation docs. The Connect introduction lists current implementation sections without Rust [connect-introduction-multiprotocol]{6}. Revisit if a Rust Connect server implementation becomes an explicit dependency candidate and passes conformance/maintenance review.

## Disconfirming analysis

Evidence against a naive "Connect-ES means TS-as-server" concern: the Node client docs explicitly say Node uses the same generated clients as Connect for Web with `@connectrpc/connect-node` transports [connect-node-client-transports]{1}, show a `createClient` Node example [connect-node-client-transports]{2}, and document Connect/gRPC/gRPC-Web client transports [connect-node-client-transports]{5} [connect-node-client-transports]{6} [connect-node-client-transports]{7}. This disconfirms the idea that Connect-ES is only usable when the TypeScript process is the server.

Evidence against native Connect-over-HTTP as the Rust-core default: the fetched Connect implementation page does not list Rust among the current language implementations [connect-introduction-multiprotocol]{6}, while the Rust source fetched here is tonic, a gRPC-over-HTTP/2 implementation [tonic-docs-current]{2}. This pushes the internal seam toward Connect-ES-over-gRPC rather than Connect-protocol-to-Rust.

Evidence against using browser transport constraints for the internal seam: the Node client docs say HTTP/2 clients can call all RPC types [connect-node-client-transports]{4}, while grpc-web's README says browser grpc-web lacks client-side and bidirectional streaming [grpc-web-readme-streaming]{6}. Since Patchbay's internal client is the Node/TypeScript web server, browser grpc-web limitations do not block the core↔web-server seam.

Evidence against hand-rolled streaming as the default: Connect protocol and Connect for Node already cover streaming RPC types over HTTP/2 [connect-protocol-reference]{4} [connect-protocol-reference]{5} [connect-node-client-transports]{4}, and tonic attests streaming support on the Rust side [tonic-docs-current]{7} [tonic-readme-current]{7}. A custom WebSocket/SSE framing should therefore be a fallback for a failed interop spike, not the first choice. {inferred: prioritization}

## Contradictions

No direct source contradiction surfaced. The important tension is architectural rather than factual:

- Connect-Node's getting-started guide foregrounds serving APIs in Node.js [connect-node-server-getting-started]{1} [connect-node-server-getting-started]{4}.
- Connect for Node's client page separately establishes Node as a generated-client runtime with Connect/gRPC/gRPC-Web transports [connect-node-client-transports]{1} [connect-node-client-transports]{4}.
- Tonic is mature evidence for Rust gRPC, not evidence for Rust native Connect protocol [tonic-docs-current]{1} [tonic-docs-current]{2}.

The resulting resolution is not "Connect-ES does not fit"; it is "use Connect-ES client ergonomics over gRPC/HTTP2 for the Rust seam." {inferred: resolution}

## Revisit if

- `feature-web-core-protocol-seam` requires the Rust core to serve the Connect protocol specifically, not just gRPC reachable by Connect-ES clients.
- A spike shows `@connectrpc/connect-node` `createGrpcTransport()` cannot interoperate cleanly with generated tonic services for Patchbay's service shapes, metadata, TLS, streaming, or error details.
- V0 deployment cannot reliably provide HTTP/2 between the TypeScript web server and Rust core; Node docs make HTTP/2 the condition for gRPC and bidirectional streaming [connect-node-client-transports]{4} [connect-node-client-transports]{6}.
- The subscription model requires mid-stream client-to-core control messages in the same call; then server-streaming should be promoted to bidirectional streaming, which keeps HTTP/2 mandatory [connect-protocol-reference]{5}.
- A maintained Rust Connect-protocol implementation becomes available and is preferred over tonic gRPC after source-backed review.
- Browser-facing delivery, not internal core↔web-server delivery, becomes the dominant constraint; then compare Connect-Web server-streaming, SSE, and WebSocket as a separate surface seam [connect-web-client-streaming]{3} [mdn-sse-browser-events]{2} [mdn-websocket-browser-events]{1}.

## Acquisition candidates

No load-bearing source was blocked. Enriching follow-up: a small Patchbay-local interop spike or public conformance record for `@connectrpc/connect-node` `createGrpcTransport()` against tonic-generated Rust services would complete the remaining operational confidence gap; web availability is unknown because the generic docs establish protocol compatibility but not this exact library pair under Patchbay service shapes.
