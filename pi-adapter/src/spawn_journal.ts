import { createHash, randomBytes } from "node:crypto";
import { constants } from "node:fs";
import { mkdir, open, readFile, rename, stat } from "node:fs/promises";
import { dirname, join } from "node:path";
import {
  ExternalEffectDisposition,
  SpawnExecutionPhase,
  type RuntimeGenerationRef,
  type SpawnGenerationClaim,
} from "@patchbay/contracts";

const JOURNAL_VERSION = 1;
const MAX_JOURNAL_BYTES = 256 * 1_024;
const PHASE_ORDER = new Map<SpawnExecutionPhase, number>([
  [SpawnExecutionPhase.OFFERED, 1],
  [SpawnExecutionPhase.QUIESCING_PRIOR, 2],
  [SpawnExecutionPhase.PRIOR_TERMINATED, 3],
  [SpawnExecutionPhase.LAUNCH_ATTEMPTED, 4],
  [SpawnExecutionPhase.EXTERNAL_IDENTITY_KNOWN, 5],
  [SpawnExecutionPhase.HANDSHAKE_RECONCILING, 6],
  [SpawnExecutionPhase.SUCCESS_EVIDENCE_REPORTED, 7],
]);

interface JournalClaim {
  readonly authorityDomainId: string;
  readonly claimOperationId: string;
  readonly logicalTargetId: string;
  readonly claimedGeneration: string;
  readonly expectedPrior?: JournalRuntime;
}

interface JournalRuntime {
  readonly logicalTargetId: string;
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly runtimeSessionId: string;
  readonly generation: string;
}

export interface PiSpawnClaimJournalRecord {
  readonly exactClaim: SpawnGenerationClaim;
  readonly launchNonce: string;
  readonly targetFingerprint: string;
  readonly createdAt: string;
}

export interface PiSpawnPhaseRecord {
  readonly claimOperationId: string;
  readonly phase: SpawnExecutionPhase;
  readonly externalEffectDisposition: ExternalEffectDisposition;
  readonly recordedAt: string;
  readonly poisoned?: boolean;
}

export interface PiExternalIdentityRecord {
  readonly claimOperationId: string;
  readonly runtime: RuntimeGenerationRef;
  readonly processToken: string;
  readonly pid: number;
  readonly recordedAt: string;
}

interface StoredPhase {
  readonly phase: SpawnExecutionPhase;
  readonly externalEffectDisposition: ExternalEffectDisposition;
  readonly recordedAt: string;
}

interface StoredExternalIdentity {
  readonly runtime: JournalRuntime;
  readonly processToken: string;
  readonly pid: number;
  readonly recordedAt: string;
}

interface StoredJournalState {
  readonly version: typeof JOURNAL_VERSION;
  readonly claim: JournalClaim;
  readonly launchNonce: string;
  readonly targetFingerprint: string;
  readonly createdAt: string;
  readonly phases: readonly StoredPhase[];
  readonly externalIdentity?: StoredExternalIdentity;
  readonly poisoned: boolean;
  readonly promoted: boolean;
}

export interface PiSpawnJournalState {
  readonly exactClaim: SpawnGenerationClaim;
  readonly launchNonce: string;
  readonly targetFingerprint: string;
  readonly phases: readonly StoredPhase[];
  readonly externalIdentity?: PiExternalIdentityRecord;
  readonly poisoned: boolean;
  readonly promoted: boolean;
}

export interface SpawnEffectJournal {
  beginClaim(record: PiSpawnClaimJournalRecord): Promise<void>;
  recordPhase(record: PiSpawnPhaseRecord): Promise<void>;
  recordExternalIdentity(record: PiExternalIdentityRecord): Promise<void>;
  reconcile(claimOperationId: string): Promise<PiSpawnJournalState | undefined>;
  markPromoted(claimOperationId: string): Promise<void>;
}

/** 0600 atomic evidence journal. It never chooses or increments a generation. */
export class FileSpawnEffectJournal implements SpawnEffectJournal {
  readonly #directory: string;
  #tail: Promise<void> = Promise.resolve();

  constructor(directory: string) {
    if (!directory) throw new Error("spawn journal directory must not be empty");
    this.#directory = directory;
  }

