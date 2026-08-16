import { createHash } from "node:crypto";
import { constants, type BigIntStats } from "node:fs";
import { lstat, open, realpath, stat } from "node:fs/promises";
import { isAbsolute, normalize, relative, sep } from "node:path";
import { isDeepStrictEqual } from "node:util";
import {
  PiReloadableResourceKind,
  PiSessionIntegrityFailure,
} from "@patchbay/contracts";
import {
  PATCHBAY_CONTROL_CHALLENGE_BYTES,
  PATCHBAY_CONTROL_EXTENSION_EPOCH_BYTES,
  PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE,
  PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE,
  PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE,
  PATCHBAY_SUPPORTED_SESSION_VERSION,
} from "../extensions/patchbay-control.js";
import type { PiControlHandshake } from "./control_handshake.js";

const MAX_LOCAL_PATH_BYTES = 4_096;
const MAX_ID_BYTES = 128;
const MAX_TYPE_BYTES = 128;
const MAX_TEXT_BYTES = 1_048_576;
const DEFAULT_MAX_SESSION_BYTES = 64 * 1_024 * 1_024;
const DEFAULT_MAX_POST_SEAL_ENTRIES = 32;
const BASE64URL_PATTERN = /^[A-Za-z0-9_-]+$/u;
const BOUNDED_ID_PATTERN = /^[A-Za-z0-9._:-]+$/u;
const CHALLENGE_LENGTH = Math.ceil((PATCHBAY_CONTROL_CHALLENGE_BYTES * 4) / 3);
const EPOCH_LENGTH = Math.ceil((PATCHBAY_CONTROL_EXTENSION_EPOCH_BYTES * 4) / 3);
const ADMITTED_RELOAD_RESOURCES = new Set<PiReloadableResourceKind>([
  PiReloadableResourceKind.EXTENSION_ENTRYPOINT,
  PiReloadableResourceKind.SKILL,
  PiReloadableResourceKind.PROMPT,
  PiReloadableResourceKind.THEME,
  PiReloadableResourceKind.CONTEXT_FILE,
]);
const ENTRY_TYPES = new Set([
  "message",
  "thinking_level_change",
  "model_change",
  "compaction",
  "branch_summary",
  "custom",
  "custom_message",
  "label",
  "session_info",
]);

export interface MaterializedSessionSeal {
  readonly canonicalPath: string;
  readonly sessionRootId: string;
  readonly sessionId: string;
  readonly device: bigint;
  readonly inode: bigint;
  readonly size: bigint;
  readonly contentDigest: string;
  readonly treeDigest: string;
  readonly orderedEntryIds: readonly string[];
  readonly leafId: string;
}

export type PiSessionMaterialization =
  | {
      readonly kind: "memory_only";
      readonly sessionId: string;
      readonly declaredPath: string;
    }
  | { readonly kind: "materialized"; readonly seal: MaterializedSessionSeal }
  | { readonly kind: "invalid"; readonly failure: PiSessionIntegrityFailure };

export interface PiSessionFileValidationOptions {
  readonly sessionId: string;
  readonly declaredPath: string;
  readonly allowedRoot: string;
  readonly rpcEntries: readonly unknown[];
  readonly rpcLeafId: string | null;
  readonly maxSessionBytes?: number;
  /** A deterministic race hook used by filesystem-integrity tests. */
  readonly afterOpen?: () => Promise<void> | void;
}

export interface PiSessionSealVerificationOptions extends PiSessionFileValidationOptions {
  readonly seal: MaterializedSessionSeal;
}

export interface PiResumedSessionVerificationOptions extends PiSessionSealVerificationOptions {
  readonly handshake: PiControlHandshake;
  readonly maxPostSealEntries?: number;
}

type StrictEntry = Readonly<Record<string, unknown>> & {
  readonly type: string;
  readonly id: string;
  readonly parentId: string | null;
  readonly timestamp: string;
};

interface StableFile {
  readonly canonicalPath: string;
  readonly bytes: Buffer;
  readonly stats: BigIntStats;
}

interface ParsedSession {
  readonly header: Readonly<Record<string, unknown>>;
  readonly entries: readonly StrictEntry[];
  readonly rootId: string;
}

interface ValidatedSession {
  readonly stableFile: StableFile;
  readonly parsed: ParsedSession;
  readonly leafId: string;
  readonly seal: MaterializedSessionSeal;
}

class IntegrityFault extends Error {
  readonly failure: PiSessionIntegrityFailure;

  constructor(failure: PiSessionIntegrityFailure) {
    super("Pi session integrity validation failed");
    this.failure = failure;
  }
}

