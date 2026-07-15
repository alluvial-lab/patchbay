---
id: feature-v0-protocol-seam
kind: feature
stage: implementing
tags: [protocol, adapter, security]
parent: epic-v0-1-0-implementation
depends_on: [epic-v0-core]
created: 2026-06-28
updated: 2026-07-15
gate_origin: null
release_binding: null
research_origin: null
---

# Feature: Web↔core internal protocol seam

With the v0 process topology settled (a TypeScript web server terminates HTTP for the browser cockpit and speaks the generated Protobuf/Connect contract to the Rust coordination core), the internal seam between the web server and the core needs genuine design work. This is a design-bearing feature, not prose — it pins RPC shapes, a streaming channel, an internal auth boundary, and failure modes.

## Scope

- The RPC surface the web server calls on the core: command submission, snapshot/cursor reconciliation, session list, grant/revocation operators, audit queries, adapter attach/detach.
- How operator-session and CSRF evidence crosses the seam: does the web server forward a verified operator-session id, or present its own service principal + a delegated operator claim? What does the core trust?
- Streaming/event channel from core to web server (and on to the browser): Connect streaming, gRPC bidirectional, or SSE-over-Connect. Reconnect and cursor resumption across the seam.
- Web server as a principal: its own grant to the core, its own endpoint/device record, audit of its calls.
- Failure modes across the seam: web server crash, core unreachable, partial submission (`SubmissionOutcome = unknown`/`failed`), backpressure on event streams.
- How the browser's operator domain composes with the web server's translations (and what stays in the browser vs. what the web server owns).
- Relationship to the shared Protobuf+Buf contract: is the internal surface the *same* contract as the browser-facing one, or a superset restricted to control-surface principals?

## Status

Promoted from backlog on 2026-07-11 into `epic-v0-1-0-implementation`. The foundation-hardening work it was waiting on (security threat model, persistence/snapshot, session-identity/adapter-contract) is now `done`, so the seam can be designed against settled foundations. Depends on `feature-v0-core` because the seam is the first consumer of the core's RPC surface.

## Expected output

A designed feature at `stage: implementing` with the seam specified, ready for Rust core + TS web server implementation. Likely spawns child stories for the core-side RPC handler and the web-server-side client/translator.

## Related

- `docs/ARCHITECTURE.md` "V0 process topology" — the committed two-process topology this seam realizes.
- `feature-security-threat-model` — grant shape, operator-session, revocation, audit.
- `feature-persistence-snapshot-model` — cursor/LSN reconciliation the web server must carry.
- `feature-session-identity-adapter-contract` — session identity/generation the seam must preserve.
- `feature-research-contract-tooling` — Protobuf+Buf as the contract source.

## Spike grounding (2026-07-15)

`story-connect-node-tonic-interop-spike` (done) retired the `v0-stack-tooling` synthesis's residual transport caveat: `@connectrpc/connect-node` 2.1.2 `createGrpcTransport()` interoperate cleanly with a tonic 0.14.6 Rust server under all five conditions the seam will exercise (unary RPC, server-streaming, richer gRPC error model with `google.rpc.Status` + custom `Any` details, metadata propagation, TLS). The topology commitment holds as designed; no design change is forced by interop. See `.research/notes/2026-07-15-connect-node-tonic-interop-spike.md`. The spike also established that `tonic-prost` (runtime codec) and `tonic-types` are real runtime deps the Rust server crate must declare.

## Design decisions

- **Compound-issuer wire evidence shape**: (a) forwarded verified session id — the web server verifies the operator session (cookie/CSRF) at its boundary and forwards a verified `OperatorSessionId` (+ derived `ActorId`) in gRPC metadata; the core trusts the web server's verification (the web server is the authenticated ingress) and independently verifies the web-server transport principal. This satisfies the committed `SECURITY.md:143` requirement (core independently verifies both transport principal and operator identity) without front-loading signed-claim crypto. Signed/attested operator claims (option b) are a reserved seam for split-deploy / multi-operator, not a v0.1.0 need. An `OperatorSessionId` message already exists in the generated contracts.
- **Internal contract shape**: (a) one proto package, principal-gated access — internal control-surface methods (`AdapterAttach`, `AdapterDetach`, audit queries) live alongside browser-reachable methods in the single `patchbay` package; the core enforces access control by principal (a browser principal cannot call admin methods). Unifying now and splitting later is easy; merging two packages later is hard. Honors the single-source-of-truth and "speak the generated contract" commitments.
- **Web-server-to-core trust-root**: (a) configured shared-secret over localhost for v0.1.0 — the web server authenticates to the core via a configured secret carried in gRPC metadata, transport-bound to localhost (or a configured bind address). Fails safe: a split-deploy attempted without configuring the secret will not authenticate. mTLS for the internal channel is a reserved seam for split deployment, not a v0.1.0 requirement (v0.1.0 is explicitly colocated per `docs/ARCHITECTURE.md`).
- **Event channel shape**: server-streaming only — `Subscribe(AuthorityDomainId, cursor) returns (stream Event)` mirrors the existing `Storage::read_after(domain, cursor)` pull model. The web server submits via unary RPCs and receives via stream; it has no need to push control messages mid-stream. Anything that looks like bidi (narrow scope, cancel subscription) is another unary RPC + re-subscribe with a new cursor. Bidirectional streaming is a reserved seam.

