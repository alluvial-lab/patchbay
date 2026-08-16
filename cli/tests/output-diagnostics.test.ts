import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import test from "node:test";
import { create, fromBinary } from "@bufbuild/protobuf";
import {
  AdapterAssuranceManifestSchema,
  AdapterAssuranceManifestV1Schema,
  AdapterCapabilitySummarySchema,
  AdapterDiagnosticDetailSchema,
  AdapterDiagnosticReportingCapabilitySchema,
  AdapterDiagnosticSeverity,
  AdapterDiagnosticState,
  AdapterReconciliationStrength,
  AdapterSnapshotSupport,
  AdapterStatusPageSchema,
  AdapterTargetCategory,
  AdapterStatusSchema,
  DiagnosticsQuerySchema,
  AuditEventKind,
  AuditPageSchema,
  AuditRecordSchema,
  AuthorityDomainIdSchema,
  CommandInspectionResultSchema,
  CommandInspectionSchema,
  CommandSummarySchema,
  EventIdSchema,
  FailureCode,
  GenerationSchema,
  GrantIdSchema,
  IdempotencyStrength,
  LsnSchema,
  OperationKind,
  OperationState,
  PayloadContentType,
  QueryDiagnosticsResponseSchema,
  ReconciliationAction,
  ResourceCapabilitySchema,
  ResourceKindSchema,
  ResourceProjectionContractSchema,
  SchemaDescriptorSchema,
  SessionSchema,
  SpawnClaimDisposition,
  SubmissionOutcome,
  SubmissionResultSchema,
} from "@patchbay/contracts";
import { adapterStatusCommand } from "../src/commands/adapter-status.js";
import { auditQueryCommand } from "../src/commands/audit-query.js";
import {
  adapterTables,
  auditPageView,
  adapterStatusPageView,
  commandInspectionView,
  inspectionTables,
  parseAuditTarget,
} from "../src/commands/diagnostics.js";
import { sessionHealthCommand } from "../src/commands/session-health.js";
import { CredentialStore } from "../src/credentials.js";
import { parseArguments, run, usage } from "../src/main.js";
import { exitCodeForSubmission, printSubmissionResult, targetScopeView } from "../src/output.js";
import { captureOutput, credentials, diagnosticsResponse, DOMAIN, session, snapshotResponse } from "./helpers.js";

function assuranceCapability(action: ReconciliationAction) {
  return create(AdapterCapabilitySummarySchema, {
    assurance: create(AdapterAssuranceManifestSchema, {
      contract: {
        case: "v1",
        value: create(AdapterAssuranceManifestV1Schema, {
          deduplicationStrength: IdempotencyStrength.NONE,
          continuationProofSupport: false,
          cursorSupport: false,
          generationFenceSupport: false,
          reconciliationStrength: AdapterReconciliationStrength.NONE,
          unprovenOutcomeAction: action,
        }),
      },
    }),
  });
}

async function credentialStore(): Promise<CredentialStore> {
  const directory = await mkdtemp(join(tmpdir(), "patchbay-cli-diagnostics-"));
  const store = new CredentialStore(join(directory, "credentials.json"));
  await store.write(credentials());
  return store;
}

test("audit target parsing separates legacy audit ids from canonical operational resources", () => {
  const legacy = parseAuditTarget("resource=principal%2Fone", DOMAIN)!;
  assert.equal(legacy.legacyAuditResourceId, "principal/one");
  assert.equal(legacy.resource, undefined);
  assert.deepEqual(targetScopeView(legacy), {
    kind: "resource",
    actorId: null,
    adapterId: null,
    runtimeSessionId: null,
    sessionGeneration: null,
    deploymentScope: null,
    projectOrGroup: null,
    legacyAuditResourceId: "principal/one",
    resource: null,
  });

  const operational = parseAuditTarget(
    "adapter=token%2Fcommune;resource-kind=provider%20pool;resource=shared%2Fone",
    DOMAIN,
  )!;
  assert.equal(operational.legacyAuditResourceId, "");
  assert.equal(operational.resource?.adapterId?.value, "token/commune");
  assert.equal(operational.resource?.resourceKind?.value, "provider pool");
  assert.equal(operational.resource?.resourceId?.value, "shared/one");
  assert.deepEqual(targetScopeView(operational), {
    kind: "resource",
    actorId: null,
    adapterId: null,
    runtimeSessionId: null,
    sessionGeneration: null,
    deploymentScope: null,
    projectOrGroup: null,
    legacyAuditResourceId: null,
    resource: {
      adapterId: "token/commune",
      resourceKind: "provider pool",
      resourceId: "shared/one",
    },
  });
  assert.throws(
    () => parseAuditTarget("adapter=a;resource-kind=pool", DOMAIN),
    /requires adapter, resource-kind, and resource/,
  );
});

