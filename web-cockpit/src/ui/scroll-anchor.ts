/**
 * Scroll anchoring across full-DOM rebuilds. The cockpit re-renders by
 * replacing the whole shell, which resets the timeline's scrollTop to 0 —
 * yanking a user reading history to the top on every streamed event. The
 * anchor captures the first partially-visible entry before the rebuild and
 * restores its viewport offset after it.
 */

export interface AnchorEntry {
  /** Stable identity: `<attr>:<id>` from a data-attribute selector. */
  key: string;
  /** Viewport-relative top (entry top minus timeline viewport top). */
  top: number;
  bottom: number;
}

export interface ScrollAnchor {
  key: string;
  offset: number;
  /** Raw scrollTop before the rebuild; fallback when the entry vanished. */
  rawScrollTop: number;
}

/** First entry visible (even partially) in the viewport, with its offset. */
export function pickAnchor(entries: readonly AnchorEntry[]): { key: string; offset: number } | undefined {
  const visible = entries.find((entry) => entry.bottom > 0);
  return visible ? { key: visible.key, offset: visible.top } : undefined;
}

/**
 * ScrollTop that restores the anchor's viewport offset after the rebuild:
 * the entry's current viewport-relative top is driven back to its captured
 * offset. Clamped to the legal scroll range.
 */
export function restoredScrollTop(
  currentScrollTop: number,
  entryCurrentRelativeTop: number,
  anchorOffset: number,
  maxScrollTop: number,
): number {
  const next = currentScrollTop + entryCurrentRelativeTop - anchorOffset;
  return Math.min(Math.max(next, 0), Math.max(maxScrollTop, 0));
}

const ANCHOR_ATTRIBUTES = ["data-observation-id", "data-command-id", "data-diagnostic-id"] as const;

function entryKey(element: Element): string | undefined {
  for (const attribute of ANCHOR_ATTRIBUTES) {
    const value = element.getAttribute(attribute);
    if (value) return `${attribute}:${value}`;
  }
  return undefined;
}

function* anchorCandidates(timeline: HTMLElement): Generator<Element> {
  // One combined query so candidates come back in document order.
  const selector = ANCHOR_ATTRIBUTES.map((attribute) => `[${attribute}]`).join(",");
  yield* timeline.querySelectorAll(selector);
}

/** Capture the scroll anchor from the live timeline before a rebuild. */
export function captureAnchor(timeline: HTMLElement): ScrollAnchor | undefined {
  const viewportTop = timeline.getBoundingClientRect().top;
  const entries: AnchorEntry[] = [];
  for (const element of anchorCandidates(timeline)) {
    const key = entryKey(element);
    if (!key) continue;
    const rect = element.getBoundingClientRect();
    entries.push({ key, top: rect.top - viewportTop, bottom: rect.bottom - viewportTop });
  }
  const picked = pickAnchor(entries);
  if (!picked) return undefined;
  return { ...picked, rawScrollTop: timeline.scrollTop };
}

/** Restore the anchor in the rebuilt timeline; falls back to raw scrollTop. */
export function restoreAnchor(timeline: HTMLElement, anchor: ScrollAnchor): void {
  for (const element of anchorCandidates(timeline)) {
    if (entryKey(element) === anchor.key) {
      const relativeTop = element.getBoundingClientRect().top - timeline.getBoundingClientRect().top;
      timeline.scrollTop = restoredScrollTop(
        timeline.scrollTop,
        relativeTop,
        anchor.offset,
        timeline.scrollHeight - timeline.clientHeight,
      );
      return;
    }
  }
  // The anchor entry vanished (e.g., its message was replaced): keep the
  // previous raw position, clamped to the new content height.
  timeline.scrollTop = Math.min(anchor.rawScrollTop, Math.max(timeline.scrollHeight - timeline.clientHeight, 0));
}