export async function classifyPiSessionMaterialization(
  options: PiSessionFileValidationOptions,
): Promise<PiSessionMaterialization> {
  try {
    validateClassificationInput(options);
    const pathState = await initialPathState(options.declaredPath);
    if (pathState === "missing") return memoryOnly(options);
    const stableFile = await readStableFile(options);
    if (stableFile.bytes.byteLength === 0) return memoryOnly(options);
    const validated = validateStableSession(stableFile, options);
    return { kind: "materialized", seal: validated.seal };
  } catch (error) {
    return invalidResult(error);
  }
}

export async function verifyMaterializedSessionSeal(
  options: PiSessionSealVerificationOptions,
): Promise<PiSessionMaterialization> {
  try {
    validateClassificationInput(options);
    const pathState = await initialPathState(options.declaredPath);
    if (pathState === "missing") {
      throw new IntegrityFault(PiSessionIntegrityFailure.SEAL_IDENTITY_MISMATCH);
    }
    const stableFile = await readStableFile(options);
    if (stableFile.bytes.byteLength === 0) {
      throw new IntegrityFault(PiSessionIntegrityFailure.SEAL_IDENTITY_MISMATCH);
    }
    const validated = validateStableSession(stableFile, options);
    requireExactSeal(options.seal, validated.seal);
    return { kind: "materialized", seal: validated.seal };
  } catch (error) {
    return invalidResult(error);
  }
}

export async function verifyResumedSessionExtension(
  options: PiResumedSessionVerificationOptions,
): Promise<PiSessionMaterialization> {
  try {
    validateClassificationInput(options);
    const maxPostSealEntries = options.maxPostSealEntries ?? DEFAULT_MAX_POST_SEAL_ENTRIES;
    if (
      !Number.isSafeInteger(maxPostSealEntries) ||
      maxPostSealEntries < 1 ||
      maxPostSealEntries > 1_000
    ) {
      throw new IntegrityFault(PiSessionIntegrityFailure.CONTROL_MARKER_MISMATCH);
    }
    const pathState = await initialPathState(options.declaredPath);
    if (pathState === "missing") {
      throw new IntegrityFault(PiSessionIntegrityFailure.SEAL_IDENTITY_MISMATCH);
    }
    const stableFile = await readStableFile(options);
    const validated = validateStableSession(stableFile, options);
    requireSamePhysicalSession(options.seal, validated.seal);
    requireSealedPrefix(options.seal, validated, maxPostSealEntries);
    requireCurrentHandshakeMarker(options.handshake, validated);
    return { kind: "materialized", seal: validated.seal };
  } catch (error) {
    return invalidResult(error);
  }
}

function validateStableSession(
  stableFile: StableFile,
  options: PiSessionFileValidationOptions,
): ValidatedSession {
  const parsed = parseStrictSession(stableFile.bytes, options.sessionId);
  if (!isDeepStrictEqual(parsed.entries, options.rpcEntries)) {
    throw new IntegrityFault(PiSessionIntegrityFailure.RPC_ENTRIES_MISMATCH);
  }
  if (
    options.rpcLeafId === null ||
    !parsed.entries.some((entry) => entry.id === options.rpcLeafId)
  ) {
    throw new IntegrityFault(PiSessionIntegrityFailure.RPC_LEAF_MISMATCH);
  }
  const seal = createSeal(stableFile, parsed, options.rpcLeafId, options.sessionId);
  return { stableFile, parsed, leafId: options.rpcLeafId, seal };
}

function parseStrictSession(bytes: Buffer, expectedSessionId: string): ParsedSession {
  if (bytes.byteLength === 0 || bytes[bytes.byteLength - 1] !== 0x0a) {
    throw new IntegrityFault(PiSessionIntegrityFailure.FRAMING_INVALID);
  }
  let content: string;
  try {
    content = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    throw new IntegrityFault(PiSessionIntegrityFailure.JSON_INVALID);
  }
  const objects: Readonly<Record<string, unknown>>[] = [];
  for (const line of content.split("\n")) {
    if (line.trim().length === 0) continue;
    let value: unknown;
    try {
      value = JSON.parse(line);
    } catch {
      throw new IntegrityFault(PiSessionIntegrityFailure.JSON_INVALID);
    }
    if (!isRecord(value)) {
      throw new IntegrityFault(PiSessionIntegrityFailure.JSON_INVALID);
    }
    objects.push(value);
  }
  if (objects.length === 0) {
    throw new IntegrityFault(PiSessionIntegrityFailure.HEADER_INVALID);
  }
  const header = objects[0];
  if (!header) throw new IntegrityFault(PiSessionIntegrityFailure.HEADER_INVALID);
  validateHeader(header, expectedSessionId);
  const entries: StrictEntry[] = [];
  for (let index = 1; index < objects.length; index += 1) {
    const value = objects[index];
    if (!value) throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
    if (value.type === "session") {
      throw new IntegrityFault(PiSessionIntegrityFailure.HEADER_INVALID);
    }
    entries.push(validateEntry(value));
  }
  if (entries.length === 0) {
    throw new IntegrityFault(PiSessionIntegrityFailure.TREE_INVALID);
  }
  const rootId = validateTreeAndReferences(entries);
  return { header, entries, rootId };
}