test("SubmissionOutcome has stable script-facing exit codes", () => {
  assert.equal(exitCodeForSubmission(SubmissionOutcome.ACCEPTED), 0);
  assert.equal(exitCodeForSubmission(SubmissionOutcome.REJECTED), 2);
  assert.equal(exitCodeForSubmission(SubmissionOutcome.FAILED), 3);
  assert.equal(exitCodeForSubmission(SubmissionOutcome.UNKNOWN), 4);
  assert.equal(exitCodeForSubmission(SubmissionOutcome.UNSPECIFIED), 1);
});

test("UNKNOWN output consumes each generated action and defaults undeclared assurance conservatively", () => {
  const result = create(SubmissionResultSchema, {
    outcome: SubmissionOutcome.UNKNOWN,
    operationState: OperationState.UNSPECIFIED,
  });

  for (const [capability, qualifier] of [
    [assuranceCapability(ReconciliationAction.NONE), "unknown"],
    [assuranceCapability(ReconciliationAction.MANUAL_REQUIRED), "manual-required"],
    [undefined, "manual-required"],
  ] as const) {
    const output = captureOutput();
    assert.equal(printSubmissionResult(result, true, output, capability), 4);
    assert.deepEqual(JSON.parse(output.out[0]!), {
      outcome: "unknown",
      outcomeQualifier: qualifier,
      commandId: null,
      operationState: "unspecified",
      failureCode: null,
      diagnosticMessage: null,
      acceptedLsn: null,
      deduplicated: false,
    });
    assert.match(output.err.join("\n"), new RegExp(`UNKNOWN \\(${qualifier}\\)`));
    assert.match(output.err.join("\n"), /reconcile via the core's command records/);

    const human = captureOutput();
    assert.equal(printSubmissionResult(result, false, human, capability), 4);
    assert.match(human.out[0]!, new RegExp(`outcome=unknown qualifier=${qualifier}`));
  }
});

test("session-health emits canonical connectivity and activity as JSON", async () => {
  const output = captureOutput();
  assert.equal(
    await sessionHealthCommand(
      { async loadSnapshot() { return snapshotResponse(); } } as never,
      DOMAIN,
      { json: true },
      output,
    ),
    0,
  );

  const rows = JSON.parse(output.out[0]!) as Array<Record<string, unknown>>;
  assert.equal(rows[0]?.["connectivity"], "live");
  assert.equal(rows[0]?.["activity"], "working");
  assert.equal(rows[0]?.["model"], "provider/model-1");
  assert.match(String(rows[0]?.["identity"]), /adapter=pi-adapter.*generation=3/);
});

test("session-health renders unavailable model as null in JSON and unknown in tables", async () => {
  const output = captureOutput();
  const unavailable = create(SessionSchema, { ...session(), model: "" });
  assert.equal(
    await sessionHealthCommand(
      { async loadSnapshot() { return snapshotResponse([unavailable]); } } as never,
      DOMAIN,
      { json: true },
      output,
    ),
    0,
  );
  assert.equal(JSON.parse(output.out[0]!)[0].model, null);

  const table = captureOutput();
  await sessionHealthCommand(
    { async loadSnapshot() { return snapshotResponse([unavailable]); } } as never,
    DOMAIN,
    { json: false },
    table,
  );
  assert.match(table.out[0]!, /MODEL/);
  assert.match(table.out[1]!, /Model unknown/);
});

test("secret-bearing arguments are rejected without echoing their value", async () => {
  const secret = "argv-secret-must-not-appear";
  const output = captureOutput();
  const exit = await run(
    ["login", `--password=${secret}`],
    output,
    { env: { PATCHBAY_CORE_SECRET: "configured" } },
  );

  assert.equal(exit, 1);
  assert.match(output.err.join("\n"), /unknown option: --password/);
  assert.doesNotMatch([...output.out, ...output.err].join("\n"), new RegExp(secret));
  assert.throws(() => parseArguments(["--setup-secret", secret]), /unknown option: --setup-secret/);
  assert.doesNotMatch(usage(), /--(?:password|setup-secret)/);
  assert.match(usage(), /PATCHBAY_OPERATOR_PASSWORD/);
});

test("state-changing dispatch refuses a missing credential store before submission", async () => {
  const output = captureOutput();
  const exit = await run(
    ["instruct", "runtime-1", "hello"],
    output,
    {
      env: {
        PATCHBAY_CORE_SECRET: "configured",
        PATCHBAY_CREDENTIALS_PATH: "/tmp/patchbay-cli-test-definitely-missing/credentials.json",
      },
    },
  );
  assert.equal(exit, 1);
  assert.match(output.err.join("\n"), /run patchbay-cli login/);
});

test("diagnostics typed empty pages are successful and JSON-safe", async () => {
  const store = await credentialStore();
  const output = captureOutput();
  const page = create(AuditPageSchema);
  const exit = await auditQueryCommand(
    { async queryDiagnostics() { return diagnosticsResponse("audit", page); } } as never,
    store,
    DOMAIN,
    { json: true },
    output,
  );
  assert.equal(exit, 0);
  const document = JSON.parse(output.out[0]!);
  assert.deepEqual(document.result.records, []);
  assert.deepEqual(document.result.page, { hasMore: false, nextBeforeEvent: null });
  assert.deepEqual(output.err, []);
});

test("pre-acceptance rejection exits 2 and never renders a typed result", async () => {
  const store = await credentialStore();
  const output = captureOutput();
  const exit = await auditQueryCommand(
    { async queryDiagnostics() {
      return diagnosticsResponse("audit", create(AuditPageSchema), {
        submission: create(SubmissionResultSchema, {
          outcome: SubmissionOutcome.REJECTED,
          operationState: OperationState.REJECTED,
          failureCode: FailureCode.VALIDATION_FAILED,
          diagnosticMessage: "invalid query",
        }),
      });
    } } as never,
    store,
    DOMAIN,
    { json: true },
    output,
  );
  assert.equal(exit, 2);
  assert.equal(JSON.parse(output.out[0]!).result, null);
});

test("accepted lifecycle failure retains submission detail and exits 3", async () => {
  const store = await credentialStore();
  const output = captureOutput();
  const response = diagnosticsResponse("audit", create(AuditPageSchema), {
    submission: create(SubmissionResultSchema, {
      outcome: SubmissionOutcome.ACCEPTED,
      operationState: OperationState.FAILED,
      failureCode: FailureCode.EXECUTION_FAILED,
      diagnosticMessage: "adapter execution failed",
    }),
  });
  const exit = await auditQueryCommand(
    { async queryDiagnostics() { return response; } } as never,
    store,
    DOMAIN,
    { json: true },
    output,
  );
  assert.equal(exit, 3);
  const document = JSON.parse(output.out[0]!);
  assert.equal(document.submission.operationState, "failed");
  assert.equal(document.submission.failureCode, "execution_failed");
  assert.equal(document.result, null);
});

test("transport errors remain failures at the command boundary", async () => {
  const store = await credentialStore();
  await assert.rejects(
    auditQueryCommand(
      { async queryDiagnostics() { throw new Error("transport unavailable"); } } as never,
      store,
      DOMAIN,
      { json: true },
      captureOutput(),
    ),
    /transport unavailable/,
  );
});

test("accepted nonterminal diagnostics lifecycle fails closed", async () => {
  const store = await credentialStore();
  await assert.rejects(
    auditQueryCommand(
      { async queryDiagnostics() {
        return diagnosticsResponse("audit", create(AuditPageSchema), {
          submission: create(SubmissionResultSchema, {
            outcome: SubmissionOutcome.ACCEPTED,
            operationState: OperationState.RUNNING,
          }),
        });
      } } as never,
      store,
      DOMAIN,
      { json: true },
      captureOutput(),
    ),
    /unexpected diagnostics operation state/,
  );
});

test("wrong and missing diagnostics result oneofs fail closed", async () => {
  const store = await credentialStore();
  const wrong = diagnosticsResponse("adapters", create(AdapterStatusPageSchema));
  await assert.rejects(
    auditQueryCommand({ async queryDiagnostics() { return wrong; } } as never, store, DOMAIN, { json: true }, captureOutput()),
    /expected audit/,
  );
  const missing = create(QueryDiagnosticsResponseSchema, {
    submission: create(SubmissionResultSchema, {
      outcome: SubmissionOutcome.ACCEPTED,
      operationState: OperationState.COMPLETED,
    }),
    resultEventId: create(EventIdSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: DOMAIN }),
      lsn: create(LsnSchema, { value: 12n }),
    }),
    asOfLsn: create(LsnSchema, { value: 12n }),
  });
  await assert.rejects(
    auditQueryCommand({ async queryDiagnostics() { return missing; } } as never, store, DOMAIN, { json: true }, captureOutput()),
    /expected audit/,
  );
});

