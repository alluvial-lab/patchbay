import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  pickAnchor,
  restoredScrollTop,
  type AnchorEntry,
} from "../src/ui/scroll-anchor.js";

describe("pickAnchor", () => {
  const entries: AnchorEntry[] = [
    { key: "obs:a", top: -400, bottom: -100 }, // fully above viewport
    { key: "obs:b", top: -50, bottom: 30 }, // partially visible at top
    { key: "obs:c", top: 30, bottom: 200 },
    { key: "obs:d", top: 220, bottom: 400 },
  ];

  it("picks the first partially-visible entry with its viewport offset", () => {
    assert.deepEqual(pickAnchor(entries), { key: "obs:b", offset: -50 });
  });

  it("picks the first entry when all are below the viewport top", () => {
    assert.deepEqual(
      pickAnchor([{ key: "obs:c", top: 30, bottom: 200 }]),
      { key: "obs:c", offset: 30 },
    );
  });

  it("returns undefined with no entries", () => {
    assert.equal(pickAnchor([]), undefined);
  });
});

describe("restoredScrollTop", () => {
  it("adjusts scrollTop so the anchor keeps its viewport offset", () => {
    // Anchor was at viewport offset -50; after rebuild the same entry now
    // sits at relative top 120 (content above it changed height).
    assert.equal(restoredScrollTop(500, 120, -50, 2000), 670);
  });

  it("clamps to zero and to max scroll", () => {
    assert.equal(restoredScrollTop(10, -500, 0, 2000), 0);
    assert.equal(restoredScrollTop(1900, 500, 0, 2000), 2000);
  });
});
