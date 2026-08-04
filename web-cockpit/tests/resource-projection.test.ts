import assert from "node:assert/strict";
import test from "node:test";

import { create } from "@bufbuild/protobuf";
import {
  PayloadContentType,
  PayloadEnvelopeSchema,
  type PayloadEnvelope,
} from "@patchbay/contracts";

import {
  RESOURCE_PROJECTION_DECODERS,
  decodeResourceProjection,
  type ResourceIdentityView,
} from "../src/domain/resource-projection.js";

const encoder = new TextEncoder();
const poolIdentity: ResourceIdentityView = {
  adapterId: "token-commune",
  resourceKind: "provider_pool",
  resourceId: "shared-anthropic",
};
const usageIdentity: ResourceIdentityView = {
  adapterId: "direct-provider",
  resourceKind: "usage_window",
  resourceId: "anthropic-5h",
};

function envelope(schemaRef: string, value: unknown, contentType = PayloadContentType.JSON): PayloadEnvelope {
  return create(PayloadEnvelopeSchema, {
    schemaRef,
    contentType,
    payload: encoder.encode(typeof value === "string" ? value : JSON.stringify(value)),
  });
}

function decode(
  identity: ResourceIdentityView,
  resourceSchema: string,
  projectionSchema: string,
  value: unknown,
  resourceType = PayloadContentType.JSON,
  projectionType = PayloadContentType.JSON,
) {
  return decodeResourceProjection(
    identity,
    envelope(resourceSchema, {}, resourceType),
    envelope(projectionSchema, value, projectionType),
  );
}

test("the closed registry contains the two local v1 resource compositors", () => {
  assert.equal(RESOURCE_PROJECTION_DECODERS.length, 2);
  assert.deepEqual(RESOURCE_PROJECTION_DECODERS.map((decoder) => decoder.resourceKind), [
    "provider_pool",
    "usage_window",
  ]);
});

test("provider pools and usage windows decode to distinct bounded local variants", () => {
  const pool = decode(poolIdentity, "provider_pool.payload.v1", "provider_pool.projection.v1", {
    displayName: "Shared Anthropic",
    providerLabel: "Anthropic",
    health: "serving",
    remainingPercent: 73.5,
    resetLabel: "resets in 2h",
    contributionCount: 4,
    serviceLabel: "token-commune",
  });
  assert.equal(pool.status, "decoded");
  if (pool.status === "decoded") {
    assert.equal(pool.value.kind, "pooled-provider-pool");
    assert.equal(pool.value.controlPosture, "administration-capable");
  }

  const usage = decode(usageIdentity, "usage_window.payload.v1", "usage_window.projection.v1", {
    displayName: "Anthropic 5 hour",
    providerLabel: "Anthropic",
    health: "low",
    remainingPercent: 18,
    accountLabel: "personal",
    planLabel: "Max",
    windowLabel: "5 hour",
    burnRateLabel: "12% / hour",
    activeSessionCount: 2,
  });
  assert.equal(usage.status, "decoded");
  if (usage.status === "decoded") {
    assert.equal(usage.value.kind, "direct-provider-usage-window");
    assert.equal(usage.value.controlPosture, "read-only");
  }
});

test("kind and both complete payload descriptors must match before decoding bytes", () => {
  const cases = [
    decode({ ...poolIdentity, resourceKind: "other" }, "provider_pool.payload.v1", "provider_pool.projection.v1", "not-json"),
    decode(poolIdentity, "wrong.payload.v1", "provider_pool.projection.v1", "not-json"),
    decode(poolIdentity, "provider_pool.payload.v1", "wrong.projection.v1", "not-json"),
    decode(poolIdentity, "provider_pool.payload.v1", "provider_pool.projection.v1", "not-json", PayloadContentType.PROTOBUF),
    decode(poolIdentity, "provider_pool.payload.v1", "provider_pool.projection.v1", "not-json", PayloadContentType.JSON, PayloadContentType.TEXT_UTF8),
  ];
  for (const result of cases) assert.equal(result.status, "unsupported");
});

test("missing payload axes are unavailable and expose no bytes", () => {
  const projection = envelope("provider_pool.projection.v1", { displayName: "Pool" });
  assert.deepEqual(decodeResourceProjection(poolIdentity, undefined, projection), { status: "unavailable" });
  assert.deepEqual(decodeResourceProjection(poolIdentity, envelope("provider_pool.payload.v1", {}), undefined), { status: "unavailable" });
});

test("semantic failures stay local invalid results without retaining raw payload", () => {
  const validBase = { displayName: "Pool", providerLabel: "Provider", health: "serving" };
  const invalidValues = [
    "{",
    [],
    { ...validBase, displayName: "" },
    { ...validBase, displayName: "x".repeat(241) },
    { ...validBase, health: "offline" },
    { ...validBase, remainingPercent: -1 },
    { ...validBase, remainingPercent: 101 },
    { ...validBase, remainingPercent: "50" },
    { ...validBase, contributionCount: -1 },
    { ...validBase, contributionCount: 1.5 },
  ];
  for (const value of invalidValues) {
    const result = decode(poolIdentity, "provider_pool.payload.v1", "provider_pool.projection.v1", value);
    assert.deepEqual(result, {
      status: "invalid",
      projection: {
        schemaRef: "provider_pool.projection.v1",
        contentType: PayloadContentType.JSON,
      },
      reason: "projection_decode_failed",
    });
    assert.equal("payload" in result, false);
  }

  const nonFinite = decodeResourceProjection(
    poolIdentity,
    envelope("provider_pool.payload.v1", {}),
    envelope("provider_pool.projection.v1", '{"displayName":"Pool","providerLabel":"Provider","health":"serving","remainingPercent":1e400}'),
  );
  assert.equal(nonFinite.status, "invalid");
});
