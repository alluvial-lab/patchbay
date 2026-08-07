import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { AdapterSnapshotSupport, AdapterTargetCategory, IdempotencyStrength, PayloadContentType } from "@patchbay/contracts";
import { Ajv2020 } from "ajv/dist/2020.js";
import ajvFormatsModule from "ajv-formats";
import { loadTokenCommuneAdapterConfig } from "../src/config.js";
import { createCompositeLocalIdentitySynthesizer } from "../src/identity.js";
import { tokenCommuneCapabilityManifest } from "../src/manifest.js";
import { TOKEN_COMMUNE_RESOURCES } from "../src/resource_contract.js";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

const schemaDescriptors = [
  { kind: "token-commune.provider-pool", descriptor: "payloadSchema", schemaRef: "patchbay.token_commune.provider_pool.payload.v1", file: "provider-pool-payload.schema.json" },
  { kind: "token-commune.provider-pool", descriptor: "projectionSchema", schemaRef: "patchbay.token_commune.provider_pool.projection.v1", file: "provider-pool-projection.schema.json" },
  { kind: "token-commune.member-draw", descriptor: "payloadSchema", schemaRef: "patchbay.token_commune.member_draw.payload.v1", file: "member-draw-payload.schema.json" },
  { kind: "token-commune.member-draw", descriptor: "projectionSchema", schemaRef: "patchbay.token_commune.member_draw.projection.v1", file: "member-draw-projection.schema.json" },
] as const;

function loadSchema(file: string): any {
  return JSON.parse(readFileSync(join(root, "schemas", file), "utf8"));
}

const capacityFixture = {
  window: "5h", usedFraction: null, usedUnits: null, limitUnits: 100, resetsAt: null,
  source: "usage_endpoint", observedAt: "2026-08-06T00:00:00.000Z",
};
const fingerprintStateFixture = {
  templateSource: "compiled", capturedAt: null, capturePresent: false,
  holdReason: null, heldAt: null, diffPresent: false,
};
const contributionFixture = {
  subKey: "local:anonymous-contribution:0123456789abcdef01234567:1",
  subKeySource: "synthesized-content-hash", subKeyStability: "snapshot-local", attribution: "unavailable",
  declaredShare: 0.5, health: { state: "exhausted", exhaustedUntil: "2026-08-06T01:00:00.000Z" },
  telemetryState: "readings", capacityReadings: [capacityFixture],
  fingerprint: { state: "unknown", templateSource: "compiled", since: null, diffPresent: false },
};
const contributionListing = { status: "reported", contributions: [contributionFixture] };
const statusTelemetry = {
  status: "reported", gatewayOk: true, anthropicHealth: { state: "auth_broken", reason: "expired credential" },
  joinability: "unjoinable-with-pool-rows",
  contributions: [{ contributionId: "contribution-1", provider: "anthropic", readings: [capacityFixture] }],
};
const modelFixture = {
  id: "claude-sonnet-4-5", provider: "anthropic", surface: "messages", upstreamModel: null,
  contextWindow: 200_000, maxTokens: 8_192, reasoning: true, available: true,
};
const modelCatalog = { status: "reported", models: [modelFixture] };
const providerFingerprint = { status: "reported", probe: "anthropic", value: fingerprintStateFixture };
const drawReportFixture = {
  provider: "anthropic", limitFraction: 0.5, fromDecree: false, consumedUnits: 5,
  drawUnits: null, exceeded: false, enforceable: true, resetsAt: null,
};
const schemaFixtures: Record<string, unknown> = {
  "provider-pool-payload.schema.json": {
    identityStrategy: "composite-local", gatewayDeploymentKey: "deployment", provider: "anthropic",
    contributionListing, statusTelemetry, modelCatalog, fingerprint: providerFingerprint,
    limitations: {
      snapshotCompleteness: "partial", contributorAttribution: "unavailable",
      contributionIdentity: "snapshot-local-synthesized", statusPoolJoin: "unavailable", capacityAggregation: "none",
    },
  },
  "provider-pool-projection.schema.json": {
    provider: "anthropic", contributionListing,
    credentialHealthCounts: { fresh: 0, exhausted: 1, authBroken: 0 }, totalDeclaredShare: 0.5,
    statusTelemetry, modelCatalog, fingerprint: providerFingerprint, capacityAggregation: "none",
  },
  "member-draw-payload.schema.json": {
    identityStrategy: "composite-local", gatewayDeploymentKey: "deployment", memberDisplayName: "Ada",
    provider: "anthropic", reports: [drawReportFixture],
    limitations: { snapshotCompleteness: "partial", stableMemberIdentity: "unavailable" },
  },
  "member-draw-projection.schema.json": {
    memberDisplayName: "Ada", provider: "anthropic", reports: [drawReportFixture],
  },
};

