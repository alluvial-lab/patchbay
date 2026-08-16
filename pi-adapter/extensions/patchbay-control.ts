import { randomBytes } from "node:crypto";
import { lstatSync, readFileSync } from "node:fs";
import {
  PiReloadableResourceKind,
  type PiControlHandshakeMarker,
  type PiReloadCompletionMarker,
  type PiReloadRequestMarker,
} from "@patchbay/contracts";
import type {
  ExtensionAPI,
  ExtensionCommandContext,
  ExtensionContext,
} from "@earendil-works/pi-coding-agent";

export const PATCHBAY_CONTROL_PROFILE_VERSION = "patchbay.control.profile.v1";
export const PATCHBAY_CONTROL_HANDSHAKE_COMMAND = "patchbay-control-handshake";
export const PATCHBAY_CONTROL_RELOAD_COMMAND = "patchbay-control-reload";
export const PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE = "patchbay.control.handshake.v1";
export const PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE = "patchbay.control.reload-request.v1";
export const PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE =
  "patchbay.control.reload-completion.v1";
export const PATCHBAY_CONTROL_CHALLENGE_BYTES = 32;
export const PATCHBAY_CONTROL_EXTENSION_EPOCH_BYTES = 16;
export const PATCHBAY_SUPPORTED_SESSION_VERSION = 3;
export const PATCHBAY_LAUNCH_NONCE_ENV = "PATCHBAY_LAUNCH_NONCE";

const BASE64URL_PATTERN = /^[A-Za-z0-9_-]+$/u;
const BOUNDED_ID_PATTERN = /^[A-Za-z0-9._:-]{1,128}$/u;
const MAX_LOCAL_PATH_BYTES = 4_096;
const MAX_RELOAD_ARGUMENT_BYTES = 4_096;
const MAX_RELOAD_RESOURCES = 16;
const REQUIRED_NONCE_LENGTH = base64UrlLength(PATCHBAY_CONTROL_CHALLENGE_BYTES);
const REQUIRED_EPOCH_LENGTH = base64UrlLength(PATCHBAY_CONTROL_EXTENSION_EPOCH_BYTES);
const ADMITTED_RELOAD_RESOURCES = new Set<PiReloadableResourceKind>([
  PiReloadableResourceKind.EXTENSION_ENTRYPOINT,
  PiReloadableResourceKind.SKILL,
  PiReloadableResourceKind.PROMPT,
  PiReloadableResourceKind.THEME,
  PiReloadableResourceKind.CONTEXT_FILE,
]);

type MarkerData<T> = Omit<T, "$typeName">;
export type PiControlHandshakeMarkerData = MarkerData<PiControlHandshakeMarker>;
export type PiReloadRequestMarkerData = MarkerData<PiReloadRequestMarker>;
export type PiReloadCompletionMarkerData = MarkerData<PiReloadCompletionMarker>;

interface ReloadArgument {
  commandId: string;
  nonce: string;
  priorExtensionEpoch: string;
  resources: PiReloadableResourceKind[];
}

export default function patchbayControlExtension(pi: ExtensionAPI): void {
  const launchNonce = requireBoundedBase64Url(
    process.env[PATCHBAY_LAUNCH_NONCE_ENV],
    REQUIRED_NONCE_LENGTH,
    "launch nonce",
  );
  const extensionEpoch = randomBytes(PATCHBAY_CONTROL_EXTENSION_EPOCH_BYTES).toString("base64url");

  pi.registerCommand(PATCHBAY_CONTROL_HANDSHAKE_COMMAND, {
    description: "Emit a challenged Patchbay control marker",
    handler: async (args, ctx) => {
      const challenge = requireBoundedBase64Url(
        args.trim(),
        REQUIRED_NONCE_LENGTH,
        "handshake challenge",
      );
      const marker = handshakeMarker(challenge, launchNonce, extensionEpoch, ctx);
      pi.appendEntry<PiControlHandshakeMarkerData>(
        PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE,
        marker,
      );
    },
  });

  pi.registerCommand(PATCHBAY_CONTROL_RELOAD_COMMAND, {
    description: "Reload the bounded Pi resource set under Patchbay control",
    handler: async (args, ctx) => {
      const request = parseReloadArgument(args, extensionEpoch);
      const priorEntryIds = new Set(ctx.sessionManager.getEntries().map((entry) => entry.id));
      pi.appendEntry<PiReloadRequestMarkerData>(
        PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE,
        request,
      );
      const appended = ctx.sessionManager.getEntries().filter(
        (entry) =>
          !priorEntryIds.has(entry.id)
          && entry.type === "custom"
          && entry.customType === PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE
          && storedReloadRequestEquals(entry.data, request),
      );
      if (appended.length !== 1 || !appended[0]) {
        throw new Error("Patchbay reload request marker was not appended exactly once");
      }
      requireMaterializedReloadRequest(ctx, appended[0].id, request);
      await ctx.reload();
      return;
    },
  });

  pi.on("session_start", (event, ctx) => {
    if (event.reason !== "reload") return;
    appendReloadCompletion(pi, ctx, extensionEpoch);
  });
}

