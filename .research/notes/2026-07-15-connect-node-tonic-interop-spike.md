# Spike report: @connectrpc/connect-node ↔ tonic interop validation

- **Story:** `story-connect-node-tonic-interop-spike`
- **Research origin:** `v0-stack-tooling` campaign (`analysis/campaigns/v0-stack-tooling/parent.md`)
- **Date:** 2026-07-15
- **Verdict:** revisit-trigger **RETIRED**. The pair interoperates cleanly under all five conditions the `feature-v0-protocol-seam` implementation will exercise.

## Claim validated

The `v0-stack-tooling` synthesis's headline internal-seam finding:

> the v0 internal seam is Connect-ES Node client over gRPC/HTTP2 to a tonic Rust server, and this does not reopen `feature-web-core-protocol-seam`

rested on *generic* protocol-compatibility evidence. No fetched source attested this **exact library pair** interoperating under realistic service shapes. The synthesis's own `## Revisit if` named the residual caveat:

> A spike shows `@connectrpc/connect-node` `createGrpcTransport()` cannot interoperate cleanly with generated tonic services.

This spike ran the pair under the conditions `feature-v0-protocol-seam` will actually exercise. The caveat is retired; no trigger fired.

## Library versions used

| Side | Crate / package | Version |
|---|---|---|
| Rust server | `tonic` | 0.14.6 |
| Rust server | `tonic-prost` (runtime codec) | 0.14.6 |
| Rust codegen | `tonic-prost-build` | 0.14.6 |
| Rust error model | `tonic-types` (`google.rpc.Status`) | 0.14.6 |
| Rust protobuf | `prost` | 0.14.4 |
| Rust runtime | `tokio` | 1.52.3 |
| Rust HTTP/2 | `hyper` | 1.10.1 |
| TS client | `@connectrpc/connect` | 2.1.2 |
| TS client | `@connectrpc/connect-node` (`createGrpcTransport`) | 2.1.2 |
| TS protobuf | `@bufbuild/protobuf` (Protobuf-ES) | 2.12.1 |

These match the current-fetched versions recorded in the `v0-stack-tooling` synthesis (`@connectrpc/connect`/`connect-node` 2.1.2, tonic 0.14.6, tokio 1.52.3, prost 0.14). The spike was the first code in the repo to actually exercise the pair.

## What was built

A throwaway, self-contained interop harness under `spikes/connect-tonic-interop/` (NOT part of the Patchbay protocol contract; the real service definitions are owned by `feature-v0-protocol-seam`):

- `proto/spike.proto` — a minimal `SpikeControl` service with shapes that mirror what the seam will exercise: `Submit(SubmitRequest) -> SubmitResult` (unary, command-submission shape), `Subscribe(SubscribeRequest) -> stream SubscribeEvent` (server-streaming, `Subscribe(cursor) returns (stream CoreEvent)` shape), and a `SubmissionFailureDetail` message for the richer-error-model condition.
- `rust/` — a tonic server (`spike-server`) exposing `SpikeControl` in two modes: `plain` (h2c) and `tls` (HTTP/2 over TLS). The `Submit` handler returns a `tonic::Status` with `with_details` carrying a serialized `google.rpc.Status` (built via `tonic_types::pb::Status`) whose `details` vector holds the custom `SubmissionFailureDetail` wrapped as a `prost_types::Any`. It also echoes request metadata into the response diagnostic so the client can assert propagation.
- `ts/` — a `@connectrpc/connect-node` client using `createGrpcTransport()` (the exact call site under test — the gRPC transport, not the Connect protocol transport) that runs all five conditions and prints a structured PASS/FAIL report. Exits 0 only if every condition passes.

## Results

### h2c (cleartext HTTP/2)

```
[PASS] 1-unary: commandId=c1 lsn=42 diag=ok; op_session= csrf=
[PASS] 2-streaming: lsns=[101,102,103,104]
[PASS] 3-error-mapping: code=3 (ConnectError) detail={"commandId":"c3","failureCode":"FAILURE_CODE_VALIDATION_FAILED","reason":"rejected by spike; op_session= csrf="}
[PASS] 4-metadata: diag=ok; op_session=op-sess-abc csrf=csrf-token-xyz
[PASS] 5-tls: N/A in h2c mode (conditions 1-4 ran over cleartext HTTP/2); TLS is exercised by re-running with --tls against a TLS server

== SPIKE RESULT: 5/5 conditions passed (0 failed) ==
```

### TLS (HTTP/2 over TLS, self-signed cert)

```
[PASS] 1-unary: commandId=c1 lsn=42 diag=ok; op_session= csrf=
[PASS] 2-streaming: lsns=[101,102,103,104]
[PASS] 3-error-mapping: code=3 (ConnectError) detail={"commandId":"c3","failureCode":"FAILURE_CODE_VALIDATION_FAILED","reason":"rejected by spike; op_session= csrf="}
[PASS] 4-metadata: diag=ok; op_session=op-sess-abc csrf=csrf-token-xyz
[PASS] 5-tls: TLS unary accepted=true

== SPIKE RESULT: 5/5 conditions passed (0 failed) ==
```

