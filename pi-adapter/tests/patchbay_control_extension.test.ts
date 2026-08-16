import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test, { after } from "node:test";
import { PiReloadableResourceKind } from "@patchbay/contracts";
import type {
  ExtensionAPI,
  ExtensionCommandContext,
  ExtensionContext,
  SessionEntry,
} from "@earendil-works/pi-coding-agent";
import patchbayControlExtension, {
  PATCHBAY_CONTROL_HANDSHAKE_COMMAND,
  PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE,
  PATCHBAY_CONTROL_RELOAD_COMMAND,
  PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE,
  PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE,
  PATCHBAY_LAUNCH_NONCE_ENV,
} from "../extensions/patchbay-control.js";

const launchNonce = Buffer.alloc(32, 4).toString("base64url");
const challenge = Buffer.alloc(32, 5).toString("base64url");
const reloadNonce = Buffer.alloc(32, 6).toString("base64url");
const contextRoots: string[] = [];
after(() => {
  for (const root of contextRoots) rmSync(root, { recursive: true, force: true });
});

test("extension handshake command appends bounded initialized context evidence", async () => {
  await withLaunchNonce(async () => {
    const harness = createExtensionHarness();
    patchbayControlExtension(harness.api);
    const handler = harness.commands.get(PATCHBAY_CONTROL_HANDSHAKE_COMMAND);
    assert.ok(handler);
    const ctx = context([]);
    await handler(challenge, ctx);
    assert.equal(harness.appended.length, 1);
    const entry = harness.appended[0];
    assert.equal(entry?.customType, PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE);
    assert.deepEqual(entry?.data, {
      challenge,
      launchNonce,
      extensionEpoch: (entry?.data as { extensionEpoch: string }).extensionEpoch,
      cwd: process.cwd(),
      sessionId: "extension-session",
      sessionFile: ctx.sessionManager.getSessionFile(),
    });
    assert.match(
      (entry?.data as { extensionEpoch: string }).extensionEpoch,
      /^[A-Za-z0-9_-]{22}$/u,
    );
    await assert.rejects(handler("short", context([])), /invalid Patchbay handshake challenge/u);
  });
});

test("reload markers are bounded and completion comes from a new extension epoch", async () => {
  await withLaunchNonce(async () => {
    const oldHarness = createExtensionHarness();
    patchbayControlExtension(oldHarness.api);
    const handshake = oldHarness.commands.get(PATCHBAY_CONTROL_HANDSHAKE_COMMAND);
    const reload = oldHarness.commands.get(PATCHBAY_CONTROL_RELOAD_COMMAND);
    assert.ok(handshake);
    assert.ok(reload);
    await handshake(challenge, context([]));
    const oldEpoch = (oldHarness.appended[0]?.data as { extensionEpoch: string }).extensionEpoch;
    const argument = encodeReloadArgument({
      commandId: "operation-1",
      nonce: reloadNonce,
      priorExtensionEpoch: oldEpoch,
      resources: [
        PiReloadableResourceKind.EXTENSION_ENTRYPOINT,
        PiReloadableResourceKind.SKILL,
      ],
    });
    const oldContext = context([]);
    await reload(argument, oldContext);
    assert.equal(oldContext.reloadCalls, 1);
    const request = oldHarness.appended.at(-1);
    assert.equal(request?.customType, PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE);

    const requestEntry = {
      type: "custom",
      id: "request0001",
      parentId: null,
      timestamp: "2026-08-12T00:00:00.000Z",
      customType: PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE,
      data: request?.data,
    } as SessionEntry;
    const newHarness = createExtensionHarness();
    patchbayControlExtension(newHarness.api);
    const sessionStart = newHarness.sessionStartHandlers.at(-1);
    assert.ok(sessionStart);
    await sessionStart({ type: "session_start", reason: "reload" }, context([requestEntry]));
    const completion = newHarness.appended.at(-1);
    assert.equal(completion?.customType, PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE);
    assert.equal((completion?.data as { requestEntryId: string }).requestEntryId, "request0001");
    assert.equal((completion?.data as { priorExtensionEpoch: string }).priorExtensionEpoch, oldEpoch);
    assert.notEqual((completion?.data as { extensionEpoch: string }).extensionEpoch, oldEpoch);
  });
});

test("reload command refuses an in-memory-only request marker before ctx.reload", async () => {
  await withLaunchNonce(async () => {
    const harness = createExtensionHarness();
    patchbayControlExtension(harness.api);
    const handshake = harness.commands.get(PATCHBAY_CONTROL_HANDSHAKE_COMMAND);
    const reload = harness.commands.get(PATCHBAY_CONTROL_RELOAD_COMMAND);
    assert.ok(handshake);
    assert.ok(reload);
    const ctx = context([]);
    await handshake(challenge, ctx);
    const epoch = (harness.appended[0]?.data as { extensionEpoch: string }).extensionEpoch;
    ctx.persistAppends = false;
    await assert.rejects(
      reload(encodeReloadArgument({
        commandId: "operation-1",
        nonce: reloadNonce,
        priorExtensionEpoch: epoch,
        resources: [PiReloadableResourceKind.EXTENSION_ENTRYPOINT],
      }), ctx),
      /not materialized/u,
    );
    assert.equal(ctx.reloadCalls, 0);
  });
});