const requiredMutationPaths: Record<string, readonly string[]> = {
  "provider-pool-payload.schema.json": [
    "identityStrategy", "gatewayDeploymentKey", "provider", "contributionListing", "statusTelemetry", "modelCatalog", "fingerprint", "limitations",
    "limitations.snapshotCompleteness", "limitations.contributorAttribution", "limitations.contributionIdentity", "limitations.statusPoolJoin", "limitations.capacityAggregation",
    "contributionListing.status", "contributionListing.contributions", "contributionListing.contributions.0.subKey",
    "contributionListing.contributions.0.subKeySource", "contributionListing.contributions.0.subKeyStability", "contributionListing.contributions.0.attribution",
    "contributionListing.contributions.0.declaredShare", "contributionListing.contributions.0.health",
    "contributionListing.contributions.0.health.exhaustedUntil", "contributionListing.contributions.0.telemetryState",
    "contributionListing.contributions.0.capacityReadings", "contributionListing.contributions.0.fingerprint",
    "contributionListing.contributions.0.capacityReadings.0.window", "contributionListing.contributions.0.capacityReadings.0.usedFraction",
    "contributionListing.contributions.0.capacityReadings.0.usedUnits", "contributionListing.contributions.0.capacityReadings.0.limitUnits",
    "contributionListing.contributions.0.capacityReadings.0.resetsAt", "contributionListing.contributions.0.capacityReadings.0.source",
    "contributionListing.contributions.0.capacityReadings.0.observedAt", "statusTelemetry.gatewayOk", "statusTelemetry.anthropicHealth",
    "statusTelemetry.anthropicHealth.reason", "statusTelemetry.joinability", "statusTelemetry.contributions",
    "modelCatalog.status", "modelCatalog.models", "modelCatalog.models.0.id", "modelCatalog.models.0.upstreamModel",
    "fingerprint.status", "fingerprint.probe", "fingerprint.value",
  ],
  "provider-pool-projection.schema.json": [
    "provider", "contributionListing", "credentialHealthCounts", "totalDeclaredShare", "statusTelemetry", "modelCatalog", "fingerprint", "capacityAggregation",
    "credentialHealthCounts.fresh", "credentialHealthCounts.exhausted", "credentialHealthCounts.authBroken",
  ],
  "member-draw-payload.schema.json": [
    "identityStrategy", "gatewayDeploymentKey", "memberDisplayName", "provider", "reports", "limitations",
    "limitations.snapshotCompleteness", "limitations.stableMemberIdentity",
    "reports.0.provider", "reports.0.limitFraction", "reports.0.fromDecree", "reports.0.consumedUnits",
    "reports.0.drawUnits", "reports.0.exceeded", "reports.0.enforceable", "reports.0.resetsAt",
  ],
  "member-draw-projection.schema.json": ["memberDisplayName", "provider", "reports"],
};