function handshakeMarker(
  challenge: string,
  launchNonce: string,
  extensionEpoch: string,
  ctx: ExtensionCommandContext,
): PiControlHandshakeMarkerData {
  const sessionId = requireBoundedText(ctx.sessionManager.getSessionId(), 128, "session id");
  const sessionFile = requireBoundedText(
    ctx.sessionManager.getSessionFile(),
    MAX_LOCAL_PATH_BYTES,
    "session file",
  );
  const cwd = requireBoundedText(ctx.cwd, MAX_LOCAL_PATH_BYTES, "cwd");
  return { challenge, launchNonce, extensionEpoch, cwd, sessionId, sessionFile };
}

function parseReloadArgument(args: string, extensionEpoch: string): PiReloadRequestMarkerData {
  const encoded = args.trim();
  if (
    encoded.length === 0 ||
    Buffer.byteLength(encoded) > MAX_RELOAD_ARGUMENT_BYTES ||
    !BASE64URL_PATTERN.test(encoded)
  ) {
    throw new Error("invalid Patchbay reload argument");
  }

  let value: unknown;
  try {
    value = JSON.parse(Buffer.from(encoded, "base64url").toString("utf8"));
  } catch {
    throw new Error("invalid Patchbay reload argument");
  }
  if (!isRecord(value) || !hasExactKeys(value, ["commandId", "nonce", "priorExtensionEpoch", "resources"])) {
    throw new Error("invalid Patchbay reload argument");
  }
  const commandId = value.commandId;
  const nonce = value.nonce;
  const priorExtensionEpoch = value.priorExtensionEpoch;
  const resources = value.resources;
  if (
    typeof commandId !== "string" ||
    !BOUNDED_ID_PATTERN.test(commandId) ||
    typeof nonce !== "string" ||
    !isBoundedBase64Url(nonce, REQUIRED_NONCE_LENGTH) ||
    typeof priorExtensionEpoch !== "string" ||
    !isBoundedBase64Url(priorExtensionEpoch, REQUIRED_EPOCH_LENGTH) ||
    priorExtensionEpoch !== extensionEpoch ||
    !Array.isArray(resources) ||
    resources.length === 0 ||
    resources.length > MAX_RELOAD_RESOURCES
  ) {
    throw new Error("invalid Patchbay reload argument");
  }
  const parsedResources: PiReloadableResourceKind[] = [];
  const seen = new Set<number>();
  for (const resource of resources) {
    if (
      typeof resource !== "number" ||
      !Number.isInteger(resource) ||
      !ADMITTED_RELOAD_RESOURCES.has(resource as PiReloadableResourceKind) ||
      seen.has(resource)
    ) {
      throw new Error("invalid Patchbay reload argument");
    }
    seen.add(resource);
    parsedResources.push(resource as PiReloadableResourceKind);
  }
  return { commandId, nonce, priorExtensionEpoch, resources: parsedResources };
}

function appendReloadCompletion(
  pi: ExtensionAPI,
  ctx: ExtensionContext,
  extensionEpoch: string,
): void {
  const entries = ctx.sessionManager.getEntries();
  const completedRequestIds = new Set(
    entries
      .map((entry) => {
        if (
          entry.type !== "custom" ||
          entry.customType !== PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE
        ) {
          return undefined;
        }
        const data = entry.data;
        return isRecord(data) && typeof data.requestEntryId === "string"
          ? data.requestEntryId
          : undefined;
      })
      .filter((id): id is string => id !== undefined),
  );
  for (let index = entries.length - 1; index >= 0; index -= 1) {
    const entry = entries[index];
    if (
      !entry ||
      entry.type !== "custom" ||
      entry.customType !== PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE ||
      completedRequestIds.has(entry.id)
    ) {
      continue;
    }
    const request = parseStoredReloadRequest(entry.data);
    if (!request || request.priorExtensionEpoch === extensionEpoch) return;
    const completion: PiReloadCompletionMarkerData = {
      commandId: request.commandId,
      nonce: request.nonce,
      requestEntryId: entry.id,
      priorExtensionEpoch: request.priorExtensionEpoch,
      extensionEpoch,
    };
    pi.appendEntry<PiReloadCompletionMarkerData>(
      PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE,
      completion,
    );
    return;
  }
}