function validateHeader(header: Readonly<Record<string, unknown>>, expectedSessionId: string): void {
  requireAllowedKeys(header, ["type", "version", "id", "timestamp", "cwd", "parentSession"], PiSessionIntegrityFailure.HEADER_INVALID);
  if (
    header.type !== "session" ||
    header.version !== PATCHBAY_SUPPORTED_SESSION_VERSION ||
    header.id !== expectedSessionId ||
    !isBoundedId(header.id) ||
    !isCanonicalTimestamp(header.timestamp) ||
    !isCanonicalAbsolutePath(header.cwd)
  ) {
    throw new IntegrityFault(PiSessionIntegrityFailure.HEADER_INVALID);
  }
  if (header.parentSession !== undefined && !isCanonicalAbsolutePath(header.parentSession)) {
    throw new IntegrityFault(PiSessionIntegrityFailure.HEADER_INVALID);
  }
}

function validateEntry(value: Readonly<Record<string, unknown>>): StrictEntry {
  if (typeof value.type !== "string" || !ENTRY_TYPES.has(value.type)) {
    throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_TYPE_UNSUPPORTED);
  }
  if (
    !isBoundedId(value.id) ||
    !(value.parentId === null || isBoundedId(value.parentId)) ||
    !isCanonicalTimestamp(value.timestamp)
  ) {
    throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
  }
  switch (value.type) {
    case "message":
      requireAllowedKeys(value, ["type", "id", "parentId", "timestamp", "message"]);
      validateMessage(value.message);
      break;
    case "thinking_level_change":
      requireAllowedKeys(value, ["type", "id", "parentId", "timestamp", "thinkingLevel"]);
      requireBoundedText(value.thinkingLevel, MAX_TYPE_BYTES);
      break;
    case "model_change":
      requireAllowedKeys(value, ["type", "id", "parentId", "timestamp", "provider", "modelId"]);
      requireBoundedText(value.provider, MAX_TYPE_BYTES);
      requireBoundedText(value.modelId, MAX_TYPE_BYTES);
      break;
    case "compaction":
      requireAllowedKeys(value, [
        "type",
        "id",
        "parentId",
        "timestamp",
        "summary",
        "firstKeptEntryId",
        "tokensBefore",
        "details",
        "usage",
        "fromHook",
      ]);
      requireBoundedText(value.summary, MAX_TEXT_BYTES);
      if (!isBoundedId(value.firstKeptEntryId) || !isNonnegativeSafeInteger(value.tokensBefore)) {
        throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
      }
      requireOptionalRecord(value.usage);
      requireOptionalBoolean(value.fromHook);
      break;
    case "branch_summary":
      requireAllowedKeys(value, [
        "type",
        "id",
        "parentId",
        "timestamp",
        "fromId",
        "summary",
        "details",
        "usage",
        "fromHook",
      ]);
      if (!isBoundedId(value.fromId)) {
        throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
      }
      requireBoundedText(value.summary, MAX_TEXT_BYTES);
      requireOptionalRecord(value.usage);
      requireOptionalBoolean(value.fromHook);
      break;
    case "custom":
      requireAllowedKeys(value, ["type", "id", "parentId", "timestamp", "customType", "data"]);
      requireBoundedText(value.customType, MAX_TYPE_BYTES);
      break;
    case "custom_message":
      requireAllowedKeys(value, [
        "type",
        "id",
        "parentId",
        "timestamp",
        "customType",
        "content",
        "details",
        "display",
      ]);
      requireBoundedText(value.customType, MAX_TYPE_BYTES);
      validateTextOrMediaContent(value.content, false);
      if (typeof value.display !== "boolean") {
        throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
      }
      break;
    case "label":
      requireAllowedKeys(value, ["type", "id", "parentId", "timestamp", "targetId", "label"]);
      if (!isBoundedId(value.targetId)) {
        throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
      }
      if (value.label !== undefined) requireBoundedText(value.label, MAX_TYPE_BYTES);
      break;
    case "session_info":
      requireAllowedKeys(value, ["type", "id", "parentId", "timestamp", "name"]);
      if (value.name !== undefined) requireBoundedText(value.name, MAX_TYPE_BYTES);
      break;
    default:
      throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_TYPE_UNSUPPORTED);
  }
  return value as StrictEntry;
}

