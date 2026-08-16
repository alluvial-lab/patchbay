import assert from "node:assert/strict";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import { OperationKind, OperationSchema, type Operation } from "@patchbay/contracts";
import {
  AgentSession,
  SessionManager,
  type AgentSessionEvent,
  type AgentSessionEventListener,
} from "@earendil-works/pi-coding-agent";
import {
  createFauxCore,
  fauxAssistantMessage,
  fauxToolCall,
} from "@earendil-works/pi-ai/providers/faux";
import { AgentSessionRuntimeFixture } from "../src/pi_session.js";
import {
  createOfflineFixtureServices,
  createOfflineModelRuntime,
} from "./offline_agent_fixture.js";
import type { TranscriptEvent } from "../src/transcript_event.js";

const cwd = process.cwd();

test("real AgentSession prompt emits transcript events and honors the approval gate", async () => {
  const provider = "patchbay-faux";
  const faux = createFauxCore({ provider, api: provider, tokensPerSecond: 0 });
  faux.setResponses([
    fauxAssistantMessage(fauxToolCall("read", { path: "README.md" }, { id: "tool-1" }), {
      stopReason: "toolUse",
      timestamp: 10,
    }),
    fauxAssistantMessage("approval was enforced", { timestamp: 11 }),
  ]);

  const modelRuntime = await createOfflineModelRuntime();
  const model = faux.getModel();
  modelRuntime.registerProvider(provider, {
    name: "Patchbay test provider",
    apiKey: "test-key",
    baseUrl: "http://localhost:0",
    api: model.api,
    streamSimple: faux.streamSimple,
    models: [
      {
        id: model.id,
        name: model.name,
        api: model.api,
        baseUrl: "http://localhost:0",
        reasoning: model.reasoning,
        input: model.input,
        cost: model.cost,
        contextWindow: model.contextWindow,
        maxTokens: model.maxTokens,
      },
    ],
  });

  const liveHookListeners: AgentSessionEventListener[] = [];
  const originalSubscribe = AgentSession.prototype.subscribe;
  AgentSession.prototype.subscribe = function (listener) {
    const unsubscribe = originalSubscribe.call(this, listener);
    liveHookListeners.push(listener);
    return unsubscribe;
  };
  const pi = await AgentSessionRuntimeFixture.create({
    cwd,
    runtimeSessionId: "runtime-1",
    generation: 1,
    model: `${provider}/${model.id}`,
    services: await createOfflineFixtureServices(
      cwd,
      modelRuntime,
      SessionManager.inMemory(cwd),
    ),
    tools: ["read"],
  });
  const observed: TranscriptEvent[] = [];
  pi.onTranscript((event) => observed.push(event));
  let approvals = 0;
  let denialGateStarted!: () => void;
  const denialGate = new Promise<void>((resolve) => {
    denialGateStarted = resolve;
  });
  pi.setApprovalHandler((request) => {
    approvals += 1;
    assert.equal(request.toolCallId, "tool-1");
    assert.equal(request.tool, "read");
    denialGateStarted();
    return new Promise<boolean>(() => undefined);
  });

  try {
    const deniedRun = pi.prompt("exercise the approval gate");
    await denialGate;
    pi.resolveApproval(approvalOperation(), false);
    await deniedRun;
    assert.equal(approvals, 1);
    assert.ok(observed.some((event) => event.kind === "user_confirmed"));
    assert.ok(
      observed.some(
        (event) => event.kind === "tool_requested" && event.toolCallId === "tool-1",
      ),
    );
    assert.ok(
      observed.some(
        (event) => event.kind === "assistant_committed" && event.text === "approval was enforced",
      ),
    );
    assert.ok(
      observed.some(
        (event) => event.kind === "tool_finished" && event.toolCallId === "tool-1" && event.error,
      ),
      "a delivered DENIED decision blocks the pending tool call",
    );
    faux.appendResponses([
      fauxAssistantMessage(
        fauxToolCall("read", { path: "../docs/VISION.md" }, { id: "tool-2" }),
        { stopReason: "toolUse", timestamp: 12 },
      ),
      fauxAssistantMessage("approved tool completed", { timestamp: 13 }),
    ]);
    let approvalGateStarted!: () => void;
    const approvalGate = new Promise<void>((resolve) => {
      approvalGateStarted = resolve;
    });
    pi.setApprovalHandler((request) => {
      approvals += 1;
      assert.equal(request.toolCallId, "tool-2");
      assert.equal(request.tool, "read");
      approvalGateStarted();
      return new Promise<boolean>(() => undefined);
    });
    const approvedRun = pi.prompt("approve this read");
    await approvalGate;
    pi.resolveApproval(approvalOperation(), true);
    await approvedRun;
    assert.equal(approvals, 2);
    assert.ok(
      observed.some(
        (event) => event.kind === "tool_finished" && event.toolCallId === "tool-2" && !event.error,
      ),
      "a delivered APPROVED decision allows the pending tool call",
    );

    assert.equal(pi.getState().idle, true);
    assert.ok((await pi.getEntries()).entries.length > 0);
    assert.ok((await pi.getAvailableModels()).some((candidate) => candidate.id === model.id));
    const initialHook = liveHookListeners.at(-1);
    assert.ok(initialHook, "PiSession subscribes to the live AgentSession hook");
    const duplicateHookEvent = duplicateLiveHookEvent("live-duplicate-entry");
    initialHook(duplicateHookEvent);
    initialHook(duplicateHookEvent);
    const initialLiveEvent = observed.find(
      (event) => event.kind === "user_confirmed" && event.messageId === "live-duplicate-entry",
    );
    assert.ok(initialLiveEvent, "the first live hook event reaches the transcript listener");
    assert.equal(
      observed.filter((event) => event.eventId === initialLiveEvent.eventId).length,
      1,
      "duplicate live Pi hooks with one stable event id are delivered once",
    );

    await pi.setModel(provider, model.id);
    await pi.setThinkingLevel("off");
    await assert.rejects(
      async () => (pi as unknown as { newSession(): Promise<number> }).newSession(),
      /newSession is not a function/,
      "the SDK fixture exposes no adapter-owned generation increment path",
    );
    await pi.cancel();
  } finally {
    await pi.dispose();
    AgentSession.prototype.subscribe = originalSubscribe;
  }
});

