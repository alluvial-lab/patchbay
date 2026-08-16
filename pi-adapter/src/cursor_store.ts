import { createHash, randomBytes } from "node:crypto";
import { constants } from "node:fs";
import { mkdir, open, readFile, realpath, rename, stat, unlink } from "node:fs/promises";
import { isAbsolute, normalize, relative, sep } from "node:path";
import {
  externalCursorScopeKey,
  type AtomicExternalCursorProjectionStore,
  type AtomicExternalCursorProjectionStoreTestInstrumentation,
  type ExternalCursorProjectionRecord,
  type ExternalCursorScope,
  type ProjectionReplacement,
} from "@patchbay/operator-domain/reconciliation/external-cursor";
import {
  piProjectedEntriesEqual,
  piProjectionLeavesEqual,
  piTreeDigest,
  validateProjectedEntries,
  type PiProjectedEntry,
  type PiProjectionCursor,
  type PiProjectionLeaf,
} from "./pi_projection.js";

const STORE_VERSION = 1;
const MAX_STORE_BYTES = 16 * 1_048_576;
const ID_PATTERN = /^[^\0]{1,1024}$/u;

export interface PiSessionContinuityKey {
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly piSessionId: string;
  readonly sessionRootId: string;
  /** Adapter-local only. This field must never enter a generated envelope. */
  readonly rootRelativePath: string;
}

export interface PiExternalCursorScope extends ExternalCursorScope {
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly externalContinuityId: string;
}

export type PiCursorProjectionRecord = ExternalCursorProjectionRecord<
  PiProjectedEntry,
  PiProjectionCursor,
  PiProjectionLeaf
>;

interface StoredScope {
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly externalContinuityId: string;
}

interface StoredProjectedEntry {
  readonly stableEntryId: string;
  readonly parentEntryId: string | null;
  readonly contentDigest: string;
  readonly presentationItems: readonly {
    readonly membershipId: string;
    readonly transcriptEventJson: string;
  }[];
}

interface StoredProjection {
  readonly replacementEpoch: string;
  readonly exactEntries: readonly StoredProjectedEntry[];
  readonly cursor: string | null;
  readonly leaf: { readonly entryId: string | null; readonly treeDigest: string };
}

interface StoredPending {
  readonly kind: "fetching" | "staged";
  readonly replacementEpoch: string;
  readonly exactEntries?: readonly StoredProjectedEntry[];
  readonly leaf?: { readonly entryId: string | null; readonly treeDigest: string };
}

interface StoredRecord {
  readonly recordVersion: string;
  readonly freshness: "current" | "stale";
  readonly projection: StoredProjection;
  readonly pendingReplacement?: StoredPending;
}

interface StoredCursorState {
  readonly version: typeof STORE_VERSION;
  readonly scope: StoredScope;
  readonly logicalTargetId: string;
  readonly record?: StoredRecord;
}

export const PI_CURSOR_VALUES = Object.freeze({
  entryIdentity: (entry: PiProjectedEntry): string => entry.stableEntryId,
  entriesEqual: piProjectedEntriesEqual,
  cursorsEqual: (left: PiProjectionCursor, right: PiProjectionCursor): boolean => left === right,
  leavesEqual: piProjectionLeavesEqual,
});

/**
 * Derive the wire-visible opaque continuity id from verified Pi identity. The
 * canonical root-relative path is returned only in the local key.
 */
export async function derivePiSessionContinuityKey(input: {
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly piSessionId: string;
  readonly sessionRootId: string;
  readonly configuredSessionRoot: string;
  readonly canonicalSessionPath: string;
}): Promise<{ readonly key: PiSessionContinuityKey; readonly scope: PiExternalCursorScope }> {
  for (const [name, value] of Object.entries({
    adapterId: input.adapterId,
    deploymentScope: input.deploymentScope,
    piSessionId: input.piSessionId,
    sessionRootId: input.sessionRootId,
  })) {
    if (!ID_PATTERN.test(value)) throw new Error(`Pi continuity ${name} is invalid`);
  }
  if (!isAbsolute(input.configuredSessionRoot) || !isAbsolute(input.canonicalSessionPath)) {
    throw new Error("Pi continuity paths must be absolute");
  }
  const canonicalRoot = await realpath(input.configuredSessionRoot);
  const canonicalPath = normalize(input.canonicalSessionPath);
  if (canonicalPath !== input.canonicalSessionPath) {
    throw new Error("Pi continuity session path is not canonical");
  }
  const rootRelativePath = relative(canonicalRoot, canonicalPath);
  if (
    rootRelativePath.length === 0
    || rootRelativePath === ".."
    || rootRelativePath.startsWith(`..${sep}`)
    || isAbsolute(rootRelativePath)
    || rootRelativePath.includes("\0")
  ) {
    throw new Error("Pi continuity session path is outside its configured root");
  }
  const key = Object.freeze({
    adapterId: input.adapterId,
    deploymentScope: input.deploymentScope,
    piSessionId: input.piSessionId,
    sessionRootId: input.sessionRootId,
    rootRelativePath,
  });
  const externalContinuityId = `pi1:${lengthFramedDigest([
    input.piSessionId,
    input.sessionRootId,
    rootRelativePath,
  ])}`;
  return Object.freeze({
    key,
    scope: Object.freeze({
      adapterId: input.adapterId,
      deploymentScope: input.deploymentScope,
      externalContinuityId,
    }),
  });
}

