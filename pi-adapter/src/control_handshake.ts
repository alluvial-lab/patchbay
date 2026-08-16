import { randomBytes } from "node:crypto";
import { realpath } from "node:fs/promises";
import { isAbsolute, normalize } from "node:path";
import { create } from "@bufbuild/protobuf";
import {
  PiControlExtensionProfileSchema,
  PiControlHandshakeFailure,
  type PiControlExtensionProfile,
} from "@patchbay/contracts";
import {
  PATCHBAY_CONTROL_CHALLENGE_BYTES,
  PATCHBAY_CONTROL_EXTENSION_EPOCH_BYTES,
  PATCHBAY_CONTROL_HANDSHAKE_COMMAND,
  PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE,
  PATCHBAY_CONTROL_PROFILE_VERSION,
  PATCHBAY_CONTROL_RELOAD_COMMAND,
  PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE,
  PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE,
  PATCHBAY_SUPPORTED_SESSION_VERSION,
  type PiControlHandshakeMarkerData,
} from "../extensions/patchbay-control.js";

const BASE64URL_PATTERN = /^[A-Za-z0-9_-]+$/u;
const CHALLENGE_LENGTH = base64UrlLength(PATCHBAY_CONTROL_CHALLENGE_BYTES);
const EPOCH_LENGTH = base64UrlLength(PATCHBAY_CONTROL_EXTENSION_EPOCH_BYTES);
const MAX_LOCAL_PATH_BYTES = 4_096;
const MAX_SESSION_ID_BYTES = 128;
const DEFAULT_MAX_ENTRY_POLLS = 20;
const DEFAULT_POLL_INTERVAL_MS = 25;
const DEFAULT_RPC_TIMEOUT_MS = 2_000;

export interface PiRpcCommandInfo {
  readonly name: string;
  readonly source: string;
  readonly path?: string;
}

export interface PiRpcSessionIdentity {
  readonly sessionId: string;
  readonly sessionFile: string;
}

export interface PiRpcEntries {
  readonly entries: readonly unknown[];
  readonly leafId: string | null;
}

export interface PiControlRpc {
  getCommands(): Promise<readonly PiRpcCommandInfo[]>;
  prompt(message: string): Promise<{ readonly success: boolean }>;
  getEntries(): Promise<PiRpcEntries>;
  getState(): Promise<PiRpcSessionIdentity>;
  getSessionStats(): Promise<PiRpcSessionIdentity>;
}

export interface PiControlHandshake {
  readonly challenge: string;
  readonly launchNonce: string;
  readonly extensionEpoch: string;
  readonly cwd: string;
  readonly sessionId: string;
  readonly sessionFile: string;
  readonly markerEntryId: string;
}

export interface PiControlHandshakeOptions {
  readonly rpc: PiControlRpc;
  readonly launchNonce: string;
  readonly expectedProjectCwd: string;
  readonly expectedExtensionPath: string;
  readonly requiredExtensionEpoch?: string;
  readonly previousExtensionEpoch?: string;
  readonly maxEntryPolls?: number;
  readonly pollIntervalMs?: number;
  readonly rpcTimeoutMs?: number;
  readonly randomBytes?: (size: number) => Uint8Array;
  readonly sleep?: (milliseconds: number) => Promise<void>;
}

export class PiControlHandshakeError extends Error {
  readonly code: PiControlHandshakeFailure;

  constructor(code: PiControlHandshakeFailure) {
    super(`Pi control handshake failed (${failureName(code)})`);
    this.name = "PiControlHandshakeError";
    this.code = code;
  }
}

export function piControlExtensionProfile(): PiControlExtensionProfile {
  return create(PiControlExtensionProfileSchema, {
    profileVersion: PATCHBAY_CONTROL_PROFILE_VERSION,
    handshakeCommand: PATCHBAY_CONTROL_HANDSHAKE_COMMAND,
    reloadCommand: PATCHBAY_CONTROL_RELOAD_COMMAND,
    handshakeCustomType: PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE,
    reloadRequestCustomType: PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE,
    reloadCompletionCustomType: PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE,
    challengeBytes: PATCHBAY_CONTROL_CHALLENGE_BYTES,
    extensionEpochBytes: PATCHBAY_CONTROL_EXTENSION_EPOCH_BYTES,
    supportedSessionVersion: PATCHBAY_SUPPORTED_SESSION_VERSION,
  });
}