## Architectural choice

**A tonic gRPC server binary (`patchbay-core-server`) that wraps the existing core library crate.** The seam is a thin adapter layer translating between the generated Protobuf service and the core's existing `acceptance::submit` / `Storage::read_after` / `SessionRegistry` / `IssuerContext` APIs. It owns NO domain logic — it is the Ports & Adapters "driving adapter" that exposes the core's ports over the wire. This fits the project's Ports & Adapters principle exactly: domain logic stays in `patchbay-core`; the server crate is the HTTP-terminating... no, the gRPC-terminating adapter (HTTP termination stays with the TS web server per the v0.1.0 topology).

The web-server side (a separate feature, `feature-v0-web-server`) will consume the generated TS Connect client against this service. This feature delivers only the Rust server half + the proto service definitions; the TS client/translator is `feature-v0-web-server`'s job.

This was chosen over two alternatives considered:
- *Embed the server in the `patchbay-core` library crate* — rejected: the core is deliberately a domain-logic library with no runtime/HTTP dependency; adding `tonic`/`tokio` server deps to it would violate Ports & Adapters and re-couple domain to transport. A separate `server/` crate at the workspace root is the clean boundary.
- *Generate the service into `contracts/`* — rejected: the proto service definitions belong with the contract (so both Rust and TS sides generate from one source), but the *server implementation* belongs in a runtime crate. The proto `.proto` `service` blocks go in `contracts/proto/`; the server crate depends on `patchbay-contracts` + `patchbay-core`.

## Implementation Units

### Unit 1: Proto service definitions (the contract surface)
**File**: `contracts/proto/patchbay/control.proto` (new)
**Story**: `story-v0-protocol-seam-proto-services`

Defines the gRPC services the seam exposes. These are the *first* proto `service` blocks in the repo (the existing protos define only messages/enums). One package (`patchbay`), principal-gated access.

```proto
// The internal control-plane service the TS web server speaks to the Rust
// core over gRPC/HTTP2. Browser-facing surfaces are a subset reachable
// through the web server; control-surface methods are principal-gated.
service ControlService {
  // Submit an Operation for acceptance. Mirrors acceptance::submit.
  // The verified operator-session id + web-server principal arrive in
  // request metadata; the body is the Operation to submit.
  rpc Submit(SubmitRequest) returns (SubmissionResult);

  // Reconcile / tail the durable event stream from a cursor.
  // Mirrors Storage::read_after(domain, cursor). Server-streaming.
  rpc Subscribe(SubscribeRequest) returns (stream SubscribeEvent);

  // Load the latest snapshot at or before an LSN (cursor reconciliation
  // path on reconnect). Mirrors Storage::load_latest_snapshot.
  rpc LoadSnapshot(LoadSnapshotRequest) returns (LoadSnapshotResponse);
}

message SubmitRequest {
  // The Operation to submit. sender/recipient/kind/target_scope/payload
  // are all in the Operation message (operations.proto).
  patchbay.Operation operation = 1;
}
// SubmissionResult is reused from operations.proto (already generated).

message SubscribeRequest {
  patchbay.AuthorityDomainId authority_domain_id = 1;
  patchbay.Lsn cursor = 2;
}

message SubscribeEvent {
  // The recorded event: identity (EventId = authority_domain_id + LSN)
  // + payload (StoredEventPayload = kind + serialized inner message).
  patchbay.EventId event_id = 1;
  patchbay.StoredEventPayload payload = 2;
}

message LoadSnapshotRequest {
  patchbay.AuthorityDomainId authority_domain_id = 1;
  // None = latest overall; Some(lsn) = latest at or before lsn.
  optional patchbay.Lsn at_or_before = 2;
}

message LoadSnapshotResponse {
  bool present = 1;
  patchbay.EventId event_id = 2;        // the snapshot's identity
  bytes snapshot_payload = 3;           // opaque materialized payload
}
```

