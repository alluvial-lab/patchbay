import assert from "node:assert/strict";
import test from "node:test";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import { AdapterSnapshotSupport, PayloadContentType } from "@patchbay/contracts";
import type { GatewayCredential } from "../src/credential.js";
import { createHttpTokenCommuneGatewayClient } from "../src/gateway_client.js";
import { createCompositeLocalIdentitySynthesizer } from "../src/identity.js";
import { encodeResourceEnvelope, ResourceEnvelopeValidationError } from "../src/resource_envelope.js";
import {
  projectTokenCommuneSnapshot,
  SnapshotProjectionError,
  type TokenCommuneSnapshotProjectionInput,
} from "../src/snapshot_projection.js";
import { allSourcesGateway } from "./fixtures/snapshot_projection.js";

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

function decoded(envelope: { payload: Uint8Array }): any {
  return JSON.parse(new TextDecoder().decode(envelope.payload));
}

function projectedRows(input: TokenCommuneSnapshotProjectionInput, viewIndex: number): Array<{
  identity: string;
  payload: any;
  projection: any;
}> {
  const report = projectTokenCommuneSnapshot(input);
  assert.equal(report.report.case, "snapshot");
  if (report.report.case !== "snapshot") assert.fail("expected snapshot report");
  const view = report.report.value.views[viewIndex];
  return (view?.mutations ?? []).map((mutation) => {
    assert.equal(mutation.mutation.case, "upsert");
    if (mutation.mutation.case !== "upsert") assert.fail("expected upsert");
    assert.ok(mutation.mutation.value.resourcePayload);
    assert.ok(mutation.mutation.value.projectionPayload);
    return {
      identity: mutation.identity?.resourceId?.value ?? "",
      payload: decoded(mutation.mutation.value.resourcePayload),
      projection: decoded(mutation.mutation.value.projectionPayload),
    };
  });
}

function providerRows(input: TokenCommuneSnapshotProjectionInput) {
  return projectedRows(input, 0);
}

function memberRows(input: TokenCommuneSnapshotProjectionInput) {
  return projectedRows(input, 1);
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

  const capacityReading = {
    window: "5h", usedFraction: null, usedUnits: null, limitUnits: 100, resetsAt: null,
    source: "usage_endpoint", observedAt: "2026-08-07T11:57:00.000Z",
  };
  const contribution = {
    subKey: "local:anonymous-contribution:0123456789abcdef01234567:1",
    subKeySource: "synthesized-content-hash", subKeyStability: "snapshot-local", attribution: "unavailable",
    declaredShare: 0.5, health: { state: "fresh" }, telemetryState: "readings",
    capacityReadings: [capacityReading],
    fingerprint: { state: "unknown", templateSource: "compiled", since: null, diffPresent: false },
  };
  const payloadWithContribution = {
    ...providerPayload,
    contributionListing: { status: "reported", contributions: [contribution] },
  };
  for (const [telemetryState, capacityReadings] of [
    ["readings", []],
    ["no-readings", [capacityReading]],
  ] as const) {
    const contradictory = structuredClone(payloadWithContribution);
    contradictory.contributionListing.contributions[0]!.telemetryState = telemetryState;
    contradictory.contributionListing.contributions[0]!.capacityReadings = [...capacityReadings];
    assert.throws(
      () => encodeResourceEnvelope("providerPool", "payload", contradictory),
      ResourceEnvelopeValidationError,
    );
  }
  const invalidDate = structuredClone(payloadWithContribution);
  invalidDate.contributionListing.contributions[0]!.capacityReadings[0]!.observedAt = "not-a-date";
  assert.throws(
    () => encodeResourceEnvelope("providerPool", "payload", invalidDate),
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
  for (const seconds of [-62_135_596_801n, 253_402_300_800n]) {
    assert.throws(
      () => projectTokenCommuneSnapshot({
        ...baseInput,
        observedAt: { ...baseInput.observedAt, seconds },
      }),
      (error: unknown) => error instanceof SnapshotProjectionError && error.code === "invalid-context",
    );
  }
  for (const seconds of [-62_135_596_800n, 253_402_300_799n]) {
    assert.doesNotThrow(() => projectTokenCommuneSnapshot({
      ...baseInput,
      observedAt: { ...baseInput.observedAt, seconds },
    }));
  }
});

