import assert from "node:assert/strict";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import {
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

test("SessionRegistry keys sessions by runtime identity and rejects collisions", () => {
  const registry = new SessionRegistry();
  const session = { runtimeSessionId: "runtime-1", dispose() {} } as unknown as PiSession;
  registry.register("runtime-1", session);
  assert.equal(registry.resolve("runtime-1"), session);
  assert.throws(() => registry.register("runtime-1", session), /already registered/);
  registry.dispose();
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
