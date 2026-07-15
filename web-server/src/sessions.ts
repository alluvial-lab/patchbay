import { randomBytes, scrypt, timingSafeEqual } from "node:crypto";
import { promisify } from "node:util";

const scryptAsync = promisify(scrypt);
const TOKEN_BYTES = 32;
const PASSWORD_HASH_BYTES = 64;
const DEFAULT_SESSION_TTL_MS = 8 * 60 * 60 * 1000;

export type SessionStatus = "active" | "revoked" | "expired";

export interface OperatorSession {
  sessionId: string;
  operatorActorId: string;
  status: SessionStatus;
  csrfSecret: string;
  createdAt: number;
  lastUsedAt: number;
  expiresAt: number;
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

  create(operatorActorId: string): OperatorSession {
    if (operatorActorId.length === 0) {
      throw new Error("operator actor id must not be empty");
    }
    const now = this.#now();
    let sessionId = this.#randomToken();
    while (this.#sessions.has(sessionId)) {
      sessionId = this.#randomToken();
    }
    const session: OperatorSession = {
      sessionId,
      operatorActorId,
      status: "active",
      csrfSecret: this.#randomToken(),
      createdAt: now,
      lastUsedAt: now,
      expiresAt: now + this.#sessionTtlMs,
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
    if (session.status === "active") session.status = "revoked";
    return true;
  }

  revokeAllForOperator(operatorActorId: string): number {
    let revoked = 0;
    for (const session of this.#sessions.values()) {
      if (session.operatorActorId === operatorActorId && session.status === "active") {
        session.status = "revoked";
        revoked += 1;
      }
    }
    return revoked;
  }

  get size(): number {
    return this.#sessions.size;
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
