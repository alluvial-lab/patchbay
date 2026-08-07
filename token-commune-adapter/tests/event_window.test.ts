import assert from "node:assert/strict";
import test from "node:test";
import { LatestEventWindowTracker } from "../src/event_window.js";
import type { GatewayEvent, GatewayEventsPage } from "../src/gateway_client.js";

function event(id: string, occurredAt = `2026-08-07T12:00:${id.padStart(2, "0")}.000Z`): GatewayEvent {
  return { id, occurredAt, kind: "member", provider: "zai", contributionId: null, message: id };
}
function page(ids: readonly string[]): GatewayEventsPage {
  return { historyMode: "latest-50-no-cursor", events: ids.map((id) => event(id)) };
}

test("fresh install is a non-replayed baseline with one acknowledgement-aware boundary", () => {
  const tracker = new LatestEventWindowTracker();
  const first = tracker.plan(page(["01", "02"]));
  assert.equal(first.baselineOnly, true);
  assert.deepEqual(first.events, []);
  assert.equal(first.gap?.reason, "initial-baseline");
  assert.deepEqual({
    previousWindowSize: first.gap?.previousWindowSize,
    visibleWindowSize: first.gap?.visibleWindowSize,
    overlapCount: first.gap?.overlapCount,
    reconstruction: first.gap?.reconstruction,
    continuity: first.gap?.continuity,
  }, {
    previousWindowSize: 0,
    visibleWindowSize: 2,
    overlapCount: 0,
    reconstruction: "visible-window-only",
    continuity: "unknown-before-visible-window",
  });
  tracker.acknowledgeGap(first.gap!.key);
  assert.equal(tracker.plan(page(["01", "02"])).gap, undefined, "accepted boundary is not duplicated before page commit");
  tracker.commitWindow(page(["01", "02"]));
  assert.deepEqual(tracker.plan(page(["01", "02"])).events, []);
});

test("overlap emits only newly visible ids in deterministic source-time/id order", () => {
  const tracker = new LatestEventWindowTracker();
  tracker.commitWindow(page(["01", "02"]));
  const input: GatewayEventsPage = {
    historyMode: "latest-50-no-cursor",
    events: [event("04", "2026-08-07T12:00:04.000Z"), event("02"), event("03", "2026-08-07T12:00:04.000Z")],
  };
  const plan = tracker.plan(input);
  assert.equal(plan.gap, undefined);
  assert.deepEqual(plan.events.map(({ id }) => id), ["03", "04"]);
  tracker.acknowledgeEvent("03");
  assert.deepEqual(tracker.plan(input).events.map(({ id }) => id), ["04"], "only core-acknowledged ids dedup");
  tracker.commitWindow(input);
  assert.deepEqual(tracker.plan(input).events, []);
});

test("failed ids remain retryable while acknowledged ids do not repeat", () => {
  const tracker = new LatestEventWindowTracker();
  tracker.commitWindow(page(["01"]));
  const next = page(["01", "02", "03"]);
  assert.deepEqual(tracker.plan(next).events.map(({ id }) => id), ["02", "03"]);
  tracker.acknowledgeEvent("02");
  assert.deepEqual(tracker.plan(next).events.map(({ id }) => id), ["03"]);
  tracker.acknowledgeEvent("03");
  tracker.commitWindow(next);
  assert.deepEqual(tracker.plan(next).events, []);
});

test("empty/short/full and rollover transitions classify only observable gap evidence", () => {
  const empty = new LatestEventWindowTracker();
  empty.commitWindow(page([]));
  assert.equal(empty.plan(page(["01"])).gap, undefined, "empty to short remains observable within the window");
  const fifty = Array.from({ length: 50 }, (_, index) => String(index + 1).padStart(2, "0"));
  assert.equal(empty.plan(page(fifty)).gap?.reason, "window-saturated-without-anchor");

  const rollover = new LatestEventWindowTracker();
  rollover.commitWindow(page(["01", "02"]));
  const shortGap = rollover.plan(page(["10", "11"]));
  assert.equal(shortGap.gap?.reason, "window-discontinuity");
  assert.equal(shortGap.gap?.overlapCount, 0);
  assert.deepEqual(shortGap.events.map(({ id }) => id), ["10", "11"]);
  assert.equal(rollover.plan(page(fifty.map((id) => `x${id}`))).gap?.reason, "window-saturated-without-anchor");
  assert.equal(rollover.plan(page([])).gap?.reason, "history-became-empty");
});

test("declared-only ids use a separate bounded consumption path", () => {
  const tracker = new LatestEventWindowTracker();
  tracker.commitWindow(page(["01"]));
  const declared: GatewayEventsPage = {
    historyMode: "latest-50-no-cursor",
    events: [event("01"), { ...event("02"), kind: "calibration" }],
  };
  assert.deepEqual(tracker.plan(declared).events.map(({ id }) => id), ["02"]);
  tracker.consumeDeclaredOnly("02");
  assert.deepEqual(tracker.plan(declared).events, []);
  tracker.commitWindow(declared);
  assert.deepEqual(tracker.plan(declared).events, []);
});

test("duplicate ids, oversized pages, and invented history modes fail closed", () => {
  const tracker = new LatestEventWindowTracker();
  assert.throws(() => tracker.plan({ historyMode: "latest-50-no-cursor", events: [event("01"), event("01")] }), /duplicate/);
  assert.throws(() => tracker.plan(page(Array.from({ length: 51 }, (_, index) => `x${index}`))), /latest-50/);
  assert.throws(() => tracker.plan({ historyMode: "cursor" as any, events: [] }), /history mode/);
});