## Per-condition findings

1. **Unary RPC** — Round-trips cleanly. `createGrpcTransport()` + `createClient()` calls a tonic unary method; the generated Protobuf-ES response decodes with the expected field values. The gRPC wire framing (5-byte gRPC length-prefix header) is handled identically by both sides.

2. **Server-streaming** — `Subscribe(cursor) returns (stream SubscribeEvent)` delivers an async iterable of events to the Node client over HTTP/2. Four events arrived in LSN order (101-104). A 5ms server-side delay between sends makes a fused single-frame batch implausible (the events are yielded to the client async iterable as they arrive, not as one buffered blob), though this spike did not capture a packet trace to assert frame-level boundaries. Connect-ES `createClient().subscribe()` returns an async iterable directly.

3. **Error mapping (the critical condition)** — This is the interop seam the spike brief flagged as most likely to break, and it works. A tonic `Status::with_details(Code::InvalidArgument, msg, <serialized google.rpc.Status>)` — where the `google.rpc.Status` carries a custom `SubmissionFailureDetail` as a `google.protobuf.Any` detail in the `grpc-status-details-bin` trailer — surfaces on the Node client as a structured `ConnectError`:
   - `error.code === Code.InvalidArgument` (numeric 3), not an opaque transport failure.
   - `error.findDetails(SubmissionFailureDetailSchema)` returns the decoded detail with all fields intact (`commandId`, `failureCode`, `reason`).
   - The detail's `type_url` was set manually to `type.googleapis.com/patchbay.spike.SubmissionFailureDetail` (the generated prost structs derive `prost::Message` but not `prost::Name`, so `type_url()` was unavailable; the conventional `type.googleapis.com/<package>.<Message>` form is what Connect-ES decodes against).

4. **Metadata propagation** — Client-set gRPC metadata (`x-patchbay-operator-session`, `x-patchbay-csrf` — the operator-session / CSRF-evidence-forwarding shape the seam will need) arrived at the tonic handler via `Request::metadata()` and was echoed back in the response diagnostic. Lowercase header names, ASCII values, both survive. This is the shape `feature-v0-protocol-seam` will use to forward the web server's verified operator-session + CSRF evidence to the core.

5. **TLS** — The pair works over TLS. tonic's `Server::builder().tls_config(ServerTlsConfig::new().identity(Identity::from_pem(cert, key)))` (gated behind tonic's `tls-ring` feature) terminates TLS; the Node client connects via the same `createGrpcTransport` with `nodeOptions: { ca, rejectUnauthorized: false }` (the self-signed cert's CN mismatch with 127.0.0.1 is a spike artifact, not a real interop gap). All four prior conditions also re-validated green over TLS.

## No interop gap found

No condition failed, partially worked, or required a workaround that would indicate a real interop incompatibility. The one non-obvious detail — the manual `type_url` string for the custom error detail — is a codegen-ergonomics point (prost structs don't derive `prost::Name` by default), not an interop failure: the value on the wire is correct and Connect-ES decodes it. `feature-v0-protocol-seam` can either derive `Name` explicitly or keep the manual string.

## Implications for `feature-v0-protocol-seam`

- The committed v0 internal-seam topology — **Connect-ES Node client over gRPC/HTTP2 to a tonic Rust server** — holds as stated. No design change is forced by interop.
- The richer gRPC error model is available end-to-end: the core can return structured `google.rpc.Status` errors with typed `Any` details, and the web server (and through it, the browser) can decode them by type. This is the seam's failure-mode vocabulary and does not need to be downgraded to opaque transport errors.
- gRPC metadata propagation is confirmed for the operator-session + CSRF-evidence-forwarding shape. The seam's auth-boundary design (web server as principal forwarding verified evidence) has a working transport.
- TLS is viable for the internal channel when deployment is non-localhost (the synthesis already noted HTTP/2 as a v0 deployment requirement for the internal channel; TLS over HTTP/2 is confirmed here).
- The `tonic-prost` runtime crate and `tonic-types` are real runtime dependencies the Rust core will need (not just `tonic-prost-build` for codegen). `feature-v0-protocol-seam`/`feature-v0-web-server` implementation should declare them.

## Spike artifacts

The harness is kept under `spikes/connect-tonic-interop/` (throwaway, gitignored build artifacts). It is not part of the Patchbay protocol contract surface. To re-run:

```sh
# Rust server (h2c)
cd spikes/connect-tonic-interop/rust
CARGO_HOME=<writable-cargo-home> PORT=50061 ./target/release/spike-server plain &

# TS client
cd ../ts && node dist/run.js --host 127.0.0.1 --port 50061
```

## Verdict on the revisit-trigger

**RETIRED.** The `v0-stack-tooling` synthesis's `## Revisit if` entry — *"A spike shows `@connectrpc/connect-node` `createGrpcTransport()` cannot interoperate cleanly with generated tonic services"* — is discharged. The spike confirmed clean interoperation under all five conditions. No follow-up is filed; `feature-v0-protocol-seam` may proceed against the gRPC/HTTP2 topology as designed.