  async beginClaim(record: PiSpawnClaimJournalRecord): Promise<void> {
    await this.#serialized(async () => {
      validateLaunchNonce(record.launchNonce);
      validateTimestamp(record.createdAt);
      if (!/^[a-f0-9]{64}$/u.test(record.targetFingerprint)) {
        throw new Error("spawn journal target fingerprint is invalid");
      }
      const claim = claimToJournal(record.exactClaim);
      const existing = await this.#readStored(claim.claimOperationId);
      const next: StoredJournalState = Object.freeze({
        version: JOURNAL_VERSION,
        claim,
        launchNonce: record.launchNonce,
        targetFingerprint: record.targetFingerprint,
        createdAt: record.createdAt,
        phases: Object.freeze([]),
        poisoned: false,
        promoted: false,
      });
      if (existing) {
        if (JSON.stringify(existing) !== JSON.stringify(next)) {
          throw new Error("spawn journal claim conflicts with existing external-effect evidence");
        }
        return;
      }
      await this.#writeStored(claim.claimOperationId, next);
    });
  }

  async recordPhase(record: PiSpawnPhaseRecord): Promise<void> {
    await this.#serialized(async () => {
      validateTimestamp(record.recordedAt);
      const order = PHASE_ORDER.get(record.phase);
      if (order === undefined) throw new Error("spawn journal phase is unsupported");
      if (record.externalEffectDisposition === ExternalEffectDisposition.UNSPECIFIED) {
        throw new Error("spawn journal external-effect disposition is unspecified");
      }
      const state = await this.#requiredStored(record.claimOperationId);
      if (state.promoted) throw new Error("spawn journal is already promoted");
      const previous = state.phases.at(-1);
      const previousOrder = previous ? PHASE_ORDER.get(previous.phase) : undefined;
      if (previousOrder !== undefined && order < previousOrder) {
        throw new Error("spawn journal phase cannot regress");
      }
      const duplicate = previous?.phase === record.phase;
      if (
        duplicate &&
        previous.externalEffectDisposition === record.externalEffectDisposition
      ) {
        return;
      }
      if (record.phase === SpawnExecutionPhase.LAUNCH_ATTEMPTED && state.phases.some(
        (phase) => phase.phase === SpawnExecutionPhase.LAUNCH_ATTEMPTED,
      )) {
        throw new Error("spawn journal records at most one launch attempt per claim");
      }
      const phase: StoredPhase = Object.freeze({
        phase: record.phase,
        externalEffectDisposition: record.externalEffectDisposition,
        recordedAt: record.recordedAt,
      });
      await this.#writeStored(record.claimOperationId, Object.freeze({
        ...state,
        phases: Object.freeze([...state.phases, phase]),
        poisoned: state.poisoned || record.poisoned === true ||
          record.externalEffectDisposition === ExternalEffectDisposition.MAY_EXIST,
      }));
    });
  }

  async recordExternalIdentity(record: PiExternalIdentityRecord): Promise<void> {
    await this.#serialized(async () => {
      validateTimestamp(record.recordedAt);
      if (!Number.isSafeInteger(record.pid) || record.pid <= 0) {
        throw new Error("spawn journal external pid is invalid");
      }
      if (!record.processToken || record.processToken.length > 256) {
        throw new Error("spawn journal process token is invalid");
      }
      const state = await this.#requiredStored(record.claimOperationId);
      const runtime = runtimeToJournal(record.runtime);
      requireRuntimeMatchesClaim(runtime, state.claim);
      const externalIdentity: StoredExternalIdentity = Object.freeze({
        runtime,
        processToken: record.processToken,
        pid: record.pid,
        recordedAt: record.recordedAt,
      });
      if (state.externalIdentity) {
        if (JSON.stringify(state.externalIdentity) !== JSON.stringify(externalIdentity)) {
          throw new Error("spawn journal external identity conflicts with the recorded claim");
        }
        return;
      }
      await this.#writeStored(record.claimOperationId, Object.freeze({
        ...state,
        externalIdentity,
      }));
    });
  }

  async reconcile(claimOperationId: string): Promise<PiSpawnJournalState | undefined> {
    return this.#serialized(async () => {
      const state = await this.#readStored(claimOperationId);
      if (!state) return undefined;
      const exactClaim = journalToClaim(state.claim);
      return Object.freeze({
        exactClaim,
        launchNonce: state.launchNonce,
        targetFingerprint: state.targetFingerprint,
        phases: state.phases,
        ...(state.externalIdentity
          ? {
              externalIdentity: Object.freeze({
                claimOperationId,
                runtime: journalToRuntime(state.externalIdentity.runtime),
                processToken: state.externalIdentity.processToken,
                pid: state.externalIdentity.pid,
                recordedAt: state.externalIdentity.recordedAt,
              }),
            }
          : {}),
        poisoned: state.poisoned,
        promoted: state.promoted,
      });
    });
  }

  async markPromoted(claimOperationId: string): Promise<void> {
    await this.#serialized(async () => {
      const state = await this.#requiredStored(claimOperationId);
      if (!state.externalIdentity) throw new Error("cannot promote a spawn journal without identity");
      if (state.promoted) return;
      await this.#writeStored(claimOperationId, Object.freeze({ ...state, promoted: true }));
    });
  }

  async #requiredStored(claimOperationId: string): Promise<StoredJournalState> {
    const state = await this.#readStored(claimOperationId);
    if (!state) throw new Error("spawn journal claim has not been recorded");
    return state;
  }

  async #readStored(claimOperationId: string): Promise<StoredJournalState | undefined> {
    const path = this.#pathFor(claimOperationId);
    try {
      const metadata = await stat(path);
      if (!metadata.isFile() || metadata.size <= 0 || metadata.size > MAX_JOURNAL_BYTES) {
        throw new Error("spawn journal file is invalid");
      }
      if ((metadata.mode & 0o077) !== 0) throw new Error("spawn journal file permissions are unsafe");
      const parsed: unknown = JSON.parse(await readFile(path, "utf8"));
      return validateStoredState(parsed, claimOperationId);
    } catch (error) {
      const code = typeof error === "object" && error !== null && "code" in error ? error.code : undefined;
      if (code === "ENOENT") return undefined;
      throw error;
    }
  }

  async #writeStored(claimOperationId: string, state: StoredJournalState): Promise<void> {
    const path = this.#pathFor(claimOperationId);
    await mkdir(dirname(path), { recursive: true, mode: 0o700 });
    const directoryHandle = await open(dirname(path), constants.O_RDONLY);
    const temporary = `${path}.${process.pid}.${randomBytes(8).toString("hex")}.tmp`;
    let handle;
    try {
      handle = await open(temporary, "wx", 0o600);
      const bytes = Buffer.from(`${JSON.stringify(state)}\n`, "utf8");
      if (bytes.byteLength > MAX_JOURNAL_BYTES) throw new Error("spawn journal exceeds its bound");
      await handle.writeFile(bytes);
      await handle.sync();
      await handle.close();
      handle = undefined;
      await rename(temporary, path);
      await directoryHandle.sync();
    } finally {
      await handle?.close();
      await directoryHandle.close();
      try {
        const temporaryHandle = await open(temporary, constants.O_RDONLY);
        await temporaryHandle.close();
      } catch {
        // The successful rename removes the temporary name. Failed writes leave
        // a bounded 0600 file which is ignored by reconciliation.
      }
    }
  }

  #pathFor(claimOperationId: string): string {
    if (!claimOperationId || claimOperationId.length > 1_024) {
      throw new Error("spawn journal claim operation id is invalid");
    }
    const digest = createHash("sha256").update(claimOperationId).digest("hex");
    return join(this.#directory, `${digest}.json`);
  }

  async #serialized<T>(action: () => Promise<T>): Promise<T> {
    const previous = this.#tail;
    let release!: () => void;
    this.#tail = new Promise<void>((resolve) => {
      release = resolve;
    });
    await previous;
    try {
      return await action();
    } finally {
      release();
    }
  }
}