test("provider-pool projection preserves raw readings, health detail, models, and exact probe coverage", () => {
  const rows = providerRows({ ...baseInput, gateway: allSourcesGateway });
  assert.deepEqual(rows.map(({ projection }) => projection.provider), [
    "anthropic", "kimi-coding", "openai-codex", "status-only", "zai",
  ]);

  const anthropic = rows.find(({ projection }) => projection.provider === "anthropic");
  assert.ok(anthropic);
  assert.deepEqual(anthropic.projection.credentialHealthCounts, { fresh: 0, exhausted: 2, authBroken: 1 });
  assert.equal(anthropic.projection.totalDeclaredShare, 0.7);
  const contributions = anthropic.projection.contributionListing.contributions;
  const noReadings = contributions.filter((row: any) => row.telemetryState === "no-readings");
  assert.equal(noReadings.length, 2);
  assert.ok(noReadings.every((row: any) => row.capacityReadings.length === 0));
  assert.deepEqual(noReadings.map((row: any) => row.health.exhaustedUntil), [
    "2026-08-08T00:00:00.000Z", "2026-08-08T00:00:00.000Z",
  ]);
  assert.match(noReadings[0].subKey, /^local:anonymous-contribution:[0-9a-f]{24}:1$/);
  assert.match(noReadings[1].subKey, /^local:anonymous-contribution:[0-9a-f]{24}:2$/);
  assert.equal(noReadings[0].subKey.replace(/:1$/, ""), noReadings[1].subKey.replace(/:2$/, ""));
  assert.ok(noReadings.every((row: any) => row.subKeySource === "synthesized-content-hash" && row.subKeyStability === "snapshot-local" && row.attribution === "unavailable"));

  const sevenDay = contributions.find((row: any) => row.capacityReadings.some((reading: any) => reading.window === "7d"));
  assert.ok(sevenDay);
  assert.equal(sevenDay.health.state, "auth_broken");
  assert.equal(sevenDay.health.reason, "revoked key");
  assert.equal(sevenDay.telemetryState, "readings");
  assert.equal(sevenDay.capacityReadings.some((reading: any) => reading.window === "5h"), false);
  assert.deepEqual(sevenDay.capacityReadings[0], {
    window: "7d", usedFraction: 0, usedUnits: 0, limitUnits: null, resetsAt: null,
    source: "usage_endpoint", observedAt: "2026-08-07T11:57:00.000Z",
  });
  assert.equal(Object.hasOwn(sevenDay, "contributionId"), false, "status ids are not joined to anonymous pool rows");
  assert.deepEqual(anthropic.projection.statusTelemetry.anthropicHealth, {
    state: "auth_broken", reason: "upstream credential expired",
  });
  assert.deepEqual(anthropic.projection.statusTelemetry.contributions.map((row: any) => row.contributionId), ["status-anthropic"]);
  assert.deepEqual(anthropic.projection.modelCatalog.models.map((model: any) => [model.id, model.upstreamModel, model.available]), [
    ["claude-sonnet-4-5", null, false],
  ]);
  assert.equal(anthropic.projection.fingerprint.status, "reported");
  assert.equal(anthropic.projection.fingerprint.probe, "anthropic");

  const modelOnly = rows.find(({ projection }) => projection.provider === "kimi-coding");
  assert.ok(modelOnly);
  assert.deepEqual(modelOnly.projection.contributionListing, { status: "not-reported", contributions: [] });
  assert.deepEqual(modelOnly.projection.statusTelemetry, { status: "not-reported", contributions: [] });
  assert.deepEqual(modelOnly.projection.modelCatalog.models.map((model: any) => [model.id, model.upstreamModel]), [["k3", null]]);
  assert.deepEqual(modelOnly.projection.fingerprint, { status: "unknown", probe: null, reason: "not-probed" });

  const codex = rows.find(({ projection }) => projection.provider === "openai-codex");
  assert.ok(codex);
  assert.equal(codex.projection.fingerprint.status, "reported");
  assert.equal(codex.projection.fingerprint.probe, "openai-codex");
  assert.deepEqual(codex.projection.modelCatalog.models.map((model: any) => model.id), ["gpt-5.5"]);
  assert.equal(codex.projection.modelCatalog.models.some((model: any) => model.id === "gpt-5.6"), false);

  const unprobed = rows.find(({ projection }) => projection.provider === "zai");
  assert.ok(unprobed);
  assert.deepEqual(unprobed.projection.fingerprint, { status: "unknown", probe: null, reason: "not-probed" });
  assert.equal(unprobed.projection.fingerprint.value, undefined, "no-probe providers cannot fabricate fingerprint ok");

  for (const { identity, payload, projection } of rows) {
    assert.match(identity, /^local:provider-pool:/);
    assert.equal(payload.provider, projection.provider);
    assert.equal(payload.limitations.capacityAggregation, "none");
    assert.equal(projection.capacityAggregation, "none");
    assert.equal(["usedFraction", "remainingPercentage", "selectedWindow", "highest5h"].some((key) => Object.hasOwn(projection, key)), false);
    assert.equal(projection.contributionListing.contributions.some((row: any) => row.subKey === identity), false);
  }
});

