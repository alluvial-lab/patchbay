import assert from "node:assert/strict";
import test from "node:test";

import { create, toBinary, type DescMessage, type MessageShape } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  AuthorityDomainIdSchema,
  CommandIdSchema,
  ElicitationIdSchema,
  ElicitationSchema,
  ElicitationState,
  EventIdSchema,
  GenerationSchema,
  LoadSnapshotResponseSchema,
  LsnSchema,
  ObservationKind,
  ObservationSchema,
  OperationKind,
  OperationSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  QuestionContractSchema,
  ResponseContractKind,
  ResponseContractSchema,
  RuntimeSessionIdSchema,
  SessionActivityState,
  SessionConnectivityState,
  SessionSchema,
  SessionSnapshotSchema,
  SessionStateSchema,
  StoredEventKind,
  StoredEventPayloadSchema,
  SubscribeEventSchema,
  TargetScopeKind,
  TargetScopeSchema,
  type AuthorityDomainId,
  type LoadSnapshotRequest,
  type LoadSnapshotResponse,
  type SessionSnapshot,
  type SubscribeEvent,
  type SubscribeRequest,
} from "@patchbay/contracts";
import fc from "fast-check";

import { PresentationProjection } from "../src/domain/model.js";
import {
  Reconciler,
  type ReconcileClient,
  type ReconcileProjection,
} from "../src/domain/reconcile.js";

const DOMAIN = create(AuthorityDomainIdSchema, { value: "operator-domain" });
const ADAPTER = create(AdapterIdSchema, { value: "pi" });
const RUNTIME = create(RuntimeSessionIdSchema, { value: "session-1" });

class RecordingProjection implements ReconcileProjection {
  readonly folded: bigint[] = [];
  readonly marks: Array<"stream-break" | "event-gap"> = [];
  visibleConnectivity = SessionConnectivityState.LIVE;
  snapshots = 0;

  markUnreconciled(reason: "stream-break" | "event-gap"): void {
    this.marks.push(reason);
    this.visibleConnectivity = SessionConnectivityState.STALE;
  }

  replaceFromSnapshot(snapshot: SessionSnapshot, replayEvents: readonly SubscribeEvent[]): void {
    this.snapshots += 1;
    this.folded.splice(0, this.folded.length, ...replayEvents.map(eventLsn));
    this.visibleConnectivity =
      snapshot.sessions[0]?.state?.connectivity ?? SessionConnectivityState.UNKNOWN;
  }

  foldEvent(event: SubscribeEvent): void {
    this.folded.push(eventLsn(event));
  }
}

test("stream breaks rebuild the projection prefix before resuming from the snapshot cursor", async () => {
  const projection = new RecordingProjection();
  const cursors: bigint[] = [];
  let subscription = 0;
  const client: ReconcileClient = {
    subscribe(request) {
      cursors.push(request.cursor!.value);
      subscription += 1;
      if (subscription === 1) return brokenAfter([event(1n)]);
      if (subscription === 2) return values([event(1n)]);
      return values([event(2n)]);
    },
    async loadSnapshot() {
      assert.equal(projection.visibleConnectivity, SessionConnectivityState.STALE);
      return snapshotResponse(1n);
    },
  };

  const reconciler = new Reconciler(client, projection, { retryDelayMs: 0 });
  const received: bigint[] = [];
  for await (const next of reconciler.subscribe(DOMAIN)) {
    received.push(eventLsn(next));
    if (received.length === 2) break;
  }

  assert.deepEqual(received, [1n, 2n]);
  assert.deepEqual(projection.folded, [1n, 2n]);
  assert.deepEqual(cursors, [0n, 0n, 1n]);
  assert.equal(reconciler.currentCursor, 2n);
  assert.deepEqual(projection.marks, ["stream-break"]);
});

test("clean completion at the durable tail stays reconciled and re-subscribes", async () => {
  const projection = new RecordingProjection();
  const cursors: bigint[] = [];
  let subscription = 0;
  const client: ReconcileClient = {
    subscribe(request) {
      cursors.push(request.cursor!.value);
      subscription += 1;
      return subscription === 1 ? values([event(1n)]) : values([event(2n)]);
    },
    async loadSnapshot() {
      assert.fail("clean completion must not trigger snapshot reconciliation");
    },
  };

  const reconciler = new Reconciler(client, projection, { retryDelayMs: 0 });
  const received: bigint[] = [];
  for await (const next of reconciler.subscribe(DOMAIN)) {
    received.push(eventLsn(next));
    if (received.length === 2) break;
  }

  assert.deepEqual(received, [1n, 2n]);
  assert.deepEqual(cursors, [0n, 1n]);
  assert.deepEqual(projection.marks, []);
  assert.equal(projection.visibleConnectivity, SessionConnectivityState.LIVE);
  assert.equal(reconciler.currentCursor, 2n);
});

