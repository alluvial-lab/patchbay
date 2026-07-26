import assert from "node:assert/strict";
import test from "node:test";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import {
  AdapterDiagnosticPayloadSchema,
  AdapterDiagnosticSeverity,
  AdapterDiagnosticState,
  AdapterStatusPageSchema,
  AuthorityDomainIdSchema,
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