test("gateway provider canonicalization prevents whitespace aliases from emitting colliding mutations", async () => {
  const credential: GatewayCredential = {
    apply() {}, redactionSecrets: () => [], dispose() {},
  };
  const client = createHttpTokenCommuneGatewayClient({
    baseUrl: new URL("https://gateway.example/"),
    credential,
    fetch: async (request) => {
      const path = new URL(request instanceof URL ? request.href : request.toString()).pathname;
      if (path === "/commune/pool") {
        const fingerprint = { state: "unknown", templateSource: "compiled", since: null, diff: null };
        return Response.json({ providers: [
          { provider: "zai", declaredShare: 0.5, health: { state: "fresh" }, capacity: [], fingerprint },
          { provider: " zai ", declaredShare: 0.5, health: { state: "fresh" }, capacity: [], fingerprint },
        ] });
      }
      if (path === "/v1/models") {
        return Response.json({ data: [{
          id: "glm-5", provider: " zai ", surface: "chat",
          context_window: 200_000, max_tokens: 8_192, reasoning: true, available: true,
        }] });
      }
      assert.fail(`unexpected gateway path ${path}`);
    },
  });
  const [pool, models] = await Promise.all([client.getPool(), client.getModels()]);
  const rows = providerRows({
    ...baseInput,
    gateway: { ...baseInput.gateway, pool: { status: "reported", value: pool }, models: { status: "reported", value: models } },
  });
  assert.equal(rows.length, 1);
  assert.equal(rows[0]?.payload.provider, "zai");
  assert.equal(rows[0]?.projection.contributionListing.contributions.length, 2);
});

test("projection rejects duplicate synthesized resource identities", () => {
  const duplicatingIdentities = {
    ...identities,
    providerPool() { return identities.providerPool("duplicate"); },
  };
  assert.throws(
    () => projectTokenCommuneSnapshot({ ...baseInput, identities: duplicatingIdentities, gateway: allSourcesGateway }),
    (error: unknown) => error instanceof SnapshotProjectionError && error.code === "duplicate-identity",
  );
});

