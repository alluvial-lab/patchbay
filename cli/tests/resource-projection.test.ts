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
  ObservationKind,
  ObservationSchema,
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
  StoredEventKind,
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

function poolProjection(options: {
  provider?: string;
  modelId?: string;
  modelProvider?: string;
  upstreamModel?: string | null;
  readingState?: "current" | "old" | "none" | "null";
} = {}) {
  const provider = options.provider ?? "openai-codex";
  const readingState = options.readingState ?? "current";
  const capacityReadings = readingState === "none" ? [] : [{
    ...reading(readingState === "null" ? null : 0.35),
    observedAt: readingState === "old" ? "2026-08-07T04:00:00Z" : "2026-08-07T10:00:00Z",
  }];
  return {
    provider,
    contributionListing: {
      status: "reported",
      contributions: [{
        subKey: "local:anonymous-contribution:0123456789abcdef01234567:1",
        subKeySource: "synthesized-content-hash",
        subKeyStability: "snapshot-local",
        attribution: "unavailable",
        declaredShare: 1,
        health: { state: "fresh" },
        telemetryState: capacityReadings.length ? "readings" : "no-readings",
        capacityReadings,
        fingerprint: { state: "ok", templateSource: "compiled", since: null, diffPresent: false },
      }],
    },
    credentialHealthCounts: { fresh: 1, exhausted: 0, authBroken: 0 },
    totalDeclaredShare: 1,
    statusTelemetry: { status: "not-reported", contributions: [] },
    modelCatalog: { status: "reported", models: [{
      id: options.modelId ?? "gpt-5.5",
      provider: options.modelProvider ?? provider,
      surface: "codex",
      upstreamModel: options.upstreamModel ?? null,
      contextWindow: 200000,
      maxTokens: 8192,
      reasoning: true,
      available: true,
    }] },
    fingerprint: { status: "unknown", probe: null, reason: "not-probed" },
    capacityAggregation: "none",
  };
}