test("an event gap is repaired with a bounded snapshot and complete prefix replay", async () => {
  const projection = new RecordingProjection();
  const snapshotRequests: LoadSnapshotRequest[] = [];
  let subscription = 0;
  const client: ReconcileClient = {
    subscribe(request) {
      subscription += 1;
      if (subscription === 1) return values([event(1n), event(3n)]);
      assert.equal(request.cursor!.value, 0n);
      return values([event(1n), event(2n)]);
    },
    async loadSnapshot(request) {
      snapshotRequests.push(request);
      assert.equal(projection.visibleConnectivity, SessionConnectivityState.STALE);
      return snapshotResponse(2n);
    },
  };

  const reconciler = new Reconciler(client, projection, { retryDelayMs: 0 });
  for await (const next of reconciler.subscribe(DOMAIN)) {
    if (eventLsn(next) === 3n) break;
  }

  assert.deepEqual(projection.folded, [1n, 2n, 3n]);
  assert.equal(projection.snapshots, 1);
  assert.equal(snapshotRequests[0]!.atOrBefore!.value, 2n);
  assert.equal(reconciler.currentCursor, 3n);
  assert.deepEqual(projection.marks, ["event-gap"]);
});

test("snapshot reconciliation replays non-session events hidden behind the higher snapshot LSN", async () => {
  const projection = new PresentationProjection();
  let subscription = 0;
  const prefix = [operationEvent(1n), elicitationEvent(2n), observationEvent(3n)];
  const client: ReconcileClient = {
    subscribe(request) {
      subscription += 1;
      if (subscription === 1) return brokenAfter([prefix[0]!]);
      if (subscription === 2) {
        assert.equal(request.cursor!.value, 0n);
        return values(prefix);
      }
      assert.equal(request.cursor!.value, 3n);
      return values([event(4n)]);
    },
    async loadSnapshot() {
      return snapshotResponse(3n);
    },
  };
  const reconciler = new Reconciler(client, projection, { retryDelayMs: 0 });

  for await (const next of reconciler.subscribe(DOMAIN)) {
    if (eventLsn(next) === 4n) break;
  }

  assert.equal(projection.model.cursor, 4n);
  assert.equal(projection.model.reconciled, true);
  assert.equal(projection.model.commands.has("command-1"), true);
  assert.equal(projection.model.elicitations.has("elicitation-1"), true);
  assert.equal(projection.model.observations.some((item) => item.id === "message-1"), true);
  assert.equal(projection.model.sessions.size, 1);
});

test("the cursor does not advance when projection folding throws", async () => {
  const controller = new AbortController();
  const projection: ReconcileProjection = {
    markUnreconciled() {},
    replaceFromSnapshot() {},
    foldEvent() {
      throw new Error("injected fold failure");
    },
  };
  const client: ReconcileClient = {
    subscribe: () => values([event(1n)]),
    async loadSnapshot() {
      return snapshotResponse(0n);
    },
  };
  const reconciler = new Reconciler(client, projection, {
    retryDelayMs: 1,
    delay: async () => controller.abort(),
  });

  for await (const _ of reconciler.subscribe(DOMAIN, controller.signal)) {
    assert.fail("a failed fold must not yield an event");
  }

  assert.equal(reconciler.currentCursor, 0n);
});

test("unreconciled state is never visible as live across generated stream breaks", async () => {
  await fc.assert(
    fc.asyncProperty(fc.integer({ min: 1, max: 20 }), async (breakAfter) => {
      const projection = new RecordingProjection();
      const observedAtSnapshotLoad: SessionConnectivityState[] = [];
      let subscription = 0;
      const firstEvents = Array.from({ length: breakAfter }, (_, index) => event(BigInt(index + 1)));
      const client: ReconcileClient = {
        subscribe(request: SubscribeRequest) {
          subscription += 1;
          if (subscription === 1) {
            assert.equal(request.cursor!.value, 0n);
            return brokenAfter(firstEvents);
          }
          if (subscription === 2) {
            assert.equal(request.cursor!.value, 0n);
            return values(firstEvents);
          }
          assert.equal(request.cursor!.value, BigInt(breakAfter));
          return values([event(BigInt(breakAfter + 1))]);
        },
        async loadSnapshot(): Promise<LoadSnapshotResponse> {
          observedAtSnapshotLoad.push(projection.visibleConnectivity);
          return snapshotResponse(BigInt(breakAfter));
        },
      };
      const reconciler = new Reconciler(client, projection, { retryDelayMs: 0 });

      let count = 0;
      for await (const _ of reconciler.subscribe(DOMAIN)) {
        count += 1;
        if (count === breakAfter + 1) break;
      }

      assert.deepEqual(observedAtSnapshotLoad, [SessionConnectivityState.STALE]);
      assert.equal(reconciler.currentCursor, BigInt(breakAfter + 1));
      assert.equal(new Set(projection.folded).size, projection.folded.length);
    }),
    { numRuns: 100 },
  );
});