/** 0600, temp-fsync-rename, process-serialized CAS store. */
export class FilePiCursorStore implements AtomicExternalCursorProjectionStore<
  PiExternalCursorScope,
  PiProjectedEntry,
  PiProjectionCursor,
  PiProjectionLeaf
> {
  readonly #directory: string;
  #tail: Promise<void> = Promise.resolve();
  #pauseAfterEarliestObservableMutation: (() => Promise<void>) | undefined;

  readonly conformanceTestInstrumentation: AtomicExternalCursorProjectionStoreTestInstrumentation = {
    setPauseAfterEarliestObservableMutation: (pause): void => {
      this.#pauseAfterEarliestObservableMutation = pause;
    },
  };

  constructor(directory: string) {
    if (!directory || !isAbsolute(directory)) throw new Error("Pi cursor-store directory must be absolute");
    this.#directory = normalize(directory);
  }

  async bindLogicalTarget(scope: PiExternalCursorScope, logicalTargetId: string): Promise<void> {
    validateScope(scope);
    validateLogicalTarget(logicalTargetId);
    await this.#serialized(async () => {
      const current = await this.#read(scope);
      if (current) {
        requireBinding(current, scope, logicalTargetId);
        return;
      }
      await this.#write(scope, {
        version: STORE_VERSION,
        scope: storedScope(scope),
        logicalTargetId,
      });
    });
  }

  /**
   * Install the only legal first durable shape for unknown-cursor recovery: an
   * old stale baseline with a null cursor and an immediately pending fetch.
   */
  async ensureReplacementBaseline(
    scope: PiExternalCursorScope,
    logicalTargetId: string,
    baseline: ProjectionReplacement<PiProjectedEntry, PiProjectionCursor, PiProjectionLeaf>,
  ): Promise<void> {
    validateScope(scope);
    validateLogicalTarget(logicalTargetId);
    const safeBaseline = validatedProjection({ ...baseline, cursor: null });
    await this.#serialized(async () => {
      const current = await this.#read(scope);
      if (!current) throw new Error("Pi continuity must be bound before cursor initialization");
      requireBinding(current, scope, logicalTargetId);
      if (current.record) return;
      const record: PiCursorProjectionRecord = {
        recordVersion: 1n,
        freshness: "stale",
        projection: safeBaseline,
        pendingReplacement: {
          kind: "fetching",
          replacementEpoch: safeBaseline.replacementEpoch + 1n,
        },
      };
      await this.#write(scope, { ...current, record: recordToStored(record) });
    });
  }

  /** Test/setup seam: create an already-current record under its reverse binding. */
  async initializeCurrent(
    scope: PiExternalCursorScope,
    logicalTargetId: string,
    record: PiCursorProjectionRecord,
  ): Promise<void> {
    validateScope(scope);
    validateLogicalTarget(logicalTargetId);
    const safe = validateRecord(record);
    await this.#serialized(async () => {
      const current = await this.#read(scope);
      if (current) {
        requireBinding(current, scope, logicalTargetId);
        if (current.record && !recordsEqual(recordFromStored(current.record), safe)) {
          throw new Error("Pi cursor store is already initialized with different content");
        }
        if (current.record) return;
      }
      await this.#write(scope, {
        version: STORE_VERSION,
        scope: storedScope(scope),
        logicalTargetId,
        record: recordToStored(safe),
      });
    });
  }

  async load(scope: PiExternalCursorScope): Promise<PiCursorProjectionRecord | undefined> {
    validateScope(scope);
    const state = await this.#read(scope);
    return state?.record ? cloneRecord(recordFromStored(state.record)) : undefined;
  }

  async compareAndSwap(
    scope: PiExternalCursorScope,
    expectedRecordVersion: bigint,
    next: PiCursorProjectionRecord,
  ): Promise<void> {
    validateScope(scope);
    const safeNext = validateRecord(next);
    if (safeNext.recordVersion !== expectedRecordVersion + 1n) {
      throw new Error("Pi cursor CAS must advance recordVersion exactly once");
    }
    await this.#serialized(async () => {
      const current = await this.#read(scope);
      if (!current?.record) throw new Error("Pi cursor CAS has no initialized record");
      const record = recordFromStored(current.record);
      if (record.recordVersion !== expectedRecordVersion) {
        throw new Error("Pi cursor compare-and-swap version mismatch");
      }
      await this.#write(scope, { ...current, record: recordToStored(safeNext) });
      await this.#pauseAfterEarliestObservableMutation?.();
    });
  }

  async logicalTarget(scope: PiExternalCursorScope): Promise<string | undefined> {
    validateScope(scope);
    return (await this.#read(scope))?.logicalTargetId;
  }

  async #read(scope: PiExternalCursorScope): Promise<StoredCursorState | undefined> {
    const path = this.#path(scope);
    try {
      const metadata = await stat(path);
      if (!metadata.isFile() || metadata.size <= 0 || metadata.size > MAX_STORE_BYTES) {
        throw new Error("Pi cursor-store file is invalid");
      }
      if ((metadata.mode & 0o077) !== 0) throw new Error("Pi cursor-store file permissions are unsafe");
      const parsed: unknown = JSON.parse(await readFile(path, "utf8"));
      return validateStoredState(parsed, scope);
    } catch (error) {
      if (errorCode(error) === "ENOENT") return undefined;
      throw error;
    }
  }

  async #write(scope: PiExternalCursorScope, state: StoredCursorState): Promise<void> {
    const safe = validateStoredState(state, scope);
    await mkdir(this.#directory, { recursive: true, mode: 0o700 });
    const directoryMetadata = await stat(this.#directory);
    if (!directoryMetadata.isDirectory() || (directoryMetadata.mode & 0o077) !== 0) {
      throw new Error("Pi cursor-store directory permissions are unsafe");
    }
    const path = this.#path(scope);
    const temporary = `${path}.${process.pid}.${randomBytes(8).toString("hex")}.tmp`;
    const directoryHandle = await open(this.#directory, constants.O_RDONLY);
    let temporaryHandle;
    try {
      temporaryHandle = await open(temporary, "wx", 0o600);
      const bytes = Buffer.from(`${JSON.stringify(safe)}\n`, "utf8");
      if (bytes.byteLength > MAX_STORE_BYTES) throw new Error("Pi cursor-store record exceeds its bound");
      await temporaryHandle.writeFile(bytes);
      await temporaryHandle.sync();
      await temporaryHandle.close();
      temporaryHandle = undefined;
      await rename(temporary, path);
      // rename is the first externally visible mutation; the conformance hook
      // runs in compareAndSwap immediately after this complete record appears.
      await directoryHandle.sync();
    } finally {
      await temporaryHandle?.close();
      await directoryHandle.close();
      await unlink(temporary).catch((error: unknown) => {
        if (errorCode(error) !== "ENOENT") throw error;
      });
    }
  }

  #path(scope: PiExternalCursorScope): string {
    return `${this.#directory}/${createHash("sha256").update(externalCursorScopeKey(scope)).digest("hex")}.json`;
  }

  async #serialized<T>(action: () => Promise<T>): Promise<T> {
    const previous = this.#tail;
    let release!: () => void;
    this.#tail = new Promise<void>((resolve) => { release = resolve; });
    await previous;
    try {
      return await action();
    } finally {
      release();
    }
  }
}