function drawProjection(limitFraction = 0.25, provider = "openai-codex") {
  return {
    memberDisplayName: "private-member-name",
    provider,
    reports: [{
      provider, limitFraction, fromDecree: false, consumedUnits: 4,
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

type PoolProjectionState = "decoded" | "invalid" | "unsupported" | "unavailable";

type ProjectionOptions = {
  poolProjectionState?: PoolProjectionState;
  provider?: string;
  modelId?: string;
  modelProvider?: string;
  upstreamModel?: string | null;
  readingState?: "current" | "old" | "none" | "null";
};

function resource(
  adapterId: string,
  kind: "pool" | "draw",
  stale = false,
  options: ProjectionOptions = {},
) {
  const pool = kind === "pool";
  const resourceKind = pool ? "token-commune.provider-pool" : "token-commune.member-draw";
  const projectionState = options.poolProjectionState ?? "decoded";
  const hasPayloads = !pool || projectionState !== "unavailable";
  const projectionSchema = pool
    ? projectionState === "unsupported"
      ? "patchbay.token_commune.provider_pool.projection.v2"
      : "patchbay.token_commune.provider_pool.projection.v1"
    : "patchbay.token_commune.member_draw.projection.v1";
  const projection = pool
    ? projectionState === "invalid" ? "{" : poolProjection(options)
    : drawProjection(adapterId === "token-commune" ? 0.25 : 0.8, options.provider);
  return create(ResourceSchema, {
    authorityDomainId: domain,
    identity: identity(adapterId, resourceKind, `${kind}-opaque`),
    ...(hasPayloads ? {
      resourcePayload: envelope(
        pool ? "patchbay.token_commune.provider_pool.payload.v1" : "patchbay.token_commune.member_draw.payload.v1",
        {},
      ),
      projectionPayload: envelope(projectionSchema, projection),
    } : {}),
    freshness: stale ? ResourceFreshnessState.STALE : ResourceFreshnessState.CURRENT,
    sourceAdapterGeneration: { value: 2n },
    revisionLsn: create(LsnSchema, { value: pool ? 9n : 10n }),
    observedAt: timestampFromDate(new Date("2026-08-07T10:01:00Z")),
  });
}

function client(options: {
  stalePool?: boolean;
  grants?: "both" | "pool" | "none";
} & ProjectionOptions = {}) {
  const resources = [
    resource("token-commune", "pool", options.stalePool, options),
    resource("token-commune", "draw", false, options),
    resource("other-adapter", "draw", false, options),
  ];
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
    async *subscribe() {
      const poolEvent = create(ObservationSchema, {
        authorityDomainId: domain,
        sender: { actorId: { value: "token-commune" } },
        kind: ObservationKind.STATUS,
        targetScope: create(TargetScopeSchema, {
          kind: TargetScopeKind.RESOURCE,
          resource: identity("token-commune", "token-commune.provider-pool", "pool-opaque"),
        }),
        payload: envelope("patchbay.token_commune.pool_event.v1", {
          sourceEventId: "event-safe", kind: "capacity_shift", provider: options.provider ?? "openai-codex",
          contributionId: "private-contribution", message: "private source message",
          occurredAt: "2026-08-07T10:02:00Z", deliveryModel: "polling", historyMode: "latest-50-no-cursor",
        }),
      });
      yield { payload: { kind: StoredEventKind.OBSERVATION, payload: toBinary(ObservationSchema, poolEvent) } };
    },
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
  assert.match(rendered, /PROVIDER\s+DRAW\s+CONTRIBUTIONS\s+5H CAPACITY\s+FINGERPRINT\s+VERDICT\s+FRESHNESS\s+MODELS\s+EVENTS/);
  assert.match(rendered, /openai-codex/);
  assert.match(rendered, /25%/);
  assert.match(rendered, /consumed 4; reset unavailable/);
  assert.match(rendered, /1 anonymous \/ 100% declared share/);
  assert.match(rendered, /35% used.*source .* ago; reset unavailable/);
  assert.match(rendered, /unknown \(not-probed\)/);
  assert.match(rendered, /pool:capacity_shift@2026-08-07T10:02:00Z/);
  assert.match(rendered, /runnable/);
  assert.match(rendered, /Patchbay synthesis/);
  assert.doesNotMatch(rendered, /private-member-name|subKey|anonymous-contribution|remaining|average|weighted/i);

  const jsonOutput = captureOutput();
  await resourceQueryCommand(client() as never, DOMAIN, { adapterId: "token-commune", provider: "openai-codex", json: true }, jsonOutput);
  const json = JSON.parse(jsonOutput.out[0]!);
  assert.equal(json.summaries[0].draw.limitFraction, "0.25");
  assert.equal(json.summaries[0].capacity5h.usedFraction, "0.35");
  assert.equal(json.summaries[0].credentials.totalDeclaredShare, "1");
  assert.equal(json.summaries[0].fingerprint.status, "unknown");
  assert.equal(json.summaries[0].wrapperFreshness, "current");
  assert.equal(json.summaries[0].readingFreshness, "current");
  assert.equal(json.summaries[0].recentEvents[0].code, "capacity_shift");
  assert.equal(json.summaries[0].models[0].id, "gpt-5.5");
  assert.equal(json.summaries[0].models[0].upstreamModel, null);
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

test("CLI keeps wrapper freshness separate from old source age and labels both missing-reading states unavailable", async () => {
  const oldOutput = captureOutput();
  await resourceQueryCommand(client({ readingState: "old" }) as never, DOMAIN, { json: true }, oldOutput);
  const old = JSON.parse(oldOutput.out[0]!).summaries[0];
  assert.equal(old.wrapperFreshness, "current");
  assert.equal(old.readingFreshness, "current");
  assert.equal(old.capacity5h.observedAt, "2026-08-07T04:00:00Z");

  for (const readingState of ["none", "null"] as const) {
    const output = captureOutput();
    await resourceQueryCommand(client({ readingState }) as never, DOMAIN, { json: true }, output);
    const summary = JSON.parse(output.out[0]!).summaries[0];
    assert.equal(summary.wrapperFreshness, "current");
    assert.equal(summary.readingFreshness, "unavailable");
    assert.notEqual(summary.readingFreshness, "current");
  }
});

test("cross-provider models are omitted instead of displayed under the enclosing pool", async () => {
  const output = captureOutput();
  await resourceQueryCommand(client({ modelProvider: "anthropic" }) as never, DOMAIN, { json: true }, output);
  const json = JSON.parse(output.out[0]!);
  assert.deepEqual(json.summaries[0].models, []);
  assert.equal(json.summaries[0].verdict, "unknown");
  assert.equal(JSON.stringify(json).includes("anthropic"), false);
});

test("invalid, unsupported, and unavailable canonical pools query and inspect as honest unknown summaries", async () => {
  for (const poolProjectionState of ["invalid", "unsupported", "unavailable"] as const) {
    const queryOutput = captureOutput();
    assert.equal(await resourceQueryCommand(
      client({ poolProjectionState }) as never,
      DOMAIN,
      { json: true },
      queryOutput,
    ), 0);
    const query = JSON.parse(queryOutput.out[0]!);
    assert.equal(query.summaries.length, 1);
    assert.equal(query.summaries[0].provider, "provider unavailable");
    assert.equal(query.summaries[0].draw.state, "unknown");
    assert.equal(query.summaries[0].credentials.state, "unknown");
    assert.equal(query.summaries[0].capacity5h.state, "unknown");
    assert.equal(query.summaries[0].wrapperFreshness, "unknown");
    assert.equal(query.summaries[0].readingFreshness, "unknown");
    assert.deepEqual(query.summaries[0].models, []);
    assert.equal(query.summaries[0].verdict, "unknown");

    const inspectOutput = captureOutput();
    assert.equal(await resourceInspectCommand(client({ poolProjectionState }) as never, DOMAIN, {
      identity: "adapter=token-commune;resource-kind=token-commune.provider-pool;resource=pool-opaque",
      json: true,
    }, inspectOutput), 0);
    const inspect = JSON.parse(inspectOutput.out[0]!);
    assert.equal(inspect.summary.provider, "provider unavailable");
    assert.equal(inspect.summary.verdict, "unknown");
  }
});

test("text tables escape adapter control characters while JSON preserves exact values", async () => {
  const provider = "openai\n\u001b[31mcodex";
  const modelId = "model\n\u001b[2Jspoof\u009bC1";
  const textOutput = captureOutput();
  await resourceQueryCommand(client({ provider, modelId }) as never, DOMAIN, { json: false }, textOutput);
  assert.equal(textOutput.out.every((line) => !line.includes("\n") && !line.includes("\u001b")), true);
  const text = textOutput.out.join("\n");
  assert.match(text, /openai\\n\\x1b\[31mcodex/);
  assert.match(text, /model\\n\\x1b\[2Jspoof\\x9bC1/);

  const jsonOutput = captureOutput();
  await resourceQueryCommand(client({ provider, modelId }) as never, DOMAIN, { json: true }, jsonOutput);
  const json = JSON.parse(jsonOutput.out[0]!);
  assert.equal(json.summaries[0].provider, provider);
  assert.equal(json.summaries[0].models[0].id, modelId);
  assert.equal(json.summaries[0].models[0].upstreamModel, null);
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