Adapter attach/detach + audit-query RPCs are deferred: they depend on adapter-registration and audit-projection surfaces the core exposes via separate features. This feature ships the three RPCs the walking skeleton needs first (submit, subscribe, snapshot). Adding methods to a proto service is non-breaking for clients, so this is forward-compatible.

**Acceptance Criteria**:
- [ ] `control.proto` defines `ControlService` with `Submit`, `Subscribe`, `SubscribeEvent` exactly as above.
- [ ] `buf generate` regenerates Rust + TS bindings; `cargo build -p patchbay-contracts` and `npm run build` in `contracts/ts` both succeed.
- [ ] Gen diff is additions-only (no drift in existing generated files).

---

### Unit 2: gRPC server crate (`patchbay-core-server`)
**File**: `server/Cargo.toml`, `server/src/main.rs`, `server/src/service.rs`, `server/src/issuer.rs`
**Story**: `story-v0-protocol-seam-grpc-server`

The driving adapter: a tonic server binary that constructs the core's `Storage` (rusqlite), `SessionRegistry`, `GrantCheck`, `TargetResolver`, `CommandStateLookup` and wires them into a `ControlService` impl. Owns the shared-secret auth interceptor + the `IssuerContext` impl that reads verified operator-session id + actor from gRPC metadata.

```rust
// server/src/service.rs — the gRPC service impl (sketch)
use patchbay_core::{acceptance, session::SessionRegistry, storage::Storage};
use patchbay_contracts::patchbay::{
    SubmitRequest, SubmissionResult, SubscribeRequest, SubscribeEvent,
    LoadSnapshotRequest, LoadSnapshotResponse, OperatorSessionId,
};

pub struct ControlServiceImpl<S: Storage> {
    storage: S,
    // In-memory projections rebuilt from the log at startup.
    // (Arc<Mutex<...>> or actor-wrapped; see implementation notes.)
    grant_check: Arc<dyn GrantCheck>,
    target_resolver: Arc<dyn TargetResolver>,
    state_lookup: Arc<dyn CommandStateLookup>,
}

#[tonic::async_trait]
impl control_service_server::ControlService for ControlServiceImpl<...> {
    async fn submit(
        &self,
        req: Request<SubmitRequest>,
    ) -> Result<Response<SubmissionResult>, Status> {
        // 1. Auth interceptor already verified the web-server principal
        //    (shared secret) before dispatch.
        // 2. Build IssuerContext from request metadata:
        //    - verified_actor: derived from x-patchbay-operator-session-id
        //      (the web server vouches for this; it verified the cookie).
        //    - verified_endpoint: the web-server endpoint id (its principal).
        //    - authority_domain_id: from the Operation body.
        let issuer = MetadataIssuerContext::from_request(&req)?;
        // 3. Delegate to the core's existing acceptance pipeline.
        let result = acceptance::submit(
            &self.storage, &*self.grant_check, &*self.target_resolver,
            &*self.state_lookup, &issuer, req.into_inner().operation.unwrap(),
        ).await
        .map_err(map_acceptance_error_to_status)?;
        Ok(Response::new(result))
    }

    type SubscribeStream = Pin<Box<dyn Stream<Item = Result<SubscribeEvent, Status>> + Send>>;
    async fn subscribe(
        &self, req: Request<SubscribeRequest>,
    ) -> Result<Response<Self::SubscribeStream>, Status> {
        // Cursor reconciliation: read_after(domain, cursor) once.
        // (Live tailing — blocking read_after until new events — is a
        //  follow-on; v0.1.0 returns the current consistent prefix and
        //  the client re-subscribes with the new cursor. This matches
        //  the spike's streaming shape and avoids a long-held read txn.)
        let inner = req.into_inner();
        let events = self.storage.read_after(
            inner.authority_domain_id.unwrap().reference(),
            inner.cursor.unwrap_or_default(),
        ).await.map_err(map_storage_error_to_status)?;
        let stream = tokio_stream::iter(events.into_iter().map(|e| {
            Ok(SubscribeEvent { event_id: Some(e.event_id), payload: Some(e.payload) })
        }));
        Ok(Response::new(Box::pin(stream)))
    }

    async fn load_snapshot(
        &self, req: Request<LoadSnapshotRequest>,
    ) -> Result<Response<LoadSnapshotResponse>, Status> {
        // Mirrors Storage::load_latest_snapshot.
        let inner = req.into_inner();
        let snap = self.storage.load_latest_snapshot(
            inner.authority_domain_id.unwrap().reference(),
            inner.at_or_before.map(|l| l.value),
        ).await.map_err(map_storage_error_to_status)?;
        Ok(Response::new(match snap {
            None => LoadSnapshotResponse { present: false, event_id: None, snapshot_payload: vec![] },
            Some(s) => LoadSnapshotResponse {
                present: true, event_id: Some(s.event_id), snapshot_payload: s.payload,
            },
        }))
    }
}
```

