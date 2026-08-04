import assert from "node:assert/strict";
import test from "node:test";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import {
  AdapterDiagnosticDetailSchema,
  AdapterDiagnosticPayloadSchema,
  AdapterDiagnosticSeverity,
  AdapterDiagnosticState,
  AdapterStatusPageSchema,
  AdapterStatusSchema,
  AuditRecordSchema,
  AuthorityDomainIdSchema,
  EventIdSchema,
  GenerationSchema,
  OperationState,
  DiagnosticsQuerySchema,
  FailureCode,
  ObservationKind,
  ObservationSchema,
  LsnSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  TargetScopeKind,
  TargetScopeSchema,
  QueryDiagnosticsResponseSchema,
  SubmissionOutcome,
} from "@patchbay/contracts";
import {
  adapterConnectionPresentation,
  buildAdapterStatusQueryOperation,
  foldAdapterDiagnosticObservation,
  mergeAdapterStatusResult,
} from "../src/domain/adapter-diagnostics.js";
import { emptyPresentationModel } from "../src/domain/model.js";

test("adapter status query uses generated diagnostics payload and explicit recent limit", () => {
  const operation = buildAdapterStatusQueryOperation(
    create(AuthorityDomainIdSchema, { value: "main" }),
    "pi",
    { commandId: "query-1", idempotencyKey: "query-key" },
  );
  assert.equal(operation.kind, 6);
  assert.equal(operation.targetScope?.kind, TargetScopeKind.AUTHORITY_DOMAIN);
  const decoded = requirePayload(operation.payload?.payload);
  assert.equal(decoded.query.case, "adapters");
  assert.equal(decoded.query.value.recentDiagnosticLimit, 20);
});

test("live diagnostics merge by source LSN and never changes adapter connectivity", () => {
  const model = emptyPresentationModel();
  model.authorityDomainId = "main";
  foldAdapterDiagnosticObservation(model, create(ObservationSchema, {
    kind: ObservationKind.EVENT,
    targetScope: create(TargetScopeSchema, {
      kind: TargetScopeKind.ADAPTER,
      adapterId: { value: "pi" },
    }),
    payload: create(PayloadEnvelopeSchema, {
      contentType: PayloadContentType.PROTOBUF,
      schemaRef: "patchbay.AdapterDiagnosticPayload",
      payload: toBinary(AdapterDiagnosticPayloadSchema, create(AdapterDiagnosticPayloadSchema, {
        code: "pi_adapter_started",
        severity: AdapterDiagnosticSeverity.INFO,
        adapterGeneration: { value: 1n },
        count: 1,
      })),
    }),
    failureCode: FailureCode.UNSPECIFIED,
  }), 12n);
  assert.equal(model.adapters.get("pi")?.recentDiagnostics[0]?.lsn, 12n);
  assert.equal(model.adapters.get("pi")?.status, undefined);

  const merged = mergeAdapterStatusResult(model, create(QueryDiagnosticsResponseSchema, {
    asOfLsn: create(LsnSchema, { value: 11n }),
    result: { case: "adapters", value: create(AdapterStatusPageSchema) },
  }));
  assert.equal(merged.adapters.get("pi")?.recentDiagnostics[0]?.lsn, 12n);
});

test("mixed runtime-session diagnostic targets are rejected without attribution", () => {
  const model = emptyPresentationModel();
  model.authorityDomainId = "main";
  foldAdapterDiagnosticObservation(model, create(ObservationSchema, {
    kind: ObservationKind.EVENT,
    targetScope: create(TargetScopeSchema, {
      kind: TargetScopeKind.RUNTIME_SESSION,
      adapterId: { value: "pi" },
      deploymentScope: "laptop",
      runtimeSessionId: { value: "session-1" },
      sessionGeneration: { value: 1n },
      resource: {
        adapterId: { value: "pi" },
        resourceKind: { value: "provider_pool" },
        resourceId: { value: "pool-1" },
      },
    }),
    payload: create(PayloadEnvelopeSchema, {
      contentType: PayloadContentType.PROTOBUF,
      schemaRef: "patchbay.AdapterDiagnosticPayload",
      payload: toBinary(AdapterDiagnosticPayloadSchema, create(AdapterDiagnosticPayloadSchema, {
        code: "pi_session_delivery_failed",
        severity: AdapterDiagnosticSeverity.ERROR,
        adapterGeneration: { value: 1n },
        count: 1,
      })),
    }),
    failureCode: FailureCode.EXECUTION_FAILED,
  }), 13n);

  assert.equal(model.adapters.get("pi"), undefined);
});