function validateTreeAndReferences(entries: readonly StrictEntry[]): string {
  const seen = new Set<string>();
  const toolCalls = new Set<string>();
  let rootId: string | undefined;
  for (const entry of entries) {
    if (seen.has(entry.id)) {
      throw new IntegrityFault(PiSessionIntegrityFailure.DUPLICATE_ENTRY_ID);
    }
    if (entry.parentId === entry.id || (entry.parentId !== null && !seen.has(entry.parentId))) {
      throw new IntegrityFault(PiSessionIntegrityFailure.PARENT_INVALID);
    }
    if (entry.parentId === null) {
      if (rootId !== undefined) {
        throw new IntegrityFault(PiSessionIntegrityFailure.TREE_INVALID);
      }
      rootId = entry.id;
    }
    validateSecondaryReferences(entry, seen, toolCalls);
    collectToolCalls(entry, toolCalls);
    seen.add(entry.id);
  }
  if (rootId === undefined) {
    throw new IntegrityFault(PiSessionIntegrityFailure.TREE_INVALID);
  }
  return rootId;
}

function validateSecondaryReferences(
  entry: StrictEntry,
  earlierEntryIds: ReadonlySet<string>,
  toolCalls: ReadonlySet<string>,
): void {
  const referencedEntryId =
    entry.type === "label"
      ? entry.targetId
      : entry.type === "compaction"
        ? entry.firstKeptEntryId
        : entry.type === "branch_summary"
          ? entry.fromId
          : undefined;
  if (referencedEntryId !== undefined) {
    if (typeof referencedEntryId !== "string" || !earlierEntryIds.has(referencedEntryId)) {
      throw new IntegrityFault(PiSessionIntegrityFailure.REFERENCE_INVALID);
    }
  }
  if (entry.type === "message" && isRecord(entry.message) && entry.message.role === "toolResult") {
    if (typeof entry.message.toolCallId !== "string" || !toolCalls.has(entry.message.toolCallId)) {
      throw new IntegrityFault(PiSessionIntegrityFailure.REFERENCE_INVALID);
    }
  }
}

function collectToolCalls(entry: StrictEntry, toolCalls: Set<string>): void {
  if (entry.type !== "message" || !isRecord(entry.message) || entry.message.role !== "assistant") {
    return;
  }
  const content = entry.message.content;
  if (!Array.isArray(content)) return;
  for (const block of content) {
    if (!isRecord(block) || block.type !== "toolCall" || typeof block.id !== "string") continue;
    if (toolCalls.has(block.id)) {
      throw new IntegrityFault(PiSessionIntegrityFailure.REFERENCE_INVALID);
    }
    toolCalls.add(block.id);
  }
}

function validateMessage(value: unknown): void {
  if (!isRecord(value) || typeof value.role !== "string" || !isTimestampNumber(value.timestamp)) {
    throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
  }
  switch (value.role) {
    case "user":
      validateTextOrMediaContent(value.content, true);
      return;
    case "assistant":
      if (
        !Array.isArray(value.content) ||
        !value.content.every(isAssistantContentBlock) ||
        !isNonemptyString(value.api) ||
        !isNonemptyString(value.provider) ||
        !isNonemptyString(value.model) ||
        !isUsage(value.usage) ||
        !isTerminalStopReason(value.stopReason)
      ) {
        throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
      }
      return;
    case "toolResult":
      if (
        !isBoundedId(value.toolCallId) ||
        !isNonemptyString(value.toolName) ||
        !Array.isArray(value.content) ||
        !value.content.every(isTextOrImageBlock) ||
        (value.usage !== undefined && !isUsage(value.usage)) ||
        (value.addedToolNames !== undefined &&
          (!Array.isArray(value.addedToolNames) ||
            !value.addedToolNames.every(isNonemptyString))) ||
        typeof value.isError !== "boolean"
      ) {
        throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
      }
      return;
    case "bashExecution":
      if (
        typeof value.command !== "string" ||
        typeof value.output !== "string" ||
        !(value.exitCode === undefined || Number.isSafeInteger(value.exitCode)) ||
        typeof value.cancelled !== "boolean" ||
        typeof value.truncated !== "boolean"
      ) {
        throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
      }
      return;
    case "custom":
      if (
        typeof value.customType !== "string" ||
        typeof value.display !== "boolean"
      ) {
        throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
      }
      validateTextOrMediaContent(value.content, true);
      return;
    case "branchSummary":
      if (typeof value.summary !== "string" || !isBoundedId(value.fromId)) {
        throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
      }
      return;
    case "compactionSummary":
      if (typeof value.summary !== "string" || !isNonnegativeSafeInteger(value.tokensBefore)) {
        throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
      }
      return;
    default:
      throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
  }
}

