import assert from "node:assert/strict";
import test from "node:test";
import {
  nextSessionReportSequence,
  type SessionReportSequence,
} from "../src/session_report_sequencer.js";

const MAX_UINT64 = (1n << 64n) - 1n;

test("session report sequence advances only within one producer and runtime epoch", () => {
  const first = nextSessionReportSequence(undefined, 4, 7);
  const second = nextSessionReportSequence(first, 4, 7);
  const runtimeReplacement = nextSessionReportSequence(second, 4, 8);
  const adapterReplacement = nextSessionReportSequence(runtimeReplacement, 5, 8);

  assert.deepEqual(first, { adapterGeneration: 4, sessionGeneration: 7, revision: 1n });
  assert.deepEqual(second, { adapterGeneration: 4, sessionGeneration: 7, revision: 2n });
  assert.deepEqual(runtimeReplacement, {
    adapterGeneration: 4,
    sessionGeneration: 8,
    revision: 1n,
  });
  assert.deepEqual(adapterReplacement, {
    adapterGeneration: 5,
    sessionGeneration: 8,
    revision: 1n,
  });
  assert.equal(Object.isFrozen(second), true);
});

test("session report sequence fails before uint64 overflow but replacement may reset", () => {
  const exhausted: SessionReportSequence = {
    adapterGeneration: 4,
    sessionGeneration: 7,
    revision: MAX_UINT64,
  };

  assert.throws(
    () => nextSessionReportSequence(exhausted, 4, 7),
    /revision exhausted uint64/,
  );
  assert.equal(nextSessionReportSequence(exhausted, 4, 8).revision, 1n);
  assert.equal(nextSessionReportSequence(exhausted, 5, 7).revision, 1n);
});
