import { createHash, randomBytes } from "node:crypto";
import { constants } from "node:fs";
import { mkdir, open, readFile, readdir, rename, stat } from "node:fs/promises";
import { dirname, join } from "node:path";
import {
  ContinuationContextStatus,
  ExternalEffectDisposition,
  SpawnExecutionPhase,
  type RuntimeGenerationRef,
  type SpawnGenerationClaim,
} from "@patchbay/contracts";

const JOURNAL_VERSION = 1;
const MAX_JOURNAL_BYTES = 2 * 1_048_576;
const PHASE_ORDER = new Map<SpawnExecutionPhase, number>([
  [SpawnExecutionPhase.OFFERED, 1],
  [SpawnExecutionPhase.QUIESCING_PRIOR, 2],
  [SpawnExecutionPhase.PRIOR_TERMINATED, 3],
  [SpawnExecutionPhase.LAUNCH_ATTEMPTED, 4],
  [SpawnExecutionPhase.EXTERNAL_IDENTITY_KNOWN, 5],
  [SpawnExecutionPhase.HANDSHAKE_RECONCILING, 6],
  [SpawnExecutionPhase.SUCCESS_EVIDENCE_REPORTED, 7],
]);
const FRESH_PHASE_CHAIN = Object.freeze([
  SpawnExecutionPhase.LAUNCH_ATTEMPTED,
  SpawnExecutionPhase.EXTERNAL_IDENTITY_KNOWN,
  SpawnExecutionPhase.HANDSHAKE_RECONCILING,
  SpawnExecutionPhase.SUCCESS_EVIDENCE_REPORTED,
]);
const CONTINUATION_PHASE_CHAIN = Object.freeze([
  SpawnExecutionPhase.QUIESCING_PRIOR,
  SpawnExecutionPhase.PRIOR_TERMINATED,
  ...FRESH_PHASE_CHAIN,
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

export interface PiStagedPublicationRecord {
  readonly claimOperationId: string;
  readonly runtime: RuntimeGenerationRef;
  readonly readinessDigest: string;
  readonly entryCount: number;
  readonly continuationContextStatus: ContinuationContextStatus;
  readonly entries: readonly unknown[];
  readonly leafId: string | null;
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

interface StoredStagedPublication {
  readonly runtime: JournalRuntime;
  readonly readinessDigest: string;
  readonly entryCount: number;
  readonly continuationContextStatus: ContinuationContextStatus;
  readonly entries: readonly unknown[];
  readonly leafId: string | null;
}

interface StoredJournalState {
  readonly version: typeof JOURNAL_VERSION;
  readonly claim: JournalClaim;
  readonly launchNonce: string;
  readonly targetFingerprint: string;
  readonly createdAt: string;
  readonly phases: readonly StoredPhase[];
  readonly externalIdentity?: StoredExternalIdentity;
  readonly stagedPublication?: StoredStagedPublication;
  readonly poisoned: boolean;
  readonly promotionObserved: boolean;
  readonly publicationCommitted: boolean;
}

export interface PiSpawnJournalState {
  readonly exactClaim: SpawnGenerationClaim;
  readonly launchNonce: string;
  readonly targetFingerprint: string;
  readonly phases: readonly StoredPhase[];
  readonly externalIdentity?: PiExternalIdentityRecord;
  readonly stagedPublication?: PiStagedPublicationRecord;
  readonly poisoned: boolean;
  readonly promotionObserved: boolean;
  readonly publicationCommitted: boolean;
  /** Compatibility projection: true only after local publication/cursor commit. */
  readonly promoted: boolean;
}

export interface SpawnEffectJournal {
  beginClaim(record: PiSpawnClaimJournalRecord): Promise<void>;
  recordPhase(record: PiSpawnPhaseRecord): Promise<void>;
  recordExternalIdentity(record: PiExternalIdentityRecord): Promise<void>;
  recordStagedPublication(record: PiStagedPublicationRecord): Promise<void>;
  reconcile(claimOperationId: string): Promise<PiSpawnJournalState | undefined>;
  reconcileAll(): Promise<readonly PiSpawnJournalState[]>;
  markPromotionObserved(claimOperationId: string, runtime: RuntimeGenerationRef): Promise<void>;
  markPublicationCommitted(claimOperationId: string): Promise<void>;
}

/** Promotion replay is authorized only by a complete, semantically ordered journal. */
export function assertPromotionReplayReady(
  state: PiSpawnJournalState,
): asserts state is PiSpawnJournalState & {
  readonly externalIdentity: PiExternalIdentityRecord;
  readonly stagedPublication: PiStagedPublicationRecord;
} {
  const semanticChain = validatePhaseChain(
    state.exactClaim.expectedPrior !== undefined,
    state.phases,
  );
  if (!semanticChain.complete || !state.externalIdentity || !state.stagedPublication) {
    throw new Error("spawn journal promotion replay lacks its complete semantic chain");
  }
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
        promotionObserved: false,
        publicationCommitted: false,
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
      if (state.publicationCommitted) throw new Error("spawn journal is already promoted");
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
        (candidate) => candidate.phase === SpawnExecutionPhase.LAUNCH_ATTEMPTED,
      )) {
        throw new Error("spawn journal records at most one launch attempt per claim");
      }
      const phase: StoredPhase = Object.freeze({
        phase: record.phase,
        externalEffectDisposition: record.externalEffectDisposition,
        recordedAt: record.recordedAt,
      });
      if (duplicate) {
        if (
          previous.externalEffectDisposition !== ExternalEffectDisposition.PROVED_NONE ||
          record.externalEffectDisposition !== ExternalEffectDisposition.MAY_EXIST
        ) {
          throw new Error("spawn journal repeated phase effect claim cannot weaken or contradict");
        }
        await this.#writeStored(record.claimOperationId, Object.freeze({
          ...state,
          phases: Object.freeze([...state.phases.slice(0, -1), phase]),
          poisoned: true,
        }));
        return;
      }
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

  async recordStagedPublication(record: PiStagedPublicationRecord): Promise<void> {
    await this.#serialized(async () => {
      const state = await this.#requiredStored(record.claimOperationId);
      if (!state.externalIdentity) {
        throw new Error("cannot stage spawn publication before external identity");
      }
      const runtime = runtimeToJournal(record.runtime);
      requireRuntimeMatchesClaim(runtime, state.claim);
      if (JSON.stringify(runtime) !== JSON.stringify(state.externalIdentity.runtime)) {
        throw new Error("staged spawn publication runtime mismatches external identity");
      }
      if (!/^[a-f0-9]{64}$/u.test(record.readinessDigest) ||
          !Number.isSafeInteger(record.entryCount) || record.entryCount < 0 ||
          record.continuationContextStatus === ContinuationContextStatus.UNSPECIFIED &&
            state.claim.expectedPrior !== undefined) {
        throw new Error("staged spawn publication evidence is invalid");
      }
      const entries = immutableJsonArray(record.entries);
      if (entries.length !== record.entryCount ||
          !(record.leafId === null || isBoundedText(record.leafId, 4_096))) {
        throw new Error("staged spawn publication payload is invalid");
      }
      const stagedPublication: StoredStagedPublication = Object.freeze({
        runtime,
        readinessDigest: record.readinessDigest,
        entryCount: record.entryCount,
        continuationContextStatus: record.continuationContextStatus,
        entries,
        leafId: record.leafId,
      });
      if (state.stagedPublication) {
        if (JSON.stringify(state.stagedPublication) !== JSON.stringify(stagedPublication)) {
          throw new Error("staged spawn publication conflicts with durable evidence");
        }
        return;
      }
      await this.#writeStored(record.claimOperationId, Object.freeze({
        ...state,
        stagedPublication,
      }));
    });
  }

  async reconcile(claimOperationId: string): Promise<PiSpawnJournalState | undefined> {
    return this.#serialized(async () => {
      const state = await this.#readStored(claimOperationId);
      return state ? projectStoredState(state) : undefined;
    });
  }

  async reconcileAll(): Promise<readonly PiSpawnJournalState[]> {
    return this.#serialized(async () => {
      let names: string[];
      try {
        names = await readdir(this.#directory);
      } catch (error) {
        const code = typeof error === "object" && error !== null && "code" in error ? error.code : undefined;
        if (code === "ENOENT") return [];
        throw error;
      }
      const states: PiSpawnJournalState[] = [];
      for (const name of names.sort()) {
        if (!/^[a-f0-9]{64}\.json$/u.test(name)) continue;
        const path = join(this.#directory, name);
        const stored = await readStoredPath(path);
        const claimOperationId = stored.claim.claimOperationId;
        const expectedName = `${createHash("sha256").update(claimOperationId).digest("hex")}.json`;
        if (name !== expectedName) throw new Error("spawn journal filename mismatches exact claim");
        states.push(projectStoredState(stored));
      }
      return Object.freeze(states);
    });
  }

  async markPromotionObserved(
    claimOperationId: string,
    runtime: RuntimeGenerationRef,
  ): Promise<void> {
    await this.#serialized(async () => {
      const state = await this.#requiredStored(claimOperationId);
      if (!state.externalIdentity || !state.stagedPublication) {
        throw new Error("cannot observe promotion without identity and staged publication");
      }
      const promotedRuntime = runtimeToJournal(runtime);
      if (JSON.stringify(promotedRuntime) !== JSON.stringify(state.externalIdentity.runtime)) {
        throw new Error("promoted runtime mismatches durable external identity");
      }
      if (state.promotionObserved) return;
      await this.#writeStored(claimOperationId, Object.freeze({
        ...state,
        promotionObserved: true,
      }));
    });
  }

  async markPublicationCommitted(claimOperationId: string): Promise<void> {
    await this.#serialized(async () => {
      const state = await this.#requiredStored(claimOperationId);
      if (!state.promotionObserved || !state.stagedPublication) {
        throw new Error("cannot commit publication before exact promotion");
      }
      if (state.publicationCommitted) return;
      await this.#writeStored(claimOperationId, Object.freeze({
        ...state,
        publicationCommitted: true,
      }));
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
      return await readStoredPath(path, claimOperationId);
    } catch (error) {
      const code = typeof error === "object" && error !== null && "code" in error ? error.code : undefined;
      if (code === "ENOENT") return undefined;
      throw error;
    }
  }

  async #writeStored(claimOperationId: string, state: StoredJournalState): Promise<void> {
    validateStoredState(state, claimOperationId);
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