function validateTextOrMediaContent(value: unknown, allowString: boolean): void {
  if (allowString && typeof value === "string") return;
  if (!Array.isArray(value) || !value.every(isTextOrImageBlock)) {
    throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
  }
}

function isTextOrImageBlock(value: unknown): boolean {
  if (!isRecord(value)) return false;
  if (value.type === "text") return typeof value.text === "string";
  return (
    value.type === "image" &&
    typeof value.data === "string" &&
    typeof value.mimeType === "string"
  );
}

function isAssistantContentBlock(value: unknown): boolean {
  if (!isRecord(value)) return false;
  if (value.type === "text") return typeof value.text === "string";
  if (value.type === "thinking") return typeof value.thinking === "string";
  return (
    value.type === "toolCall" &&
    isBoundedId(value.id) &&
    isNonemptyString(value.name) &&
    isRecord(value.arguments)
  );
}

function isUsage(value: unknown): boolean {
  if (!isRecord(value) || !isRecord(value.cost)) return false;
  const cost = value.cost;
  return (
    ["input", "output", "cacheRead", "cacheWrite", "totalTokens"].every((key) =>
      isNonnegativeFiniteNumber(value[key]),
    ) &&
    ["input", "output", "cacheRead", "cacheWrite", "total"].every((key) =>
      isNonnegativeFiniteNumber(cost[key]),
    ) &&
    (value.cacheWrite1h === undefined || isNonnegativeFiniteNumber(value.cacheWrite1h)) &&
    (value.reasoning === undefined || isNonnegativeFiniteNumber(value.reasoning))
  );
}

function isTerminalStopReason(value: unknown): boolean {
  return (
    value === "stop" ||
    value === "length" ||
    value === "toolUse" ||
    value === "error" ||
    value === "aborted" ||
    value === "deferred"
  );
}

async function initialPathState(path: string): Promise<"exists" | "missing"> {
  try {
    const info = await lstat(path, { bigint: true });
    if (info.isSymbolicLink()) throw new IntegrityFault(PiSessionIntegrityFailure.SYMLINK);
    if (!info.isFile()) {
      throw new IntegrityFault(PiSessionIntegrityFailure.NOT_REGULAR_FILE);
    }
    return "exists";
  } catch (error) {
    if (isNodeError(error) && error.code === "ENOENT") return "missing";
    if (error instanceof IntegrityFault) throw error;
    throw new IntegrityFault(PiSessionIntegrityFailure.IO);
  }
}

async function readStableFile(options: PiSessionFileValidationOptions): Promise<StableFile> {
  let handle;
  try {
    handle = await open(options.declaredPath, constants.O_RDONLY | constants.O_NOFOLLOW);
  } catch (error) {
    if (isNodeError(error) && error.code === "ELOOP") {
      throw new IntegrityFault(PiSessionIntegrityFailure.SYMLINK);
    }
    if (isNodeError(error) && error.code === "ENOENT") {
      throw new IntegrityFault(PiSessionIntegrityFailure.UNSTABLE_FILE);
    }
    throw new IntegrityFault(PiSessionIntegrityFailure.IO);
  }
  try {
    const before = await handle.stat({ bigint: true });
    if (!before.isFile()) {
      throw new IntegrityFault(PiSessionIntegrityFailure.NOT_REGULAR_FILE);
    }
    const maxSessionBytes = options.maxSessionBytes ?? DEFAULT_MAX_SESSION_BYTES;
    if (!Number.isSafeInteger(maxSessionBytes) || maxSessionBytes < 1) {
      throw new IntegrityFault(PiSessionIntegrityFailure.FILE_TOO_LARGE);
    }
    if (before.size > BigInt(maxSessionBytes)) {
      throw new IntegrityFault(PiSessionIntegrityFailure.FILE_TOO_LARGE);
    }
    const [canonicalRoot, canonicalPath] = await Promise.all([
      canonicalAllowedRoot(options.allowedRoot),
      canonicalFdPath(handle.fd),
    ]);
    if (!isWithinRoot(canonicalRoot, canonicalPath)) {
      throw new IntegrityFault(PiSessionIntegrityFailure.PATH_OUTSIDE_ALLOWED_ROOT);
    }
    await options.afterOpen?.();
    const bytes = await handle.readFile();
    const after = await handle.stat({ bigint: true });
    if (!sameStableStats(before, after) || BigInt(bytes.byteLength) !== after.size) {
      throw new IntegrityFault(PiSessionIntegrityFailure.UNSTABLE_FILE);
    }
    await verifyPathStillNamesFile(options.declaredPath, canonicalPath, after);
    return { canonicalPath, bytes, stats: after };
  } finally {
    await handle.close();
  }
}

