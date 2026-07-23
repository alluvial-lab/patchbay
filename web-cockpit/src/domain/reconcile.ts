import { create, fromBinary } from "@bufbuild/protobuf";
import {
  LoadSnapshotRequestSchema,
  LsnSchema,
  SessionSnapshotSchema,
  SubscribeRequestSchema,
  type AuthorityDomainId,
  type LoadSnapshotRequest,
  type LoadSnapshotResponse,
  type SessionSnapshot,
  type SubscribeEvent,
  type SubscribeRequest,
} from "@patchbay/contracts";

export interface ReconcileClient {
  subscribe(input: SubscribeRequest, options?: { signal?: AbortSignal }): AsyncIterable<SubscribeEvent>;
  loadSnapshot(input: LoadSnapshotRequest): Promise<LoadSnapshotResponse>;
}

/** Port implemented by the pure presentation projection in model.ts. */
export interface ReconcileProjection {
  markUnreconciled(reason: "stream-break" | "event-gap"): void;
  replaceFromSnapshot(
    snapshot: SessionSnapshot,
    replayEvents: readonly SubscribeEvent[],
  ): void | Promise<void>;
  foldEvent(event: SubscribeEvent): void | Promise<void>;
}

export interface ReconcilerOptions {
  initialCursor?: bigint;
  retryDelayMs?: number;
  delay?: (milliseconds: number, signal?: AbortSignal) => Promise<void>;
}

/**
 * Cursor-based reconnect driver. The cursor advances only after the projection
 * has folded an event, and snapshots replace rather than merge the projection.
 */
export class Reconciler {
  private cursor: bigint;
  private readonly retryDelayMs: number;
  private readonly delay: (milliseconds: number, signal?: AbortSignal) => Promise<void>;

  constructor(
    private readonly client: ReconcileClient,
    private readonly projection: ReconcileProjection,
    options: ReconcilerOptions = {},
  ) {
    this.cursor = options.initialCursor ?? 0n;
    this.retryDelayMs = options.retryDelayMs ?? 500;
    this.delay = options.delay ?? abortableDelay;
  }

  get currentCursor(): bigint {
    return this.cursor;
  }

  async *subscribe(
    authorityDomainId: AuthorityDomainId,
    signal?: AbortSignal,
  ): AsyncIterable<SubscribeEvent> {
    assertId(authorityDomainId.value, "authority domain");

    while (!signal?.aborted) {
      try {
        const request = create(SubscribeRequestSchema, {
          authorityDomainId,
          cursor: create(LsnSchema, { value: this.cursor }),
        });
        for await (const event of this.client.subscribe(request, { signal })) {
          if (signal?.aborted) return;
          const lsn = eventLsn(event, authorityDomainId.value);
          if (lsn <= this.cursor) continue;

          if (lsn > this.cursor + 1n) {
            this.projection.markUnreconciled("event-gap");
            await this.reconcile(authorityDomainId);
            if (lsn <= this.cursor) {
              // Reconciliation replayed this visible event while adopting the
              // current snapshot. Yield it once so presentation subscribers
              // render the atomically replaced model.
              yield event;
              continue;
            }
            if (lsn !== this.cursor + 1n) {
              throw new Error(`snapshot did not bridge event gap before LSN ${lsn}`);
            }
          }

          await this.projection.foldEvent(event);
          this.cursor = lsn;
          yield event;
        }
        if (signal?.aborted) return;
        // Subscribe returns the currently durable prefix and then completes.
        // Completion at the tail is the normal polling boundary, so retain the
        // reconciled projection and resume from the last folded cursor.
        await this.delay(this.retryDelayMs, signal);
      } catch (error) {
        if (signal?.aborted || isAbortError(error)) return;
        this.projection.markUnreconciled("stream-break");
        try {
          await this.reconcile(authorityDomainId);
        } catch {
          // The projection remains explicitly unreconciled. Retry from the
          // last folded cursor; cached UI state never becomes authority.
        }
        await this.delay(this.retryDelayMs, signal);
      }
    }
  }

