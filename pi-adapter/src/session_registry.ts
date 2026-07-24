import type { PiSession } from "./pi_session.js";
import type { TranscriptEvent } from "./transcript_event.js";

export interface RuntimeSessionConfig {
  runtimeSessionId: string;
  deploymentScope: string;
  project?: string;
  cwd: string;
  name?: string;
}

export interface RuntimeSessionEntry extends RuntimeSessionConfig {
  session: PiSession;
}

export type TranscriptObserver = (
  entry: RuntimeSessionEntry,
  event: TranscriptEvent,
) => void;

export type ModelChangeObserver = (entry: RuntimeSessionEntry, model: string) => void;

interface OwnedRuntimeSessionEntry extends RuntimeSessionEntry {
  unsubscribeTranscript: () => void;
  unsubscribeModelChange: () => void;
}

/** Complete runtime registry; future spawn adds entries through the same path. */
export class SessionRegistry {
  readonly #entries = new Map<string, OwnedRuntimeSessionEntry>();

  register(
    config: RuntimeSessionConfig,
    session: PiSession,
    observeTranscript: TranscriptObserver,
    observeModelChange: ModelChangeObserver,
  ): RuntimeSessionEntry {
    const runtimeSessionId = config.runtimeSessionId;
    if (!runtimeSessionId) throw new Error("runtimeSessionId must not be empty");
    if (session.runtimeSessionId !== runtimeSessionId) {
      throw new Error("registry key does not match PiSession runtime identity");
    }
    if (this.#entries.has(runtimeSessionId)) {
      throw new Error(`runtime session is already registered: ${runtimeSessionId}`);
    }
    const entry: OwnedRuntimeSessionEntry = {
      ...config,
      session,
      unsubscribeTranscript: () => undefined,
      unsubscribeModelChange: () => undefined,
    };
    entry.unsubscribeTranscript = session.onTranscript((event) =>
      observeTranscript(entry, event),
    );
    entry.unsubscribeModelChange = session.onModelChange((model) =>
      observeModelChange(entry, model),
    );
    this.#entries.set(runtimeSessionId, entry);
    return entry;
  }

  resolve(runtimeSessionId: string): RuntimeSessionEntry | undefined {
    return this.#entries.get(runtimeSessionId);
  }

  entries(): IterableIterator<[string, RuntimeSessionEntry]> {
    return this.#entries.entries();
  }

  async dispose(): Promise<void> {
    const disposals: Promise<void>[] = [];
    for (const entry of this.#entries.values()) {
      entry.unsubscribeTranscript();
      entry.unsubscribeModelChange();
      disposals.push(entry.session.dispose());
    }
    this.#entries.clear();
    await Promise.all(disposals);
  }
}