function event(lsn: bigint): SubscribeEvent {
  return create(SubscribeEventSchema, {
    eventId: create(EventIdSchema, {
      authorityDomainId: DOMAIN,
      lsn: create(LsnSchema, { value: lsn }),
    }),
    payload: create(StoredEventPayloadSchema, {
      kind: StoredEventKind.GRANT,
      payload: new Uint8Array(),
    }),
  });
}

function operationEvent(lsn: bigint): SubscribeEvent {
  return stored(
    lsn,
    StoredEventKind.OPERATION,
    OperationSchema,
    create(OperationSchema, {
      commandId: create(CommandIdSchema, { value: "command-1" }),
      authorityDomainId: DOMAIN,
      kind: OperationKind.INSTRUCT,
      targetScope: target(),
      idempotencyKey: "command-1-key",
    }),
  );
}

function elicitationEvent(lsn: bigint): SubscribeEvent {
  return stored(
    lsn,
    StoredEventKind.ELICITATION,
    ElicitationSchema,
    create(ElicitationSchema, {
      elicitationId: create(ElicitationIdSchema, { value: "elicitation-1" }),
      authorityDomainId: DOMAIN,
      targetContext: target(),
      responseContract: create(ResponseContractSchema, {
        contractKind: ResponseContractKind.QUESTION,
        contractBody: {
          case: "question",
          value: create(QuestionContractSchema, { allowFreeText: true }),
        },
      }),
      state: ElicitationState.PENDING,
      payload: create(PayloadEnvelopeSchema, {
        contentType: PayloadContentType.TEXT_UTF8,
        payload: new TextEncoder().encode("Continue?"),
      }),
    }),
  );
}

function observationEvent(lsn: bigint): SubscribeEvent {
  return stored(
    lsn,
    StoredEventKind.OBSERVATION,
    ObservationSchema,
    create(ObservationSchema, {
      authorityDomainId: DOMAIN,
      kind: ObservationKind.EVENT,
      targetScope: target(),
      payload: create(PayloadEnvelopeSchema, {
        contentType: PayloadContentType.JSON,
        schemaRef: "patchbay.pi.TranscriptEvent.v1",
        payload: new TextEncoder().encode(JSON.stringify({
          kind: "assistant_committed",
          messageId: "message-1",
          text: "Recovered",
        })),
      }),
    }),
  );
}

function target() {
  return create(TargetScopeSchema, {
    kind: TargetScopeKind.RUNTIME_SESSION,
    adapterId: ADAPTER,
    deploymentScope: "laptop",
    runtimeSessionId: RUNTIME,
    sessionGeneration: create(GenerationSchema, { value: 1n }),
  });
}

function snapshotResponse(lsn: bigint, authorityDomainId: AuthorityDomainId = DOMAIN): LoadSnapshotResponse {
  const snapshot = create(SessionSnapshotSchema, {
    authorityDomainId,
    snapshotLsn: create(LsnSchema, { value: lsn }),
    sessions: [
      create(SessionSchema, {
        authorityDomainId,
        adapterId: ADAPTER,
        deploymentScope: "laptop",
        runtimeSessionId: RUNTIME,
        sessionGeneration: create(GenerationSchema, { value: 1n }),
        state: create(SessionStateSchema, {
          connectivity: SessionConnectivityState.LIVE,
          activity: SessionActivityState.IDLE,
        }),
      }),
    ],
  });
  return create(LoadSnapshotResponseSchema, {
    present: true,
    eventId: create(EventIdSchema, {
      authorityDomainId,
      lsn: create(LsnSchema, { value: lsn }),
    }),
    snapshotPayload: toBinary(SessionSnapshotSchema, snapshot),
  });
}

function stored<D extends DescMessage>(
  lsn: bigint,
  kind: StoredEventKind,
  schema: D,
  message: MessageShape<D>,
): SubscribeEvent {
  return create(SubscribeEventSchema, {
    eventId: create(EventIdSchema, {
      authorityDomainId: DOMAIN,
      lsn: create(LsnSchema, { value: lsn }),
    }),
    payload: create(StoredEventPayloadSchema, {
      kind,
      payload: toBinary(schema, message),
    }),
  });
}

function eventLsn(event: SubscribeEvent): bigint {
  return event.eventId!.lsn!.value;
}

async function* values(events: SubscribeEvent[]): AsyncIterable<SubscribeEvent> {
  yield* events;
}

async function* brokenAfter(events: SubscribeEvent[]): AsyncIterable<SubscribeEvent> {
  yield* events;
  throw new Error("injected stream break");
}