```rust
// server/src/issuer.rs — IssuerContext impl from gRPC metadata.
use patchbay_core::authority::IssuerContext;
use patchbay_contracts::patchbay::{
    ActorId, AuthorityDomainId, DeviceId, EndpointId, Generation, OperatorSessionId,
};

/// The verified operator-session id + actor the web server forwarded.
/// The web server vouches for these (it verified the operator's cookie);
/// the core trusts the web server's verification because the web server
/// authenticated to the core via the shared secret.
pub struct MetadataIssuerContext {
    verified_actor: Option<ActorId>,
    verified_endpoint: Option<EndpointId>,  // the web-server principal
    authority_domain_id: AuthorityDomainId,
    // device + endpoint_generation: not carried in v0.1.0 metadata
    // (single-operator, single web-server). Reserved seam.
}

impl MetadataIssuerContext {
    /// Builds from gRPC request metadata. The auth interceptor has already
    /// verified the shared secret; this reads the operator-session evidence.
    pub fn from_request(req: &Request<impl Sized>) -> Result<Self, Status> { /* ... */ }
}

impl IssuerContext for MetadataIssuerContext { /* trait impl */ }
```

**Implementation Notes**:
- **Projection state ownership**: `SessionRegistry`, `GrantCheck` impls, etc. are currently in-memory projections in `patchbay-core`. The server must own them and rebuild from the log at startup (replay). They are not `Send + Sync` by default (e.g. `SessionRegistry` has `&mut self` `observe`); the server wraps them in an actor / `Arc<Mutex<>>` so `acceptance::submit` (which takes `&self` refs) can be called from the async tonic handler. This is the trickiest unit — see Unit 2b.
- **Auth interceptor**: a tonic `Interceptor` that checks the `x-patchbay-core-secret` metadata against the configured value; rejects with `Status::unauthenticated` if absent/mismatched. Runs before every RPC. Fail safe: no secret configured = no RPC authenticates (server refuses to start with a clear error, rather than running open).
- **Error mapping**: `AcceptanceError` and `StorageError` map to gRPC statuses. `StorageError::Unavailable` → `Status::unavailable` (retryable hint carried as `RetryInfo` detail via `tonic-types`); `IdempotencyConflict` → `Status::failed_precondition`; `CorruptRecord` → `Status::internal`. Uses the richer gRPC error model confirmed by the spike so the TS client gets structured details.
- **Cursor semantics**: `Subscribe` returns the consistent prefix `> cursor` and completes; the web server re-subscribes with the highest LSN it received (polling/reconnect reconcile per `docs/PROTOCOL.md`). No long-held blocking read transaction in v0.1.0.

**Acceptance Criteria**:
- [ ] `patchbay-core-server` binary starts, binds a gRPC (h2c) listener on a configurable address, and refuses to start without a configured shared secret.
- [ ] At startup it rebuilds in-memory projections (`SessionRegistry`, grant registry) by replaying the log via `read_after(domain, 0)`.
- [ ] A `createGrpcTransport()` client can `Submit` an Operation and receive a `SubmissionResult`; an unauthorized secret returns `Status::unauthenticated`.
- [ ] `Subscribe(cursor)` returns `> cursor` events and completes; `LoadSnapshot` returns the latest snapshot.
- [ ] `StorageError` variants map to the gRPC statuses above.

---

### Unit 2b: Projection-state concurrency wrapper (the trickiest unit)
**File**: `server/src/state.rs`
**Story**: `story-v0-protocol-seam-grpc-server` (same story — this is the highest-risk part of it)

The core's in-memory projections (`SessionRegistry`, authority registry, command-state lookup) were authored as `&mut self` / non-async single-threaded types for the core's proptests. The gRPC server is multi-threaded async (tonic + tokio). This unit designs the wrapper that makes them safe to call from concurrent handlers WITHOUT leaking that concern back into `patchbay-core`.

