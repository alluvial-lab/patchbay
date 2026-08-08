import assert from "node:assert/strict";

export interface ReconnectReferenceStep {
  process: number;
  page_ids: readonly string[];
  acknowledged: readonly string[];
  commit_window: boolean;
  report_accepted: boolean;
}

export interface ReconnectReferenceOutcome {
  baselineOnly: boolean;
  emittedIds: readonly string[];
  gap: null | "initial-baseline" | "window-discontinuity" | "window-saturated-without-anchor" | "history-became-empty";
  reportPrecedesEvents: boolean;
  fabricatedMissedCount: null;
}

export interface PartialObservation {
  views: readonly {
    resourceKind: string;
    completeness: string;
    mutations: readonly { payload: unknown; projection: unknown }[];
  }[];
}

export interface DegradationObservation {
  emptyPartialViewCount: number;
  reusedPriorEndpointValue: boolean;
  pollingEstablishesLiveness: boolean;
  states: Readonly<Record<string, "current" | "stale" | "unknown">>;
}

export interface LifecycleFact {
  state: "accepted" | "delivered" | "failed" | "completed";
  failureCode?: "unsupported_command" | "adapter_unavailable";
  eventLsn: bigint;
}

export interface SecretScanTarget {
  name: string;
  bytes: Uint8Array;
}

export function reconnectReferenceModel(
  steps: readonly ReconnectReferenceStep[],
): readonly ReconnectReferenceOutcome[] {
  let process: number | undefined;
  let previous: Set<string> | undefined;
  const acknowledged = new Set<string>();
  const outcomes: ReconnectReferenceOutcome[] = [];
  for (const step of steps) {
    assert.equal(step.report_accepted, true, "event repair cannot precede an unaccepted resource report");
    assert.equal(new Set(step.page_ids).size, step.page_ids.length, "event ids must be unique");
    assert.ok(step.page_ids.length <= 50, "latest-50 boundary exceeded");
    if (process !== step.process) {
      process = step.process;
      previous = undefined;
      acknowledged.clear();
    }
    const visible = new Set(step.page_ids);
    let gap: ReconnectReferenceOutcome["gap"] = null;
    let baselineOnly = false;
    let emittedIds: string[] = [];
    if (previous === undefined) {
      baselineOnly = true;
      gap = "initial-baseline";
    } else {
      const overlap = [...visible].filter((id) => previous!.has(id)).length;
      if (previous.size === 0 && visible.size === 50) gap = "window-saturated-without-anchor";
      else if (previous.size > 0 && visible.size === 0) gap = "history-became-empty";
      else if (previous.size > 0 && visible.size > 0 && overlap === 0) {
        gap = visible.size === 50 ? "window-saturated-without-anchor" : "window-discontinuity";
      }
      emittedIds = step.page_ids.filter((id) => !previous!.has(id) && !acknowledged.has(id)).sort();
    }
    for (const id of step.acknowledged) acknowledged.add(id);
    if (step.commit_window) previous = visible;
    outcomes.push({ baselineOnly, emittedIds, gap, reportPrecedesEvents: true, fabricatedMissedCount: null });
  }
  return outcomes;
}

export function assertReconnectOracle(
  input: { steps: readonly ReconnectReferenceStep[] },
  observed: readonly (ReconnectReferenceOutcome & { missedCount?: number; order: readonly string[] })[],
): void {
  const expected = reconnectReferenceModel(input.steps);
  assert.equal(observed.length, expected.length);
  for (let index = 0; index < expected.length; index += 1) {
    const actual = observed[index]!;
    assert.deepEqual({
      baselineOnly: actual.baselineOnly,
      emittedIds: actual.emittedIds,
      gap: actual.gap,
      reportPrecedesEvents: actual.order[0] === "report",
      fabricatedMissedCount: actual.missedCount ?? null,
    }, expected[index]);
  }
}

export function assertPartialSnapshotOracle(
  input: Record<string, unknown>,
  observed: PartialObservation,
): void {
  const gateway = object(input.gateway, "gateway");
  const expectedProviderCount = expectedProviders(gateway).size;
  const me = object(gateway.me, "gateway.me");
  const expectedDrawCount = me.status === "reported"
    ? new Set(array(object(me.value, "gateway.me.value").reports, "gateway.me.value.reports")
      .map((report) => text(object(report, "draw report").provider, "draw provider"))).size
    : 0;
  assert.deepEqual(observed.views.map((view) => view.resourceKind), [
    "token-commune.provider-pool",
    "token-commune.member-draw",
  ]);
  assert.ok(observed.views.every((view) => view.completeness === "partial"));
  assert.deepEqual(observed.views.map((view) => view.mutations.length), [expectedProviderCount, expectedDrawCount]);
  for (const view of observed.views) {
    for (const mutation of view.mutations) {
      assertNoForbiddenAggregate(mutation.payload);
      assertNoForbiddenAggregate(mutation.projection);
    }
  }
  if (object(gateway.fingerprints, "gateway.fingerprints").status === "unavailable") {
    const poolRows = observed.views[0]?.mutations ?? [];
    for (const row of poolRows) {
      const projection = object(row.projection, "provider projection");
      const fingerprint = object(projection.fingerprint, "provider projection fingerprint");
      assert.notEqual(fingerprint.status, "reported", "unavailable fingerprint source cannot be reused");
    }
  }
}

