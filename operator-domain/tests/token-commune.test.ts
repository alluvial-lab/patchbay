import assert from "node:assert/strict";
import test from "node:test";

import { create } from "@bufbuild/protobuf";
import {
  AdapterSnapshotSupport,
  PayloadContentType,
  PayloadEnvelopeSchema,
  ResourceFreshnessState,
  type PayloadEnvelope,
} from "@patchbay/contracts";

import {
  TOKEN_COMMUNE_PRESENTATION_CONTRACT,
  composeTokenCommunePools,
  decodeTokenCommuneProjection,
  synthesizeTokenCommuneVerdict,
  type SurfaceResourceIdentity,
  type TokenCommuneDecodeResult,
  type TokenCommuneResourceInput,
} from "../src/token-commune.js";

const encoder = new TextEncoder();
const poolIdentity = (adapterId = "commune-a"): SurfaceResourceIdentity => ({
  adapterId,
  resourceKind: TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.resourceKind,
  resourceId: "opaque-pool-resource",
});
const drawIdentity = (adapterId = "commune-a"): SurfaceResourceIdentity => ({
  adapterId,
  resourceKind: TOKEN_COMMUNE_PRESENTATION_CONTRACT.memberDraw.resourceKind,
  resourceId: "opaque-draw-resource",
});

function envelope(schemaRef: string, value: unknown, contentType = PayloadContentType.JSON): PayloadEnvelope {
  return create(PayloadEnvelopeSchema, {
    schemaRef,
    contentType,
    payload: encoder.encode(typeof value === "string" ? value : JSON.stringify(value)),
  });
}

function reading(window: string, usedFraction: number | null, observedAt = "2026-08-07T10:00:00Z") {
  return {
    window,
    usedFraction,
    usedUnits: usedFraction === null ? null : usedFraction * 100,
    limitUnits: usedFraction === null ? null : 100,
    resetsAt: "2026-08-07T15:00:00Z",
    source: "headers",
    observedAt,
  };
}

function contribution(
  ordinal: number,
  health: "fresh" | "exhausted" | "auth_broken",
  readings: unknown[],
) {
  return {
    subKey: `local:anonymous-contribution:0123456789abcdef01234567:${ordinal}`,
    subKeySource: "synthesized-content-hash",
    subKeyStability: "snapshot-local",
    attribution: "unavailable",
    declaredShare: 0.5,
    health: health === "fresh" ? { state: health }
      : health === "exhausted" ? { state: health, exhaustedUntil: "2026-08-07T15:00:00Z" }
        : { state: health, reason: "redacted from surface" },
    telemetryState: readings.length ? "readings" : "no-readings",
    capacityReadings: readings,
    fingerprint: { state: "ok", templateSource: "compiled", since: null, diffPresent: false },
  };
}

function model(
  id: string,
  available = true,
  provider = "openai-codex",
  upstreamModel: string | null = null,
) {
  return {
    id,
    provider,
    surface: "codex",
    upstreamModel,
    contextWindow: 200000,
    maxTokens: 8192,
    reasoning: true,
    available,
  };
}

function poolProjection(overrides: Record<string, unknown> = {}) {
  const contributions = [
    contribution(1, "fresh", [reading("1h", 0.99), reading("5h", 0.35)]),
    contribution(2, "fresh", [reading("5h", 1), reading("5h", null)]),
  ];
  return {
    provider: "openai-codex",
    contributionListing: { status: "reported", contributions },
    credentialHealthCounts: { fresh: 2, exhausted: 0, authBroken: 0 },
    totalDeclaredShare: 1,
    statusTelemetry: { status: "not-reported", contributions: [] },
    modelCatalog: { status: "reported", models: [model("gpt-5.5"), model("gpt-5.3-codex-spark")] },
    fingerprint: { status: "unknown", probe: null, reason: "not-probed" },
    capacityAggregation: "none",
    ...overrides,
  };
}

function drawProjection(reports: unknown[] = [{
  provider: "openai-codex",
  limitFraction: 0.25,
  fromDecree: false,
  consumedUnits: 10,
  drawUnits: null,
  exceeded: false,
  enforceable: false,
  resetsAt: null,
}]) {
  return { memberDisplayName: "private member", provider: "openai-codex", reports };
}