interface ValidatedPhaseChain {
  readonly launchAttempted: boolean;
  readonly externalIdentityKnown: boolean;
  readonly handshakeReconciled: boolean;
  readonly successReported: boolean;
  readonly complete: boolean;
  readonly poisoned: boolean;
}

function validatePhaseChain(
  continuation: boolean,
  phases: readonly StoredPhase[],
): ValidatedPhaseChain {
  const expected = continuation ? CONTINUATION_PHASE_CHAIN : FRESH_PHASE_CHAIN;
  const observed = new Set<SpawnExecutionPhase>();
  let expectedIndex = 0;
  let previous: StoredPhase | undefined;
  let poisoned = false;

  for (const phase of phases) {
    if (
      !isRecord(phase) ||
      !PHASE_ORDER.has(phase.phase) ||
      !validJournalPhaseDisposition(phase.phase, phase.externalEffectDisposition)
    ) {
      throw new Error("spawn journal phase is malformed");
    }
    validateTimestamp(phase.recordedAt);
    if (observed.has(phase.phase)) {
      throw new Error("spawn journal phase is duplicated or contradictory");
    }
    if (phase.phase !== expected[expectedIndex]) {
      throw new Error("spawn journal semantic phase chain is missing or reordered");
    }
    if (
      previous?.externalEffectDisposition === ExternalEffectDisposition.MAY_EXIST &&
      previous.phase !== SpawnExecutionPhase.LAUNCH_ATTEMPTED
    ) {
      throw new Error("spawn journal continues after an ambiguous pre-launch phase");
    }
    expectedIndex += 1;
    observed.add(phase.phase);
    poisoned ||= phase.externalEffectDisposition === ExternalEffectDisposition.MAY_EXIST;
    previous = phase;
  }
  return Object.freeze({
    launchAttempted: observed.has(SpawnExecutionPhase.LAUNCH_ATTEMPTED),
    externalIdentityKnown: observed.has(SpawnExecutionPhase.EXTERNAL_IDENTITY_KNOWN),
    handshakeReconciled: observed.has(SpawnExecutionPhase.HANDSHAKE_RECONCILING),
    successReported: observed.has(SpawnExecutionPhase.SUCCESS_EVIDENCE_REPORTED),
    complete: expectedIndex === expected.length,
    poisoned,
  });
}

