import assert from "node:assert/strict";
import test from "node:test";
import { create, fromBinary } from "@bufbuild/protobuf";
import {
  AdapterDiagnosticPayloadSchema,
  AdapterDiagnosticReportResultSchema,
  AdapterDiagnosticSeverity,
  FailureCode,
  OperationKind,
} from "@patchbay/contracts";
import {
  CoreDiagnosticsForwarder,
  composeAdapterDiagnostics,
  PI_FORWARDED_DIAGNOSTIC_CODES,
} from "../src/core_diagnostics_forwarder.js";
import { NOOP_ADAPTER_DIAGNOSTICS, type AdapterDiagnostics } from "../src/adapter_diagnostics.js";

test("forwarder maps only the shared registry and keeps reports structurally safe", async () => {
  const reports: any[] = [];
  const forwarder = new CoreDiagnosticsForwarder(
    async (report) => {
      reports.push(report);
      return create(AdapterDiagnosticReportResultSchema, { accepted: true });
    },
    { authorityDomainId: "main", adapterId: "pi", adapterGeneration: 4 },
    { reportsPerSecond: 1_000, now: () => new Date("2026-01-02T03:04:05.000Z") },
  );
  forwarder.record({
    event: "delivery.failed",
    level: "error",
    session: { runtimeSessionId: "runtime-1", deploymentScope: "machine-a", generation: 2 },
    commandId: "command-1",
    operationKind: OperationKind.INSTRUCT,
    failureCode: FailureCode.EXECUTION_FAILED,
    reason: "prompt must not cross the diagnostics boundary",
    error: { name: "Error", code: "secret-stack" },
  });
  await forwarder.flush();

  assert.equal(reports.length, 1);
  const report = reports[0];
  assert.equal(report.authorityDomainId.value, "main");
  assert.equal(report.targetScope.runtimeSessionId.value, "runtime-1");
  assert.equal(report.targetScope.sessionGeneration.value, 2n);
  assert.equal(report.correlations[0].ref.value.value, "command-1");
  assert.equal(report.failureCode, FailureCode.EXECUTION_FAILED);
  const payload = fromBinary(AdapterDiagnosticPayloadSchema, report.payload.payload);
  assert.equal(payload.code, PI_FORWARDED_DIAGNOSTIC_CODES["delivery.failed"]);
  assert.equal(payload.severity, AdapterDiagnosticSeverity.ERROR);
  assert.equal(payload.adapterGeneration?.value, 4n);
  assert.equal(payload.operationKind, OperationKind.INSTRUCT);
  assert.equal(payload.count, 1);
  const serialized = JSON.stringify(report, (_key, value) => typeof value === "bigint" ? value.toString() : value);
  assert.equal(serialized.includes("prompt must not"), false);
  assert.equal(serialized.includes("secret-stack"), false);
});

test("matching reports coalesce and never exceed the bounded count", async () => {
  const reports: any[] = [];
  const forwarder = new CoreDiagnosticsForwarder(
    async (report) => {
      reports.push(report);
      return create(AdapterDiagnosticReportResultSchema, { accepted: true });
    },
    { authorityDomainId: "main", adapterId: "pi", adapterGeneration: 1 },
    { reportsPerSecond: 1_000 },
  );
  for (let index = 0; index < 2_000; index += 1) {
    forwarder.record({ event: "adapter.started", level: "info" });
  }
  await forwarder.flush();
  assert.equal(reports.length, 1);
  assert.equal(fromBinary(AdapterDiagnosticPayloadSchema, reports[0].payload.payload).count, 1_000);
});

test("sink fanout isolates a throwing sink", async () => {
  const seen: string[] = [];
  const throwing: AdapterDiagnostics = {
    record() { throw new Error("broken sink"); },
    async flush() { throw new Error("broken flush"); },
    async close() { throw new Error("broken close"); },
  };
  const healthy: AdapterDiagnostics = {
    record() { seen.push("record"); },
    async flush() { seen.push("flush"); },
    async close() { seen.push("close"); },
  };
  const composed = composeAdapterDiagnostics([throwing, healthy, NOOP_ADAPTER_DIAGNOSTICS]);
  assert.doesNotThrow(() => composed.record({ event: "adapter.started", level: "info" }));
  await assert.doesNotReject(composed.flush());
  await assert.doesNotReject(composed.close());
  assert.deepEqual(seen, ["record", "flush", "close"]);
});

