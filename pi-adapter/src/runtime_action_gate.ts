import type { PiRpcRuntime } from "./pi_process.js";

export type RuntimeActionKind =
  | "delivery"
  | "query"
  | "reconfigure"
  | "reload"
  | "compaction"
  | "termination";

export class RuntimeActionFencedError extends Error {
  readonly failureCode = "superseded";

  constructor() {
    super("runtime actions are fenced by an exact pending replacement");
    this.name = "RuntimeActionFencedError";
  }
}

export class RuntimeActionGateError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "RuntimeActionGateError";
  }
}

export type RuntimeActionBusyReason = "direct_rpc_busy";

export class RuntimeActionBusyError extends Error {
  readonly reason: RuntimeActionBusyReason;

  constructor(reason: RuntimeActionBusyReason) {
    super("runtime action gate is busy");
    this.name = "RuntimeActionBusyError";
    this.reason = reason;
  }
}

export interface SettledRuntimeSnapshot {
  readonly pid: number;
  readonly processToken: string;
  readonly sessionId: string;
  readonly sessionFile: string;
  readonly isStreaming: boolean;
  readonly isCompacting: boolean;
  readonly pendingMessageCount: number;
  readonly lastActivityStartEpoch: number;
  readonly lastAgentSettledEpoch: number;
  readonly noActivityStarted: boolean;
  readonly settledAfterLatestActivity: boolean;
}

interface FenceState {
  readonly claimOperationId: string;
  readonly poisoned: boolean;
}

/**
 * One stdin/action owner for a managed logical target. Ordinary actions fail
 * rather than queue behind an active replacement fence: executing them after
 * promotion would target a different runtime. Replacement decisions have a
 * separate target mutex so exact-envelope validation and journal ownership are
 * serialized before the accepted fence is activated on the action mutex.
 */
export class RuntimeActionGate {
  #actionTail: Promise<void> = Promise.resolve();
  #replacementTail: Promise<void> = Promise.resolve();
  #fence: FenceState | undefined;
  #activeTargetLock: symbol | undefined;
  #activeLease: symbol | undefined;
  #actionReservations = 0;
  #activityEventEpoch = 0;
  #observationsFenced = false;
  #lastActivityStartEpoch = 0;
  #lastAgentSettledEpoch = 0;

  get fencedClaimOperationId(): string | undefined {
    return this.#fence?.claimOperationId;
  }

  get poisoned(): boolean {
    return this.#fence?.poisoned ?? false;
  }

  get observationsFenced(): boolean {
    return this.#observationsFenced;
  }

