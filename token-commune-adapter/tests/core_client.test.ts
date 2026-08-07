import assert from "node:assert/strict";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import { Code, ConnectError } from "@connectrpc/connect";
import {
  AttachResultSchema, CommandIdSchema, EventIdSchema, FailureCode, LsnSchema,
  ObservationResultSchema, OperationKind, OperationSchema, TargetScopeKind,
  TargetScopeSchema, type AttachRequest, type ObservationRequest,
} from "@patchbay/contracts";
import { PatchbayCoreClient } from "../src/core_client.js";

function event(value: bigint) { return create(EventIdSchema, { authorityDomainId: { value: "default" }, lsn: create(LsnSchema, { value }) }); }
function operation() {
  return create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: "command-1" }), kind: OperationKind.QUERY,
    targetScope: create(TargetScopeSchema, { kind: TargetScopeKind.ADAPTER, adapterId: { value: "token-commune" } }),
  });
}

test("attach sends exact identity, evidence, generation, and honest manifest and requires a token", async () => {
  let request: AttachRequest | undefined;
  let token: string | undefined;
  const fake = {
    attach: async (value: AttachRequest) => { request = value; token = "token-1"; return create(AttachResultSchema, { accepted: true, attachEventId: event(1n) }); },
    ingestObservation: async () => create(ObservationResultSchema),
    reportDiagnostics: async () => { throw new Error("unused"); },
    receiveDeliveries: async function* () {},
  } as any;
  const client = new PatchbayCoreClient({
    coreAddress: "http://core", adapterId: "token-commune", authorityDomainId: "default", attachmentEvidence: "attach-secret",
    testClient: fake, testAttachmentToken: () => token,
  });
  const attached = await client.attach(3);
  assert.equal(attached.lsn?.value, 1n);
  assert.equal(request?.registration?.adapterId?.value, "token-commune");
  assert.equal(request?.registration?.endpointId?.value, "token-commune-endpoint");
  assert.equal(request?.registration?.authorityDomainId?.value, "default");
  assert.equal(request?.registration?.adapterGeneration?.value, 3n);
  assert.equal(new TextDecoder().decode(request?.attachmentEvidence), "attach-secret");
  assert.deepEqual(request?.registration?.capability?.supportedOperationKinds, []);
  assert.equal(request?.registration?.capability?.resourceCapabilities.length, 2);

  const missingToken = new PatchbayCoreClient({
    coreAddress: "http://core", adapterId: "token-commune", authorityDomainId: "default", attachmentEvidence: "attach-secret", testClient: fake,
  });
  await assert.rejects(missingToken.attach(1), /missing the adapter attachment token/);
});

test("concurrent unauthenticated calls single-flight same-generation reattach and retry", async () => {
  let token: string | undefined;
  let attaches = 0;
  let initialFailures = 2;
  const observations: ObservationRequest[] = [];
  const fake = {
    attach: async () => {
      attaches += 1;
      await new Promise<void>((resolve) => setTimeout(resolve, 5));
      token = `token-${attaches}`;
      return create(AttachResultSchema, { accepted: true, attachEventId: event(BigInt(attaches)) });
    },
    ingestObservation: async (request: ObservationRequest) => {
      observations.push(request);
      if (initialFailures > 0) { initialFailures -= 1; throw new ConnectError("expired", Code.Unauthenticated); }
      return create(ObservationResultSchema, { eventId: event(10n) });
    },
    reportDiagnostics: async () => { throw new Error("unused"); },
    receiveDeliveries: async function* () {},
  } as any;
  const client = new PatchbayCoreClient({
    coreAddress: "http://core", adapterId: "token-commune", authorityDomainId: "default", attachmentEvidence: "attach-secret",
    testClient: fake, testAttachmentToken: () => token,
  });
  await client.attach(7);
  await Promise.all([client.acknowledgeDelivery(operation()), client.failUnsupported(operation())]);
  assert.equal(attaches, 2, "one initial attach plus one shared refresh");
  assert.equal(observations.length, 4, "two failed attempts and two retries");
  const unsupported = observations.map((item) => item.observation.value).find((item: any) => item?.failureCode === FailureCode.UNSUPPORTED_COMMAND);
  assert.ok(unsupported);
});

test("diagnostic reporting bypasses attachment refresh", async () => {
  let reportCalls = 0;
  let attachCalls = 0;
  let token: string | undefined;
  const fake = {
    attach: async () => { attachCalls += 1; token = "token"; return create(AttachResultSchema, { accepted: true, attachEventId: event(1n) }); },
    ingestObservation: async () => create(ObservationResultSchema),
    reportDiagnostics: async () => { reportCalls += 1; throw new ConnectError("no auth", Code.Unauthenticated); },
    receiveDeliveries: async function* () {},
  } as any;
  const client = new PatchbayCoreClient({ coreAddress: "http://core", adapterId: "token-commune", authorityDomainId: "default", attachmentEvidence: "attach-secret", testClient: fake, testAttachmentToken: () => token });
  await client.attach(1);
  await assert.rejects(client.reportDiagnostic({} as any));
  assert.equal(reportCalls, 1);
  assert.equal(attachCalls, 1);
});
