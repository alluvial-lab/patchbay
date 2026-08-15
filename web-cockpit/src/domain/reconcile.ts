import { create, fromBinary } from "@bufbuild/protobuf";
import {
  LoadSecuritySnapshotRequestSchema,
  LoadSnapshotRequestSchema,
  LsnSchema,
  ResourceSnapshotSchema,
  SessionSnapshotSchema,
  SnapshotViewKind,
  SubscribeRequestSchema,
  type AuthorityDomainId,
  type LoadSecuritySnapshotRequest,
  type LoadSecuritySnapshotResponse,
  type LoadSnapshotRequest,
  type LoadSnapshotResponse,
  type SecuritySnapshot,
  type SubscribeEvent,
  type SubscribeRequest,
} from "@patchbay/contracts";

import type { SnapshotBaselines } from "./model.js";

export interface ReconcileClient {
  subscribe(input: SubscribeRequest, options?: { signal?: AbortSignal }): AsyncIterable<SubscribeEvent>;
  loadSnapshot(input: LoadSnapshotRequest): Promise<LoadSnapshotResponse>;
  loadSecuritySnapshot?(input: LoadSecuritySnapshotRequest): Promise<LoadSecuritySnapshotResponse>;
}

/** Port implemented by the pure presentation projection in model.ts. */
export interface ReconcileProjection {
  markUnreconciled(reason: "stream-break" | "event-gap"): void;
  replaceFromSnapshots(
    snapshots: SnapshotBaselines,
    replayEvents: readonly SubscribeEvent[],
  ): void | Promise<void>;
  replaceSecuritySnapshot?(snapshot: SecuritySnapshot): void | Promise<void>;
  foldEvent(event: SubscribeEvent): void | Promise<void>;
}

export interface ReconcilerOptions {
  initialCursor?: bigint;
  retryDelayMs?: number;
  delay?: (milliseconds: number, signal?: AbortSignal) => Promise<void>;
  /** Fires after an actual stream-loss snapshot has been installed. */
  onReconciliationComplete?: (reason: "stream-reconnect") => void;
}

/**
 * Cursor-based reconnect driver. The cursor advances only after the projection
 * has folded an event, and snapshots replace rather than merge the projection.
 */
export class Reconciler {
  private cursor: bigint;
  private readonly retryDelayMs: number;
  private readonly delay: (milliseconds: number, signal?: AbortSignal) => Promise<void>;
  private readonly onReconciliationComplete?: (reason: "stream-reconnect") => void;

  constructor(
    private readonly client: ReconcileClient,
    private readonly projection: ReconcileProjection,
    options: ReconcilerOptions = {},
  ) {
    this.cursor = options.initialCursor ?? 0n;
    this.retryDelayMs = options.retryDelayMs ?? 500;
    this.delay = options.delay ?? abortableDelay;
    this.onReconciliationComplete = options.onReconciliationComplete;
  }

  get currentCursor(): bigint {
    return this.cursor;
  }

