import assert from "node:assert/strict";
import test from "node:test";
import { timestampDate, timestampFromDate } from "@bufbuild/protobuf/wkt";
import { FailureCode, ObservationKind, PayloadContentType, TargetScopeKind, type Observation } from "@patchbay/contracts";
import { createCompositeLocalIdentitySynthesizer } from "../src/identity.js";
import {
  mapEventGap,
  mapPoolEvent,
  parseGatewayEventKind,
  TOKEN_COMMUNE_OBSERVATION_SCHEMAS,
} from "../src/event_observation.js";
import type { GatewayEvent } from "../src/gateway_client.js";

const identities = createCompositeLocalIdentitySynthesizer({
  adapterId: "token-commune",
  gatewayBaseUrl: new URL("https://gateway.example/"),
});
const base = {
  authorityDomainId: "default",
  adapterId: "token-commune",
  identities,
};
const occurredAt = "2026-08-07T12:00:01.000Z";

function event(kind: GatewayEvent["kind"]): GatewayEvent {
  return {
    id: `event-${kind}`,
    occurredAt,
    kind,
    provider: "openai-codex",
    contributionId: null,
    message: `${kind} observed`,
  };
}

function payload(observation: Observation): any {
  assert.ok(observation.payload);
  return JSON.parse(new TextDecoder().decode(observation.payload.payload));
}

test("the five independent production fixtures map to resource-scoped STATUS observations", () => {
  for (const kind of ["capacity_shift", "auth_broken", "windfall", "fingerprint", "member"] as const) {
    const result = mapPoolEvent({ ...base, event: event(kind) });
    assert.equal(result.status, "mapped");
    if (result.status !== "mapped") assert.fail("expected mapped event");
    const observation = result.observation;
    assert.equal(observation.kind, ObservationKind.STATUS);
    assert.equal(observation.failureCode, FailureCode.UNSPECIFIED);
    assert.equal(observation.authorityDomainId?.value, "default");
    assert.equal(observation.sender?.actorId?.value, "token-commune");
    assert.equal(observation.targetScope?.kind, TargetScopeKind.RESOURCE);
    assert.deepEqual([
      observation.targetScope?.resource?.adapterId?.value,
      observation.targetScope?.resource?.resourceKind?.value,
      observation.targetScope?.resource?.resourceId?.value,
    ], [
      "token-commune",
      "token-commune.provider-pool",
      identities.providerPool("openai-codex").resourceId,
    ]);
    assert.equal(observation.payload?.contentType, PayloadContentType.JSON);
    assert.equal(observation.payload?.schemaRef, "patchbay.token_commune.pool_event.v1");
    assert.equal(timestampDate(observation.observedAt!).toISOString(), occurredAt);
    assert.deepEqual(payload(observation), {
      sourceEventId: `event-${kind}`,
      kind,
      provider: "openai-codex",
      contributionId: null,
      message: `${kind} observed`,
      occurredAt,
      deliveryModel: "polling",
      historyMode: "latest-50-no-cursor",
    });
  }
});

test("declared-only kinds remain decodable but never become Observations", () => {
  for (const kind of ["window_exhausted", "calibration"] as const) {
    assert.deepEqual(mapPoolEvent({ ...base, event: event(kind) }), {
      status: "declared-but-unemitted",
      kind,
    });
  }
  assert.deepEqual(TOKEN_COMMUNE_OBSERVATION_SCHEMAS, {
    poolEvent: "patchbay.token_commune.pool_event.v1",
    eventGap: "patchbay.token_commune.event_gap.v1",
  });
});

test("unknown kinds and malformed event fields fail closed before ingress", () => {
  assert.throws(() => parseGatewayEventKind("future_kind"), /unknown/);
  assert.throws(() => mapPoolEvent({ ...base, event: { ...event("member"), kind: "future_kind" } as any }), /unknown/);
  assert.throws(() => mapPoolEvent({ ...base, event: { ...event("member"), occurredAt: "poll-time-ish" } }), /timestamp/);
  assert.throws(() => mapPoolEvent({ ...base, event: { ...event("member"), message: "" } }), /message/);
  assert.throws(() => mapPoolEvent({
    ...base,
    identities: { ...identities, providerPool: () => ({ ...identities.providerPool("x"), adapterId: "other" }) },
    event: event("member"),
  }), /provider pool/);
});

test("gap status carries measured window evidence and cannot claim a missed count or continuity", () => {
  const detectedAt = timestampFromDate(new Date("2026-08-07T12:05:00.000Z"));
  const observations = mapEventGap({
    authorityDomainId: "default",
    adapterId: "token-commune",
    targets: [identities.providerPool("zai"), identities.providerPool("anthropic")],
    detectedAt,
    gap: {
      key: "internal-only",
      reason: "window-discontinuity",
      previousWindowSize: 4,
      visibleWindowSize: 3,
      overlapCount: 0,
      reconstruction: "visible-window-only",
      continuity: "unknown-before-visible-window",
    },
  });
  assert.equal(observations.length, 2);
  assert.deepEqual(observations.map((item) => item.targetScope?.resource?.resourceId?.value), [
    identities.providerPool("anthropic").resourceId,
    identities.providerPool("zai").resourceId,
  ].sort());
  for (const observation of observations) {
    assert.equal(observation.kind, ObservationKind.STATUS);
    assert.equal(observation.payload?.schemaRef, "patchbay.token_commune.event_gap.v1");
    assert.equal(timestampDate(observation.observedAt!).toISOString(), "2026-08-07T12:05:00.000Z");
    const body = payload(observation);
    assert.deepEqual(body, {
      reason: "window-discontinuity",
      previousWindowSize: 4,
      visibleWindowSize: 3,
      overlapCount: 0,
      detectedAt: "2026-08-07T12:05:00.000Z",
      deliveryModel: "polling",
      historyMode: "latest-50-no-cursor",
      reconstruction: "visible-window-only",
      continuity: "unknown-before-visible-window",
    });
    assert.equal(Object.hasOwn(body, "missedCount"), false);
    assert.equal(Object.hasOwn(body, "continuous"), false);
    assert.equal(JSON.stringify(body).includes("authoritative"), false);
  }
});
