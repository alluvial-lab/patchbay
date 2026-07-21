import assert from "node:assert/strict";
import test from "node:test";

import { create, toBinary } from "@bufbuild/protobuf";
import {
  AuthorityDomainIdSchema,
  EventIdSchema,
  LoadSnapshotResponseSchema,
  LsnSchema,
  SessionActivityState,
  SessionConnectivityState,
  SessionSchema,
  SessionSnapshotSchema,
  SessionStateSchema,
  StoredEventKind,
  StoredEventPayloadSchema,
  SubscribeEventSchema,
  type AuthorityDomainId,
  type LoadSnapshotRequest,
  type LoadSnapshotResponse,
  type SessionSnapshot,
  type SubscribeEvent,
  type SubscribeRequest,
} from "@patchbay/contracts";
import fc from "fast-check";

import {
  Reconciler,
  type ReconcileClient,
  type ReconcileProjection,
} from "../src/domain/reconcile.js";

const DOMAIN = create(AuthorityDomainIdSchema, { value: "operator-domain" });

class RecordingProjection implements ReconcileProjection {
  readonly folded: bigint[] = [];
  readonly marks: Array<"stream-break" | "event-gap"> = [];
  visibleConnectivity = SessionConnectivityState.LIVE;
  snapshots = 0;

  markUnreconciled(reason: "stream-break" | "event-gap"): void {
    this.marks.push(reason);
    this.visibleConnectivity = SessionConnectivityState.STALE;
  }

  replaceFromSnapshot(snapshot: SessionSnapshot): void {
    this.snapshots += 1;
    this.visibleConnectivity =
      snapshot.sessions[0]?.state?.connectivity ?? SessionConnectivityState.UNKNOWN;
  }

  foldEvent(event: SubscribeEvent): void {
    this.folded.push(event.eventId!.lsn!.value);
  }
}

test("stream breaks reconcile before resuming from the last folded cursor", async () => {
  const projection = new RecordingProjection();
  const cursors: bigint[] = [];
  let subscription = 0;
  const client: ReconcileClient = {
    subscribe(request) {
      cursors.push(request.cursor!.value);
      subscription += 1;
      return subscription === 1 ? brokenAfter([event(1n)]) : values([event(2n)]);
    },
    async loadSnapshot() {
      assert.equal(projection.visibleConnectivity, SessionConnectivityState.STALE);
      return snapshotResponse(1n);
    },
  };

  const reconciler = new Reconciler(client, projection, { retryDelayMs: 0 });
  const received: bigint[] = [];
  for await (const next of reconciler.subscribe(DOMAIN)) {
    received.push(next.eventId!.lsn!.value);
    if (received.length === 2) break;
  }

  assert.deepEqual(received, [1n, 2n]);
  assert.deepEqual(projection.folded, [1n, 2n]);
  assert.deepEqual(cursors, [0n, 1n]);
  assert.equal(reconciler.currentCursor, 2n);
  assert.deepEqual(projection.marks, ["stream-break"]);
});

test("an event gap is repaired with a bounded replacing snapshot before folding", async () => {
  const projection = new RecordingProjection();
  const snapshotRequests: LoadSnapshotRequest[] = [];
  const client: ReconcileClient = {
    subscribe: () => values([event(1n), event(3n)]),
    async loadSnapshot(request) {
      snapshotRequests.push(request);
      assert.equal(projection.visibleConnectivity, SessionConnectivityState.STALE);
      return snapshotResponse(2n);
    },
  };

  const reconciler = new Reconciler(client, projection, { retryDelayMs: 0 });
  for await (const next of reconciler.subscribe(DOMAIN)) {
    if (next.eventId!.lsn!.value === 3n) break;
  }

  assert.deepEqual(projection.folded, [1n, 3n]);
  assert.equal(projection.snapshots, 1);
  assert.equal(snapshotRequests[0]!.atOrBefore!.value, 2n);
  assert.equal(reconciler.currentCursor, 3n);
  assert.deepEqual(projection.marks, ["event-gap"]);
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
          assert.equal(request.cursor!.value, subscription === 1 ? 0n : BigInt(breakAfter));
          return subscription === 1
            ? brokenAfter(firstEvents)
            : values([event(BigInt(breakAfter + 1))]);
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
      kind: StoredEventKind.OPERATION,
      payload: new Uint8Array(),
    }),
  });
}

function snapshotResponse(lsn: bigint, authorityDomainId: AuthorityDomainId = DOMAIN): LoadSnapshotResponse {
  const snapshot = create(SessionSnapshotSchema, {
    authorityDomainId,
    snapshotLsn: create(LsnSchema, { value: lsn }),
    sessions: [
      create(SessionSchema, {
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

async function* values(events: SubscribeEvent[]): AsyncIterable<SubscribeEvent> {
  yield* events;
}

async function* brokenAfter(events: SubscribeEvent[]): AsyncIterable<SubscribeEvent> {
  yield* events;
  throw new Error("injected stream break");
}
