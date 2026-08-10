import assert from "node:assert/strict";
import { once } from "node:events";
import { createServer } from "node:http2";
import type { AddressInfo } from "node:net";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import { Code, ConnectError } from "@connectrpc/connect";
import { connectNodeAdapter } from "@connectrpc/connect-node";
import {
  AdapterControlService,
  AttachResultSchema,
  AuthorityDomainIdSchema,
  EventIdSchema,
  GenerationSchema,
  LsnSchema,
  ObservationResultSchema,
  SessionActivityState,
  SessionConnectivityState,
  type SessionReport,
} from "@patchbay/contracts";
import { PatchbayCoreClient, type SessionIdentity } from "../src/core_client.js";
import type { SessionReportOrder } from "../src/session_report_sequencer.js";

const attachmentTokenHeader = "x-patchbay-adapter-attachment-token";

test("PatchbayCoreClient reattach retry reuses the exact immutable session report cursor", async () => {
  const attachGenerations: bigint[] = [];
  const attemptedReports: SessionReport[] = [];
  let attachment = 0;
  const handler = connectNodeAdapter({
    routes(router) {
      router.service(AdapterControlService, {
        attach(request, context) {
          attachGenerations.push(request.registration?.adapterGeneration?.value ?? 0n);
          attachment += 1;
          context.responseHeader.set(attachmentTokenHeader, `attachment-${attachment}`);
          return create(AttachResultSchema, {
            accepted: true,
            attachEventId: create(EventIdSchema, {
              authorityDomainId: create(AuthorityDomainIdSchema, { value: "authority-test" }),
              lsn: create(LsnSchema, { value: BigInt(attachment) }),
            }),
          });
        },
        ingestObservation(request) {
          assert.equal(request.observation.case, "sessionReport");
          attemptedReports.push(structuredClone(request.observation.value));
          if (attemptedReports.length === 1) {
            throw new ConnectError("attachment expired", Code.Unauthenticated);
          }
          return create(ObservationResultSchema, {
            eventId: create(EventIdSchema, {
              authorityDomainId: create(AuthorityDomainIdSchema, { value: "authority-test" }),
              lsn: create(LsnSchema, { value: 3n }),
            }),
          });
        },
      });
    },
  });
  const server = createServer(handler);
  server.listen(0, "127.0.0.1");
  await once(server, "listening");
  const address = server.address() as AddressInfo;

  const client = new PatchbayCoreClient({
    coreAddress: `http://127.0.0.1:${address.port}`,
    adapterId: "pi",
    authorityDomainId: "authority-test",
    attachmentEvidence: "adapter-test-secret",
  });
  const identity: SessionIdentity = Object.freeze({
    runtimeSessionId: "runtime-1",
    deploymentScope: "machine-a",
    generation: 4,
    project: "patchbay",
    cwd: "/work/patchbay",
    name: "main",
    model: "provider/model-a",
  });
  const sourceOrder: SessionReportOrder = Object.freeze({
    adapterGeneration: 7,
    revision: 9n,
  });

  try {
    await client.attach(7);
    await client.reportSession(
      identity,
      SessionActivityState.WORKING,
      SessionConnectivityState.LIVE,
      sourceOrder,
    );

    assert.deepEqual(attachGenerations, [7n, 7n]);
    assert.equal(attemptedReports.length, 2);
    assert.deepEqual(
      attemptedReports[1],
      attemptedReports[0],
      "reattach must retry the captured report rather than allocate or reread state",
    );
    assert.deepEqual(attemptedReports[1]?.sourceCursor, {
      $typeName: "patchbay.SessionReportSourceCursor",
      adapterGeneration: create(GenerationSchema, { value: 7n }),
      revision: 9n,
    });
    assert.equal(attemptedReports[1]?.model, "provider/model-a");
  } finally {
    server.close();
    await once(server, "close");
  }
});