async function canonicalAllowedRoot(root: string): Promise<string> {
  if (!isCanonicalAbsolutePath(root)) {
    throw new IntegrityFault(PiSessionIntegrityFailure.PATH_OUTSIDE_ALLOWED_ROOT);
  }
  try {
    const canonical = await realpath(root);
    const info = await stat(canonical, { bigint: true });
    if (!info.isDirectory()) {
      throw new IntegrityFault(PiSessionIntegrityFailure.PATH_OUTSIDE_ALLOWED_ROOT);
    }
    return canonical;
  } catch (error) {
    if (error instanceof IntegrityFault) throw error;
    throw new IntegrityFault(PiSessionIntegrityFailure.PATH_OUTSIDE_ALLOWED_ROOT);
  }
}

async function canonicalFdPath(fd: number): Promise<string> {
  try {
    return await realpath(`/proc/self/fd/${fd}`);
  } catch {
    throw new IntegrityFault(PiSessionIntegrityFailure.IO);
  }
}

async function verifyPathStillNamesFile(
  declaredPath: string,
  canonicalPath: string,
  expected: BigIntStats,
): Promise<void> {
  try {
    const finalLstat = await lstat(declaredPath, { bigint: true });
    if (finalLstat.isSymbolicLink()) {
      throw new IntegrityFault(PiSessionIntegrityFailure.UNSTABLE_FILE);
    }
    const finalCanonical = await realpath(declaredPath);
    const finalStat = await stat(finalCanonical, { bigint: true });
    if (
      finalCanonical !== canonicalPath ||
      finalStat.dev !== expected.dev ||
      finalStat.ino !== expected.ino ||
      finalStat.size !== expected.size
    ) {
      throw new IntegrityFault(PiSessionIntegrityFailure.UNSTABLE_FILE);
    }
  } catch (error) {
    if (error instanceof IntegrityFault) throw error;
    throw new IntegrityFault(PiSessionIntegrityFailure.UNSTABLE_FILE);
  }
}

function sameStableStats(left: BigIntStats, right: BigIntStats): boolean {
  return (
    left.dev === right.dev &&
    left.ino === right.ino &&
    left.size === right.size &&
    left.mtimeNs === right.mtimeNs &&
    left.ctimeNs === right.ctimeNs
  );
}

function createSeal(
  stableFile: StableFile,
  parsed: ParsedSession,
  leafId: string,
  sessionId: string,
): MaterializedSessionSeal {
  return {
    canonicalPath: stableFile.canonicalPath,
    sessionRootId: parsed.rootId,
    sessionId,
    device: stableFile.stats.dev,
    inode: stableFile.stats.ino,
    size: stableFile.stats.size,
    contentDigest: digest(stableFile.bytes),
    treeDigest: treeDigest(parsed.entries),
    orderedEntryIds: parsed.entries.map((entry) => entry.id),
    leafId,
  };
}

function requireExactSeal(
  expected: MaterializedSessionSeal,
  actual: MaterializedSessionSeal,
): void {
  requireSamePhysicalSession(expected, actual);
  if (
    expected.size !== actual.size ||
    expected.contentDigest !== actual.contentDigest ||
    expected.treeDigest !== actual.treeDigest ||
    expected.leafId !== actual.leafId ||
    !sameStrings(expected.orderedEntryIds, actual.orderedEntryIds)
  ) {
    throw new IntegrityFault(PiSessionIntegrityFailure.SEALED_PREFIX_MISMATCH);
  }
}

function requireSamePhysicalSession(
  expected: MaterializedSessionSeal,
  actual: MaterializedSessionSeal,
): void {
  if (
    expected.canonicalPath !== actual.canonicalPath ||
    expected.sessionRootId !== actual.sessionRootId ||
    expected.sessionId !== actual.sessionId ||
    expected.device !== actual.device ||
    expected.inode !== actual.inode
  ) {
    throw new IntegrityFault(PiSessionIntegrityFailure.SEAL_IDENTITY_MISMATCH);
  }
}