test("reload command rejects stale epochs, duplicate resources, and malformed payloads before effect", async () => {
  await withLaunchNonce(async () => {
    const harness = createExtensionHarness();
    patchbayControlExtension(harness.api);
    const reload = harness.commands.get(PATCHBAY_CONTROL_RELOAD_COMMAND);
    assert.ok(reload);
    const ctx = context([]);
    await assert.rejects(
      reload(
        encodeReloadArgument({
          commandId: "operation-1",
          nonce: reloadNonce,
          priorExtensionEpoch: Buffer.alloc(16, 9).toString("base64url"),
          resources: [PiReloadableResourceKind.SKILL],
        }),
        ctx,
      ),
      /invalid Patchbay reload argument/u,
    );
    await assert.rejects(
      reload(
        encodeReloadArgument({
          commandId: "operation-1",
          nonce: reloadNonce,
          priorExtensionEpoch: Buffer.alloc(16, 9).toString("base64url"),
          resources: [PiReloadableResourceKind.SKILL, PiReloadableResourceKind.SKILL],
        }),
        ctx,
      ),
      /invalid Patchbay reload argument/u,
    );
    assert.equal(ctx.reloadCalls, 0);
    assert.equal(harness.appended.length, 0);
  });
});

interface Appended {
  customType: string;
  data: unknown;
}

function createExtensionHarness(): {
  api: ExtensionAPI;
  commands: Map<string, (args: string, ctx: ExtensionCommandContext) => Promise<void>>;
  sessionStartHandlers: Array<(event: { type: "session_start"; reason: "reload" }, ctx: ExtensionContext) => Promise<void> | void>;
  appended: Appended[];
} {
  const commands = new Map<string, (args: string, ctx: ExtensionCommandContext) => Promise<void>>();
  let activeContext: TestExtensionContext | undefined;
  const sessionStartHandlers: Array<
    (event: { type: "session_start"; reason: "reload" }, ctx: ExtensionContext) => Promise<void> | void
  > = [];
  const appended: Appended[] = [];
  const api = {
    registerCommand(
      name: string,
      options: { handler: (args: string, ctx: ExtensionCommandContext) => Promise<void> },
    ) {
      commands.set(name, async (args, ctx) => {
        activeContext = ctx as TestExtensionContext;
        try {
          await options.handler(args, ctx);
        } finally {
          activeContext = undefined;
        }
      });
    },
    appendEntry(customType: string, data: unknown) {
      appended.push({ customType, data });
      activeContext?.appendCustom(customType, data);
    },
    on(event: string, handler: unknown) {
      if (event === "session_start") {
        const typed = handler as (
          event: { type: "session_start"; reason: "reload" },
          ctx: ExtensionContext,
        ) => Promise<void> | void;
        sessionStartHandlers.push(async (startEvent, ctx) => {
          activeContext = ctx as TestExtensionContext;
          try {
            await typed(startEvent, ctx);
          } finally {
            activeContext = undefined;
          }
        });
      }
    },
  } as unknown as ExtensionAPI;
  return { api, commands, sessionStartHandlers, appended };
}

interface TestExtensionContext extends ExtensionCommandContext {
  reloadCalls: number;
  persistAppends: boolean;
  appendCustom(customType: string, data: unknown): void;
}

function context(entries: readonly SessionEntry[]): TestExtensionContext {
  const root = mkdtempSync(join(tmpdir(), "patchbay-control-extension-"));
  contextRoots.push(root);
  const sessionFile = join(root, "extension-session.jsonl");
  const mutableEntries = [...entries];
  let sequence = 0;
  const persist = () => {
    const header = {
      type: "session",
      version: 3,
      id: "extension-session",
      timestamp: "2026-08-12T00:00:00.000Z",
      cwd: process.cwd(),
    };
    writeFileSync(
      sessionFile,
      `${[header, ...mutableEntries].map((entry) => JSON.stringify(entry)).join("\n")}\n`,
      { mode: 0o600 },
    );
  };
  persist();
  const value = {
    cwd: process.cwd(),
    sessionManager: {
      getSessionId: () => "extension-session",
      getSessionFile: () => sessionFile,
      getEntries: () => [...mutableEntries],
    },
    reloadCalls: 0,
    persistAppends: true,
    appendCustom(customType: string, data: unknown) {
      sequence += 1;
      mutableEntries.push({
        type: "custom",
        id: `control${sequence.toString().padStart(3, "0")}`,
        parentId: mutableEntries.at(-1)?.id ?? null,
        timestamp: new Date(Date.UTC(2026, 7, 12, 0, 0, sequence)).toISOString(),
        customType,
        data,
      } as SessionEntry);
      if (value.persistAppends) persist();
    },
    async reload() {
      value.reloadCalls += 1;
    },
  };
  return value as unknown as TestExtensionContext;
}

function encodeReloadArgument(value: object): string {
  return Buffer.from(JSON.stringify(value), "utf8").toString("base64url");
}

async function withLaunchNonce(action: () => Promise<void>): Promise<void> {
  const prior = process.env[PATCHBAY_LAUNCH_NONCE_ENV];
  process.env[PATCHBAY_LAUNCH_NONCE_ENV] = launchNonce;
  try {
    await action();
  } finally {
    if (prior === undefined) delete process.env[PATCHBAY_LAUNCH_NONCE_ENV];
    else process.env[PATCHBAY_LAUNCH_NONCE_ENV] = prior;
  }
}
