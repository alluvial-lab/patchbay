const DEFAULT_MAX_FAILURES = 5;
const DEFAULT_WINDOW_MS = 60_000;
const DEFAULT_MAX_CONCURRENT_VERIFICATIONS = 2;
const DEFAULT_MAX_TRACKED_NETWORKS = 1_024;

export type LoginLimitDimension = "account" | "network";

export interface LoginLimiterOptions {
  now?: () => number;
  windowMs?: number;
  accountMaxFailures?: number;
  networkMaxFailures?: number;
  maxConcurrentVerifications?: number;
  maxTrackedNetworks?: number;
}

export type LoginLimitDecision =
  | { allowed: true }
  | {
      allowed: false;
      blockedDimensions: readonly LoginLimitDimension[];
      retryAfterMs: number;
    };

interface AttemptWindow {
  windowStartedAt: number;
  failures: number;
  inFlight: number;
  lastTouchedAt: number;
}

/**
 * A process-local limiter for the one configured v0.1.0 operator account.
 * Failed-attempt windows decay, and concurrent verification is capped so a
 * burst cannot queue an unbounded number of scrypt calls before failures land.
 */
export class LoginLimiter {
  readonly #now: () => number;
  readonly #windowMs: number;
  readonly #accountMaxFailures: number;
  readonly #networkMaxFailures: number;
  readonly #maxConcurrentVerifications: number;
  readonly #maxTrackedNetworks: number;
  readonly #account: AttemptWindow;
  readonly #networks = new Map<string, AttemptWindow>();

  constructor(options: LoginLimiterOptions = {}) {
    this.#now = options.now ?? Date.now;
    this.#windowMs = positiveInteger(options.windowMs ?? DEFAULT_WINDOW_MS, "windowMs");
    this.#accountMaxFailures = positiveInteger(
      options.accountMaxFailures ?? DEFAULT_MAX_FAILURES,
      "accountMaxFailures",
    );
    this.#networkMaxFailures = positiveInteger(
      options.networkMaxFailures ?? DEFAULT_MAX_FAILURES,
      "networkMaxFailures",
    );
    this.#maxConcurrentVerifications = positiveInteger(
      options.maxConcurrentVerifications ?? DEFAULT_MAX_CONCURRENT_VERIFICATIONS,
      "maxConcurrentVerifications",
    );
    this.#maxTrackedNetworks = positiveInteger(
      options.maxTrackedNetworks ?? DEFAULT_MAX_TRACKED_NETWORKS,
      "maxTrackedNetworks",
    );
    if (this.#maxTrackedNetworks < this.#maxConcurrentVerifications) {
      throw new Error("maxTrackedNetworks must cover maxConcurrentVerifications");
    }

    const now = this.#now();
    this.#account = newWindow(now);
  }

  beginAttempt(networkAddress: string): LoginLimitDecision {
    const now = this.#now();
    this.#refresh(this.#account, now);
    const network = this.#networkWindow(networkAddress, now);

    const blockedDimensions: LoginLimitDimension[] = [];
    if (
      this.#account.failures >= this.#accountMaxFailures ||
      this.#account.inFlight >= this.#maxConcurrentVerifications
    ) {
      blockedDimensions.push("account");
    }
    if (
      network.failures >= this.#networkMaxFailures ||
      network.inFlight >= this.#maxConcurrentVerifications
    ) {
      blockedDimensions.push("network");
    }

    if (blockedDimensions.length > 0) {
      return {
        allowed: false,
        blockedDimensions,
        retryAfterMs: Math.max(
          ...blockedDimensions.map((dimension) =>
            this.#retryAfterMs(dimension === "account" ? this.#account : network, now),
          ),
        ),
      };
    }

    this.#account.inFlight += 1;
    network.inFlight += 1;
    return { allowed: true };
  }

  recordFailure(networkAddress: string): void {
    const now = this.#now();
    this.#refresh(this.#account, now);
    const network = this.#networkWindow(networkAddress, now);
    this.#account.inFlight = Math.max(0, this.#account.inFlight - 1);
    network.inFlight = Math.max(0, network.inFlight - 1);
    this.#account.failures = Math.min(this.#account.failures + 1, this.#accountMaxFailures);
    network.failures = Math.min(network.failures + 1, this.#networkMaxFailures);
  }

  recordSuccess(networkAddress: string): void {
    const now = this.#now();
    this.#refresh(this.#account, now);
    const network = this.#networkWindow(networkAddress, now);
    this.#account.inFlight = Math.max(0, this.#account.inFlight - 1);
    network.inFlight = Math.max(0, network.inFlight - 1);
    this.#account.failures = 0;
    this.#account.windowStartedAt = now;
    network.failures = 0;
    network.windowStartedAt = now;
  }

  #networkWindow(networkAddress: string, now: number): AttemptWindow {
    const existing = this.#networks.get(networkAddress);
    if (existing) {
      this.#refresh(existing, now);
      existing.lastTouchedAt = now;
      return existing;
    }

    this.#pruneNetworks(now);
    if (this.#networks.size >= this.#maxTrackedNetworks) {
      this.#evictOldestIdleNetwork();
    }

    const created = newWindow(now);
    this.#networks.set(networkAddress, created);
    return created;
  }

  #refresh(window: AttemptWindow, now: number): void {
    if (now - window.windowStartedAt >= this.#windowMs) {
      window.windowStartedAt = now;
      window.failures = 0;
    }
    window.lastTouchedAt = now;
  }

  #retryAfterMs(window: AttemptWindow, now: number): number {
    if (window.failures > 0) {
      return Math.max(1, window.windowStartedAt + this.#windowMs - now);
    }
    return 1_000;
  }

  #pruneNetworks(now: number): void {
    for (const [address, window] of this.#networks) {
      if (window.inFlight === 0 && now - window.windowStartedAt >= this.#windowMs) {
        this.#networks.delete(address);
      }
    }
  }

  #evictOldestIdleNetwork(): void {
    let oldestAddress: string | undefined;
    let oldestTouchedAt = Number.POSITIVE_INFINITY;
    for (const [address, window] of this.#networks) {
      if (window.inFlight === 0 && window.lastTouchedAt < oldestTouchedAt) {
        oldestAddress = address;
        oldestTouchedAt = window.lastTouchedAt;
      }
    }
    if (oldestAddress !== undefined) this.#networks.delete(oldestAddress);
  }
}

function newWindow(now: number): AttemptWindow {
  return { windowStartedAt: now, failures: 0, inFlight: 0, lastTouchedAt: now };
}

function positiveInteger(value: number, name: string): number {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(`${name} must be a positive integer`);
  }
  return value;
}
