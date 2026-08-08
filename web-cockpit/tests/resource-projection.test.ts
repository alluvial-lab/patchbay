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

test("the closed registry contains generic and manifest-bound local compositors", () => {
  assert.equal(RESOURCE_PROJECTION_DECODERS.length, 4);
  assert.deepEqual(RESOURCE_PROJECTION_DECODERS.map((decoder) => decoder.resourceKind), [
    "provider_pool",
    "usage_window",
    "token-commune.provider-pool",
    "token-commune.member-draw",
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

test("snapshot and live fold decoder path recognizes the exact token-commune manifest contract", () => {
  const identity: ResourceIdentityView = {
    adapterId: "token-commune",
    resourceKind: "token-commune.provider-pool",
    resourceId: "opaque",
  };
  const result = decode(
    identity,
    "patchbay.token_commune.provider_pool.payload.v1",
    "patchbay.token_commune.provider_pool.projection.v1",
    {
      provider: "openai-codex",
      contributionListing: {
        status: "reported",
        contributions: [{
          subKey: "local:anonymous-contribution:0123456789abcdef01234567:1",
          subKeySource: "synthesized-content-hash",
          subKeyStability: "snapshot-local",
          attribution: "unavailable",
          declaredShare: 1,
          health: { state: "fresh" },
          telemetryState: "readings",
          capacityReadings: [{
            window: "5h", usedFraction: 0.2, usedUnits: 20, limitUnits: 100,
            resetsAt: null, source: "headers", observedAt: "2026-08-07T10:00:00Z",
          }],
          fingerprint: { state: "ok", templateSource: "compiled", since: null, diffPresent: false },
        }],
      },
      credentialHealthCounts: { fresh: 1, exhausted: 0, authBroken: 0 },
      totalDeclaredShare: 1,
      statusTelemetry: { status: "not-reported", contributions: [] },
      modelCatalog: { status: "reported", models: [{
        id: "gpt-5.5", provider: "openai-codex", surface: "codex", upstreamModel: null,
        contextWindow: 200000, maxTokens: 8192, reasoning: true, available: true,
      }] },
      fingerprint: { status: "unknown", probe: null, reason: "not-probed" },
      capacityAggregation: "none",
    },
  );
  assert.equal(result.status, "decoded");
  if (result.status === "decoded") assert.equal(result.value.kind, "token-commune-provider-pool");
});