export function spawnTargetFingerprint(parts: readonly string[]): string {
  const hash = createHash("sha256");
  for (const part of parts) {
    hash.update(String(Buffer.byteLength(part)));
    hash.update(":");
    hash.update(part);
    hash.update("\0");
  }
  return hash.digest("hex");
}

function claimToJournal(claim: SpawnGenerationClaim): JournalClaim {
  const authorityDomainId = claim.authorityDomainId?.value;
  const claimOperationId = claim.claimOperationId?.value;
  const logicalTargetId = claim.logicalTargetId?.value;
  const claimedGeneration = claim.claimedGeneration?.value;
  if (!authorityDomainId || !claimOperationId || !logicalTargetId || !claimedGeneration || claimedGeneration <= 0n) {
    throw new Error("spawn journal exact claim is incomplete");
  }
  return Object.freeze({
    authorityDomainId,
    claimOperationId,
    logicalTargetId,
    claimedGeneration: claimedGeneration.toString(),
    ...(claim.expectedPrior ? { expectedPrior: runtimeToJournal(claim.expectedPrior) } : {}),
  });
}

function runtimeToJournal(runtime: RuntimeGenerationRef): JournalRuntime {
  const logicalTargetId = runtime.logicalTargetId?.value;
  const external = runtime.externalRuntime;
  const adapterId = external?.adapterId?.value;
  const runtimeSessionId = external?.runtimeSessionId?.value;
  const generation = external?.generation?.value;
  if (!logicalTargetId || !adapterId || !external?.deploymentScope || !runtimeSessionId || !generation || generation <= 0n) {
    throw new Error("spawn journal runtime identity is incomplete");
  }
  return Object.freeze({
    logicalTargetId,
    adapterId,
    deploymentScope: external.deploymentScope,
    runtimeSessionId,
    generation: generation.toString(),
  });
}