  /** Refresh the dedicated, redacted security inventory projection. */
  async loadSecuritySnapshot(authorityDomainId: AuthorityDomainId): Promise<void> {
    if (!this.client.loadSecuritySnapshot || !this.projection.replaceSecuritySnapshot) return;
    const response = await this.client.loadSecuritySnapshot(
      create(LoadSecuritySnapshotRequestSchema, { authorityDomainId }),
    );
    if (!response.snapshot) throw new Error("security snapshot is unavailable");
    const snapshotDomain = required(response.snapshot.authorityDomainId?.value, "security snapshot authority domain");
    if (snapshotDomain !== authorityDomainId.value) throw new Error("cross-domain security snapshot rejected");
    await this.projection.replaceSecuritySnapshot(response.snapshot);
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

          // The server intentionally filters authority/audit records from
          // this operator-facing stream. Their LSNs therefore create normal
          // holes, including the lifecycle records emitted by this very
          // diagnostics query. A transport/error path is the actual stream
          // loss signal and is handled below; a successful filtered stream
          // must not replace the snapshot and erase a just-returned status.
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

  /** Force one authoritative snapshot/security reconciliation after an
   * operation whose transport outcome is unknown. */
  async reconcileNow(authorityDomainId: AuthorityDomainId): Promise<void> {
    await this.reconcile(authorityDomainId);
  }

  private async reconcile(authorityDomainId: AuthorityDomainId): Promise<void> {
    // Load and validate both independently materialized axes before touching
    // the cached projection. A failed second read leaves the old model stale.
    const sessionResponse = await this.loadSnapshotView(authorityDomainId, SnapshotViewKind.SESSION);
    const resourceResponse = await this.loadSnapshotView(authorityDomainId, SnapshotViewKind.RESOURCE);
    const session = fromBinary(SessionSnapshotSchema, sessionResponse.snapshotPayload);
    const resource = fromBinary(ResourceSnapshotSchema, resourceResponse.snapshotPayload);
    const sessionCoreGeneration = requiredPositiveBigint(
      session.coreGeneration?.value,
      "session snapshot core generation",
    );
    const resourceCoreGeneration = requiredPositiveBigint(
      resource.coreGeneration?.value,
      "resource snapshot core generation",
    );
    if (sessionCoreGeneration !== resourceCoreGeneration) {
      throw new Error("cross-generation snapshot baselines rejected");
    }
    const sessionLsn = validateSnapshotIdentity(
      session.authorityDomainId?.value,
      session.snapshotLsn?.value,
      sessionResponse,
      authorityDomainId.value,
      "session",
    );
    const resourceLsn = validateSnapshotIdentity(
      resource.authorityDomainId?.value,
      resource.snapshotLsn?.value,
      resourceResponse,
      authorityDomainId.value,
      "resource",
    );
    const horizon = sessionLsn > resourceLsn ? sessionLsn : resourceLsn;
    if (horizon < this.cursor) throw new Error("older snapshot horizon rejected");

    const replayEvents = await this.replayThrough(authorityDomainId, horizon);
    await this.projection.replaceFromSnapshots({ session, resource }, replayEvents);
    await this.loadSecuritySnapshot(authorityDomainId);
    this.cursor = horizon;
    this.onReconciliationComplete?.("stream-reconnect");
  }

  private async loadSnapshotView(
    authorityDomainId: AuthorityDomainId,
    viewKind: SnapshotViewKind.SESSION | SnapshotViewKind.RESOURCE,
  ): Promise<LoadSnapshotResponse> {
    const response = await this.client.loadSnapshot(
      create(LoadSnapshotRequestSchema, { authorityDomainId, viewKind }),
    );
    if (!response.present) throw new Error(`${snapshotViewName(viewKind)} snapshot is unavailable`);
    if (response.viewKind !== viewKind) {
      throw new Error(`core returned the wrong ${snapshotViewName(viewKind)} snapshot view`);
    }
    return response;
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

function validateSnapshotIdentity(
  snapshotDomain: string | undefined,
  snapshotLsn: bigint | undefined,
  response: LoadSnapshotResponse,
  expectedDomain: string,
  viewName: "session" | "resource",
): bigint {
  const payloadDomain = required(snapshotDomain, `${viewName} snapshot authority domain`);
  const responseDomain = required(response.eventId?.authorityDomainId?.value, `${viewName} snapshot event domain`);
  if (payloadDomain !== expectedDomain || responseDomain !== expectedDomain) {
    throw new Error(`cross-domain ${viewName} snapshot rejected`);
  }
  const payloadLsn = requiredBigint(snapshotLsn, `${viewName} snapshot LSN`);
  const responseLsn = requiredBigint(response.eventId?.lsn?.value, `${viewName} snapshot event LSN`);
  if (payloadLsn !== responseLsn) throw new Error(`${viewName} snapshot LSN does not match response event LSN`);
  return payloadLsn;
}

function snapshotViewName(viewKind: SnapshotViewKind): "session" | "resource" {
  return viewKind === SnapshotViewKind.SESSION ? "session" : "resource";
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

function requiredPositiveBigint(value: bigint | undefined, name: string): bigint {
  const parsed = requiredBigint(value, name);
  if (parsed === 0n) throw new Error(`${name} must be positive`);
  return parsed;
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
