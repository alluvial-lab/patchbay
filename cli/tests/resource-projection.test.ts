import assert from "node:assert/strict";
import test from "node:test";

import { create, toBinary } from "@bufbuild/protobuf";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import {
  AdapterSnapshotSupport,
  AuthorityDomainIdSchema,
  EventIdSchema,
  GrantSummarySchema,
  LoadSecuritySnapshotResponseSchema,
  LoadSnapshotResponseSchema,
  LsnSchema,
  OperationKind,
  PayloadContentType,
  PayloadEnvelopeSchema,
  ResourceFreshnessState,
  ResourceIdSchema,
  ResourceIdentitySchema,
  ResourceKindSchema,
  ResourceSchema,
  ResourceSnapshotSchema,
  ResourceViewRevisionSchema,
  SecuritySnapshotSchema,
  SnapshotViewKind,
  TargetScopeKind,
  TargetScopeSchema,
} from "@patchbay/contracts";

import { resourceInspectCommand, resourceQueryCommand } from "../src/commands/resources.js";
import { parseCanonicalResourceIdentity } from "../src/commands/token-commune-projection.js";
import { captureOutput, DOMAIN } from "./helpers.js";

const encoder = new TextEncoder();
const domain = create(AuthorityDomainIdSchema, { value: DOMAIN });

function envelope(schemaRef: string, value: unknown) {
  return create(PayloadEnvelopeSchema, {
    schemaRef,
    contentType: PayloadContentType.JSON,
    payload: encoder.encode(JSON.stringify(value)),
  });
}

function reading(usedFraction: number | null) {
  return {
    window: "5h", usedFraction, usedUnits: usedFraction === null ? null : usedFraction * 100,
    limitUnits: usedFraction === null ? null : 100, resetsAt: null, source: "headers",
    observedAt: "2026-08-07T10:00:00Z",
  };
}

function poolProjection() {
  return {
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
        capacityReadings: [reading(0.35)],
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
  };
}

function drawProjection(limitFraction = 0.25) {
  return {
    memberDisplayName: "private-member-name",
    provider: "openai-codex",
    reports: [{
      provider: "openai-codex", limitFraction, fromDecree: false, consumedUnits: 4,
      drawUnits: null, exceeded: false, enforceable: false, resetsAt: null,
    }],
  };
}

function identity(adapterId: string, resourceKind: string, resourceId: string) {
  return create(ResourceIdentitySchema, {
    adapterId: { value: adapterId },
    resourceKind: create(ResourceKindSchema, { value: resourceKind }),
    resourceId: create(ResourceIdSchema, { value: resourceId }),
  });
}

function resource(adapterId: string, kind: "pool" | "draw", stale = false) {
  const pool = kind === "pool";
  const resourceKind = pool ? "token-commune.provider-pool" : "token-commune.member-draw";
  return create(ResourceSchema, {
    authorityDomainId: domain,
    identity: identity(adapterId, resourceKind, `${kind}-opaque`),
    resourcePayload: envelope(
      pool ? "patchbay.token_commune.provider_pool.payload.v1" : "patchbay.token_commune.member_draw.payload.v1",
      {},
    ),
    projectionPayload: envelope(
      pool ? "patchbay.token_commune.provider_pool.projection.v1" : "patchbay.token_commune.member_draw.projection.v1",
      pool ? poolProjection() : drawProjection(adapterId === "token-commune" ? 0.25 : 0.8),
    ),
    freshness: stale ? ResourceFreshnessState.STALE : ResourceFreshnessState.CURRENT,
    sourceAdapterGeneration: { value: 2n },
    revisionLsn: create(LsnSchema, { value: pool ? 9n : 10n }),
    observedAt: timestampFromDate(new Date("2026-08-07T10:01:00Z")),
  });
}

function client(options: { stalePool?: boolean; grants?: "both" | "pool" | "none" } = {}) {
  const resources = [resource("token-commune", "pool", options.stalePool), resource("token-commune", "draw"), resource("other-adapter", "draw")];
  const snapshot = create(ResourceSnapshotSchema, {
    authorityDomainId: domain,
    snapshotLsn: create(LsnSchema, { value: 12n }),
    resources,
    viewRevisions: [
      ["token-commune", "token-commune.provider-pool"],
      ["token-commune", "token-commune.member-draw"],
      ["other-adapter", "token-commune.member-draw"],
    ].map(([adapterId, resourceKind]) => create(ResourceViewRevisionSchema, {
      adapterId: { value: adapterId! }, resourceKind: create(ResourceKindSchema, { value: resourceKind! }),
      completeness: AdapterSnapshotSupport.PARTIAL, sourceAdapterGeneration: { value: 2n },
      revisionLsn: create(LsnSchema, { value: 12n }), observedAt: timestampFromDate(new Date("2026-08-07T10:01:00Z")),
    })),
  });
  const grants = options.grants ?? "both";
  const allowedKinds = grants === "none" ? [] : grants === "pool"
    ? ["token-commune.provider-pool"]
    : ["token-commune.provider-pool", "token-commune.member-draw"];
  const security = create(SecuritySnapshotSchema, {
    authorityDomainId: domain,
    snapshotLsn: create(LsnSchema, { value: 12n }),
    grants: allowedKinds.map((resourceKind, index) => create(GrantSummarySchema, {
      grantId: { value: `grant-${index}` },
      subjectActorId: { value: "operator" },
      targetScope: create(TargetScopeSchema, {
        kind: TargetScopeKind.RESOURCE,
        resource: identity("token-commune", resourceKind, resourceKind.endsWith("provider-pool") ? "pool-opaque" : "draw-opaque"),
      }),
      allowedOperationKinds: [OperationKind.QUERY],
    })),
  });
  return {
    async loadSnapshot() {
      return create(LoadSnapshotResponseSchema, {
        present: true,
        viewKind: SnapshotViewKind.RESOURCE,
        eventId: create(EventIdSchema, { authorityDomainId: domain, lsn: create(LsnSchema, { value: 12n }) }),
        snapshotPayload: toBinary(ResourceSnapshotSchema, snapshot),
      });
    },
    async loadSecuritySnapshot() {
      return create(LoadSecuritySnapshotResponseSchema, { snapshot: security });
    },
  };
}

