import type { TranscriptEvent } from "./transcript_event.js";

/** Process-local partial-snapshot log. Pi's persisted session remains durable. */
export class TranscriptEventLog {
  readonly #events: TranscriptEvent[] = [];
  readonly #seen = new Set<string>();

  append(event: TranscriptEvent): boolean {
    if (this.#seen.has(event.eventId)) return false;
    this.#seen.add(event.eventId);
    this.#events.push(event);
    return true;
  }

  appendAll(events: readonly TranscriptEvent[]): number {
    let appended = 0;
    for (const event of events) if (this.append(event)) appended += 1;
    return appended;
  }

  clear(): void {
    this.#events.length = 0;
    this.#seen.clear();
  }

  forSession(sessionId: string): readonly TranscriptEvent[] {
    return this.#events.filter((event) => event.sessionId === sessionId);
  }

  entries(): readonly TranscriptEvent[] {
    return [...this.#events];
  }
}
