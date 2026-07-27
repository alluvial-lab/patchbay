/**
 * Coalesces render scheduling: every domain event still folds immediately,
 * but at most one render is scheduled per frame, always rendering the latest
 * model. The scheduler is injected so the policy is testable without a
 * browser frame loop (production passes `requestAnimationFrame`).
 */
export interface RenderCoalescer {
  /** Records that new state is available; schedules a render if none is pending. */
  notify(): void;
  /** Renders synchronously when a render is pending; no-op otherwise. */
  flush(): void;
}

export function createRenderCoalescer(
  schedule: (callback: () => void) => void,
  render: () => void,
): RenderCoalescer {
  let pending = false;
  return {
    notify() {
      if (pending) return;
      pending = true;
      schedule(() => {
        if (!pending) return;
        pending = false;
        render();
      });
    },
    flush() {
      if (!pending) return;
      pending = false;
      render();
    },
  };
}
