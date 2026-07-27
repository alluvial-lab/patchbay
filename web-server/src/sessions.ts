import { randomBytes, scrypt, timingSafeEqual } from "node:crypto";
import { promisify } from "node:util";

const scryptAsync = promisify(scrypt);
const TOKEN_BYTES = 32;
const PASSWORD_HASH_BYTES = 64;
const DEFAULT_SESSION_TTL_MS = 8 * 60 * 60 * 1000;

export type SessionStatus = "active" | "revoked" | "expired";

export interface SessionIdentity {
  operatorActorId: string;
  endpointId: string;
  deviceId: string;
  sessionGeneration: bigint;
  coreSessionId?: string;
}

export interface OperatorSession {
  sessionId: string;
  operatorActorId: string;
  endpointId: string;
  deviceId: string;
  sessionGeneration: bigint;
  coreSessionId?: string;
  status: SessionStatus;
  csrfSecret: string;
  createdAt: number;
  lastUsedAt: number;
  expiresAt: number;
  revokedAt: number | null;
}

export interface OperatorRecord {
  actorId: string;
  passwordHash: string;
}

export interface SessionStoreOptions {
  now?: () => number;
  sessionTtlMs?: number;
  randomToken?: () => string;
}

export class SessionStore {
  readonly #sessions = new Map<string, OperatorSession>();
  readonly #now: () => number;
  readonly #sessionTtlMs: number;
  readonly #randomToken: () => string;

  constructor(options: SessionStoreOptions = {}) {
    this.#now = options.now ?? Date.now;
    this.#sessionTtlMs = options.sessionTtlMs ?? DEFAULT_SESSION_TTL_MS;
    this.#randomToken = options.randomToken ?? secureToken;
    if (this.#sessionTtlMs <= 0) {
      throw new Error("session TTL must be positive");
    }
  }

  create(identity: SessionIdentity): OperatorSession;
  /** Compatibility overload for local-only tests and non-core authentication. */
  create(operatorActorId: string, coreSessionId?: string): OperatorSession;
  create(identityOrActor: SessionIdentity | string, coreSessionId?: string): OperatorSession {
    const identity: SessionIdentity = typeof identityOrActor === "string"
      ? {
          operatorActorId: identityOrActor,
          endpointId: "local-endpoint",
          deviceId: "local-device",
          sessionGeneration: 1n,
          ...(coreSessionId ? { coreSessionId } : {}),
        }
      : identityOrActor;
    if (identity.operatorActorId.length === 0) {
      throw new Error("operator actor id must not be empty");
    }
    if (identity.endpointId.length === 0 || identity.deviceId.length === 0) {
      throw new Error("operator session endpoint and device ids must not be empty");
    }
    if (identity.sessionGeneration <= 0n) {
      throw new Error("operator session generation must be positive");
    }
    const now = this.#now();
    let sessionId = this.#randomToken();
    while (this.#sessions.has(sessionId)) {
      sessionId = this.#randomToken();
    }
    const session: OperatorSession = {
      sessionId,
      operatorActorId: identity.operatorActorId,
      endpointId: identity.endpointId,
      deviceId: identity.deviceId,
      sessionGeneration: identity.sessionGeneration,
      ...(identity.coreSessionId ? { coreSessionId: identity.coreSessionId } : {}),
      status: "active",
      csrfSecret: this.#randomToken(),
      createdAt: now,
      lastUsedAt: now,
      expiresAt: now + this.#sessionTtlMs,
      revokedAt: null,
    };
    this.#sessions.set(sessionId, session);
    return session;
  }

  lookup(sessionId: string): OperatorSession | null {
    const session = this.#sessions.get(sessionId);
    if (!session) return null;

    const now = this.#now();
    if (session.status === "active" && now >= session.expiresAt) {
      session.status = "expired";
    } else if (session.status === "active") {
      session.lastUsedAt = now;
    }
    return session;
  }

  revoke(sessionId: string): boolean {
    const session = this.#sessions.get(sessionId);
    if (!session) return false;
    if (session.status === "active") this.markRevoked(session);
    return true;
  }

  /** Removes a browser session whose core-side authority no longer exists. */
  invalidate(sessionId: string): boolean {
    return this.#sessions.delete(sessionId);
  }

  revokeAllForOperator(operatorActorId: string): number {
    let revoked = 0;
    for (const session of this.#sessions.values()) {
      if (session.operatorActorId === operatorActorId && session.status === "active") {
        this.markRevoked(session);
        revoked += 1;
      }
    }
    return revoked;
  }

  revokeForEndpoint(endpointId: string): number {
    return this.revokeMatching((session) => session.endpointId === endpointId);
  }

  revokeForDevice(deviceId: string): number {
    return this.revokeMatching((session) => session.deviceId === deviceId);
  }

  get size(): number {
    return this.#sessions.size;
  }

  private revokeMatching(predicate: (session: OperatorSession) => boolean): number {
    let revoked = 0;
    for (const session of this.#sessions.values()) {
      if (session.status === "active" && predicate(session)) {
        this.markRevoked(session);
        revoked += 1;
      }
    }
    return revoked;
  }

  private markRevoked(session: OperatorSession): void {
    session.status = "revoked";
    if (session.revokedAt === null) session.revokedAt = this.#now();
  }
}

export async function hashPassword(password: string, salt = randomBytes(16)): Promise<string> {
  const derived = (await scryptAsync(password, salt, PASSWORD_HASH_BYTES)) as Buffer;
  return `scrypt$${salt.toString("base64url")}$${derived.toString("base64url")}`;
}

export function assertPasswordHash(passwordHash: string): void {
  const parsed = parsePasswordHash(passwordHash);
  if (!parsed) throw new Error("PATCHBAY_OPERATOR_PASSWORD_HASH must use scrypt$<salt>$<hash>");
}

export async function verifyPassword(password: string, passwordHash: string): Promise<boolean> {
  const parsed = parsePasswordHash(passwordHash);
  if (!parsed) return false;
  const actual = (await scryptAsync(password, parsed.salt, parsed.expected.length)) as Buffer;
  return timingSafeEqual(actual, parsed.expected);
}

function parsePasswordHash(
  passwordHash: string,
): { salt: Buffer; expected: Buffer } | null {
  const [algorithm, encodedSalt, encodedHash, extra] = passwordHash.split("$");
  if (algorithm !== "scrypt" || !encodedSalt || !encodedHash || extra !== undefined) return null;
  const salt = Buffer.from(encodedSalt, "base64url");
  const expected = Buffer.from(encodedHash, "base64url");
  if (salt.length < 16 || expected.length !== PASSWORD_HASH_BYTES) return null;
  return { salt, expected };
}

function secureToken(): string {
  return randomBytes(TOKEN_BYTES).toString("base64url");
}