  async runAction<T>(kind: RuntimeActionKind, action: () => Promise<T>): Promise<T> {
    if (this.#fence) throw new RuntimeActionFencedError();
    this.#actionReservations += 1;
    const release = await this.#acquireAction();
    try {
      if (this.#fence) throw new RuntimeActionFencedError();
      if (!kind) throw new RuntimeActionGateError("runtime action kind must not be empty");
      return await action();
    } finally {
      this.#actionReservations -= 1;
      release();
    }
  }

  noteActivityStart(): void {
    this.#activityEventEpoch += 1;
    this.#lastActivityStartEpoch = this.#activityEventEpoch;
  }

  noteAgentSettled(): void {
    this.#activityEventEpoch += 1;
    this.#lastAgentSettledEpoch = this.#activityEventEpoch;
  }

  /**
   * Fail-fast ownership for reload admission. Unlike ordinary actions, reload
   * never waits behind existing RPC work and then acts on a later idle moment.
   * Once admitted, later actions serialize behind this owner through command
   * invocation and all rehydration work.
   */
  async withExclusiveCurrent<T>(
    runtime: PiRpcRuntime,
    action: (snapshot: SettledRuntimeSnapshot) => Promise<T>,
  ): Promise<T> {
    if (this.#fence) throw new RuntimeActionFencedError();
    if (this.#actionReservations !== 0 || runtime.rpc.pendingRequestCount !== 0) {
      throw new RuntimeActionBusyError("direct_rpc_busy");
    }
    this.#actionReservations += 1;
    const release = await this.#acquireAction();
    try {
      if (this.#fence) throw new RuntimeActionFencedError();
      if (runtime.rpc.pendingRequestCount !== 0) {
        throw new RuntimeActionBusyError("direct_rpc_busy");
      }
      this.#observationsFenced = true;
      const state = await runtime.rpc.request<Record<string, unknown>>({ type: "get_state" });
      if (runtime.rpc.pendingRequestCount !== 0) {
        throw new RuntimeActionGateError("exclusive get_state left an outstanding RPC request");
      }
      return await action(this.#settledSnapshot(runtime, state));
    } finally {
      this.#observationsFenced = false;
      this.#actionReservations -= 1;
      release();
    }
  }

  /**
   * Serialize replacement decisions before validation or journal work without
   * prematurely fencing ordinary actions. The caller must either activate the
   * accepted fence or release this target lock.
   */
  async acquireReplacementTarget(claimOperationId: string): Promise<RuntimeReplacementTargetLock> {
    if (!isBoundedId(claimOperationId)) {
      throw new RuntimeActionGateError("replacement claim operation id is invalid");
    }
    if (this.#fence && this.#fence.claimOperationId !== claimOperationId) {
      throw new RuntimeActionFencedError();
    }
    const releaseTarget = await acquireTail(
      () => this.#replacementTail,
      (next) => { this.#replacementTail = next; },
    );
    if (this.#fence && this.#fence.claimOperationId !== claimOperationId) {
      releaseTarget();
      throw new RuntimeActionFencedError();
    }
    if (this.#activeTargetLock) {
      releaseTarget();
      throw new RuntimeActionGateError("replacement target lock is already active");
    }
    const token = Symbol(claimOperationId);
    this.#activeTargetLock = token;
    return new RuntimeReplacementTargetLock(this, token, claimOperationId, releaseTarget);
  }

  /** Convenience path for callers which have no validation/journal prefix. */
  async acquireReplacement(claimOperationId: string): Promise<RuntimeReplacementLease> {
    const target = await this.acquireReplacementTarget(claimOperationId);
    try {
      return await target.activateFence();
    } catch (error) {
      target.release();
      throw error;
    }
  }

  async activateReplacement(
    targetToken: symbol,
    claimOperationId: string,
    releaseTarget: () => void,
  ): Promise<RuntimeReplacementLease> {
    this.assertTargetLock(targetToken, claimOperationId);
    if (this.#fence && this.#fence.claimOperationId !== claimOperationId) {
      throw new RuntimeActionFencedError();
    }
    // Consume the accepted fence before waiting for the current stdin owner.
    // Actions arriving during that wait must reject, not queue past promotion.
    this.#fence = Object.freeze({ claimOperationId, poisoned: this.#fence?.poisoned ?? false });
    const releaseAction = await this.#acquireAction();
    try {
      this.assertTargetLock(targetToken, claimOperationId);
      if (this.#activeLease) {
        throw new RuntimeActionGateError("replacement lease is already active");
      }
      const token = Symbol(claimOperationId);
      this.#activeLease = token;
      return new RuntimeReplacementLease(
        this,
        token,
        targetToken,
        claimOperationId,
        releaseAction,
        releaseTarget,
      );
    } catch (error) {
      releaseAction();
      throw error;
    }
  }

  assertTargetLock(token: symbol, claimOperationId: string): void {
    if (this.#activeTargetLock !== token || !isBoundedId(claimOperationId)) {
      throw new RuntimeActionGateError("replacement target lock is stale");
    }
  }

  releaseTargetLock(token: symbol, claimOperationId: string, release: () => void): void {
    this.assertTargetLock(token, claimOperationId);
    if (this.#activeLease) {
      throw new RuntimeActionGateError("cannot release a target lock with an active replacement lease");
    }
    this.#activeTargetLock = undefined;
    release();
  }

  assertLease(token: symbol, claimOperationId: string): void {
    if (
      this.#activeLease !== token ||
      this.#fence?.claimOperationId !== claimOperationId
    ) {
      throw new RuntimeActionGateError("replacement lease is stale");
    }
  }

  finishLease(
    token: symbol,
    targetToken: symbol,
    claimOperationId: string,
    releaseAction: () => void,
    releaseTarget: () => void,
    disposition: "promoted" | "released" | "retain" | "poison",
  ): void {
    this.assertLease(token, claimOperationId);
    this.assertTargetLock(targetToken, claimOperationId);
    this.#activeLease = undefined;
    this.#activeTargetLock = undefined;
    if (disposition === "promoted" || disposition === "released") {
      this.#fence = undefined;
    } else {
      this.#fence = Object.freeze({
        claimOperationId,
        poisoned: disposition === "poison" || this.#fence?.poisoned === true,
      });
    }
    releaseAction();
    releaseTarget();
  }

  #settledSnapshot(
    runtime: PiRpcRuntime,
    state: Readonly<Record<string, unknown>>,
  ): SettledRuntimeSnapshot {
    const sessionId = boundedStateString(state["sessionId"]);
    const sessionFile = boundedStateString(state["sessionFile"]);
    const isStreaming = booleanStateField(state, "isStreaming");
    const isCompacting = booleanStateField(state, "isCompacting");
    const pendingMessageCount = nonnegativeStateInteger(state["pendingMessageCount"]);
    return Object.freeze({
      pid: runtime.pid,
      processToken: runtime.processToken,
      sessionId,
      sessionFile,
      isStreaming,
      isCompacting,
      pendingMessageCount,
      lastActivityStartEpoch: this.#lastActivityStartEpoch,
      lastAgentSettledEpoch: this.#lastAgentSettledEpoch,
      noActivityStarted: this.#lastActivityStartEpoch === 0,
      settledAfterLatestActivity:
        this.#lastActivityStartEpoch === 0
        || this.#lastAgentSettledEpoch > this.#lastActivityStartEpoch,
    });
  }

  #acquireAction(): Promise<() => void> {
    return acquireTail(
      () => this.#actionTail,
      (next) => { this.#actionTail = next; },
    );
  }
}

export class RuntimeReplacementTargetLock {
  #closed = false;
  readonly #releaseTarget: () => void;

  constructor(
    readonly gate: RuntimeActionGate,
    readonly token: symbol,
    readonly claimOperationId: string,
    releaseTarget: () => void,
  ) {
    this.#releaseTarget = releaseTarget;
  }

  async activateFence(): Promise<RuntimeReplacementLease> {
    this.assertCurrent();
    const lease = await this.gate.activateReplacement(
      this.token,
      this.claimOperationId,
      this.#releaseTarget,
    );
    this.#closed = true;
    return lease;
  }

  release(): void {
    this.assertCurrent();
    this.#closed = true;
    this.gate.releaseTargetLock(this.token, this.claimOperationId, this.#releaseTarget);
  }

  private assertCurrent(): void {
    if (this.#closed) throw new RuntimeActionGateError("replacement target lock is closed");
    this.gate.assertTargetLock(this.token, this.claimOperationId);
  }
}

export class RuntimeReplacementLease {
  #closed = false;
  readonly #releaseAction: () => void;
  readonly #releaseTarget: () => void;

  constructor(
    readonly gate: RuntimeActionGate,
    readonly token: symbol,
    readonly targetToken: symbol,
    readonly claimOperationId: string,
    releaseAction: () => void,
    releaseTarget: () => void,
  ) {
    this.#releaseAction = releaseAction;
    this.#releaseTarget = releaseTarget;
  }

  assertCurrent(): void {
    if (this.#closed) throw new RuntimeActionGateError("replacement lease is closed");
    this.gate.assertLease(this.token, this.claimOperationId);
  }

  promoted(): void {
    this.#finish("promoted");
  }

  release(): void {
    this.#finish("released");
  }

  retainFence(): void {
    this.#finish("retain");
  }

  poison(): void {
    this.#finish("poison");
  }

  #finish(disposition: "promoted" | "released" | "retain" | "poison"): void {
    this.assertCurrent();
    this.#closed = true;
    this.gate.finishLease(
      this.token,
      this.targetToken,
      this.claimOperationId,
      this.#releaseAction,
      this.#releaseTarget,
      disposition,
    );
  }
}

async function acquireTail(
  current: () => Promise<void>,
  replace: (next: Promise<void>) => void,
): Promise<() => void> {
  const previous = current();
  let release!: () => void;
  replace(new Promise<void>((resolve) => { release = resolve; }));
  await previous;
  let released = false;
  return () => {
    if (released) return;
    released = true;
    release();
  };
}

function isBoundedId(value: string): boolean {
  return value.length > 0 && value.length <= 1_024 && !value.includes("\0");
}

function boundedStateString(value: unknown): string {
  if (typeof value !== "string" || value.length === 0 || value.length > 4_096 || value.includes("\0")) {
    throw new RuntimeActionGateError("exclusive get_state identity is malformed");
  }
  return value;
}

function booleanStateField(value: Readonly<Record<string, unknown>>, field: string): boolean {
  if (typeof value[field] !== "boolean") {
    throw new RuntimeActionGateError(`exclusive get_state ${field} is malformed`);
  }
  return value[field];
}

function nonnegativeStateInteger(value: unknown): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    throw new RuntimeActionGateError("exclusive get_state pendingMessageCount is malformed");
  }
  return value as number;
}