function requireSealedPrefix(
  expected: MaterializedSessionSeal,
  actual: ValidatedSession,
  maxPostSealEntries: number,
): void {
  if (actual.stableFile.bytes.byteLength < expected.size) {
    throw new IntegrityFault(PiSessionIntegrityFailure.SEALED_PREFIX_MISMATCH);
  }
  const expectedSize = Number(expected.size);
  if (!Number.isSafeInteger(expectedSize)) {
    throw new IntegrityFault(PiSessionIntegrityFailure.SEALED_PREFIX_MISMATCH);
  }
  if (digest(actual.stableFile.bytes.subarray(0, expectedSize)) !== expected.contentDigest) {
    throw new IntegrityFault(PiSessionIntegrityFailure.SEALED_PREFIX_MISMATCH);
  }
  const prefixEntries = actual.parsed.entries.slice(0, expected.orderedEntryIds.length);
  if (
    !sameStrings(
      prefixEntries.map((entry) => entry.id),
      expected.orderedEntryIds,
    ) ||
    treeDigest(prefixEntries) !== expected.treeDigest
  ) {
    throw new IntegrityFault(PiSessionIntegrityFailure.SEALED_PREFIX_MISMATCH);
  }
  const suffix = actual.parsed.entries.slice(expected.orderedEntryIds.length);
  if (suffix.length === 0 || suffix.length > maxPostSealEntries) {
    throw new IntegrityFault(PiSessionIntegrityFailure.CONTROL_MARKER_MISMATCH);
  }
  let expectedParent = expected.leafId;
  for (const entry of suffix) {
    if (entry.parentId !== expectedParent || !isAllowedPostSealEntry(entry)) {
      throw new IntegrityFault(PiSessionIntegrityFailure.SEALED_PREFIX_MISMATCH);
    }
    expectedParent = entry.id;
  }
}

function requireCurrentHandshakeMarker(
  handshake: PiControlHandshake,
  actual: ValidatedSession,
): void {
  const entry = actual.parsed.entries.at(-1);
  if (
    !entry ||
    entry.id !== handshake.markerEntryId ||
    actual.leafId !== handshake.markerEntryId ||
    entry.type !== "custom" ||
    entry.customType !== PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE ||
    !isRecord(entry.data) ||
    !hasExactKeys(entry.data, [
      "challenge",
      "launchNonce",
      "extensionEpoch",
      "cwd",
      "sessionId",
      "sessionFile",
    ]) ||
    entry.data.challenge !== handshake.challenge ||
    entry.data.launchNonce !== handshake.launchNonce ||
    entry.data.extensionEpoch !== handshake.extensionEpoch ||
    entry.data.cwd !== handshake.cwd ||
    entry.data.sessionId !== handshake.sessionId ||
    entry.data.sessionFile !== handshake.sessionFile
  ) {
    throw new IntegrityFault(PiSessionIntegrityFailure.CONTROL_MARKER_MISMATCH);
  }
}

function isAllowedPostSealEntry(entry: StrictEntry): boolean {
  if (
    entry.type === "model_change" ||
    entry.type === "thinking_level_change" ||
    entry.type === "session_info"
  ) {
    return true;
  }
  if (entry.type !== "custom" || !isRecord(entry.data)) return false;
  switch (entry.customType) {
    case PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE:
      return (
        hasExactKeys(entry.data, [
          "challenge",
          "launchNonce",
          "extensionEpoch",
          "cwd",
          "sessionId",
          "sessionFile",
        ]) &&
        isExactBase64Url(entry.data.challenge, CHALLENGE_LENGTH) &&
        isExactBase64Url(entry.data.launchNonce, CHALLENGE_LENGTH) &&
        isExactBase64Url(entry.data.extensionEpoch, EPOCH_LENGTH) &&
        isCanonicalAbsolutePath(entry.data.cwd) &&
        isBoundedId(entry.data.sessionId) &&
        isCanonicalAbsolutePath(entry.data.sessionFile)
      );
    case PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE:
      return (
        hasExactKeys(entry.data, ["commandId", "nonce", "priorExtensionEpoch", "resources"]) &&
        isBoundedId(entry.data.commandId) &&
        isExactBase64Url(entry.data.nonce, CHALLENGE_LENGTH) &&
        isExactBase64Url(entry.data.priorExtensionEpoch, EPOCH_LENGTH) &&
        isAdmittedReloadResources(entry.data.resources)
      );
    case PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE:
      return (
        hasExactKeys(entry.data, [
          "commandId",
          "nonce",
          "requestEntryId",
          "priorExtensionEpoch",
          "extensionEpoch",
        ]) &&
        isBoundedId(entry.data.commandId) &&
        isExactBase64Url(entry.data.nonce, CHALLENGE_LENGTH) &&
        isBoundedId(entry.data.requestEntryId) &&
        isExactBase64Url(entry.data.priorExtensionEpoch, EPOCH_LENGTH) &&
        isExactBase64Url(entry.data.extensionEpoch, EPOCH_LENGTH) &&
        entry.data.extensionEpoch !== entry.data.priorExtensionEpoch
      );
    default:
      return false;
  }
}

