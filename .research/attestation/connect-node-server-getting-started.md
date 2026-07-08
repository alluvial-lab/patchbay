---
source_handle: connect-node-server-getting-started
fetched: 2026-07-07
source_url: https://connectrpc.com/docs/node/getting-started/
provenance: source-direct
---

# Attestation: Connect for Node — server getting started

## Summary

The Connect for Node getting-started guide presents Connect-Node as a Node.js library for serving Connect, gRPC, and gRPC-Web-compatible HTTP APIs. The guide walks through defining a Protobuf service, generating code with Buf, implementing an Eliza service, plugging routes into Node/Fastify, and calling the service using curl and a Connect client. It also says Node supports three protocols and that a server can support both gRPC and Connect protocols.

## Key passages

1. The opening states: "Connect-Node is a library for serving Connect, gRPC, and gRPC-Web compatible HTTP APIs using Node.js."

2. The opening also says Connect-Node supports "all four types of remote procedure calls: unary and the three variations of streaming."

3. Under "Start a server", the guide says Connect services can be plugged into vanilla Node.js servers, Next.js, Express, or Fastify.

4. The server example installs `fastify`, `@connectrpc/connect-node`, and `@connectrpc/connect-fastify`, registers `fastifyConnectPlugin`, passes generated `routes`, and starts listening.

5. Under "Make requests", the guide says the simplest way to consume the API is an HTTP/1.1 POST with JSON payload, and later shows a Connect client using `createClient` and `createConnectTransport` from `@connectrpc/connect-node`.

6. Under "From the browser", it says the same client can run from a browser by swapping out the transport to `@connectrpc/connect-web`.

7. Under "Use the gRPC protocol instead of the Connect protocol", it says Node supports three protocols: gRPC, gRPC-Web, and Connect.

8. After configuring HTTP/2/TLS, the guide says that along with gRPC-Web and Connect, "any gRPC client can access it too" and shows `buf curl --protocol grpc`.

9. The closing says: "With just a few lines of hand-written code, you’ve built a real API server that supports both the gRPC and Connect protocols."
