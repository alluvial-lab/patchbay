import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { AdapterSnapshotSupport, AdapterTargetCategory, IdempotencyStrength, PayloadContentType } from "@patchbay/contracts";
import { loadTokenCommuneAdapterConfig } from "../src/config.js";
import { createCompositeLocalIdentitySynthesizer } from "../src/identity.js";
import { tokenCommuneCapabilityManifest } from "../src/manifest.js";
import { TOKEN_COMMUNE_RESOURCES } from "../src/resource_contract.js";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

test("manifest derives two honest PARTIAL operational-resource contracts from one registry", () => {
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
  assert.equal(manifest.resourceCapabilities.length, Object.keys(TOKEN_COMMUNE_RESOURCES).length);
  for (const [index, contract] of Object.values(TOKEN_COMMUNE_RESOURCES).entries()) {
    const capability = manifest.resourceCapabilities[index];
    assert.equal(capability?.resourceKind?.value, contract.kind);
    assert.equal(capability?.snapshotSupport, AdapterSnapshotSupport.PARTIAL);
    assert.equal(capability?.projectionContract?.targetCategory, AdapterTargetCategory.OPERATIONAL_RESOURCE);
    assert.equal(capability?.projectionContract?.payloadSchema?.schemaRef, contract.payloadSchema);
    assert.equal(capability?.projectionContract?.payloadSchema?.contentType, PayloadContentType.JSON);
    assert.equal(capability?.projectionContract?.projectionSchema?.schemaRef, contract.projectionSchema);
  }
});

test("resource schemas close secret-bearing fields and keep nullable telemetry explicit", () => {
  const files = [
    "provider-pool-payload.schema.json", "provider-pool-projection.schema.json",
    "member-draw-payload.schema.json", "member-draw-projection.schema.json",
  ];
  for (const file of files) {
    const schema = JSON.parse(readFileSync(join(root, "schemas", file), "utf8")) as Record<string, unknown>;
    assert.equal(schema["$schema"], "https://json-schema.org/draft/2020-12/schema");
    assert.equal(schema["additionalProperties"], false);
    assert.ok(Array.isArray(schema["required"]));
    assert.equal(/credential|password|prompt|response|diagnostic/i.test(JSON.stringify(schema)), false);
  }
  const pool = JSON.parse(readFileSync(join(root, "schemas/provider-pool-payload.schema.json"), "utf8")) as any;
  assert.deepEqual(pool.$defs.capacity.properties.usedFraction.type, ["number", "null"]);
  assert.deepEqual(pool.$defs.capacity.properties.usedUnits.type, ["number", "null"]);
  assert.deepEqual(pool.$defs.capacity.properties.limitUnits.type, ["number", "null"]);
  assert.deepEqual(pool.$defs.capacity.properties.resetsAt.type, ["string", "null"]);
  const draw = JSON.parse(readFileSync(join(root, "schemas/member-draw-payload.schema.json"), "utf8")) as any;
  assert.deepEqual(draw.$defs.drawReport.properties.drawUnits.type, ["number", "null"]);
});

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
  assert.throws(() => loadTokenCommuneAdapterConfig({ ...valid, PATCHBAY_ADAPTER_GENERATION: "1.5" }), /PATCHBAY_ADAPTER_GENERATION/);
  assert.throws(() => loadTokenCommuneAdapterConfig({ ...valid, PATCHBAY_TOKEN_COMMUNE_POLL_INTERVAL_MS: "0" }), /PATCHBAY_TOKEN_COMMUNE_POLL_INTERVAL_MS/);
  assert.throws(() => loadTokenCommuneAdapterConfig({ ...valid, PATCHBAY_TOKEN_COMMUNE_GATEWAY_URL: "https://gateway.example/?key=secret" }), /PATCHBAY_TOKEN_COMMUNE_GATEWAY_URL/);
});