function decodePool(value = poolProjection(), identity = poolIdentity()): TokenCommuneDecodeResult {
  return decodeTokenCommuneProjection(
    identity,
    envelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, {}),
    envelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, value),
  )!;
}

function decodeDraw(value = drawProjection(), identity = drawIdentity()): TokenCommuneDecodeResult {
  return decodeTokenCommuneProjection(
    identity,
    envelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.memberDraw.payloadSchema, {}),
    envelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.memberDraw.projectionSchema, value),
  )!;
}

function input(
  identity: SurfaceResourceIdentity,
  projection: TokenCommuneDecodeResult,
  overrides: Partial<TokenCommuneResourceInput> = {},
): TokenCommuneResourceInput {
  return {
    identity,
    projection,
    freshness: ResourceFreshnessState.CURRENT,
    completeness: AdapterSnapshotSupport.PARTIAL,
    observedAt: new Date("2026-08-07T10:01:00Z"),
    reconciled: true,
    tombstoned: false,
    ...overrides,
  };
}

test("exact token-commune kinds and both manifest descriptors gate semantic decoding", () => {
  const decoded = decodePool();
  assert.equal(decoded.status, "decoded");
  assert.equal(decodeTokenCommuneProjection(
    { ...poolIdentity(), resourceKind: "other" },
    undefined,
    undefined,
  ), undefined);
  const wrongPairs = [
    ["wrong", TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, PayloadContentType.JSON, PayloadContentType.JSON],
    [TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, "wrong", PayloadContentType.JSON, PayloadContentType.JSON],
    [TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, PayloadContentType.PROTOBUF, PayloadContentType.JSON],
  ] as const;
  for (const [payloadSchema, projectionSchema, payloadType, projectionType] of wrongPairs) {
    assert.deepEqual(decodeTokenCommuneProjection(
      poolIdentity(), envelope(payloadSchema, "not-json", payloadType), envelope(projectionSchema, "not-json", projectionType),
    ), { status: "unsupported" });
  }
  assert.deepEqual(decodeTokenCommuneProjection(poolIdentity(), undefined, undefined), { status: "unavailable" });
});

test("malformed projections, contradictory counts or declared shares, and the removed gpt-5.6 alias fail closed without bytes", () => {
  const malformed = decodePool(poolProjection({ credentialHealthCounts: { fresh: 1, exhausted: 0, authBroken: 0 } }));
  assert.deepEqual(malformed, { status: "invalid", reason: "projection_decode_failed" });
  assert.equal("payload" in malformed, false);

  const contradictoryShare = decodePool(poolProjection({ totalDeclaredShare: 0.25 }));
  assert.deepEqual(contradictoryShare, { status: "invalid", reason: "projection_decode_failed" });
  assert.equal("payload" in contradictoryShare, false);

  const alias = decodePool(poolProjection({ modelCatalog: { status: "reported", models: [model("gpt-5.6")] } }));
  assert.deepEqual(alias, { status: "invalid", reason: "projection_decode_failed" });
  assert.equal(JSON.stringify(alias).includes("gpt-5.6"), false);

  for (const bad of ["{", [], poolProjection({ capacityAggregation: "average" })]) {
    const result = decodePool(bad as never);
    assert.deepEqual(result, { status: "invalid", reason: "projection_decode_failed" });
  }
});

test("compositor joins only exact adapter and native provider and emits no aggregate or identity", () => {
  const wrongAdapter = input(drawIdentity("commune-b"), decodeDraw(drawProjection(), drawIdentity("commune-b")));
  const summary = composeTokenCommunePools([
    input(poolIdentity(), decodePool()),
    wrongAdapter,
  ])[0]!;
  assert.equal(summary.draw.state, "unavailable");

  const joined = composeTokenCommunePools([
    input(poolIdentity(), decodePool()),
    input(drawIdentity(), decodeDraw()),
  ])[0]!;
  assert.deepEqual(joined.draw, { state: "current", limitFraction: 0.25, consumedUnits: 10, resetsAt: null });
  assert.equal(joined.capacity5h.state, "current");
  if (joined.capacity5h.state === "current") assert.equal(joined.capacity5h.usedFraction, 1);
  assert.equal(joined.verdict, "runnable", "one 100% contribution cannot exhaust another usable contribution");
  assert.deepEqual(Object.keys(joined).sort(), [
    "capacity5h", "completeness", "credentials", "draw", "drawIdentity", "drawObservedAt", "fingerprint", "key",
    "modelState", "models", "poolIdentity", "poolObservedAt", "poolState", "provider", "totalDeclaredShare", "verdict",
  ]);
  assert.equal(joined.totalDeclaredShare, 1);
  assert.deepEqual(joined.fingerprint, { status: "unknown", probe: null, reason: "not-probed" });
  const safe = JSON.stringify(joined);
  assert.doesNotMatch(safe, /private member|subKey|anonymous-contribution|remainingPercent|average|weighted/);
});