export function generateControlChallenge(
  source: (size: number) => Uint8Array = randomBytes,
): string {
  const bytes = source(PATCHBAY_CONTROL_CHALLENGE_BYTES);
  if (bytes.byteLength !== PATCHBAY_CONTROL_CHALLENGE_BYTES) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.BOUND_EXCEEDED);
  }
  return Buffer.from(bytes).toString("base64url");
}

export async function performPiControlHandshake(
  options: PiControlHandshakeOptions,
): Promise<PiControlHandshake> {
  requireBase64Url(options.launchNonce, CHALLENGE_LENGTH);
  if (options.requiredExtensionEpoch !== undefined) {
    requireBase64Url(options.requiredExtensionEpoch, EPOCH_LENGTH);
  }
  if (options.previousExtensionEpoch !== undefined) {
    requireBase64Url(options.previousExtensionEpoch, EPOCH_LENGTH);
  }
  const maxEntryPolls = options.maxEntryPolls ?? DEFAULT_MAX_ENTRY_POLLS;
  const pollIntervalMs = options.pollIntervalMs ?? DEFAULT_POLL_INTERVAL_MS;
  const rpcTimeoutMs = options.rpcTimeoutMs ?? DEFAULT_RPC_TIMEOUT_MS;
  if (!Number.isSafeInteger(maxEntryPolls) || maxEntryPolls < 1 || maxEntryPolls > 1_000) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.BOUND_EXCEEDED);
  }
  if (!Number.isSafeInteger(pollIntervalMs) || pollIntervalMs < 0 || pollIntervalMs > 60_000) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.BOUND_EXCEEDED);
  }
  if (!Number.isSafeInteger(rpcTimeoutMs) || rpcTimeoutMs < 1 || rpcTimeoutMs > 60_000) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.BOUND_EXCEEDED);
  }

  const commands = await rpcCall(
    () => options.rpc.getCommands(),
    PiControlHandshakeFailure.RPC_CROSS_CHECK_FAILED,
    rpcTimeoutMs,
  );
  const matchingCommands = commands.filter(
    (command) => command.name === PATCHBAY_CONTROL_HANDSHAKE_COMMAND,
  );
  if (matchingCommands.length === 0) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.COMMAND_MISSING);
  }
  if (matchingCommands.length !== 1) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.COMMAND_SOURCE_MISMATCH);
  }
  const command = matchingCommands[0];
  if (!command || command.source !== "extension" || !command.path) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.COMMAND_SOURCE_MISMATCH);
  }
  const [actualExtensionPath, expectedExtensionPath] = await Promise.all([
    canonicalExistingPath(command.path, PiControlHandshakeFailure.COMMAND_SOURCE_MISMATCH),
    canonicalExistingPath(
      options.expectedExtensionPath,
      PiControlHandshakeFailure.COMMAND_SOURCE_MISMATCH,
    ),
  ]);
  if (actualExtensionPath !== expectedExtensionPath) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.COMMAND_SOURCE_MISMATCH);
  }

  const challenge = generateControlChallenge(options.randomBytes ?? randomBytes);
  const prompt = await rpcCall(
    () => options.rpc.prompt(`/${PATCHBAY_CONTROL_HANDSHAKE_COMMAND} ${challenge}`),
    PiControlHandshakeFailure.PROMPT_REJECTED,
    rpcTimeoutMs,
  );
  if (!prompt.success) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.PROMPT_REJECTED);
  }

  const sleep = options.sleep ?? defaultSleep;
  let sawHandshakeMarker = false;
  for (let attempt = 0; attempt < maxEntryPolls; attempt += 1) {
    const rpcEntries = await rpcCall(
      () => options.rpc.getEntries(),
      PiControlHandshakeFailure.RPC_CROSS_CHECK_FAILED,
      rpcTimeoutMs,
    );
    const challengeMarkers: Array<{
      readonly entryId: string;
      readonly marker: PiControlHandshakeMarkerData;
    }> = [];
    for (const entry of rpcEntries.entries) {
      const candidate = parseHandshakeEntry(entry);
      if (!candidate) continue;
      sawHandshakeMarker = true;
      if (candidate.marker.challenge === challenge) challengeMarkers.push(candidate);
    }
    if (challengeMarkers.length > 1) {
      throw new PiControlHandshakeError(PiControlHandshakeFailure.MARKER_AMBIGUOUS);
    }
    const candidate = challengeMarkers[0];
    if (candidate) {
      validateMarkerCorrelation(candidate.marker, options);
      if (rpcEntries.leafId !== candidate.entryId) {
        throw new PiControlHandshakeError(PiControlHandshakeFailure.MARKER_NOT_CURRENT_LEAF);
      }
      const [markerCwd, projectCwd] = await Promise.all([
        canonicalExistingPath(candidate.marker.cwd, PiControlHandshakeFailure.CWD_MISMATCH),
        canonicalExistingPath(
          options.expectedProjectCwd,
          PiControlHandshakeFailure.CWD_MISMATCH,
        ),
      ]);
      if (candidate.marker.cwd !== markerCwd || markerCwd !== projectCwd) {
        throw new PiControlHandshakeError(PiControlHandshakeFailure.CWD_MISMATCH);
      }
      const [state, stats] = await Promise.all([
        rpcCall(
          () => options.rpc.getState(),
          PiControlHandshakeFailure.RPC_CROSS_CHECK_FAILED,
          rpcTimeoutMs,
        ),
        rpcCall(
          () => options.rpc.getSessionStats(),
          PiControlHandshakeFailure.RPC_CROSS_CHECK_FAILED,
          rpcTimeoutMs,
        ),
      ]);
      validateRpcIdentity(candidate.marker, state, stats);
      return {
        challenge,
        launchNonce: candidate.marker.launchNonce,
        extensionEpoch: candidate.marker.extensionEpoch,
        cwd: markerCwd,
        sessionId: candidate.marker.sessionId,
        sessionFile: candidate.marker.sessionFile,
        markerEntryId: candidate.entryId,
      };
    }
    if (attempt + 1 < maxEntryPolls) await sleep(pollIntervalMs);
  }
  throw new PiControlHandshakeError(
    sawHandshakeMarker
      ? PiControlHandshakeFailure.CHALLENGE_MISMATCH
      : PiControlHandshakeFailure.MARKER_MISSING,
  );
}