test("diagnostic audit and adapter projections include safe diagnostic fields", () => {
  const record = create(AuditRecordSchema, {
    kind: AuditEventKind.ADAPTER_DIAGNOSTIC_REPORTED,
    failureCode: FailureCode.UNSPECIFIED,
    adapterDiagnostic: create(AdapterDiagnosticDetailSchema, {
      adapterGeneration: create(GenerationSchema, { value: 9n }),
      severity: AdapterDiagnosticSeverity.WARNING,
      operationKind: OperationKind.INSTRUCT,
      count: 4,
    }),
  });
  const audit = auditPageView(create(AuditPageSchema, { records: [record] }));
  assert.equal(audit.records[0]?.adapterDiagnostic?.severity, "warning");
  assert.equal(audit.records[0]?.adapterDiagnostic?.adapterGeneration, "9");

  const page = create(AdapterStatusPageSchema, {
    adapters: [create(AdapterStatusSchema, {
      capability: create(AdapterCapabilitySummarySchema, {
        sessionSnapshotSupport: AdapterSnapshotSupport.PARTIAL,
        assurance: create(AdapterAssuranceManifestSchema, {
          contract: {
            case: "v1",
            value: create(AdapterAssuranceManifestV1Schema, {
              deduplicationStrength: IdempotencyStrength.AT_PATCHBAY_BOUNDARY,
              continuationProofSupport: false,
              cursorSupport: false,
              generationFenceSupport: false,
              reconciliationStrength: AdapterReconciliationStrength.NONE,
              unprovenOutcomeAction: ReconciliationAction.MANUAL_REQUIRED,
            }),
          },
        }),
        diagnosticReporting: create(AdapterDiagnosticReportingCapabilitySchema, {
          diagnosticCodes: ["heartbeat_lag"],
        }),
        targetCategories: [AdapterTargetCategory.RUNTIME_SESSION, AdapterTargetCategory.OPERATIONAL_RESOURCE],
        resourceCapabilities: [create(ResourceCapabilitySchema, {
          resourceKind: create(ResourceKindSchema, { value: "provider_pool" }),
          snapshotSupport: AdapterSnapshotSupport.AUTHORITATIVE,
          projectionContract: create(ResourceProjectionContractSchema, {
            targetCategory: AdapterTargetCategory.OPERATIONAL_RESOURCE,
            payloadSchema: create(SchemaDescriptorSchema, {
              schemaRef: "token-commune.pool.payload.v1",
              contentType: PayloadContentType.PROTOBUF,
            }),
            projectionSchema: create(SchemaDescriptorSchema, {
              schemaRef: "token-commune.pool.projection.v1",
              contentType: PayloadContentType.JSON,
            }),
          }),
        })],
      }),
      recentDiagnostics: [record],
    })],
  });
  const projected = adapterStatusPageView(page).adapters[0]!;
  assert.deepEqual(projected.capability?.diagnosticReporting, { diagnosticCodes: ["heartbeat_lag"] });
  assert.equal(projected.capability?.sessionSnapshotSupport, "partial");
  assert.deepEqual(projected.capability?.assurance, {
    contract: "v1",
    deduplicationStrength: "at_patchbay_boundary",
    continuationProofSupport: false,
    cursorSupport: false,
    generationFenceSupport: false,
    reconciliationStrength: "none",
    unprovenOutcomeAction: "manual_required",
    unknownOutcomeQualifier: "manual-required",
  });
  assert.deepEqual(projected.capability?.targetCategories, ["runtime_session", "operational_resource"]);
  assert.deepEqual(projected.capability?.resourceCapabilities, [{
    resourceKind: "provider_pool",
    snapshotSupport: "authoritative",
    projectionContract: {
      targetCategory: "operational_resource",
      payloadSchema: { schemaRef: "token-commune.pool.payload.v1", contentType: "protobuf" },
      projectionSchema: { schemaRef: "token-commune.pool.projection.v1", contentType: "json" },
    },
  }]);
  assert.equal(projected.recentDiagnostics[0]?.adapterDiagnostic?.operationKind, "instruct");
  const human = adapterTables(page).sections[0]!;
  assert.ok(human.headers.includes("SESSION_SNAPSHOT"));
  assert.ok(human.headers.includes("RESOURCE_SNAPSHOTS"));
  assert.ok(human.headers.includes("DEDUPLICATION"));
  assert.ok(human.headers.includes("CONTINUATION_PROOF"));
  assert.ok(human.headers.includes("CURSOR"));
  assert.ok(human.headers.includes("GENERATION_FENCE"));
  assert.ok(human.headers.includes("RECONCILIATION"));
  assert.ok(human.headers.includes("UNKNOWN_OUTCOME"));
  assert.ok(human.rows[0]?.includes("provider_pool=authoritative"));
  assert.ok(human.rows[0]?.includes("manual-required"));
  assert.doesNotMatch(JSON.stringify(audit.records[0]), /prompt|attachment|descriptor|BEARER/);
});