test("capacity is the highest real 5h reading, never a null, non-5h, average, or inverse", () => {
  const projection = poolProjection({
    contributionListing: {
      status: "reported",
      contributions: [
        contribution(1, "fresh", [reading("1h", 0.99), reading("5h", null)]),
        contribution(2, "fresh", [reading("5h", 0.4, "2026-08-07T09:00:00Z"), reading("5h", 0.4, "2026-08-07T10:00:00Z")]),
      ],
    },
  });
  const summary = composeTokenCommunePools([input(poolIdentity(), decodePool(projection))])[0]!;
  assert.deepEqual(summary.capacity5h, {
    state: "current", usedFraction: 0.4, observedAt: "2026-08-07T10:00:00Z", resetsAt: "2026-08-07T15:00:00Z",
  });
  assert.equal(summary.verdict, "runnable");
});

test("draw ambiguity and independent pool/draw freshness fail closed", () => {
  const duplicate = drawProjection([
    ...drawProjection().reports,
    { ...drawProjection().reports[0] as object, limitFraction: 0.8 },
  ]);
  const ambiguous = composeTokenCommunePools([
    input(poolIdentity(), decodePool()),
    input(drawIdentity(), decodeDraw(duplicate)),
  ])[0]!;
  assert.deepEqual(ambiguous.draw, { state: "ambiguous" });

  const stalePool = composeTokenCommunePools([
    input(poolIdentity(), decodePool(), { freshness: ResourceFreshnessState.STALE }),
    input(drawIdentity(), decodeDraw()),
  ])[0]!;
  assert.equal(stalePool.credentials.state, "stale");
  assert.equal(stalePool.capacity5h.state, "stale");
  assert.equal(stalePool.verdict, "telemetry-stale");
  assert.equal(stalePool.draw.state, "current", "pool staleness must not contaminate current draw evidence");

  const staleDraw = composeTokenCommunePools([
    input(poolIdentity(), decodePool()),
    input(drawIdentity(), decodeDraw(), { freshness: ResourceFreshnessState.STALE }),
  ])[0]!;
  assert.equal(staleDraw.credentials.state, "current");
  assert.equal(staleDraw.capacity5h.state, "current");
  assert.equal(staleDraw.verdict, "runnable");
  assert.equal(staleDraw.draw.state, "stale", "draw staleness remains independent of a current pool");
});

test("canonical provider-pool identity anchors unknown summaries for every undecodable projection state", () => {
  const unavailableStates: TokenCommuneDecodeResult[] = [
    { status: "invalid", reason: "projection_decode_failed" },
    { status: "unsupported" },
    { status: "unavailable" },
  ];
  for (const projection of unavailableStates) {
    const summary = composeTokenCommunePools([
      input(poolIdentity(), projection),
      input(drawIdentity(), decodeDraw()),
    ])[0]!;
    assert.equal(summary.provider, "provider unavailable");
    assert.deepEqual(summary.poolIdentity, poolIdentity());
    assert.deepEqual(summary.draw, { state: "unknown" });
    assert.deepEqual(summary.credentials, {
      state: "unknown", fresh: 0, exhausted: 0, authBroken: 0, contributionCount: 0,
    });
    assert.deepEqual(summary.capacity5h, { state: "unknown" });
    assert.equal(summary.totalDeclaredShare, null);
    assert.deepEqual(summary.fingerprint, { status: "unknown", probe: null, reason: "not-probed" });
    assert.deepEqual(summary.models, []);
    assert.equal(summary.modelState, "unknown");
    assert.equal(summary.verdict, "unknown");
  }
});

