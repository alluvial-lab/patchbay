import assert from "node:assert/strict";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import { Code, ConnectError } from "@connectrpc/connect";
import { CommandIdSchema, DeliverySchema, EventIdSchema, FailureCode, LsnSchema, OperationKind, OperationSchema, OperationState, TargetScopeKind, TargetScopeSchema } from "@patchbay/contracts";
import { AdapterProcess } from "../src/main.js";
import type { PatchbayCoreClient } from "../src/core_client.js";
import type { AdapterDiagnostics } from "../src/adapter_diagnostics.js";
import type { TokenCommuneGatewayClient } from "../src/gateway_client.js";
import type { TokenCommunePoller } from "../src/poller.js";

const gateway = {} as TokenCommuneGatewayClient;
const quietPoller = {
  async run(signal: AbortSignal) {
    if (signal.aborted) return;
    await new Promise<void>((resolve) => signal.addEventListener("abort", () => resolve(), { once: true }));
  },
} as TokenCommunePoller;
const baseOptions = {
  coreAddress: "http://core", adapterId: "token-commune", adapterGeneration: 1,
  authorityDomainId: "default", attachmentEvidence: "secret", gatewayBaseUrl: new URL("https://gateway.example/"),
  gatewayCredentialFile: "/unused", pollIntervalMs: 30_000, diagnosticPath: "/unused", gateway, poller: quietPoller,
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

test("delivery loop acknowledges then fails unsupported without invoking gateway", async () => {
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
  assert.equal(calls.filter((value) => value === "acknowledge").length, 1);
  assert.equal(calls.filter((value) => value === "unsupported").length, 1);
  assert.ok(calls.indexOf("acknowledge") < calls.indexOf("unsupported"));
  assert.ok(calls.includes("diagnostic:delivery.unsupported"));
  await host.dispose();
  await host.dispose();
  assert.equal(calls.filter((value) => value === "close").length, 1);
});

test("unsupported terminalization retries after acknowledgement without acknowledging twice", async () => {
  const calls: string[] = [];
  const transitions: Array<{ state: OperationState; failureCode: FailureCode }> = [];
  const controller = new AbortController();
  let terminalizationAttempts = 0;
  const core = {
    setDiagnostics() {}, async attach() { return create(EventIdSchema); },
    async acknowledgeDelivery() {
      calls.push("acknowledge");
      transitions.push({ state: OperationState.DELIVERED, failureCode: FailureCode.UNSPECIFIED });
      return create(EventIdSchema);
    },
    async rejectUnsupported() {
      terminalizationAttempts += 1;
      calls.push(`fail:${terminalizationAttempts}`);
      if (terminalizationAttempts === 1) throw new ConnectError("transient", Code.Unavailable);
      transitions.push({ state: OperationState.REJECTED, failureCode: FailureCode.UNSUPPORTED_COMMAND });
      return create(EventIdSchema);
    },
    async *receiveDeliveries(cursor: bigint) {
      calls.push(`receive:${cursor}`);
      if (cursor === 0n) yield delivery(9n);
      else controller.abort();
    },
    async reportDiagnostic() { throw new Error("unused"); },
  } as unknown as PatchbayCoreClient;
  const host = new AdapterProcess({ ...baseOptions, coreClient: core, retryDelayMs: 1 });
  await host.run(controller.signal);
  assert.equal(calls.filter((value) => value === "acknowledge").length, 1);
  assert.equal(terminalizationAttempts, 2);
  assert.ok(calls.indexOf("fail:2") < calls.indexOf("receive:9"), "pending terminalization must finish before reconnecting the stream");
  assert.deepEqual(transitions, [
    { state: OperationState.DELIVERED, failureCode: FailureCode.UNSPECIFIED },
    { state: OperationState.REJECTED, failureCode: FailureCode.UNSUPPORTED_COMMAND },
  ]);
  await host.dispose();
});

test("process starts exactly one delivery loop and one poller under the same abort scope", async () => {
  let subscriptions = 0;
  let polls = 0;
  let pollAborted = false;
  const core = {
    setDiagnostics() {}, async attach() { return create(EventIdSchema); },
    async acknowledgeDelivery() { return undefined; }, async rejectUnsupported() { return undefined; },
    async *receiveDeliveries(_cursor: bigint, signal: AbortSignal) {
      subscriptions += 1;
      await new Promise<void>((resolve) => signal.addEventListener("abort", () => resolve(), { once: true }));
    },
    async reportDiagnostic() { throw new Error("unused"); },
  } as unknown as PatchbayCoreClient;
  const composedPoller = {
    async run(signal: AbortSignal) {
      polls += 1;
      await new Promise<void>((resolve) => signal.addEventListener("abort", () => { pollAborted = true; resolve(); }, { once: true }));
    },
  } as unknown as TokenCommunePoller;
  const controller = new AbortController();
  const host = new AdapterProcess({ ...baseOptions, coreClient: core, poller: composedPoller });
  const running = host.run(controller.signal);
  await new Promise<void>((resolve) => setImmediate(resolve));
  assert.equal(subscriptions, 1);
  assert.equal(polls, 1);
  controller.abort();
  await running;
  assert.equal(pollAborted, true);
  await host.dispose();
});

test("a fatal poller exit aborts the held-open delivery sibling and rejects supervision", async () => {
  let deliveryAborted = false;
  const core = {
    setDiagnostics() {}, async attach() { return create(EventIdSchema); },
    async acknowledgeDelivery() { return undefined; }, async rejectUnsupported() { return undefined; },
    async *receiveDeliveries(_cursor: bigint, signal: AbortSignal) {
      await new Promise<void>((resolve) => signal.addEventListener("abort", () => { deliveryAborted = true; resolve(); }, { once: true }));
    },
    async reportDiagnostic() { throw new Error("unused"); },
  } as unknown as PatchbayCoreClient;
  const fatalPoller = { async run() { throw new Error("projection invariant failed"); } } as unknown as TokenCommunePoller;
  const host = new AdapterProcess({ ...baseOptions, coreClient: core, poller: fatalPoller });
  await assert.rejects(host.run(), /projection invariant failed/);
  assert.equal(deliveryAborted, true);
  await host.dispose();
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
