import assert from "node:assert/strict";
import test from "node:test";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import { AdapterSnapshotSupport, PayloadContentType } from "@patchbay/contracts";
import { createCompositeLocalIdentitySynthesizer } from "../src/identity.js";
import { encodeResourceEnvelope, ResourceEnvelopeValidationError } from "../src/resource_envelope.js";
import {
  projectTokenCommuneSnapshot,
  SnapshotProjectionError,
  type TokenCommuneSnapshotProjectionInput,
} from "../src/snapshot_projection.js";

const providerPayload = {
  identityStrategy: "composite-local",
  gatewayDeploymentKey: "deployment",
  provider: "zai",
  contributionListing: { status: "not-reported", contributions: [] },
  statusTelemetry: { status: "unavailable", contributions: [] },
  modelCatalog: { status: "unavailable", models: [] },
  fingerprint: { status: "unknown", probe: null, reason: "not-probed" },
  limitations: {
    snapshotCompleteness: "partial",
    contributorAttribution: "unavailable",
    contributionIdentity: "snapshot-local-synthesized",
    statusPoolJoin: "unavailable",
    capacityAggregation: "none",
  },
} as const;
const providerProjection = {
  provider: "zai",
  contributionListing: providerPayload.contributionListing,
  credentialHealthCounts: { fresh: 0, exhausted: 0, authBroken: 0 },
  totalDeclaredShare: 0,
  statusTelemetry: providerPayload.statusTelemetry,
  modelCatalog: providerPayload.modelCatalog,
  fingerprint: providerPayload.fingerprint,
  capacityAggregation: "none",
} as const;
const drawReport = {
  provider: "zai", limitFraction: 0.25, fromDecree: true, consumedUnits: 0,
  drawUnits: null, exceeded: false, enforceable: false, resetsAt: null,
};
const memberPayload = {
  identityStrategy: "composite-local", gatewayDeploymentKey: "deployment",
  memberDisplayName: "Ada", provider: "zai", reports: [drawReport],
  limitations: { snapshotCompleteness: "partial", stableMemberIdentity: "unavailable" },
} as const;
const memberProjection = { memberDisplayName: "Ada", provider: "zai", reports: [drawReport] } as const;

const identities = createCompositeLocalIdentitySynthesizer({
  adapterId: "token-commune",
  gatewayBaseUrl: new URL("https://gateway.example/"),
});
const unavailable = { status: "unavailable" } as const;
const baseInput: TokenCommuneSnapshotProjectionInput = {
  adapterId: "token-commune",
  adapterGeneration: 3,
  observedAt: timestampFromDate(new Date("2026-08-07T12:00:00.000Z")),
  identities,
  gateway: {
    status: unavailable,
    pool: unavailable,
    me: unavailable,
    fingerprints: unavailable,
    models: unavailable,
  },
};

function decoded(envelope: { payload: Uint8Array }): unknown {
  return JSON.parse(new TextDecoder().decode(envelope.payload));
}

test("manifest-bound envelope construction validates JSON and selects literal descriptors", () => {
  const cases = [
    ["providerPool", "payload", providerPayload, "patchbay.token_commune.provider_pool.payload.v1"],
    ["providerPool", "projection", providerProjection, "patchbay.token_commune.provider_pool.projection.v1"],
    ["memberDraw", "payload", memberPayload, "patchbay.token_commune.member_draw.payload.v1"],
    ["memberDraw", "projection", memberProjection, "patchbay.token_commune.member_draw.projection.v1"],
  ] as const;
  for (const [resource, role, value, schemaRef] of cases) {
    const envelope = encodeResourceEnvelope(resource, role, value);
    assert.equal(envelope.contentType, PayloadContentType.JSON);
    assert.equal(envelope.schemaRef, schemaRef);
    assert.deepEqual(decoded(envelope), value);
  }
  assert.throws(
    () => encodeResourceEnvelope("providerPool", "projection", { ...providerProjection, usedFraction: 0.5 }),
    ResourceEnvelopeValidationError,
  );
});

test("pure report factory emits two registry-ordered PARTIAL snapshot views", () => {
  const report = projectTokenCommuneSnapshot(baseInput);
  assert.equal(report.adapterId?.value, "token-commune");
  assert.equal(report.adapterGeneration?.value, 3n);
  assert.deepEqual(report.observedAt, baseInput.observedAt);
  assert.equal(report.report.case, "snapshot");
  if (report.report.case !== "snapshot") assert.fail("expected snapshot report");
  assert.deepEqual(report.report.value.views.map((view) => view.resourceKind?.value), [
    "token-commune.provider-pool",
    "token-commune.member-draw",
  ]);
  assert.ok(report.report.value.views.every((view) => view.completeness === AdapterSnapshotSupport.PARTIAL));
  assert.ok(report.report.value.views.every((view) => view.mutations.length === 0));
});

test("invalid projection context fails before returning a report", () => {
  assert.throws(
    () => projectTokenCommuneSnapshot({ ...baseInput, adapterGeneration: 0 }),
    (error: unknown) => error instanceof SnapshotProjectionError && error.code === "invalid-context",
  );
  assert.throws(
    () => projectTokenCommuneSnapshot({ ...baseInput, adapterId: " " }),
    (error: unknown) => error instanceof SnapshotProjectionError && error.code === "invalid-context",
  );
});