test("historical diagnostics reject mixed runtime-session targets without attribution", () => {
  const mixed = create(AuditRecordSchema, {
    sourceEventId: create(EventIdSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: "main" }),
      lsn: create(LsnSchema, { value: 13n }),
    }),
    targetScope: create(TargetScopeSchema, {
      kind: TargetScopeKind.RUNTIME_SESSION,
      adapterId: { value: "pi" },
      deploymentScope: "laptop",
      runtimeSessionId: { value: "session-1" },
      sessionGeneration: { value: 1n },
      resource: {
        adapterId: { value: "pi" },
        resourceKind: { value: "provider_pool" },
        resourceId: { value: "pool-1" },
      },
    }),
    failureCode: FailureCode.EXECUTION_FAILED,
    reasonCode: "pi_session_delivery_failed",
    adapterDiagnostic: create(AdapterDiagnosticDetailSchema, {
      adapterId: { value: "pi" },
      adapterGeneration: create(GenerationSchema, { value: 1n }),
      severity: AdapterDiagnosticSeverity.ERROR,
      count: 1,
    }),
  });
  const merged = mergeAdapterStatusResult(emptyPresentationModel(), create(QueryDiagnosticsResponseSchema, {
    submission: { outcome: SubmissionOutcome.ACCEPTED, operationState: OperationState.COMPLETED },
    asOfLsn: create(LsnSchema, { value: 13n }),
    result: {
      case: "adapters",
      value: create(AdapterStatusPageSchema, {
        adapters: [{ adapterId: { value: "pi" }, recentDiagnostics: [mixed] }],
      }),
    },
  }), "pi");

  assert.deepEqual(merged.adapters.get("pi")?.recentDiagnostics, []);
});

test("rejected and incomplete queries clear cached status instead of retaining stale attachment", () => {
  const model = emptyPresentationModel();
  model.adapters.set("pi", {
    adapterId: "pi",
    status: create(AdapterStatusSchema, { state: AdapterDiagnosticState.ATTACHED }),
    asOfLsn: 20n,
    recentDiagnostics: [],
  });
  const rejected = mergeAdapterStatusResult(model, create(QueryDiagnosticsResponseSchema, {
    submission: { outcome: SubmissionOutcome.REJECTED, operationState: OperationState.UNSPECIFIED },
  }), "pi");
  assert.equal(rejected.adapters.get("pi")?.status, undefined);

  const incomplete = mergeAdapterStatusResult(model, create(QueryDiagnosticsResponseSchema, {
    submission: { outcome: SubmissionOutcome.ACCEPTED, operationState: OperationState.FAILED },
    result: { case: "adapters", value: create(AdapterStatusPageSchema) },
  }), "pi");
  assert.equal(incomplete.adapters.get("pi")?.status, undefined);
});

test("older completed query cannot overwrite a newer failed status", () => {
  const model = emptyPresentationModel();
  model.adapters.set("pi", {
    adapterId: "pi",
    status: create(AdapterStatusSchema, { state: AdapterDiagnosticState.FAILED }),    asOfLsn: 20n,
    recentDiagnostics: [],
  });
  const older = mergeAdapterStatusResult(model, create(QueryDiagnosticsResponseSchema, {
    submission: { outcome: SubmissionOutcome.ACCEPTED, operationState: OperationState.COMPLETED },
    asOfLsn: create(LsnSchema, { value: 10n }),
    result: {
      case: "adapters",
      value: create(AdapterStatusPageSchema, {
        adapters: [{ adapterId: { value: "pi" }, state: AdapterDiagnosticState.ATTACHED }],
      }),
    },
  }), "pi");
  assert.equal(older.adapters.get("pi")?.status?.state, AdapterDiagnosticState.FAILED);
  assert.equal(older.adapters.get("pi")?.asOfLsn, 20n);
});

test("all adapter diagnostic states map to existing connectivity presentation", () => {
  assert.deepEqual(adapterConnectionPresentation(AdapterDiagnosticState.ATTACHED), { connectivity: "live", label: "attached" });
  assert.deepEqual(adapterConnectionPresentation(AdapterDiagnosticState.DETACHED), { connectivity: "offline", label: "detached" });
  assert.deepEqual(adapterConnectionPresentation(AdapterDiagnosticState.FAILED), { connectivity: "failed", label: "failed" });
  assert.deepEqual(adapterConnectionPresentation(AdapterDiagnosticState.UNKNOWN), { connectivity: "unknown", label: "unknown" });
});

function requirePayload(bytes: Uint8Array | undefined) {
  if (!bytes) throw new Error("missing query payload");
  return fromBinary(DiagnosticsQuerySchema, bytes);
}