test("provider mapping is deterministic across source-row reordering and rejects synthesized identity mismatch", () => {
  const reversedGateway = structuredClone(allSourcesGateway);
  if (reversedGateway.pool.status === "reported") (reversedGateway.pool.value.contributions as any[]).reverse();
  if (reversedGateway.status.status === "reported") (reversedGateway.status.value.contributions as any[]).reverse();
  if (reversedGateway.models.status === "reported") (reversedGateway.models.value.models as any[]).reverse();
  assert.deepEqual(
    providerRows({ ...baseInput, gateway: allSourcesGateway }),
    providerRows({ ...baseInput, gateway: reversedGateway }),
  );

  const mismatchedIdentities = {
    ...identities,
    providerPool(provider: string) {
      return { ...identities.providerPool(provider), adapterId: "wrong-adapter" };
    },
  };
  assert.throws(
    () => projectTokenCommuneSnapshot({ ...baseInput, identities: mismatchedIdentities, gateway: allSourcesGateway }),
    (error: unknown) => error instanceof SnapshotProjectionError && error.code === "identity-mismatch",
  );
});

test("member-draw projection retains every same-provider row and native provenance/calibration nulls", () => {
  const rows = memberRows({ ...baseInput, gateway: allSourcesGateway });
  assert.deepEqual(rows.map(({ projection }) => projection.provider), ["anthropic", "openai-codex"]);
  const anthropic = rows[0];
  assert.ok(anthropic);
  assert.match(anthropic.identity, /^local:member-draw:/);
  assert.equal(anthropic.projection.memberDisplayName, "Ada");
  assert.equal(anthropic.projection.reports.length, 2, "same-provider reports are not collapsed");
  assert.deepEqual(anthropic.projection.reports, [
    {
      provider: "anthropic", limitFraction: 0.2, fromDecree: false, consumedUnits: 0,
      drawUnits: 0, exceeded: false, enforceable: true, resetsAt: "2026-08-09T00:00:00.000Z",
    },
    {
      provider: "anthropic", limitFraction: 0.6, fromDecree: true, consumedUnits: 11,
      drawUnits: null, exceeded: false, enforceable: false, resetsAt: null,
    },
  ]);
  assert.deepEqual(anthropic.payload.reports, anthropic.projection.reports);
  assert.equal(Object.hasOwn(anthropic.projection, "enforcementState"), false);
  assert.equal(["total", "average", "selectedReport", "usedFraction"].some((key) => Object.hasOwn(anthropic.projection, key)), false);

  const codex = rows[1];
  assert.ok(codex);
  assert.deepEqual(codex.projection.reports, [{
    provider: "openai-codex", limitFraction: 0.4, fromDecree: false, consumedUnits: 3,
    drawUnits: 4.5, exceeded: true, enforceable: true, resetsAt: "2026-08-08T00:00:00.000Z",
  }]);
});

test("member-draw omission and display-name churn never infer aggregate, unknown, or retirement", () => {
  const emptyGateway = structuredClone(allSourcesGateway);
  if (emptyGateway.me.status === "reported") (emptyGateway.me.value as any).reports = [];
  assert.deepEqual(memberRows({ ...baseInput, gateway: emptyGateway }), []);
  assert.deepEqual(memberRows({
    ...baseInput,
    gateway: { ...allSourcesGateway, me: { status: "unavailable" } },
  }), []);

  const renamedGateway = structuredClone(allSourcesGateway);
  if (renamedGateway.me.status === "reported") (renamedGateway.me.value as any).displayName = "Grace";
  const before = memberRows({ ...baseInput, gateway: allSourcesGateway });
  const after = memberRows({ ...baseInput, gateway: renamedGateway });
  assert.equal(before.length, after.length);
  assert.ok(before.every((row, index) => row.identity !== after[index]?.identity));
});