function storedScope(scope: PiExternalCursorScope): StoredScope {
  return {
    adapterId: scope.adapterId,
    deploymentScope: scope.deploymentScope,
    externalContinuityId: scope.externalContinuityId,
  };
}

function validateStoredState(value: unknown, scope: PiExternalCursorScope): StoredCursorState {
  if (!isRecord(value) || value["version"] !== STORE_VERSION || !isRecord(value["scope"])) {
    throw new Error("Pi cursor-store state is malformed");
  }
  const candidate = value as unknown as StoredCursorState;
  if (
    candidate.scope.adapterId !== scope.adapterId
    || candidate.scope.deploymentScope !== scope.deploymentScope
    || candidate.scope.externalContinuityId !== scope.externalContinuityId
  ) {
    throw new Error("Pi cursor-store scope does not match its continuity filename");
  }
  validateLogicalTarget(candidate.logicalTargetId);
  if (candidate.record) recordFromStored(candidate.record);
  return candidate;
}

function requireBinding(
  state: StoredCursorState,
  scope: PiExternalCursorScope,
  logicalTargetId: string,
): void {
  validateStoredState(state, scope);
  if (state.logicalTargetId !== logicalTargetId) {
    throw new Error("Pi continuity is already bound to another logical target");
  }
}

function recordToStored(record: PiCursorProjectionRecord): StoredRecord {
  const pending = record.pendingReplacement;
  return {
    recordVersion: record.recordVersion.toString(),
    freshness: record.freshness,
    projection: projectionToStored(record.projection),
    ...(pending ? {
      pendingReplacement: pending.kind === "fetching"
        ? { kind: "fetching", replacementEpoch: pending.replacementEpoch.toString() }
        : {
            kind: "staged",
            replacementEpoch: pending.replacementEpoch.toString(),
            exactEntries: pending.exactEntries,
            leaf: pending.leaf,
          },
    } : {}),
  };
}