function journalToClaim(claim: JournalClaim): SpawnGenerationClaim {
  return {
    $typeName: "patchbay.SpawnGenerationClaim",
    authorityDomainId: { $typeName: "patchbay.AuthorityDomainId", value: claim.authorityDomainId },
    claimOperationId: { $typeName: "patchbay.CommandId", value: claim.claimOperationId },
    logicalTargetId: { $typeName: "patchbay.LogicalTargetId", value: claim.logicalTargetId },
    claimedGeneration: { $typeName: "patchbay.Generation", value: BigInt(claim.claimedGeneration) },
    ...(claim.expectedPrior ? { expectedPrior: journalToRuntime(claim.expectedPrior) } : {}),
  };
}

function journalToRuntime(runtime: JournalRuntime): RuntimeGenerationRef {
  return {
    $typeName: "patchbay.RuntimeGenerationRef",
    logicalTargetId: { $typeName: "patchbay.LogicalTargetId", value: runtime.logicalTargetId },
    externalRuntime: {
      $typeName: "patchbay.ExternalRuntimeRef",
      adapterId: { $typeName: "patchbay.AdapterId", value: runtime.adapterId },
      deploymentScope: runtime.deploymentScope,
      runtimeSessionId: { $typeName: "patchbay.RuntimeSessionId", value: runtime.runtimeSessionId },
      generation: { $typeName: "patchbay.Generation", value: BigInt(runtime.generation) },
    },
  };
}

function validateStoredState(value: unknown, claimOperationId: string): StoredJournalState {
  if (!isRecord(value) || value["version"] !== JOURNAL_VERSION || !isRecord(value["claim"])) {
    throw new Error("spawn journal state is malformed");
  }
  const state = value as unknown as StoredJournalState;
  const claim = journalToClaim(state.claim);
  if (claim.claimOperationId?.value !== claimOperationId) throw new Error("spawn journal claim id mismatch");
  validateLaunchNonce(state.launchNonce);
  validateTimestamp(state.createdAt);
  if (!/^[a-f0-9]{64}$/u.test(state.targetFingerprint) || !Array.isArray(state.phases)) {
    throw new Error("spawn journal state is malformed");
  }
  for (const phase of state.phases) {
    if (!PHASE_ORDER.has(phase.phase) || phase.externalEffectDisposition === ExternalEffectDisposition.UNSPECIFIED) {
      throw new Error("spawn journal phase is malformed");
    }
    validateTimestamp(phase.recordedAt);
  }
  if (typeof state.poisoned !== "boolean" || typeof state.promoted !== "boolean") {
    throw new Error("spawn journal flags are malformed");
  }
  if (state.externalIdentity) {
    requireRuntimeMatchesClaim(state.externalIdentity.runtime, state.claim);
    if (!Number.isSafeInteger(state.externalIdentity.pid) || state.externalIdentity.pid <= 0) {
      throw new Error("spawn journal identity is malformed");
    }
    validateTimestamp(state.externalIdentity.recordedAt);
  }
  return state;
}

function requireRuntimeMatchesClaim(runtime: JournalRuntime, claim: JournalClaim): void {
  if (runtime.logicalTargetId !== claim.logicalTargetId || runtime.generation !== claim.claimedGeneration) {
    throw new Error("spawn journal external identity does not match the exact claim");
  }
}

function validateLaunchNonce(value: string): void {
  if (!/^[A-Za-z0-9_-]{43}$/u.test(value)) throw new Error("spawn journal launch nonce is invalid");
}

function validateTimestamp(value: string): void {
  const parsed = new Date(value);
  if (Number.isNaN(parsed.valueOf()) || parsed.toISOString() !== value) {
    throw new Error("spawn journal timestamp is invalid");
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