  private async reconcile(authorityDomainId: AuthorityDomainId): Promise<void> {
    const response = await this.client.loadSnapshot(
      create(LoadSnapshotRequestSchema, { authorityDomainId }),
    );
    if (!response.present) throw new Error("authoritative snapshot is unavailable");

    const snapshot = fromBinary(SessionSnapshotSchema, response.snapshotPayload);
    const snapshotDomain = required(snapshot.authorityDomainId?.value, "snapshot authority domain");
    const responseDomain = required(response.eventId?.authorityDomainId?.value, "snapshot event domain");
    if (snapshotDomain !== authorityDomainId.value || responseDomain !== authorityDomainId.value) {
      throw new Error("cross-domain snapshot rejected");
    }

    const snapshotLsn = requiredBigint(snapshot.snapshotLsn?.value, "snapshot LSN");
    const responseLsn = requiredBigint(response.eventId?.lsn?.value, "snapshot event LSN");
    if (snapshotLsn !== responseLsn) throw new Error("snapshot LSN does not match response event LSN");
    if (snapshotLsn < this.cursor) throw new Error("older snapshot rejected");

    // SessionSnapshot is authoritative only for the session registry. Rebuild
    // every other presentation axis from the durable prefix instead of merging
    // cached browser state or skipping events hidden behind snapshot_lsn.
    const replayEvents = await this.replayThrough(authorityDomainId, snapshotLsn);
    await this.projection.replaceFromSnapshot(snapshot, replayEvents);
    this.cursor = snapshotLsn;
  }

  private async replayThrough(
    authorityDomainId: AuthorityDomainId,
    snapshotLsn: bigint,
  ): Promise<SubscribeEvent[]> {
    if (snapshotLsn === 0n) return [];

    const replayEvents: SubscribeEvent[] = [];
    let previousLsn = 0n;
    const request = create(SubscribeRequestSchema, {
      authorityDomainId,
      cursor: create(LsnSchema, { value: 0n }),
    });
    for await (const event of this.client.subscribe(request)) {
      const lsn = eventLsn(event, authorityDomainId.value);
      if (lsn <= previousLsn) {
        throw new Error(`snapshot replay is not strictly ordered at LSN ${lsn}`);
      }
      if (lsn > snapshotLsn) return replayEvents;
      replayEvents.push(event);
      previousLsn = lsn;
      if (lsn === snapshotLsn) return replayEvents;
    }
    // The operator-facing stream deliberately omits authority records while
    // retaining their LSNs. Clean completion therefore proves the complete
    // visible prefix even when its final event precedes snapshot_lsn.
    return replayEvents;
  }
}

function eventLsn(event: SubscribeEvent, authorityDomain: string): bigint {
  const eventDomain = required(event.eventId?.authorityDomainId?.value, "event authority domain");
  if (eventDomain !== authorityDomain) throw new Error("cross-domain event rejected");
  if (!event.payload) throw new Error("subscription event payload is missing");
  return requiredBigint(event.eventId?.lsn?.value, "event LSN");
}

function required(value: string | undefined, name: string): string {
  assertId(value, name);
  return value;
}

function assertId(value: string | undefined, name: string): asserts value is string {
  if (!value) throw new Error(`${name} is missing`);
}

function requiredBigint(value: bigint | undefined, name: string): bigint {
  if (value === undefined || value < 0n) throw new Error(`${name} is missing or invalid`);
  return value;
}

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === "AbortError";
}

async function abortableDelay(milliseconds: number, signal?: AbortSignal): Promise<void> {
  if (milliseconds <= 0) return;
  await new Promise<void>((resolve, reject) => {
    const timeout = setTimeout(resolve, milliseconds);
    signal?.addEventListener(
      "abort",
      () => {
        clearTimeout(timeout);
        reject(new DOMException("Aborted", "AbortError"));
      },
      { once: true },
    );
  });
}