function parseStoredReloadRequest(value: unknown): PiReloadRequestMarkerData | undefined {
  if (!isRecord(value) || !hasExactKeys(value, ["commandId", "nonce", "priorExtensionEpoch", "resources"])) {
    return undefined;
  }
  if (
    typeof value.commandId !== "string" ||
    !BOUNDED_ID_PATTERN.test(value.commandId) ||
    typeof value.nonce !== "string" ||
    !isBoundedBase64Url(value.nonce, REQUIRED_NONCE_LENGTH) ||
    typeof value.priorExtensionEpoch !== "string" ||
    !isBoundedBase64Url(value.priorExtensionEpoch, REQUIRED_EPOCH_LENGTH) ||
    !Array.isArray(value.resources) ||
    value.resources.length === 0 ||
    value.resources.length > MAX_RELOAD_RESOURCES ||
    value.resources.some(
      (resource) =>
        typeof resource !== "number" ||
        !Number.isInteger(resource) ||
        !ADMITTED_RELOAD_RESOURCES.has(resource as PiReloadableResourceKind),
    ) ||
    new Set(value.resources).size !== value.resources.length
  ) {
    return undefined;
  }
  return {
    commandId: value.commandId,
    nonce: value.nonce,
    priorExtensionEpoch: value.priorExtensionEpoch,
    resources: value.resources as PiReloadableResourceKind[],
  };
}

function requireMaterializedReloadRequest(
  ctx: ExtensionCommandContext,
  entryId: string,
  request: PiReloadRequestMarkerData,
): void {
  const sessionFile = requireBoundedText(
    ctx.sessionManager.getSessionFile(),
    MAX_LOCAL_PATH_BYTES,
    "session file",
  );
  try {
    const stats = lstatSync(sessionFile);
    if (!stats.isFile() || stats.isSymbolicLink() || stats.size <= 0 || stats.size > 64 * 1_024 * 1_024) {
      throw new Error("invalid materialization");
    }
    const bytes = readFileSync(sessionFile);
    if (bytes.at(-1) !== 0x0a) throw new Error("invalid framing");
    const matches = bytes
      .toString("utf8")
      .trimEnd()
      .split("\n")
      .flatMap((line) => {
        try {
          const value: unknown = JSON.parse(line);
          return isRecord(value)
            && value.type === "custom"
            && value.id === entryId
            && value.customType === PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE
            && storedReloadRequestEquals(value.data, request)
            ? [value]
            : [];
        } catch {
          return [];
        }
      });
    if (matches.length !== 1) throw new Error("missing durable request");
  } catch {
    throw new Error("Patchbay reload request marker is not materialized");
  }
}

function storedReloadRequestEquals(
  value: unknown,
  expected: PiReloadRequestMarkerData,
): boolean {
  const parsed = parseStoredReloadRequest(value);
  return parsed !== undefined
    && parsed.commandId === expected.commandId
    && parsed.nonce === expected.nonce
    && parsed.priorExtensionEpoch === expected.priorExtensionEpoch
    && parsed.resources.length === expected.resources.length
    && parsed.resources.every((resource, index) => resource === expected.resources[index]);
}

function requireBoundedBase64Url(
  value: string | undefined,
  exactLength: number,
  label: string,
): string {
  if (!value || !isBoundedBase64Url(value, exactLength)) {
    throw new Error(`invalid Patchbay ${label}`);
  }
  return value;
}

function isBoundedBase64Url(value: string, exactLength: number): boolean {
  return value.length === exactLength && BASE64URL_PATTERN.test(value);
}

function requireBoundedText(
  value: string | undefined,
  maxBytes: number,
  label: string,
): string {
  if (!value || value.includes("\0") || Buffer.byteLength(value) > maxBytes) {
    throw new Error(`invalid Patchbay ${label}`);
  }
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function hasExactKeys(value: Record<string, unknown>, keys: readonly string[]): boolean {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return actual.length === expected.length && actual.every((key, index) => key === expected[index]);
}

function base64UrlLength(bytes: number): number {
  return Math.ceil((bytes * 4) / 3);
}