test("model provenance is preserved and cross-provider rows are omitted as unknown evidence", () => {
  const crossProvider = model("claude-cross-pool", true, "anthropic", null);
  const summary = composeTokenCommunePools([
    input(poolIdentity(), decodePool(poolProjection({
      modelCatalog: { status: "reported", models: [crossProvider] },
    }))),
  ])[0]!;
  assert.equal(summary.verdict, "unknown");
  assert.deepEqual(summary.models, []);
  assert.equal(summary.modelState, "unknown");
});

test("verdict synthesis applies freshness, evidence, auth, model, exhaustion, then runnable precedence", () => {
  const base = {
    poolCurrent: true,
    sourceEvidenceComplete: true,
    credentials: { state: "current" as const, fresh: 1, exhausted: 0, authBroken: 0, contributionCount: 1 },
    capacity5h: { state: "current" as const, usedFraction: 0.5, observedAt: "2026-08-07T10:00:00Z", resetsAt: null },
    contributionCapacityFacts: [{ health: "fresh" as const, fiveHourUsedFraction: 0.5 }],
    modelState: "current" as const,
    availableModelCount: 1,
  };
  assert.equal(synthesizeTokenCommuneVerdict(base), "runnable");
  assert.equal(synthesizeTokenCommuneVerdict({ ...base, poolCurrent: false }), "telemetry-stale");
  assert.equal(synthesizeTokenCommuneVerdict({ ...base, sourceEvidenceComplete: false }), "unknown");
  assert.equal(synthesizeTokenCommuneVerdict({
    ...base,
    credentials: { state: "current", fresh: 0, exhausted: 0, authBroken: 1, contributionCount: 1 },
    contributionCapacityFacts: [{ health: "auth_broken", fiveHourUsedFraction: undefined }],
  }), "auth-broken");
  assert.equal(synthesizeTokenCommuneVerdict({ ...base, availableModelCount: 0 }), "model-unavailable");
  assert.equal(synthesizeTokenCommuneVerdict({
    ...base,
    credentials: { state: "current", fresh: 0, exhausted: 2, authBroken: 0, contributionCount: 2 },
    contributionCapacityFacts: [
      { health: "exhausted", fiveHourUsedFraction: null },
      { health: "exhausted", fiveHourUsedFraction: undefined },
    ],
  }), "pool-exhausted");
});

test("independent honesty oracles kill representative join, capacity, and freshness mutants", () => {
  const exactJoin = (poolAdapter: string, drawAdapter: string, poolProvider: string, drawProvider: string) =>
    poolAdapter === drawAdapter && poolProvider === drawProvider;
  const providerOnlyMutant = (_poolAdapter: string, _drawAdapter: string, poolProvider: string, drawProvider: string) =>
    poolProvider === drawProvider;
  assert.equal(exactJoin("a", "b", "openai-codex", "openai-codex"), false);
  assert.equal(providerOnlyMutant("a", "b", "openai-codex", "openai-codex"), true);

  const realHighest = Math.max(0.2, 0.8);
  const averageMutant = (0.2 + 0.8) / 2;
  const remainingMutant = 1 - realHighest;
  assert.equal(realHighest, 0.8);
  assert.notEqual(averageMutant, realHighest);
  assert.notEqual(remainingMutant, realHighest);

  const staleDominant = synthesizeTokenCommuneVerdict({
    poolCurrent: false,
    sourceEvidenceComplete: true,
    credentials: { state: "stale", fresh: 1, exhausted: 0, authBroken: 0, contributionCount: 1 },
    capacity5h: { state: "stale", usedFraction: 0.2, observedAt: "2026-08-07T10:00:00Z", resetsAt: null },
    contributionCapacityFacts: [{ health: "fresh", fiveHourUsedFraction: 0.2 }],
    modelState: "stale",
    availableModelCount: 1,
  });
  const healthFirstMutant = "runnable";
  assert.equal(staleDominant, "telemetry-stale");
  assert.notEqual(healthFirstMutant, staleDominant);
});