function validJournalPhaseDisposition(
  phase: SpawnExecutionPhase,
  disposition: ExternalEffectDisposition,
): boolean {
  switch (phase) {
    case SpawnExecutionPhase.QUIESCING_PRIOR:
    case SpawnExecutionPhase.PRIOR_TERMINATED:
      return disposition === ExternalEffectDisposition.PROVED_NONE ||
        disposition === ExternalEffectDisposition.MAY_EXIST;
    case SpawnExecutionPhase.LAUNCH_ATTEMPTED:
      return disposition === ExternalEffectDisposition.MAY_EXIST;
    case SpawnExecutionPhase.EXTERNAL_IDENTITY_KNOWN:
    case SpawnExecutionPhase.HANDSHAKE_RECONCILING:
    case SpawnExecutionPhase.SUCCESS_EVIDENCE_REPORTED:
      return disposition === ExternalEffectDisposition.IDENTIFIED;
    default:
      return false;
  }
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
  const semanticChain = validatePhaseChain(
    state.claim.expectedPrior !== undefined,
    state.phases,
  );
  if (
    typeof state.poisoned !== "boolean" ||
    typeof state.promotionObserved !== "boolean" ||
    typeof state.publicationCommitted !== "boolean" ||
    state.publicationCommitted && !state.promotionObserved ||
    state.poisoned !== semanticChain.poisoned
  ) {
    throw new Error("spawn journal flags are semantically inconsistent");
  }
  if (state.externalIdentity) {
    requireRuntimeMatchesClaim(state.externalIdentity.runtime, state.claim);
    if (
      !semanticChain.launchAttempted ||
      !Number.isSafeInteger(state.externalIdentity.pid) ||
      state.externalIdentity.pid <= 0 ||
      !isBoundedText(state.externalIdentity.processToken, 256)
    ) {
      throw new Error("spawn journal identity is malformed or precedes launch");
    }
    validateTimestamp(state.externalIdentity.recordedAt);
  }
  if (semanticChain.externalIdentityKnown && !state.externalIdentity) {
    throw new Error("spawn journal identity phase has no exact external identity");
  }
  if (state.stagedPublication) {
    requireRuntimeMatchesClaim(state.stagedPublication.runtime, state.claim);
    if (
      !state.externalIdentity ||
      !semanticChain.handshakeReconciled ||
      JSON.stringify(state.stagedPublication.runtime) !== JSON.stringify(state.externalIdentity.runtime) ||
      !/^[a-f0-9]{64}$/u.test(state.stagedPublication.readinessDigest) ||
      !Number.isSafeInteger(state.stagedPublication.entryCount) ||
      state.stagedPublication.entryCount < 0 ||
      !Array.isArray(state.stagedPublication.entries) ||
      state.stagedPublication.entries.length !== state.stagedPublication.entryCount ||
      !(state.stagedPublication.leafId === null ||
        isBoundedText(state.stagedPublication.leafId, 4_096)) ||
      !Object.values(ContinuationContextStatus).includes(state.stagedPublication.continuationContextStatus)
    ) {
      throw new Error("spawn journal staged publication is malformed or precedes handshake");
    }
  }
  if (semanticChain.successReported && !state.stagedPublication) {
    throw new Error("spawn journal success phase has no exact staged publication");
  }
  if (
    (state.promotionObserved || state.publicationCommitted) &&
    (!semanticChain.complete || !state.externalIdentity || !state.stagedPublication)
  ) {
    throw new Error("spawn journal promotion precedes its complete semantic chain");
  }
  return state;
}