test("adapter diagnostics reject sentinel and unknown numerics across every assurance enum", () => {
  type Capability = ReturnType<typeof assuranceCapability>;
  const cases: ReadonlyArray<{
    name: string;
    mutate(capability: Capability): void;
    expected: RegExp;
  }> = [
    {
      name: "deduplication sentinel",
      mutate(value) {
        if (value.assurance?.contract.case === "v1") {
          value.assurance.contract.value.deduplicationStrength = IdempotencyStrength.UNSPECIFIED;
        }
      },
      expected: /deduplication strength/,
    },
    {
      name: "deduplication unknown",
      mutate(value) {
        if (value.assurance?.contract.case === "v1") {
          value.assurance.contract.value.deduplicationStrength = 99 as IdempotencyStrength;
        }
      },
      expected: /deduplication strength/,
    },
    {
      name: "reconciliation sentinel",
      mutate(value) {
        if (value.assurance?.contract.case === "v1") {
          value.assurance.contract.value.reconciliationStrength = AdapterReconciliationStrength.UNSPECIFIED;
        }
      },
      expected: /reconciliation strength/,
    },
    {
      name: "reconciliation unknown",
      mutate(value) {
        if (value.assurance?.contract.case === "v1") {
          value.assurance.contract.value.reconciliationStrength = 99 as AdapterReconciliationStrength;
        }
      },
      expected: /reconciliation strength/,
    },
    {
      name: "action sentinel",
      mutate(value) {
        if (value.assurance?.contract.case === "v1") {
          value.assurance.contract.value.unprovenOutcomeAction = ReconciliationAction.UNSPECIFIED;
        }
      },
      expected: /unproven-outcome action/,
    },
    {
      name: "action unknown",
      mutate(value) {
        if (value.assurance?.contract.case === "v1") {
          value.assurance.contract.value.unprovenOutcomeAction = 99 as ReconciliationAction;
        }
      },
      expected: /unproven-outcome action/,
    },
  ];

  for (const { name, mutate, expected } of cases) {
    const capability = assuranceCapability(ReconciliationAction.NONE);
    mutate(capability);
    const page = create(AdapterStatusPageSchema, {
      adapters: [create(AdapterStatusSchema, { capability })],
    });
    assert.throws(() => adapterStatusPageView(page), expected, name);
  }
});