function recordFromStored(record: StoredRecord): PiCursorProjectionRecord {
  if (!isRecord(record) || (record.freshness !== "current" && record.freshness !== "stale")) {
    throw new Error("Pi cursor-store record is malformed");
  }
  const pending = record.pendingReplacement;
  return validateRecord({
    recordVersion: decimalBigint(record.recordVersion, "record version"),
    freshness: record.freshness,
    projection: projectionFromStored(record.projection),
    ...(pending ? {
      pendingReplacement: pending.kind === "fetching"
        ? {
            kind: "fetching" as const,
            replacementEpoch: decimalBigint(pending.replacementEpoch, "pending epoch"),
          }
        : pending.kind === "staged" && pending.exactEntries && pending.leaf
          ? {
              kind: "staged" as const,
              replacementEpoch: decimalBigint(pending.replacementEpoch, "pending epoch"),
              exactEntries: validateProjectedEntries(pending.exactEntries),
              leaf: validateLeaf(pending.leaf),
            }
          : (() => { throw new Error("Pi cursor-store pending replacement is malformed"); })(),
    } : {}),
  });
}

function projectionToStored(
  projection: ProjectionReplacement<PiProjectedEntry, PiProjectionCursor, PiProjectionLeaf>,
): StoredProjection {
  return {
    replacementEpoch: projection.replacementEpoch.toString(),
    exactEntries: projection.exactEntries,
    cursor: projection.cursor,
    leaf: projection.leaf,
  };
}

function projectionFromStored(projection: StoredProjection) {
  if (!isRecord(projection)) throw new Error("Pi cursor-store projection is malformed");
  return validatedProjection({
    replacementEpoch: decimalBigint(projection.replacementEpoch, "replacement epoch"),
    exactEntries: validateProjectedEntries(projection.exactEntries),
    cursor: validateNullableId(projection.cursor, "cursor"),
    leaf: validateLeaf(projection.leaf),
  });
}

function validatedProjection(
  projection: ProjectionReplacement<PiProjectedEntry, PiProjectionCursor, PiProjectionLeaf>,
): ProjectionReplacement<PiProjectedEntry, PiProjectionCursor, PiProjectionLeaf> {
  if (projection.replacementEpoch < 0n) throw new Error("Pi cursor replacement epoch is invalid");
  return Object.freeze({
    replacementEpoch: projection.replacementEpoch,
    exactEntries: validateProjectedEntries(projection.exactEntries),
    cursor: validateNullableId(projection.cursor, "cursor"),
    leaf: validateLeaf(projection.leaf),
  });
}