test("resource-query text and JSON use the shared exact summary without private or aggregate data", async () => {
  const text = captureOutput();
  assert.equal(await resourceQueryCommand(client() as never, DOMAIN, { json: false }, text), 0);
  const rendered = text.out.join("\n");
  assert.match(rendered, /PROVIDER\s+DRAW\s+CREDENTIALS\s+5H CAPACITY\s+VERDICT\s+FRESHNESS\s+MODELS/);
  assert.match(rendered, /openai-codex/);
  assert.match(rendered, /25%/);
  assert.match(rendered, /35% used/);
  assert.match(rendered, /runnable/);
  assert.match(rendered, /Patchbay synthesis/);
  assert.doesNotMatch(rendered, /private-member-name|subKey|anonymous-contribution|remaining|average|weighted/i);

  const jsonOutput = captureOutput();
  await resourceQueryCommand(client() as never, DOMAIN, { adapterId: "token-commune", provider: "openai-codex", json: true }, jsonOutput);
  const json = JSON.parse(jsonOutput.out[0]!);
  assert.equal(json.summaries[0].draw.limitFraction, "0.25");
  assert.equal(json.summaries[0].capacity5h.usedFraction, "0.35");
  assert.equal(json.summaries[0].models[0].id, "gpt-5.5");
  assert.equal(JSON.stringify(json).includes("private-member-name"), false);
});

test("resource query grant-gates draw independently and succeeds explicitly when no pool is authorized", async () => {
  const poolOnly = captureOutput();
  await resourceQueryCommand(client({ grants: "pool" }) as never, DOMAIN, { json: false }, poolOnly);
  assert.match(poolOnly.out.join("\n"), /unavailable/);

  const none = captureOutput();
  assert.equal(await resourceQueryCommand(client({ grants: "none" }) as never, DOMAIN, { json: false }, none), 0);
  assert.deepEqual(none.out, ["No locally query-authorized token-commune pools matched."]);
});

test("wrong-adapter same-provider draw cannot redirect the join", async () => {
  const output = captureOutput();
  await resourceQueryCommand(client({ grants: "pool" }) as never, DOMAIN, { json: true }, output);
  const json = JSON.parse(output.out[0]!);
  assert.equal(json.summaries[0].draw.state, "unavailable");
  assert.notEqual(json.summaries[0].draw.limitFraction, "0.8");
});

test("resource-inspect prints canonical wrapper before the same safe summary", async () => {
  const output = captureOutput();
  assert.equal(await resourceInspectCommand(client({ stalePool: true }) as never, DOMAIN, {
    identity: "adapter=token-commune;resource-kind=token-commune.provider-pool;resource=pool-opaque",
    json: false,
  }, output), 0);
  assert.equal(output.out[0], "RESOURCE");
  assert.match(output.out.join("\n"), /REVISION\s+COMPLETENESS\s+FRESHNESS/);
  assert.match(output.out.join("\n"), /telemetry-stale/);
  assert.match(output.out.join("\n"), /TOKEN-COMMUNE POOLS/);
});

test("canonical resource parser reuses strict diagnostic grammar", () => {
  assert.deepEqual(parseCanonicalResourceIdentity("adapter=token-commune;resource-kind=token-commune.provider-pool;resource=pool%3Bsafe"), {
    adapterId: "token-commune", resourceKind: "token-commune.provider-pool", resourceId: "pool;safe",
  });
  for (const invalid of [
    "adapter=a;resource-kind=k",
    "adapter=a;adapter=b;resource-kind=k;resource=r",
    "adapter=a;resource-kind=k;resource=",
    "adapter=a;resource-kind=k;resource=%ZZ",
    "adapter=a;resource-kind=k;resource=r;unknown=x",
  ]) assert.throws(() => parseCanonicalResourceIdentity(invalid));
});
