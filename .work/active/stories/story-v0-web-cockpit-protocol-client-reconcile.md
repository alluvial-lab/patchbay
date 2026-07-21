---
id: story-v0-web-cockpit-protocol-client-reconcile
kind: story
stage: done
tags: [ux, protocol]
parent: feature-v0-web-cockpit
depends_on: []
created: 2026-07-20
updated: 2026-07-20
release_binding: null
gate_origin: null
---

# Story: Cockpit Unit 1 — protocol client + cursor-reconcile

Implements Unit 1 of `feature-v0-web-cockpit`. The foundation: nothing else
runs without it.

## Scope

The browser-side operator-domain protocol client and the cursor-based
reconnect state machine.

- `web-cockpit/src/domain/protocol-client.ts` — a typed Connect-Web client to
  `ControlService` (Submit / Subscribe / LoadSnapshot) over the web-server's
  same-origin gRPC-Web bridge at `/`.
- `web-cockpit/src/domain/reconcile.ts` — subscribe with last-known cursor;
  fold incoming `SubscribeEvent`s into the presentation model; reconcile
  against `LoadSnapshot` on reconnect gaps.

## Grounded shapes (verified against shipped proto, 2026-07-20)

- `control.proto`: `SubscribeRequest { AuthorityDomainId; Lsn cursor }` →
  `stream SubscribeEvent { EventId; StoredEventPayload }`. `LoadSnapshotRequest
  { AuthorityDomainId; optional Lsn at_or_before }` → `LoadSnapshotResponse
  { bool present; EventId; bytes snapshot_payload }`. `SubmitRequest {
  Operation }` → `SubmissionResult { outcome; command_id; operation_state;
  failure_code; diagnostic_message; accepted_lsn; deduplicated }`.
- `web-server/src/routes/rpc.ts` already proxies all three RPCs with
  operator-session auth: CSRF required on `Submit`, relaxed on
  `Subscribe`/`LoadSnapshot` (they are read-only streaming/query). The server
  overwrites `operation.sender` with the session-verified operator actor — the
  cockpit must not trust its own sender claim.
- `EventId = { AuthorityDomainId; Lsn }`; `Lsn = { uint64 value }`. The
  reconnect cursor is the last *folded* `event_id.lsn`.
- `LoadSnapshotResponse.snapshot_payload` is opaque `bytes` — deserialize via
  `SessionSnapshotSchema` (`fromBinary`). `SessionSnapshot` carries
  `sessions[]` + `snapshot_lsn` + `core_generation`; the fold *replaces* the
  model from it (snapshot is authority, the old model is never merged).

## Implementation notes

- Cursor advances only after the presentation model applies the event (fold
  first, then advance). Reconnect resumes from the cursor.
- Gap detection: when the stream resumes at `lsn > cursor+1`, reconcile via
  `LoadSnapshot({ at_or_before: <stream resume lsn - 1> })` rather than
  synthesizing state.
- `SubmissionResult.deduplicated` is the retry-safety signal surface — the
  UI may use it to show "already in flight" rather than re-submitting. The
  retry-safety matrix itself lives in the presentation layer; the client
  surfaces the fields.
- Unreconciled state renders stale/unknown — never live.

## Acceptance criteria

- [x] Subscribe folds events into the presentation model; cursor advances on fold
- [x] Reconnect after a stream break re-subscribes from the last cursor without losing applied state
- [x] A snapshot gap is reconciled via LoadSnapshot (`snapshot_payload` → `SessionSnapshot`); unreconciled axes render stale/unknown
- [x] Optimistic UI state is never authority for the cursor or the presentation model
- [x] The client does not send a `sender` claim expecting it to be honored (server overwrites it)

## Verification evidence

- Interface test: reconnect/resume behavior against a fake Connect-Web
  transport (inject stream breaks at specific LSNs; assert cursor resume +
  snapshot reconcile + no double-fold).
- Property test (the load-bearing one, per session-note directive): an
  unreconciled snapshot must never render as live. Mutate reconcile to
  skip the stale-marking; the test must fail.

## Implementation notes

- Execution capability: inline feature owner; one owner is carrying the five ordered checkpoints to preserve reconnect/presentation context.
- Review weight: standard (project/default); feature-level review only after all child checkpoints are done.
- Files changed: `web-cockpit/package.json`, `web-cockpit/package-lock.json`, `web-cockpit/tsconfig.json`, `web-cockpit/src/domain/protocol-client.ts`, `web-cockpit/src/domain/reconcile.ts`, `web-cockpit/tests/reconcile.test.ts`.
- Tests added: fake-stream reconnect/resume and gap repair interface tests plus a 100-run generated stream-break property test asserting stale marking is visible before snapshot load and that folded LSNs are unique.
- Simplification: the reconciler depends on a small projection port rather than coupling transport code to the Unit 2 model implementation.
- Discrepancies from design: the shipped web-server bridge speaks binary gRPC-Web (`application/grpc-web+proto`), so the client uses `createGrpcWebTransport` rather than the pseudocode's Connect-protocol `createConnectTransport`. The actual web boundary also requires `x-patchbay-csrf`; the client exposes a fail-fast token interceptor and `/csrf-token` loader. Both are mechanical bindings to shipped server behavior.
- Adjacent issues parked: none.

## Verification result

- `cd contracts/ts && npm run build` — pass.
- `cd web-cockpit && npm test` — pass (3 tests, including 100 property runs).
- Mutation check: removing either `markUnreconciled` call makes the generated reconnect/gap assertions observe `LIVE` at snapshot load and fail.