export function assertDegradationOracle(
  input: { steps: readonly { kind: string; listed: readonly string[]; cached: readonly string[]; no_payload: readonly string[] }[] },
  observed: DegradationObservation,
): void {
  assert.equal(observed.emptyPartialViewCount, 2, "failed poll must emit both empty PARTIAL views");
  assert.equal(observed.reusedPriorEndpointValue, false);
  assert.equal(observed.pollingEstablishesLiveness, false);
  const last = input.steps.at(-1);
  assert.ok(last, "degradation trace requires a final reconnect step");
  const expected: Record<string, "current" | "stale" | "unknown"> = {};
  for (const id of new Set(input.steps.flatMap((step) => [...step.cached, ...step.no_payload, ...step.listed]))) {
    expected[id] = last.listed.includes(id) ? "current"
      : last.no_payload.includes(id) ? "unknown"
        : "stale";
  }
  assert.deepEqual(observed.states, expected);
}

export function expectedCurrentGenerationAcceptance(input: {
  authenticatedAdapterId: string;
  currentGeneration: bigint;
  requestAdapterId: string;
  requestGeneration: bigint;
  ownsTarget: boolean;
  tokenEpochCurrent: boolean;
}): boolean {
  return input.authenticatedAdapterId === input.requestAdapterId
    && input.currentGeneration === input.requestGeneration
    && input.ownsTarget
    && input.tokenEpochCurrent;
}

export function assertUnsupportedTerminalization(facts: readonly LifecycleFact[]): void {
  const ordered = [...facts].sort((left, right) => left.eventLsn < right.eventLsn ? -1 : left.eventLsn > right.eventLsn ? 1 : 0);
  const delivered = ordered.filter((fact) => fact.state === "delivered");
  const failed = ordered.filter((fact) => fact.state === "failed");
  assert.equal(delivered.length, 1, "delivery acknowledgement must be durable exactly once");
  assert.equal(failed.length, 1, "unsupported terminalization must be durable exactly once");
  assert.equal(failed[0]?.failureCode, "unsupported_command");
  assert.equal(ordered.some((fact) => fact.state === "completed"), false);
  assert.ok(delivered[0]!.eventLsn < failed[0]!.eventLsn, "delivered must precede terminal failure");
  assert.equal(ordered.at(-1)?.state, "failed", "recovery must leave no nonterminal command");
}

export function assertSecretAbsent(originalSecret: string, targets: readonly SecretScanTarget[]): void {
  assert.ok(originalSecret.length >= 24, "secret sentinel must be high entropy and long enough to avoid accidental matches");
  const forms = secretForms(originalSecret);
  for (const target of targets) {
    const raw = Buffer.from(target.bytes);
    const utf8 = raw.toString("utf8");
    const hex = raw.toString("hex");
    for (const form of forms) {
      assert.equal(raw.includes(Buffer.from(form)), false, `${target.name} contains a credential form`);
      assert.equal(utf8.includes(form), false, `${target.name} contains a credential form`);
      assert.equal(hex.includes(Buffer.from(form).toString("hex")), false, `${target.name} contains a hex-encoded credential form`);
    }
  }
}

function secretForms(secret: string): readonly string[] {
  return [...new Set([
    secret,
    `Bearer ${secret}`,
    encodeURIComponent(secret),
    Buffer.from(secret).toString("base64"),
    JSON.stringify(secret),
  ])];
}

function expectedProviders(gateway: Record<string, unknown>): Set<string> {
  const result = new Set<string>();
  for (const [endpoint, rowsKey] of [["pool", "contributions"], ["status", "contributions"], ["models", "models"]] as const) {
    const source = object(gateway[endpoint], `gateway.${endpoint}`);
    if (source.status !== "reported") continue;
    for (const row of array(object(source.value, `gateway.${endpoint}.value`)[rowsKey], `${endpoint}.${rowsKey}`)) {
      result.add(text(object(row, `${endpoint} row`).provider, `${endpoint} provider`));
    }
  }
  if (object(gateway.status, "gateway.status").status === "reported") result.add("anthropic");
  return result;
}

function assertNoForbiddenAggregate(value: unknown): void {
  const forbidden = new Set(["usedFraction", "remainingPercent", "remainingPercentage", "averageCapacity", "weightedCapacity", "missedCount"]);
  const visit = (candidate: unknown): void => {
    if (Array.isArray(candidate)) { for (const item of candidate) visit(item); return; }
    if (!candidate || typeof candidate !== "object") return;
    for (const [key, nested] of Object.entries(candidate as Record<string, unknown>)) {
      assert.equal(forbidden.has(key), false, `forbidden aggregate/fabricated field ${key}`);
      visit(nested);
    }
  };
  visit(value);
}

function object(value: unknown, name: string): Record<string, unknown> {
  assert.ok(value && typeof value === "object" && !Array.isArray(value), `${name} must be an object`);
  return value as Record<string, unknown>;
}
function array(value: unknown, name: string): unknown[] {
  assert.ok(Array.isArray(value), `${name} must be an array`);
  return value;
}
function text(value: unknown, name: string): string {
  assert.equal(typeof value, "string", `${name} must be a string`);
  return value as string;
}
