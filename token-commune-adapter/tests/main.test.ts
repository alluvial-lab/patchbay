import assert from "node:assert/strict";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import { CommandIdSchema, DeliverySchema, EventIdSchema, LsnSchema, OperationKind, OperationSchema, TargetScopeKind, TargetScopeSchema } from "@patchbay/contracts";
import { AdapterProcess } from "../src/main.js";
import type { PatchbayCoreClient } from "../src/core_client.js";
import type { AdapterDiagnostics } from "../src/adapter_diagnostics.js";
import type { TokenCommuneGatewayClient } from "../src/gateway_client.js";

const gateway = {} as TokenCommuneGatewayClient;
const baseOptions = {
  coreAddress: "http://core", adapterId: "token-commune", adapterGeneration: 1,
  authorityDomainId: "default", attachmentEvidence: "secret", gatewayBaseUrl: new URL("https://gateway.example/"),
  gatewayCredentialFile: "/unused", pollIntervalMs: 30_000, diagnosticPath: "/unused", gateway,
};

function delivery(lsn: bigint) {
  return create(DeliverySchema, {
    operation: create(OperationSchema, {
      commandId: create(CommandIdSchema, { value: "command-query" }), kind: OperationKind.QUERY,
      targetScope: create(TargetScopeSchema, { kind: TargetScopeKind.ADAPTER, adapterId: { value: "token-commune" } }),
    }),
    deliveryEventId: create(EventIdSchema, { authorityDomainId: { value: "default" }, lsn: create(LsnSchema, { value: lsn }) }),
  });
}

test("delivery loop acknowledges then rejects unsupported without invoking gateway", async () => {
  const calls: string[] = [];
  const controller = new AbortController();
  const diagnostics: AdapterDiagnostics = {
    record(input) { calls.push(`diagnostic:${input.event}`); },
    async flush() { calls.push("flush"); }, async close() { calls.push("close"); },
  };
  const core = {
    setDiagnostics() {},
    async attach(generation: number) { calls.push(`attach:${generation}`); return create(EventIdSchema); },
    async acknowledgeDelivery() { calls.push("acknowledge"); return create(EventIdSchema); },
    async rejectUnsupported() { calls.push("unsupported"); controller.abort(); return create(EventIdSchema); },
    async *receiveDeliveries(cursor: bigint) { calls.push(`receive:${cursor}`); yield delivery(9n); },
    async reportDiagnostic() { throw new Error("unused"); },
  } as unknown as PatchbayCoreClient;
  const host = new AdapterProcess({ ...baseOptions, diagnostics, coreClient: core, retryDelayMs: 1 });
  await host.run(controller.signal);
  assert.ok(calls.indexOf("attach:1") < calls.indexOf("diagnostic:adapter.started"));
  assert.ok(calls.indexOf("acknowledge") < calls.indexOf("unsupported"));
  assert.ok(calls.includes("diagnostic:delivery.unsupported"));
  await host.dispose();
  await host.dispose();
  assert.equal(calls.filter((value) => value === "close").length, 1);
});

test("finite delivery completion is retried as unavailable until abort", async () => {
  let subscriptions = 0;
  const controller = new AbortController();
  const core = {
    setDiagnostics() {}, async attach() { return create(EventIdSchema); },
    async acknowledgeDelivery() { return undefined; }, async rejectUnsupported() { return undefined; },
    async *receiveDeliveries() {
      subscriptions += 1;
      if (subscriptions === 2) controller.abort();
    },
    async reportDiagnostic() { throw new Error("unused"); },
  } as unknown as PatchbayCoreClient;
  const host = new AdapterProcess({ ...baseOptions, coreClient: core, retryDelayMs: 1 });
  await host.run(controller.signal);
  assert.equal(subscriptions, 2);
  await host.dispose();
});
