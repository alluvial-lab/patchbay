import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { createRenderCoalescer } from "../src/ui/render-coalescer.js";

describe("render coalescer", () => {
  function harness() {
    const scheduled: Array<() => void> = [];
    const rendered: string[] = [];
    let current = "v0";
    const coalescer = createRenderCoalescer(
      (callback) => scheduled.push(callback),
      () => rendered.push(current),
    );
    return {
      scheduled,
      rendered,
      notify: (version: string) => {
        current = version;
        coalescer.notify();
      },
      flushFrame: () => {
        const frame = scheduled.shift();
        assert.ok(frame, "expected a scheduled frame");
        frame();
      },
      coalescer,
    };
  }

  it("schedules exactly one render for a synchronous burst", () => {
    const h = harness();
    for (let i = 1; i <= 50; i += 1) h.notify(`v${i}`);
    assert.equal(h.scheduled.length, 1);
    assert.equal(h.rendered.length, 0);
    h.flushFrame();
    assert.deepEqual(h.rendered, ["v50"]);
  });

  it("schedules one render per frame, always with the latest model", () => {
    const h = harness();
    h.notify("v1");
    h.notify("v2");
    h.flushFrame();
    h.notify("v3");
    assert.equal(h.scheduled.length, 1);
    h.flushFrame();
    assert.deepEqual(h.rendered, ["v2", "v3"]);
  });

  it("does not schedule when nothing arrived", () => {
    const h = harness();
    assert.equal(h.scheduled.length, 0);
    assert.equal(h.rendered.length, 0);
  });

  it("flush renders pending state synchronously; stale frames are no-ops", () => {
    const h = harness();
    h.notify("v1");
    h.coalescer.flush();
    assert.deepEqual(h.rendered, ["v1"]);
    // The frame scheduled before flush() must not render a second time.
    h.flushFrame();
    assert.deepEqual(h.rendered, ["v1"]);
    h.coalescer.flush();
    assert.deepEqual(h.rendered, ["v1"]);
  });
});