function validateMarkerCorrelation(
  marker: PiControlHandshakeMarkerData,
  options: PiControlHandshakeOptions,
): void {
  if (marker.launchNonce !== options.launchNonce) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.LAUNCH_NONCE_MISMATCH);
  }
  if (!isBase64Url(marker.extensionEpoch, EPOCH_LENGTH)) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.EXTENSION_EPOCH_MISMATCH);
  }
  if (
    options.requiredExtensionEpoch !== undefined &&
    marker.extensionEpoch !== options.requiredExtensionEpoch
  ) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.EXTENSION_EPOCH_MISMATCH);
  }
  if (
    options.previousExtensionEpoch !== undefined &&
    marker.extensionEpoch === options.previousExtensionEpoch
  ) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.EXTENSION_EPOCH_MISMATCH);
  }
}

function validateRpcIdentity(
  marker: PiControlHandshakeMarkerData,
  state: PiRpcSessionIdentity,
  stats: PiRpcSessionIdentity,
): void {
  if (
    !isBoundedText(state.sessionId, MAX_SESSION_ID_BYTES) ||
    !isBoundedText(stats.sessionId, MAX_SESSION_ID_BYTES) ||
    !isCanonicalAbsolutePath(state.sessionFile) ||
    !isCanonicalAbsolutePath(stats.sessionFile)
  ) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.RPC_CROSS_CHECK_FAILED);
  }
  if (marker.sessionId !== state.sessionId || marker.sessionId !== stats.sessionId) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.SESSION_ID_MISMATCH);
  }
  if (
    marker.sessionFile !== state.sessionFile ||
    marker.sessionFile !== stats.sessionFile
  ) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.SESSION_FILE_MISMATCH);
  }
}