test("PARTIAL completeness emits only classifiable upserts and leaves omission to core staleness", () => {
  const current = projectTokenCommuneSnapshot({ ...baseInput, gateway: allSourcesGateway });
  assert.equal(current.report.case, "snapshot");
  if (current.report.case !== "snapshot") assert.fail("expected snapshot report");
  for (const view of current.report.value.views) {
    assert.equal(view.completeness, AdapterSnapshotSupport.PARTIAL);
    assert.ok(view.mutations.length > 0);
    assert.ok(view.mutations.every((mutation) => mutation.mutation.case === "upsert"));
    assert.equal(view.mutations.some((mutation) => mutation.mutation.case === "unknown"), false);
    assert.equal(view.mutations.some((mutation) => mutation.mutation.case === "tombstone"), false);
  }

  const reportedEmpty = projectTokenCommuneSnapshot({
    ...baseInput,
    gateway: {
      status: { status: "unavailable" },
      pool: { status: "reported", value: { contributions: [] } },
      me: { status: "reported", value: { displayName: "Ada", reports: [] } },
      fingerprints: {
        status: "reported",
        value: {
          anthropic: { templateSource: null, capturedAt: null, capturePresent: false, holdReason: null, heldAt: null, diffPresent: false },
          codex: { templateSource: null, capturedAt: null, capturePresent: false, holdReason: null, heldAt: null, diffPresent: false },
        },
      },
      models: { status: "reported", value: { models: [] } },
    },
  });
  assert.equal(reportedEmpty.report.case, "snapshot");
  if (reportedEmpty.report.case !== "snapshot") assert.fail("expected snapshot report");
  assert.ok(reportedEmpty.report.value.views.every((view) => view.completeness === AdapterSnapshotSupport.PARTIAL));
  assert.ok(reportedEmpty.report.value.views.every((view) => view.mutations.length === 0), "omitted identities are not encoded as stale/unknown/tombstone");
});

test("honesty invariants distinguish zero telemetry and no readings without aggregate or probe fabrication", () => {
  const rows = providerRows({ ...baseInput, gateway: allSourcesGateway });
  const anthropic = rows.find(({ projection }) => projection.provider === "anthropic");
  assert.ok(anthropic);
  const contributions = anthropic.projection.contributionListing.contributions;
  assert.ok(contributions.some((row: any) => row.telemetryState === "no-readings" && row.capacityReadings.length === 0));
  assert.ok(contributions.some((row: any) => row.telemetryState === "readings" && row.capacityReadings.some((reading: any) => reading.usedFraction === 0)));
  assert.equal(Object.hasOwn(anthropic.projection, "usedFraction"), false, "a provider-level capacity percentage is forbidden");

  const unavailablePool = providerRows({
    ...baseInput,
    gateway: { ...allSourcesGateway, pool: { status: "unavailable" } },
  });
  assert.deepEqual(
    unavailablePool.find(({ projection }) => projection.provider === "anthropic")?.projection.contributionListing,
    { status: "unavailable", contributions: [] },
  );

  const unavailableFingerprints = providerRows({
    ...baseInput,
    gateway: { ...allSourcesGateway, fingerprints: { status: "unavailable" } },
  });
  const anthropicUnavailable = unavailableFingerprints.find(({ projection }) => projection.provider === "anthropic");
  const zaiUnavailable = unavailableFingerprints.find(({ projection }) => projection.provider === "zai");
  assert.deepEqual(anthropicUnavailable?.projection.fingerprint, {
    status: "unknown", probe: "anthropic", reason: "probe-unavailable",
  });
  assert.deepEqual(zaiUnavailable?.projection.fingerprint, {
    status: "unknown", probe: null, reason: "not-probed",
  });
});

test("cast-malformed typed capacity evidence fails the atomic projection boundary", () => {
  const malformed = structuredClone(allSourcesGateway) as any;
  malformed.pool.value.contributions[0].capacity = [{
    window: "5h", usedFraction: -1, usedUnits: null, limitUnits: null, resetsAt: null,
    source: "headers", observedAt: "2026-08-07T11:00:00.000Z",
  }];
  assert.throws(
    () => projectTokenCommuneSnapshot({ ...baseInput, gateway: malformed }),
    (error: unknown) => error instanceof SnapshotProjectionError && error.code === "contract-validation-failed",
  );
});
