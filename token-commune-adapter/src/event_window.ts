import type { GatewayEvent, GatewayEventsPage } from "./gateway_client.js";

export type EventGapReason =
  | "initial-baseline"
  | "window-discontinuity"
  | "window-saturated-without-anchor"
  | "history-became-empty";

export interface EventGapEvidence {
  readonly key: string;
  readonly reason: EventGapReason;
  readonly previousWindowSize: number;
  readonly visibleWindowSize: number;
  readonly overlapCount: number;
  readonly reconstruction: "visible-window-only";
  readonly continuity: "unknown-before-visible-window";
}

export interface EventWindowPlan {
  readonly gap?: EventGapEvidence;
  readonly baselineOnly: boolean;
  readonly events: readonly GatewayEvent[];
}

const MAX_PAGE_SIZE = 50;
const MAX_PENDING_IDS = MAX_PAGE_SIZE * 2;

export class LatestEventWindowTracker {
  #previousIds: Set<string> | undefined;
  readonly #acknowledgedIds = new Map<string, true>();
  readonly #consumedDeclaredOnlyIds = new Map<string, true>();
  readonly #acknowledgedGapKeys = new Set<string>();

  plan(page: GatewayEventsPage): EventWindowPlan {
    const ids = validatedIds(page);
    const visible = new Set(ids);
    if (this.#previousIds === undefined) {
      const gap = this.#gap("initial-baseline", new Set(), visible, 0);
      return {
        baselineOnly: true,
        ...(this.#acknowledgedGapKeys.has(gap.key) ? {} : { gap }),
        events: [],
      };
    }

    const overlapCount = [...visible].filter((id) => this.#previousIds!.has(id)).length;
    const reason = classifyGap(this.#previousIds.size, visible.size, overlapCount);
    const gap = reason === undefined ? undefined : this.#gap(reason, this.#previousIds, visible, overlapCount);
    const events = page.events
      .filter(({ id }) =>
        !this.#previousIds!.has(id)
        && !this.#acknowledgedIds.has(id)
        && !this.#consumedDeclaredOnlyIds.has(id))
      .slice()
      .sort(compareEvent);
    return {
      baselineOnly: false,
      ...(gap !== undefined && !this.#acknowledgedGapKeys.has(gap.key) ? { gap } : {}),
      events,
    };
  }

  acknowledgeGap(key: string): void {
    if (!key) throw new Error("gap acknowledgement key is required");
    this.#acknowledgedGapKeys.add(key);
    while (this.#acknowledgedGapKeys.size > 2) {
      const oldest = this.#acknowledgedGapKeys.values().next().value as string | undefined;
      if (oldest === undefined) break;
      this.#acknowledgedGapKeys.delete(oldest);
    }
  }

  acknowledgeEvent(eventId: string): void {
    remember(this.#acknowledgedIds, eventId);
  }

  consumeDeclaredOnly(eventId: string): void {
    remember(this.#consumedDeclaredOnlyIds, eventId);
  }

  commitWindow(page: GatewayEventsPage): void {
    const ids = validatedIds(page);
    const visible = new Set(ids);
    this.#previousIds = visible;
    retain(this.#acknowledgedIds, visible);
    retain(this.#consumedDeclaredOnlyIds, visible);
  }

  #gap(
    reason: EventGapReason,
    previous: ReadonlySet<string>,
    visible: ReadonlySet<string>,
    overlapCount: number,
  ): EventGapEvidence {
    return {
      key: JSON.stringify([reason, [...previous].sort(), [...visible].sort()]),
      reason,
      previousWindowSize: previous.size,
      visibleWindowSize: visible.size,
      overlapCount,
      reconstruction: "visible-window-only",
      continuity: "unknown-before-visible-window",
    };
  }
}

function classifyGap(
  previousWindowSize: number,
  visibleWindowSize: number,
  overlapCount: number,
): EventGapReason | undefined {
  if (previousWindowSize === 0) {
    if (visibleWindowSize === MAX_PAGE_SIZE) return "window-saturated-without-anchor";
    return undefined;
  }
  if (visibleWindowSize === 0) return "history-became-empty";
  if (overlapCount > 0) return undefined;
  return visibleWindowSize === MAX_PAGE_SIZE
    ? "window-saturated-without-anchor"
    : "window-discontinuity";
}

function validatedIds(page: GatewayEventsPage): string[] {
  if (page.historyMode !== "latest-50-no-cursor") throw new Error("unsupported event history mode");
  if (page.events.length > MAX_PAGE_SIZE) throw new Error("event page exceeds latest-50 boundary");
  const seen = new Set<string>();
  for (const event of page.events) {
    if (!event.id.trim() || event.id.length > 512) throw new Error("event id must be bounded");
    if (seen.has(event.id)) throw new Error("event page contains duplicate ids");
    seen.add(event.id);
  }
  return [...seen];
}

function compareEvent(left: GatewayEvent, right: GatewayEvent): number {
  return left.occurredAt.localeCompare(right.occurredAt) || left.id.localeCompare(right.id);
}

function remember(target: Map<string, true>, id: string): void {
  if (!id.trim()) throw new Error("event acknowledgement id is required");
  target.delete(id);
  target.set(id, true);
  while (target.size > MAX_PENDING_IDS) {
    const oldest = target.keys().next().value as string | undefined;
    if (oldest === undefined) break;
    target.delete(oldest);
  }
}

function retain(target: Map<string, true>, ids: ReadonlySet<string>): void {
  for (const id of target.keys()) if (!ids.has(id)) target.delete(id);
}