function parseHandshakeEntry(
  value: unknown,
): { readonly entryId: string; readonly marker: PiControlHandshakeMarkerData } | undefined {
  if (!isRecord(value) || value.type !== "custom") return undefined;
  if (
    !hasExactKeys(value, ["type", "id", "parentId", "timestamp", "customType", "data"]) ||
    !isBoundedText(value.id, 128) ||
    !(value.parentId === null || isBoundedText(value.parentId, 128)) ||
    !isCanonicalTimestamp(value.timestamp) ||
    value.customType !== PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE ||
    !isRecord(value.data)
  ) {
    return undefined;
  }
  const data = value.data;
  if (
    !hasExactKeys(data, [
      "challenge",
      "launchNonce",
      "extensionEpoch",
      "cwd",
      "sessionId",
      "sessionFile",
    ]) ||
    !isBase64Url(data.challenge, CHALLENGE_LENGTH) ||
    !isBase64Url(data.launchNonce, CHALLENGE_LENGTH) ||
    !isBase64Url(data.extensionEpoch, EPOCH_LENGTH) ||
    !isCanonicalAbsolutePath(data.cwd) ||
    !isBoundedText(data.sessionId, MAX_SESSION_ID_BYTES) ||
    !isCanonicalAbsolutePath(data.sessionFile)
  ) {
    return undefined;
  }
  return {
    entryId: value.id,
    marker: {
      challenge: data.challenge,
      launchNonce: data.launchNonce,
      extensionEpoch: data.extensionEpoch,
      cwd: data.cwd,
      sessionId: data.sessionId,
      sessionFile: data.sessionFile,
    },
  };
}

async function canonicalExistingPath(
  value: string,
  failure: PiControlHandshakeFailure,
): Promise<string> {
  if (!isBoundedText(value, MAX_LOCAL_PATH_BYTES) || !isAbsolute(value)) {
    throw new PiControlHandshakeError(failure);
  }
  try {
    return await realpath(value);
  } catch {
    throw new PiControlHandshakeError(failure);
  }
}

function requireBase64Url(value: string, exactLength: number): void {
  if (!isBase64Url(value, exactLength)) {
    throw new PiControlHandshakeError(PiControlHandshakeFailure.BOUND_EXCEEDED);
  }
}

function isBase64Url(value: unknown, exactLength: number): value is string {
  return (
    typeof value === "string" &&
    value.length === exactLength &&
    BASE64URL_PATTERN.test(value)
  );
}

function isBoundedText(value: unknown, maxBytes: number): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    !value.includes("\0") &&
    Buffer.byteLength(value) <= maxBytes
  );
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

function defaultSleep(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

async function rpcCall<T>(
  action: () => Promise<T>,
  failure: PiControlHandshakeFailure,
  timeoutMs: number,
): Promise<T> {
  let timeout: NodeJS.Timeout | undefined;
  try {
    return await Promise.race([
      action(),
      new Promise<never>((_resolve, reject) => {
        timeout = setTimeout(
          () => reject(new PiControlHandshakeError(failure)),
          timeoutMs,
        );
      }),
    ]);
  } catch (error) {
    if (error instanceof PiControlHandshakeError) throw error;
    throw new PiControlHandshakeError(failure);
  } finally {
    if (timeout !== undefined) clearTimeout(timeout);
  }
}

function isCanonicalTimestamp(value: unknown): value is string {
  if (typeof value !== "string") return false;
  const parsed = new Date(value);
  return !Number.isNaN(parsed.valueOf()) && parsed.toISOString() === value;
}

function isCanonicalAbsolutePath(value: unknown): value is string {
  return (
    isBoundedText(value, MAX_LOCAL_PATH_BYTES) &&
    isAbsolute(value) &&
    normalize(value) === value
  );
}

function failureName(code: PiControlHandshakeFailure): string {
  return PiControlHandshakeFailure[code]?.toLowerCase() ?? "unknown";
}