function validateRecord(record: PiCursorProjectionRecord): PiCursorProjectionRecord {
  if (record.recordVersion <= 0n) throw new Error("Pi cursor record version must be positive");
  const projection = validatedProjection(record.projection);
  validateExactMembership(projection.exactEntries, projection.leaf);
  if (
    record.freshness === "current"
    && projection.cursor !== (projection.exactEntries.at(-1)?.stableEntryId ?? null)
  ) {
    throw new Error("Pi current cursor is not the exact projection tail");
  }
  const pending = record.pendingReplacement;
  if (pending) {
    if (record.freshness !== "stale" || pending.replacementEpoch !== projection.replacementEpoch + 1n) {
      throw new Error("Pi pending replacement is not the immediate stale successor");
    }
    if (pending.kind === "staged") {
      const exactEntries = validateProjectedEntries(pending.exactEntries);
      const leaf = validateLeaf(pending.leaf);
      validateExactMembership(exactEntries, leaf);
    }
  }
  return cloneRecord({ ...record, projection });
}

function validateExactMembership(
  entries: readonly PiProjectedEntry[],
  leaf: PiProjectionLeaf,
): void {
  if (piTreeDigest(entries) !== leaf.treeDigest) {
    throw new Error("Pi exact projection tree digest is invalid");
  }
  const ids = new Set<string>();
  let roots = 0;
  for (const entry of entries) {
    if (entry.parentEntryId === null) roots += 1;
    else if (entry.parentEntryId === entry.stableEntryId || !ids.has(entry.parentEntryId)) {
      throw new Error("Pi exact projection has an invalid append-order parent");
    }
    ids.add(entry.stableEntryId);
  }
  if ((entries.length > 0 && roots !== 1) || (entries.length === 0 && roots !== 0)) {
    throw new Error("Pi exact projection has an invalid root count");
  }
  if (
    (entries.length === 0 && leaf.entryId !== null)
    || (entries.length > 0 && (leaf.entryId === null || !ids.has(leaf.entryId)))
  ) {
    throw new Error("Pi exact projection leaf is absent from membership");
  }
}

function validateLeaf(value: PiProjectionLeaf): PiProjectionLeaf {
  if (!isRecord(value) || !/^[a-f0-9]{64}$/u.test(value.treeDigest)) {
    throw new Error("Pi projection leaf is invalid");
  }
  return Object.freeze({
    entryId: validateNullableId(value.entryId, "leaf entry"),
    treeDigest: value.treeDigest,
  });
}

function validateNullableId(value: unknown, field: string): string | null {
  if (value === null) return null;
  if (typeof value !== "string" || !ID_PATTERN.test(value)) throw new Error(`Pi projection ${field} is invalid`);
  return value;
}

function validateScope(scope: PiExternalCursorScope): void {
  externalCursorScopeKey(scope);
  if (!scope.externalContinuityId.startsWith("pi1:")) throw new Error("Pi cursor scope is not a Pi continuity digest");
}

function validateLogicalTarget(value: string): void {
  if (!ID_PATTERN.test(value)) throw new Error("Pi logical target binding is invalid");
}

function decimalBigint(value: string, field: string): bigint {
  if (typeof value !== "string" || !/^(?:0|[1-9][0-9]*)$/u.test(value)) {
    throw new Error(`Pi cursor ${field} is malformed`);
  }
  return BigInt(value);
}

function recordsEqual(left: PiCursorProjectionRecord, right: PiCursorProjectionRecord): boolean {
  return JSON.stringify(recordToStored(left)) === JSON.stringify(recordToStored(right));
}

function cloneRecord(record: PiCursorProjectionRecord): PiCursorProjectionRecord {
  const pending = record.pendingReplacement;
  return {
    recordVersion: record.recordVersion,
    freshness: record.freshness,
    projection: {
      ...record.projection,
      exactEntries: record.projection.exactEntries.map(cloneEntry),
      leaf: { ...record.projection.leaf },
    },
    ...(pending ? {
      pendingReplacement: pending.kind === "fetching"
        ? { ...pending }
        : {
            ...pending,
            exactEntries: pending.exactEntries.map(cloneEntry),
            leaf: { ...pending.leaf },
          },
    } : {}),
  };
}

function cloneEntry(entry: PiProjectedEntry): PiProjectedEntry {
  return {
    ...entry,
    presentationItems: entry.presentationItems.map((item) => ({ ...item })),
  };
}

function lengthFramedDigest(parts: readonly string[]): string {
  const hash = createHash("sha256");
  for (const part of parts) {
    hash.update(String(Buffer.byteLength(part)));
    hash.update(":");
    hash.update(part);
    hash.update("\0");
  }
  return hash.digest("base64url");
}

function errorCode(error: unknown): unknown {
  return typeof error === "object" && error !== null && "code" in error ? error.code : undefined;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