Two options considered:
- *(a) `Arc<Mutex<...>>` per projection* — simplest; `acceptance::submit` calls take the locks in order (storage, grant, target, state). Risk: lock ordering must be consistent or deadlock. Acceptable for v0.1.0 single-operator throughput.
- *(b) Actor pattern* — a dedicated task owns the projections; handlers send commands over channels. Cleaner concurrency, more boilerplate.

**Chosen: (a) `Arc<Mutex<>>` for v0.1.0.** It is the least-irreversible sound choice (promoting to an actor later is internal to the server crate; the core's `acceptance::submit` signature is unchanged). Lock ordering is documented: `storage → grant_check → target_resolver → state_lookup` (the acceptance pipeline's existing call order), matching `acceptance::submit`'s parameter order. Single-operator throughput does not warrant the actor pattern yet.

**Acceptance Criteria**:
- [ ] `acceptance::submit` is callable from concurrent tonic handlers without UB or deadlock (verified by a concurrency test: N parallel `Submit` calls).
- [ ] Lock ordering is documented and the projection types are not made `Send+Sync` by editing `patchbay-core` (the wrapper lives in the server crate).

---

### Unit 3: End-to-end interop smoke against the real core
**File**: `server/tests/grpc_smoke.rs` (integration test)
**Story**: `story-v0-protocol-seam-grpc-server` (same story)

A Rust integration test that spins up the tonic server in-process (or a `#[tokio::test]` that binds an ephemeral port) and exercises the three RPCs against the real `patchbay-core` storage + projections, using a `tonic` gRPC client (not the TS one — that's `feature-v0-web-server`'s job). This is the seam's interface test: it protects the contract that the web server will consume.

**Acceptance Criteria**:
- [ ] Test submits an Operation, asserts a `SubmissionResult` with `outcome = ACCEPTED` (or the documented rejection).
- [ ] Test subscribes from cursor 0, asserts it receives the accepted Operation's event.
- [ ] Test loads the latest snapshot (None until one is written).
- [ ] Test asserts an unauthenticated call returns `Status::unauthenticated`.

## Implementation Order

1. `story-v0-protocol-seam-proto-services` (Unit 1) — no deps; unblocks the server impl (which needs the generated types).
2. `story-v0-protocol-seam-grpc-server` (Units 2, 2b, 3) — depends on the proto story; carries the server crate, the concurrency wrapper, and the smoke test as one cohesive ownership bundle.

## Simplification

- The seam owns NO domain logic — it is pure translation. This is the elimination pass: no re-validation, no authority logic, no state machines in the server crate. Every behavior lives in `patchbay-core`; the server maps types and errors.
- Adapter attach/detach + audit-query RPCs are NOT in this feature — deferred to avoid designing surfaces that depend on not-yet-exposed core features. Adding proto methods later is non-breaking.
- No bidi streaming, no long-held read transactions (cursor polling + re-subscribe instead). This keeps v0.1.0 simple and matches the spike's confirmed shape.

## Testing

- **Interface test (Unit 3)**: the Rust gRPC smoke against the real core — protects the contract the web server consumes and the auth/error-mapping behavior. This is the load-bearing test.
- **Concurrency test (Unit 2b)**: N parallel `Submit` calls — protects the projection-state wrapper from UB/deadlock.
- **Auth fail-closed test**: server refuses to start without a secret; unauthenticated RPCs return `Status::unauthenticated`.
- No unit test per RPC method (they're thin delegations to the already-tested core); the interface test covers them.

## Risks

- **Projection concurrency (highest risk)**: the core's in-memory projections were authored single-threaded. The `Arc<Mutex<>>` wrapper is the riskiest assumption; if lock ordering proves wrong under concurrent load, the concurrency test catches it. Fallback: promote to an actor inside the server crate (no core change).
- **Cursor/polling liveness**: v0.1.0's `Subscribe` returns the prefix and completes; the web server polls. If polling latency proves too high for operator UX, a blocking/live-tail `Subscribe` is a follow-on (still server-streaming). Not a v0.1.0 blocker — the operator can reconnect/re-subscribe.
- **Auth metadata shape**: the `OperatorSessionId`-in-metadata shape is the v0.1.0 commitment; if operator-session evidence needs to carry more (device, generation) before split-deploy, that's an additive metadata field — non-breaking.