test("offline fixture rejects a missing injected catalog and auth boundary marker", async () => {
  const provider = "patchbay-offline-boundary";
  const faux = createFauxCore({ provider, api: provider, tokensPerSecond: 0 });
  const modelRuntime = await createOfflineModelRuntime();
  const model = faux.getModel();
  modelRuntime.registerProvider(provider, {
    name: "offline boundary provider",
    apiKey: "injected-only",
    baseUrl: "http://localhost:0",
    api: model.api,
    streamSimple: faux.streamSimple,
    models: [model],
  });
  const services = await createOfflineFixtureServices(cwd, modelRuntime);
  await assert.rejects(
    AgentSessionRuntimeFixture.create({
      cwd,
      runtimeSessionId: "runtime-ambient-rejected",
      generation: 1,
      model: `${provider}/${model.id}`,
      services: {
        ...services,
        modelCatalogAuthStub: { kind: "ambient" },
      } as unknown as typeof services,
      noTools: "all",
    }),
    /requires offline catalog\/auth stubs/,
  );
});

function duplicateLiveHookEvent(entryId: string): AgentSessionEvent {
  return {
    type: "entry_appended",
    entry: {
      type: "message",
      id: entryId,
      timestamp: new Date("2026-01-02T03:04:05.000Z").toISOString(),
      message: { role: "user", content: "live duplicate" },
    },
  } as unknown as AgentSessionEvent;
}

function approvalOperation(): Operation {
  return create(OperationSchema, { kind: OperationKind.APPROVAL_RESPONSE });
}

async function waitUntil(predicate: () => boolean): Promise<void> {
  const deadline = Date.now() + 2_000;
  while (!predicate()) {
    if (Date.now() >= deadline) throw new Error("condition was not reached");
    await new Promise<void>((resolve) => setImmediate(resolve));
  }
}
