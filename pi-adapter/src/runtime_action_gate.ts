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

interface FenceState {
  readonly claimOperationId: string;
  readonly poisoned: boolean;
}

/**
 * One stdin/action owner for a managed logical target. Ordinary actions fail
 * rather than queue behind a replacement fence: executing them after promotion
 * would target a different runtime. A replacement lease holds the mutex across
 * quiesce, termination, launch, verification, staging, and promotion.
 */
export class RuntimeActionGate {
  #tail: Promise<void> = Promise.resolve();
  #fence: FenceState | undefined;
  #activeLease: symbol | undefined;

  get fencedClaimOperationId(): string | undefined {
    return this.#fence?.claimOperationId;
  }

  get poisoned(): boolean {
    return this.#fence?.poisoned ?? false;
  }

  async runAction<T>(kind: RuntimeActionKind, action: () => Promise<T>): Promise<T> {
    if (this.#fence) throw new RuntimeActionFencedError();
    const release = await this.#acquire();
    try {
      if (this.#fence) throw new RuntimeActionFencedError();
      if (!kind) throw new RuntimeActionGateError("runtime action kind must not be empty");
      return await action();
    } finally {
      release();
    }
  }

  async acquireReplacement(claimOperationId: string): Promise<RuntimeReplacementLease> {
    if (!isBoundedId(claimOperationId)) {
      throw new RuntimeActionGateError("replacement claim operation id is invalid");
    }
    if (this.#fence && this.#fence.claimOperationId !== claimOperationId) {
      throw new RuntimeActionFencedError();
    }
    const release = await this.#acquire();
    if (this.#fence && this.#fence.claimOperationId !== claimOperationId) {
      release();
      throw new RuntimeActionFencedError();
    }
    if (this.#activeLease) {
      release();
      throw new RuntimeActionGateError("replacement lease is already active");
    }
    const token = Symbol(claimOperationId);
    this.#activeLease = token;
    this.#fence = Object.freeze({ claimOperationId, poisoned: this.#fence?.poisoned ?? false });
    return new RuntimeReplacementLease(this, token, claimOperationId, release);
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
    claimOperationId: string,
    release: () => void,
    disposition: "promoted" | "released" | "retain" | "poison",
  ): void {
    this.assertLease(token, claimOperationId);
    this.#activeLease = undefined;
    if (disposition === "promoted" || disposition === "released") {
      this.#fence = undefined;
    } else {
      this.#fence = Object.freeze({
        claimOperationId,
        poisoned: disposition === "poison" || this.#fence?.poisoned === true,
      });
    }
    release();
  }

  async #acquire(): Promise<() => void> {
    const previous = this.#tail;
    let release!: () => void;
    this.#tail = new Promise<void>((resolve) => {
      release = resolve;
    });
    await previous;
    let released = false;
    return () => {
      if (released) return;
      released = true;
      release();
    };
  }
}

export class RuntimeReplacementLease {
  #closed = false;
  readonly #release: () => void;

  constructor(
    readonly gate: RuntimeActionGate,
    readonly token: symbol,
    readonly claimOperationId: string,
    release: () => void,
  ) {
    this.#release = release;
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
    this.gate.finishLease(this.token, this.claimOperationId, this.#release, disposition);
  }
}

function isBoundedId(value: string): boolean {
  return value.length > 0 && value.length <= 1_024 && !value.includes("\0");
}