function projectStoredState(state: StoredJournalState): PiSpawnJournalState {
  const claimOperationId = state.claim.claimOperationId;
  return Object.freeze({
    exactClaim: journalToClaim(state.claim),
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
    ...(state.stagedPublication
      ? {
          stagedPublication: Object.freeze({
            claimOperationId,
            runtime: journalToRuntime(state.stagedPublication.runtime),
            readinessDigest: state.stagedPublication.readinessDigest,
            entryCount: state.stagedPublication.entryCount,
            continuationContextStatus: state.stagedPublication.continuationContextStatus,
            entries: state.stagedPublication.entries,
            leafId: state.stagedPublication.leafId,
          }),
        }
      : {}),
    poisoned: state.poisoned,
    promotionObserved: state.promotionObserved,
    publicationCommitted: state.publicationCommitted,
    promoted: state.publicationCommitted,
  });
}

async function readStoredPath(
  path: string,
  claimOperationId?: string,
): Promise<StoredJournalState> {
  const metadata = await stat(path);
  if (!metadata.isFile() || metadata.size <= 0 || metadata.size > MAX_JOURNAL_BYTES) {
    throw new Error("spawn journal file is invalid");
  }
  if ((metadata.mode & 0o077) !== 0) {
    throw new Error("spawn journal file permissions are unsafe");
  }
  const parsed: unknown = JSON.parse(await readFile(path, "utf8"));
  const parsedClaimId = isRecord(parsed) && isRecord(parsed["claim"])
    ? parsed["claim"]["claimOperationId"]
    : undefined;
  if (typeof parsedClaimId !== "string") throw new Error("spawn journal claim id is malformed");
  return validateStoredState(parsed, claimOperationId ?? parsedClaimId);
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

function immutableJsonArray(entries: readonly unknown[]): readonly unknown[] {
  const serialized = JSON.stringify(entries);
  if (serialized === undefined) throw new Error("staged spawn publication is not JSON");
  const parsed: unknown = JSON.parse(serialized);
  if (!Array.isArray(parsed)) throw new Error("staged spawn publication is not an array");
  return Object.freeze(parsed);
}

function isBoundedText(value: unknown, maximum: number): value is string {
  return typeof value === "string" && value.length > 0 &&
    Buffer.byteLength(value) <= maximum && !value.includes("\0");
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