test("manifest pins the two literal resource kinds and four literal schema descriptors", () => {
  assert.deepEqual(TOKEN_COMMUNE_RESOURCES, {
    providerPool: {
      kind: "token-commune.provider-pool",
      payloadSchema: "patchbay.token_commune.provider_pool.payload.v1",
      projectionSchema: "patchbay.token_commune.provider_pool.projection.v1",
    },
    memberDraw: {
      kind: "token-commune.member-draw",
      payloadSchema: "patchbay.token_commune.member_draw.payload.v1",
      projectionSchema: "patchbay.token_commune.member_draw.projection.v1",
    },
  });
  const manifest = tokenCommuneCapabilityManifest();
  assert.deepEqual(manifest.targetCategories, [AdapterTargetCategory.OPERATIONAL_RESOURCE]);
  assert.deepEqual(manifest.supportedOperationKinds, []);
  assert.equal(manifest.streamingSupport, false);
  assert.equal(manifest.cancellationSupport, false);
  assert.equal(manifest.sessionReplacementSupport, false);
  assert.equal(manifest.sessionSnapshotSupport, AdapterSnapshotSupport.UNSPECIFIED);
  assert.equal(manifest.idempotencyStrength, IdempotencyStrength.NONE);
  assert.equal(manifest.attachmentMethod?.kind, "configured-local-material");
  assert.equal(manifest.attachmentMethod?.descriptor.byteLength, 0);
  assert.equal(manifest.attachmentMethod?.descriptorContentType, PayloadContentType.BINARY);
  assert.deepEqual(manifest.resourceCapabilities.map((item) => item.resourceKind?.value), [
    "token-commune.provider-pool", "token-commune.member-draw",
  ]);
  for (const expected of schemaDescriptors) {
    const capability = manifest.resourceCapabilities.find((item) => item.resourceKind?.value === expected.kind);
    assert.ok(capability);
    assert.equal(capability.snapshotSupport, AdapterSnapshotSupport.PARTIAL);
    assert.equal(capability.projectionContract?.targetCategory, AdapterTargetCategory.OPERATIONAL_RESOURCE);
    const descriptor = capability.projectionContract?.[expected.descriptor];
    assert.equal(descriptor?.schemaRef, expected.schemaRef);
    assert.equal(descriptor?.contentType, PayloadContentType.JSON);
    assert.equal(loadSchema(expected.file).$id, expected.schemaRef);
  }
});

test("Draft 2020-12 resource schemas compile and reject every independently listed required-field deletion", () => {
  const ajv = new Ajv2020({ allErrors: true, strict: true });
  ajvFormatsModule.default(ajv);
  for (const { file } of schemaDescriptors) ajv.addSchema(loadSchema(file), file);
  for (const { file } of schemaDescriptors) {
    const schema = loadSchema(file);
    assert.equal(schema.$schema, "https://json-schema.org/draft/2020-12/schema");
    assert.equal(schema.additionalProperties, false);
    assert.equal(/password|prompt|response|diagnostic|accessToken|apiKey/i.test(JSON.stringify(schema)), false);
    const validate = ajv.getSchema(schema.$id);
    assert.ok(validate, `${file} must compile`);
    const fixture = schemaFixtures[file];
    assert.equal(validate(fixture), true, `${file} fixture must be valid: ${JSON.stringify(validate.errors)}`);
    for (const path of requiredMutationPaths[file] ?? []) {
      const mutated = structuredClone(fixture);
      deleteAtPath(mutated, path);
      assert.equal(validate(mutated), false, `${file} must reject missing ${path}`);
    }
  }
  const pool = loadSchema("provider-pool-payload.schema.json");
  const validatePool = ajv.getSchema(pool.$id);
  assert.ok(validatePool);
  for (const [telemetryState, capacityReadings] of [
    ["readings", []],
    ["no-readings", [capacityFixture]],
  ] as const) {
    const contradictory = structuredClone(schemaFixtures["provider-pool-payload.schema.json"]) as any;
    contradictory.contributionListing.contributions[0].telemetryState = telemetryState;
    contradictory.contributionListing.contributions[0].capacityReadings = capacityReadings;
    assert.equal(validatePool(contradictory), false, `${telemetryState} must agree with capacityReadings cardinality`);
  }
  const invalidDate = structuredClone(schemaFixtures["provider-pool-payload.schema.json"]) as any;
  invalidDate.contributionListing.contributions[0].capacityReadings[0].observedAt = "not-a-date";
  assert.equal(validatePool(invalidDate), false, "date-time fields require RFC 3339 values");
  assert.deepEqual(pool.$defs.capacity.properties.usedFraction.type, ["number", "null"]);
  assert.deepEqual(pool.$defs.capacity.properties.usedUnits.type, ["number", "null"]);
  assert.deepEqual(pool.$defs.capacity.properties.limitUnits.type, ["number", "null"]);
  assert.deepEqual(pool.$defs.capacity.properties.resetsAt.type, ["string", "null"]);
  assert.deepEqual(pool.$defs.health.oneOf.map((variant: any) => variant.properties.state.const), ["fresh", "exhausted", "auth_broken"]);
  assert.deepEqual(pool.$defs.contributionListing.oneOf.map((variant: any) => variant.properties.status.const), ["reported", "not-reported", "unavailable"]);
  for (const file of ["provider-pool-payload.schema.json", "provider-pool-projection.schema.json"]) {
    const rootProperties = Object.keys(loadSchema(file).properties);
    assert.equal(rootProperties.some((key) => /usedFraction|remaining|selectedWindow|percentage|percent/i.test(key)), false);
  }
  const draw = loadSchema("member-draw-payload.schema.json");
  assert.deepEqual(draw.$defs.drawReport.properties.drawUnits.type, ["number", "null"]);
});