function validateClassificationInput(options: PiSessionFileValidationOptions): void {
  if (
    !isBoundedId(options.sessionId) ||
    !isCanonicalAbsolutePath(options.declaredPath) ||
    !isCanonicalAbsolutePath(options.allowedRoot)
  ) {
    throw new IntegrityFault(PiSessionIntegrityFailure.PATH_OUTSIDE_ALLOWED_ROOT);
  }
}

function memoryOnly(options: PiSessionFileValidationOptions): PiSessionMaterialization {
  return {
    kind: "memory_only",
    sessionId: options.sessionId,
    declaredPath: options.declaredPath,
  };
}

function invalidResult(error: unknown): PiSessionMaterialization {
  return {
    kind: "invalid",
    failure:
      error instanceof IntegrityFault ? error.failure : PiSessionIntegrityFailure.IO,
  };
}

function requireAllowedKeys(
  value: Readonly<Record<string, unknown>>,
  keys: readonly string[],
  failure = PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID,
): void {
  if (!hasOnlyKeys(value, keys)) throw new IntegrityFault(failure);
}

function hasOnlyKeys(value: Readonly<Record<string, unknown>>, keys: readonly string[]): boolean {
  const allowed = new Set(keys);
  return Object.keys(value).every((key) => allowed.has(key));
}

function hasExactKeys(value: Readonly<Record<string, unknown>>, keys: readonly string[]): boolean {
  return hasOnlyKeys(value, keys) && keys.every((key) => Object.hasOwn(value, key));
}

function requireBoundedText(value: unknown, maxBytes: number): asserts value is string {
  if (
    typeof value !== "string" ||
    value.length === 0 ||
    value.includes("\0") ||
    Buffer.byteLength(value) > maxBytes
  ) {
    throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
  }
}

function requireOptionalRecord(value: unknown): void {
  if (value !== undefined && !isRecord(value)) {
    throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
  }
}

function requireOptionalBoolean(value: unknown): void {
  if (value !== undefined && typeof value !== "boolean") {
    throw new IntegrityFault(PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID);
  }
}

function isBoundedId(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    Buffer.byteLength(value) <= MAX_ID_BYTES &&
    BOUNDED_ID_PATTERN.test(value)
  );
}

function isExactBase64Url(value: unknown, exactLength: number): value is string {
  return (
    typeof value === "string" &&
    value.length === exactLength &&
    BASE64URL_PATTERN.test(value)
  );
}

function isAdmittedReloadResources(value: unknown): value is PiReloadableResourceKind[] {
  return (
    Array.isArray(value) &&
    value.length > 0 &&
    value.length <= 16 &&
    value.every(
      (resource) =>
        typeof resource === "number" &&
        Number.isInteger(resource) &&
        ADMITTED_RELOAD_RESOURCES.has(resource as PiReloadableResourceKind),
    ) &&
    new Set(value).size === value.length
  );
}

function isCanonicalTimestamp(value: unknown): value is string {
  if (typeof value !== "string" || value.length === 0) return false;
  const parsed = new Date(value);
  return !Number.isNaN(parsed.valueOf()) && parsed.toISOString() === value;
}

function isTimestampNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isNonnegativeSafeInteger(value: unknown): value is number {
  return typeof value === "number" && Number.isSafeInteger(value) && value >= 0;
}

function isNonnegativeFiniteNumber(value: unknown): value is number {
  return typeof value === "number" && Number.isFinite(value) && value >= 0;
}

function isNonemptyString(value: unknown): value is string {
  return typeof value === "string" && value.length > 0;
}

function isCanonicalAbsolutePath(value: unknown): value is string {
  return (
    typeof value === "string" &&
    value.length > 0 &&
    !value.includes("\0") &&
    Buffer.byteLength(value) <= MAX_LOCAL_PATH_BYTES &&
    isAbsolute(value) &&
    normalize(value) === value
  );
}

function isWithinRoot(root: string, candidate: string): boolean {
  const difference = relative(root, candidate);
  return (
    difference === "" ||
    (!difference.startsWith(`..${sep}`) && difference !== ".." && !isAbsolute(difference))
  );
}

function digest(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

function treeDigest(entries: readonly StrictEntry[]): string {
  return digest(
    Buffer.from(
      JSON.stringify(entries.map((entry) => [entry.id, entry.parentId])),
      "utf8",
    ),
  );
}

function sameStrings(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isNodeError(value: unknown): value is NodeJS.ErrnoException {
  return value instanceof Error && "code" in value;
}
