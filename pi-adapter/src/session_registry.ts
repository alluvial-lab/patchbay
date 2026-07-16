import type { PiSession } from "./pi_session.js";

/** Pre-provisioned runtime-session registry; future spawn only adds creation. */
export class SessionRegistry {
  readonly #sessions = new Map<string, PiSession>();

  register(runtimeSessionId: string, session: PiSession): void {
    if (!runtimeSessionId) throw new Error("runtimeSessionId must not be empty");
    if (session.runtimeSessionId !== runtimeSessionId) {
      throw new Error("registry key does not match PiSession runtime identity");
    }
    if (this.#sessions.has(runtimeSessionId)) {
      throw new Error(`runtime session is already registered: ${runtimeSessionId}`);
    }
    this.#sessions.set(runtimeSessionId, session);
  }

  resolve(runtimeSessionId: string): PiSession | undefined {
    return this.#sessions.get(runtimeSessionId);
  }

  entries(): IterableIterator<[string, PiSession]> {
    return this.#sessions.entries();
  }

  dispose(): void {
    for (const session of this.#sessions.values()) session.dispose();
    this.#sessions.clear();
  }
}
