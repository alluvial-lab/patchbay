import assert from "node:assert/strict";
import test from "node:test";
import { create, toBinary } from "@bufbuild/protobuf";
import {
  ApprovalDecision,
  ApprovalResponsePayloadSchema,
  OperationKind,
  OperationSchema,
  type Operation,
  PayloadContentType,
  PayloadEnvelopeSchema,
} from "@patchbay/contracts";
import { DeliveryTranslator, UnsupportedCommandError } from "../src/delivery.js";
import type { PiSession } from "../src/pi_session.js";
import { SessionRegistry } from "../src/session_registry.js";

const encoder = new TextEncoder();

test("DeliveryTranslator maps instruct/cancel/session-new and rejects spawn", async () => {
  const calls: string[] = [];
  const session = {
    runtimeSessionId: "runtime-1",
    prompt: async (text: string) => calls.push(`prompt:${text}`),
    cancel: async () => calls.push("cancel"),
    newSession: async () => {
      calls.push("new");
      return 2;
    },
  } as unknown as PiSession;
  const translator = new DeliveryTranslator();

  await translator.deliver(operation(OperationKind.INSTRUCT, "hello"), session);
  await translator.deliver(operation(OperationKind.CANCEL), session);
  const replaced = await translator.deliver(
    operation(OperationKind.SESSION_MANAGEMENT, JSON.stringify({ action: "new" })),
    session,
  );
  assert.deepEqual(calls, ["prompt:hello", "cancel", "new"]);
  assert.equal(replaced.sessionGenerationChanged, true);
  await assert.rejects(
    translator.deliver(operation(OperationKind.SPAWN), session),
    UnsupportedCommandError,
  );
});

test("DeliveryTranslator resolves committed approval decisions and rejects reserved/question responses", async () => {
  const resolutions: boolean[] = [];
  const session = {
    resolveApproval: async (_operation: Operation, approved: boolean) => {
      resolutions.push(approved);
    },
  } as unknown as PiSession;
  const translator = new DeliveryTranslator();

  await translator.deliver(approvalOperation(ApprovalDecision.APPROVED), session);
  await translator.deliver(approvalOperation(ApprovalDecision.DENIED), session);
  assert.deepEqual(resolutions, [true, false]);

  await assert.rejects(
    translator.deliver(approvalOperation(ApprovalDecision.RESERVED_ALLOW_ONCE), session),
    (error: unknown) =>
      error instanceof UnsupportedCommandError && error.failureCode === "unsupported_command",
  );
  await assert.rejects(
    translator.deliver(operation(OperationKind.ELICITATION_RESPONSE), session),
    UnsupportedCommandError,
  );
});

test("SessionRegistry owns complete runtime entries and observation wiring", async () => {
  const registry = new SessionRegistry();
  let transcriptListener: ((event: never) => void) | undefined;
  let unsubscribed = false;
  const session = {
    runtimeSessionId: "runtime-1",
    onTranscript(listener: (event: never) => void) {
      transcriptListener = listener;
      return () => {
        unsubscribed = true;
      };
    },
    async dispose() {},
  } as unknown as PiSession;
  const config = {
    runtimeSessionId: "runtime-1",
    deploymentScope: "machine-a",
    project: "patchbay",
    cwd: "/work/patchbay",
    name: "dynamic",
  };
  let observedEntryName = "";
  registry.register(config, session, (entry) => {
    observedEntryName = entry.name ?? "";
  });
  const entry = registry.resolve("runtime-1");
  assert.equal(entry?.session, session);
  assert.equal(entry?.deploymentScope, "machine-a");
  transcriptListener?.({} as never);
  assert.equal(observedEntryName, "dynamic");
  assert.throws(() => registry.register(config, session, () => undefined), /already registered/);
  await registry.dispose();
  assert.equal(unsubscribed, true);
});

function operation(kind: OperationKind, payload = ""): Operation {
  return create(OperationSchema, {
    kind,
    payload: create(PayloadEnvelopeSchema, {
      payload: encoder.encode(payload),
      contentType: PayloadContentType.TEXT_UTF8,
    }),
  });
}

function approvalOperation(decision: ApprovalDecision): Operation {
  return create(OperationSchema, {
    kind: OperationKind.APPROVAL_RESPONSE,
    payload: create(PayloadEnvelopeSchema, {
      payload: toBinary(
        ApprovalResponsePayloadSchema,
        create(ApprovalResponsePayloadSchema, { decision }),
      ),
      contentType: PayloadContentType.PROTOBUF,
    }),
  });
}