function deleteAtPath(value: unknown, path: string): void {
  const parts = path.split(".");
  const key = parts.pop();
  assert.ok(key);
  let target = value as Record<string, unknown>;
  for (const part of parts) target = target[part] as Record<string, unknown>;
  assert.equal(Object.hasOwn(target, key), true, `fixture is missing mutation target ${path}`);
  delete target[key];
}

test("composite-local identity is canonical, deterministic, and collision-fenced", () => {
  const left = createCompositeLocalIdentitySynthesizer({ adapterId: "tc-a", gatewayBaseUrl: new URL("HTTPS://EXAMPLE.com:443/base") });
  const same = createCompositeLocalIdentitySynthesizer({ adapterId: "tc-a", gatewayBaseUrl: new URL("https://example.com/base/") });
  const otherGateway = createCompositeLocalIdentitySynthesizer({ adapterId: "tc-a", gatewayBaseUrl: new URL("https://other.example/base/") });
  assert.deepEqual(left.providerPool("anthropic"), same.providerPool("anthropic"));
  assert.match(left.providerPool("anthropic").resourceId, /^local:provider-pool:/);
  assert.match(left.memberDraw("Ada", "anthropic").resourceId, /^local:member-draw:/);
  assert.notEqual(left.providerPool("anthropic").resourceId, left.providerPool("codex").resourceId);
  assert.notEqual(left.memberDraw("Ada", "anthropic").resourceId, left.memberDraw("Grace", "anthropic").resourceId);
  assert.notEqual(left.providerPool("anthropic").resourceId, otherGateway.providerPool("anthropic").resourceId);
  assert.notDeepEqual(left.providerPool("anthropic"), { ...left.providerPool("anthropic"), adapterId: "tc-b" });
  assert.throws(() => createCompositeLocalIdentitySynthesizer({ adapterId: "tc", gatewayBaseUrl: new URL("https://user@example.com/") }));
});

test("configuration fails fast by environment key and never echoes values", () => {
  const valid = {
    PATCHBAY_CORE_ADDR: "http://127.0.0.1:9000",
    PATCHBAY_ADAPTER_ATTACHMENT_SECRET: "attachment-secret-value",
    PATCHBAY_TOKEN_COMMUNE_GATEWAY_URL: "https://gateway.example/",
    PATCHBAY_TOKEN_COMMUNE_MEMBER_KEY_FILE: "/secret/member.key",
  };
  const config = loadTokenCommuneAdapterConfig(valid);
  assert.equal(config.adapterId, "token-commune");
  assert.equal(config.adapterGeneration, 1);
  assert.equal(config.authorityDomainId, "default");
  assert.equal(config.pollIntervalMs, 30_000);
  for (const name of ["PATCHBAY_CORE_ADDR", "PATCHBAY_ADAPTER_ATTACHMENT_SECRET", "PATCHBAY_TOKEN_COMMUNE_GATEWAY_URL", "PATCHBAY_TOKEN_COMMUNE_MEMBER_KEY_FILE"]) {
    const env = { ...valid };
    delete (env as Record<string, string>)[name];
    assert.throws(() => loadTokenCommuneAdapterConfig(env), (error: unknown) => error instanceof Error && error.message.includes(name) && !error.message.includes("attachment-secret-value"));
  }
  assert.throws(() => loadTokenCommuneAdapterConfig({ ...valid, PATCHBAY_CORE_ADDR: "not-a-url" }), /PATCHBAY_CORE_ADDR/);
  assert.throws(() => loadTokenCommuneAdapterConfig({ ...valid, PATCHBAY_ADAPTER_GENERATION: "1.5" }), /PATCHBAY_ADAPTER_GENERATION/);
  assert.throws(() => loadTokenCommuneAdapterConfig({ ...valid, PATCHBAY_TOKEN_COMMUNE_POLL_INTERVAL_MS: "0" }), /PATCHBAY_TOKEN_COMMUNE_POLL_INTERVAL_MS/);
  assert.throws(() => loadTokenCommuneAdapterConfig({ ...valid, PATCHBAY_TOKEN_COMMUNE_GATEWAY_URL: "https://gateway.example/?key=secret" }), /PATCHBAY_TOKEN_COMMUNE_GATEWAY_URL/);
});