test("adapter diagnostics fail closed on a missing or incomplete assurance contract", () => {
  const missing = create(AdapterStatusPageSchema, {
    adapters: [create(AdapterStatusSchema, {
      capability: create(AdapterCapabilitySummarySchema),
    })],
  });
  assert.throws(
    () => adapterStatusPageView(missing),
    /assurance is missing the supported V1 contract/,
  );

  const incomplete = create(AdapterStatusPageSchema, {
    adapters: [create(AdapterStatusSchema, {
      capability: create(AdapterCapabilitySummarySchema, {
        assurance: create(AdapterAssuranceManifestSchema, {
          contract: {
            case: "v1",
            value: create(AdapterAssuranceManifestV1Schema, {
              deduplicationStrength: IdempotencyStrength.NONE,
              continuationProofSupport: false,
              cursorSupport: undefined,
              generationFenceSupport: false,
              reconciliationStrength: AdapterReconciliationStrength.NONE,
              unprovenOutcomeAction: ReconciliationAction.NONE,
            }),
          },
        }),
      }),
    })],
  });
  assert.throws(() => adapterStatusPageView(incomplete), /assurance V1 is incomplete/);
});

test("inspect-command renders every canonical spawn claim disposition in JSON and text", () => {
  const cases: Array<[SpawnClaimDisposition, string]> = [
    [SpawnClaimDisposition.ACTIVE, "active"],
    [SpawnClaimDisposition.POISONED_PENDING_RECONCILIATION, "poisoned_pending_reconciliation"],
    [SpawnClaimDisposition.RELEASED_NO_EXTERNAL_EFFECT, "released_no_external_effect"],
    [SpawnClaimDisposition.PROMOTED, "promoted"],
    [SpawnClaimDisposition.TARGET_ABANDONED, "target_abandoned"],
  ];

  for (const [disposition, label] of cases) {
    const result = create(CommandInspectionResultSchema, {
      found: true,
      inspection: create(CommandInspectionSchema, {
        command: create(CommandSummarySchema, { kind: OperationKind.SPAWN }),
        currentState: OperationState.DELIVERED,
        spawnClaimDisposition: disposition,
      }),
    });
    assert.equal(commandInspectionView(result).inspection?.spawnClaimDisposition, label);
    const commandRows = inspectionTables(result).sections[0]!.rows;
    assert.deepEqual(commandRows.find(([field]) => field === "CLAIM_DISPOSITION"), ["CLAIM_DISPOSITION", label]);
  }
});

