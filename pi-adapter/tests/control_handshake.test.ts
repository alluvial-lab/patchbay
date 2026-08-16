import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
import test from "node:test";
import { PiControlHandshakeFailure } from "@patchbay/contracts";
import {
  PATCHBAY_CONTROL_HANDSHAKE_COMMAND,
  PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE,
} from "../extensions/patchbay-control.js";
import {
  PiControlHandshakeError,
  performPiControlHandshake,
  piControlExtensionProfile,
  type PiControlRpc,
} from "../src/control_handshake.js";

const cwd = process.cwd();
const extensionPath = join(process.cwd(), "extensions", "patchbay-control.ts");
const launchNonce = Buffer.alloc(32, 1).toString("base64url");
const extensionEpoch = Buffer.alloc(16, 2).toString("base64url");
const sessionId = "pi-session-control";
const sessionFile = "/adapter-local/sessions/control.jsonl";

interface RpcOverrides {
  commandSource?: string;
  commandPath?: string;
  promptSuccess?: boolean;
  marker?: Partial<Marker> | null;
  leafId?: string | null;
  stateSessionId?: string;
  stateSessionFile?: string;
  statsSessionId?: string;
  statsSessionFile?: string;
}

interface Marker {
  challenge: string;
  launchNonce: string;
  extensionEpoch: string;
  cwd: string;
  sessionId: string;
  sessionFile: string;
}

test("challenged control handshake proves extension source, cwd, and RPC session identity", async () => {
  const result = await performPiControlHandshake(baseOptions(fakeRpc()));
  assert.equal(result.launchNonce, launchNonce);
  assert.equal(result.extensionEpoch, extensionEpoch);
  assert.equal(result.cwd, cwd);
  assert.equal(result.sessionId, sessionId);
  assert.equal(result.sessionFile, sessionFile);
  assert.equal(result.markerEntryId, "marker0001");
  assert.match(result.challenge, /^[A-Za-z0-9_-]{43}$/u);

  const profile = piControlExtensionProfile();
  assert.equal(profile.handshakeCommand, PATCHBAY_CONTROL_HANDSHAKE_COMMAND);
  assert.equal(profile.handshakeCustomType, PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE);
  assert.equal(profile.supportedSessionVersion, 3);
});

test("wrong initialized cwd cannot pass with correct generic RPC path and id", async () => {
  const wrongCwd = "/";
  await rejectsWith(
    performPiControlHandshake(
      baseOptions(fakeRpc({ marker: { cwd: wrongCwd } })),
    ),
    PiControlHandshakeFailure.CWD_MISMATCH,
    [wrongCwd, sessionFile],
  );
});

test("stale challenge, launch nonce, and extension epoch are rejected", async () => {
  await rejectsWith(
    performPiControlHandshake(
      baseOptions(fakeRpc({ marker: { challenge: Buffer.alloc(32, 9).toString("base64url") } })),
    ),
    PiControlHandshakeFailure.CHALLENGE_MISMATCH,
  );
  await rejectsWith(
    performPiControlHandshake(
      baseOptions(fakeRpc({ marker: { launchNonce: Buffer.alloc(32, 8).toString("base64url") } })),
    ),
    PiControlHandshakeFailure.LAUNCH_NONCE_MISMATCH,
  );
  await rejectsWith(
    performPiControlHandshake(
      baseOptions(fakeRpc(), { requiredExtensionEpoch: Buffer.alloc(16, 7).toString("base64url") }),
    ),
    PiControlHandshakeFailure.EXTENSION_EPOCH_MISMATCH,
  );
  await rejectsWith(
    performPiControlHandshake(
      baseOptions(fakeRpc(), { previousExtensionEpoch: extensionEpoch }),
    ),
    PiControlHandshakeFailure.EXTENSION_EPOCH_MISMATCH,
  );
});

test("prompt success or marker presence alone is never proof", async () => {
  await rejectsWith(
    performPiControlHandshake(baseOptions(fakeRpc({ marker: null }))),
    PiControlHandshakeFailure.MARKER_MISSING,
  );
  await rejectsWith(
    performPiControlHandshake(
      baseOptions(fakeRpc({ stateSessionFile: "/adapter-local/sessions/other.jsonl" })),
    ),
    PiControlHandshakeFailure.SESSION_FILE_MISMATCH,
    [sessionFile],
  );
  await rejectsWith(
    performPiControlHandshake(baseOptions(fakeRpc({ leafId: "older-entry" }))),
    PiControlHandshakeFailure.MARKER_NOT_CURRENT_LEAF,
  );
});

test("command discovery requires the adapter-owned extension source", async () => {
  await rejectsWith(
    performPiControlHandshake(baseOptions(fakeRpc({ commandSource: "prompt" }))),
    PiControlHandshakeFailure.COMMAND_SOURCE_MISMATCH,
  );
  await rejectsWith(
    performPiControlHandshake(baseOptions(fakeRpc({ commandPath: fileURLToPath(import.meta.url) }))),
    PiControlHandshakeFailure.COMMAND_SOURCE_MISMATCH,
  );
});

function baseOptions(
  rpc: PiControlRpc,
  overrides: Partial<Parameters<typeof performPiControlHandshake>[0]> = {},
): Parameters<typeof performPiControlHandshake>[0] {
  return {
    rpc,
    launchNonce,
    expectedProjectCwd: cwd,
    expectedExtensionPath: extensionPath,
    maxEntryPolls: 1,
    pollIntervalMs: 0,
    randomBytes: () => Buffer.alloc(32, 3),
    sleep: async () => undefined,
    ...overrides,
  };
}

function fakeRpc(overrides: RpcOverrides = {}): PiControlRpc {
  let challenge = "";
  return {
    async getCommands() {
      return [
        {
          name: PATCHBAY_CONTROL_HANDSHAKE_COMMAND,
          source: overrides.commandSource ?? "extension",
          path: overrides.commandPath ?? extensionPath,
        },
      ];
    },
    async prompt(message) {
      challenge = message.split(" ").at(-1) ?? "";
      return { success: overrides.promptSuccess ?? true };
    },
    async getEntries() {
      if (overrides.marker === null) return { entries: [], leafId: null };
      const marker: Marker = {
        challenge,
        launchNonce,
        extensionEpoch,
        cwd,
        sessionId,
        sessionFile,
        ...overrides.marker,
      };
      return {
        entries: [
          {
            type: "custom",
            id: "marker0001",
            parentId: null,
            timestamp: "2026-08-12T00:00:00.000Z",
            customType: PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE,
            data: marker,
          },
        ],
        leafId: overrides.leafId === undefined ? "marker0001" : overrides.leafId,
      };
    },
    async getState() {
      return {
        sessionId: overrides.stateSessionId ?? sessionId,
        sessionFile: overrides.stateSessionFile ?? sessionFile,
      };
    },
    async getSessionStats() {
      return {
        sessionId: overrides.statsSessionId ?? sessionId,
        sessionFile: overrides.statsSessionFile ?? sessionFile,
      };
    },
  };
}

async function rejectsWith(
  promise: Promise<unknown>,
  code: PiControlHandshakeFailure,
  forbiddenFragments: readonly string[] = [],
): Promise<void> {
  await assert.rejects(promise, (error: unknown) => {
    assert.ok(error instanceof PiControlHandshakeError);
    assert.equal(error.code, code);
    for (const fragment of forbiddenFragments) {
      assert.equal(error.message.includes(fragment), false, "failure is redacted");
    }
    return true;
  });
}