test("diagnostics option grammar rejects cross-command and duplicate enum options", async () => {
  const env = { PATCHBAY_CORE_SECRET: "configured" };
  const wrongCommand = captureOutput();
  assert.equal(await run(["adapter-status", "--kind", "login"], wrongCommand, { env }), 1);
  assert.match(wrongCommand.err[0]!, /unknown option: --kind for adapter-status/);
  const wrongSession = captureOutput();
  assert.equal(await run(["session-health", "--limit", "1"], wrongSession, { env }), 1);
  assert.match(wrongSession.err[0]!, /unknown option: --limit for session-health/);
  assert.throws(() => parseArguments(["--kind", "login", "--kind", "logout"]), /duplicate option/);
  assert.match(usage(), /--since is inclusive/);
  assert.match(usage(), /canonical runtime identity/);
});

test("audit-query carries grant-id filters and renders grant ids safely", async () => {
  const store = await credentialStore();
  let operationPayload: Uint8Array | undefined;
  const record = create(AuditRecordSchema, {
    kind: AuditEventKind.GRANT_REVOKED,
    grantId: create(GrantIdSchema, { value: "grant-safe-1" }),
  });
  const output = captureOutput();
  const exit = await auditQueryCommand(
    { async queryDiagnostics(request: { operation?: { payload?: { payload?: Uint8Array } } }) {
      operationPayload = request.operation?.payload?.payload;
      return diagnosticsResponse("audit", create(AuditPageSchema, { records: [record] }));
    } } as never,
    store,
    DOMAIN,
    { grantId: "grant-safe-1", json: true },
    output,
  );
  assert.equal(exit, 0);
  const query = fromBinary(DiagnosticsQuerySchema, operationPayload!);
  assert.equal(query.query.case, "audit");
  if (query.query.case === "audit") assert.equal(query.query.value.grantId?.value, "grant-safe-1");
  assert.equal(JSON.parse(output.out[0]!).result.records[0].grantId, "grant-safe-1");
  assert.doesNotMatch(output.out[0]!, /payload|descriptor|BEARER/);
});

test("omitted diagnostic limits remain absent so core applies its defaults", async () => {
  const store = await credentialStore();
  let payload: Uint8Array | undefined;
  await auditQueryCommand(
    { async queryDiagnostics(request: { operation?: { payload?: { payload?: Uint8Array } } }) {
      payload = request.operation?.payload?.payload;
      return diagnosticsResponse("audit", create(AuditPageSchema));
    } } as never,
    store,
    DOMAIN,
    { json: true },
    captureOutput(),
  );
  const query = fromBinary(DiagnosticsQuerySchema, payload!);
  assert.equal(query.query.case, "audit");
  if (query.query.case === "audit") assert.equal(query.query.value.limit, undefined);
});

test("explicit empty opaque adapter cursor reaches the diagnostics wire", async () => {
  const store = await credentialStore();
  let operationPayload: Uint8Array | undefined;
  const exit = await adapterStatusCommand(
    { async queryDiagnostics(request: { operation?: { payload?: { payload?: Uint8Array } } }) {
      operationPayload = request.operation?.payload?.payload;
      return diagnosticsResponse("adapters", create(AdapterStatusPageSchema));
    } } as never,
    store,
    DOMAIN,
    { adapterIds: [], afterAdapterId: "", json: true },
    captureOutput(),
  );
  assert.equal(exit, 0);
  const query = fromBinary(DiagnosticsQuerySchema, operationPayload!);
  assert.equal(query.query.case, "adapters");
  if (query.query.case === "adapters") assert.equal(query.query.value.afterAdapterId, "");
});

test("adapter-status requests the bounded recent-diagnostics prefix by default", async () => {
  const store = await credentialStore();
  let operationPayload: Uint8Array | undefined;
  const exit = await adapterStatusCommand(
    { async queryDiagnostics(request: { operation?: { payload?: { payload?: Uint8Array } } }) {
      operationPayload = request.operation?.payload?.payload;
      return diagnosticsResponse("adapters", create(AdapterStatusPageSchema));
    } } as never,
    store,
    DOMAIN,
    { adapterIds: [], json: true },
    captureOutput(),
  );
  assert.equal(exit, 0);
  const query = fromBinary(DiagnosticsQuerySchema, operationPayload!);
  assert.equal(query.query.case, "adapters");
  if (query.query.case === "adapters") {
    assert.equal(query.query.value.limit, undefined);
    assert.equal(query.query.value.recentDiagnosticLimit, 100);
  }
});
