import type { RuntimeGenerationRef, SpawnGenerationClaim } from "@patchbay/contracts";
import type { PiSession } from "./pi_session.js";
import { RuntimeActionGate } from "./runtime_action_gate.js";
import type { TranscriptEvent } from "./transcript_event.js";

export interface RuntimeSessionConfig {
  readonly runtimeSessionId: string;
  readonly deploymentScope: string;
  readonly project?: string;
  readonly cwd: string;
  readonly name?: string;
  readonly logicalTargetId?: string;
  /** Adapter-local root used only for materialization/cursor verification. */
  readonly sessionRoot?: string;
}

export interface RuntimeSessionEntry extends RuntimeSessionConfig {
  readonly session: PiSession;
  readonly attachmentToken: symbol;
}

export type TranscriptObserver = (entry: RuntimeSessionEntry, event: TranscriptEvent) => void;
export type ModelChangeObserver = (entry: RuntimeSessionEntry, model: string) => void;
export type LifecycleObserver = (
  entry: RuntimeSessionEntry,
  event: Parameters<Parameters<PiSession["onLifecycle"]>[0]>[0],
) => void;
export type PersistedEntryObserver = (entry: RuntimeSessionEntry) => void;

interface OwnedRuntimeSessionEntry extends RuntimeSessionEntry {
  readonly promotionClaimOperationId?: string;
  readonly unsubscribeTranscript: () => void;
  readonly unsubscribeModelChange: () => void;
  readonly unsubscribeLifecycle: () => void;
  readonly unsubscribePersistedEntry: () => void;
  active: boolean;
}

interface CandidateEntry {
  readonly exactClaim: SpawnGenerationClaim;
  readonly entry: RuntimeSessionEntry;
  readonly observeTranscript: TranscriptObserver;
  readonly observeModelChange: ModelChangeObserver;
  readonly observeLifecycle: LifecycleObserver;
  readonly observePersistedEntry: PersistedEntryObserver;
}

/** Current/candidate registry with attachment-token and replacement fencing around every callback. */
export class SessionRegistry {
  readonly #entries = new Map<string, OwnedRuntimeSessionEntry>();
  readonly #byLogicalTarget = new Map<string, OwnedRuntimeSessionEntry>();
  readonly #candidates = new Map<string, CandidateEntry>();
  readonly #gates = new Map<string, RuntimeActionGate>();

  gateFor(logicalTargetId: string): RuntimeActionGate {
    if (!logicalTargetId || logicalTargetId.length > 1_024) {
      throw new Error("logical target id is invalid");
    }
    let gate = this.#gates.get(logicalTargetId);
    if (!gate) {
      gate = new RuntimeActionGate();
      this.#gates.set(logicalTargetId, gate);
    }
    return gate;
  }

  register(
    config: RuntimeSessionConfig,
    session: PiSession,
    observeTranscript: TranscriptObserver,
    observeModelChange: ModelChangeObserver,
    observeLifecycle: LifecycleObserver = () => undefined,
    observePersistedEntry: PersistedEntryObserver = () => undefined,
  ): RuntimeSessionEntry {
    const entry = this.#ownedEntry(
      config,
      session,
      observeTranscript,
      observeModelChange,
      observeLifecycle,
      observePersistedEntry,
    );
    this.#installCurrent(entry);
    return entry;
  }

  stageCandidate(
    exactClaim: SpawnGenerationClaim,
    config: RuntimeSessionConfig,
    session: PiSession,
    observeTranscript: TranscriptObserver,
    observeModelChange: ModelChangeObserver,
    observeLifecycle: LifecycleObserver = () => undefined,
    observePersistedEntry: PersistedEntryObserver = () => undefined,
  ): RuntimeSessionEntry {
    const claimOperationId = exactClaim.claimOperationId?.value;
    const logicalTargetId = exactClaim.logicalTargetId?.value;
    const claimedGeneration = exactClaim.claimedGeneration?.value;
    if (!claimOperationId || !logicalTargetId || !claimedGeneration || claimedGeneration !== BigInt(session.generation)) {
      throw new Error("candidate does not match its exact claim");
    }
    if (config.logicalTargetId !== logicalTargetId) {
      throw new Error("candidate logical target does not match its registry config");
    }
    if (!config.runtimeSessionId || config.runtimeSessionId !== session.runtimeSessionId) {
      throw new Error("candidate registry identity does not match its Pi runtime");
    }
    const existing = this.#candidates.get(claimOperationId);
    if (existing) {
      if (
        existing.entry.session.processToken === session.processToken &&
        existing.entry.runtimeSessionId === config.runtimeSessionId
      ) {
        return existing.entry;
      }
      throw new Error("claim already has another staged candidate");
    }
    // A successor is deliberately not subscribed to ordinary publication
    // callbacks until the core's exact promotion is observed.
    const entry: RuntimeSessionEntry = {
      ...config,
      session,
      attachmentToken: Symbol(config.runtimeSessionId),
    };
    this.#candidates.set(claimOperationId, {
      exactClaim,
      entry,
      observeTranscript,
      observeModelChange,
      observeLifecycle,
      observePersistedEntry,
    });
    return entry;
  }

  promoteCandidate(exactClaim: SpawnGenerationClaim, runtime: RuntimeGenerationRef): RuntimeSessionEntry {
    const claimOperationId = exactClaim.claimOperationId?.value;
    const logicalTargetId = exactClaim.logicalTargetId?.value;
    const external = runtime.externalRuntime;
    if (!claimOperationId || !logicalTargetId || runtime.logicalTargetId?.value !== logicalTargetId) {
      throw new Error("promotion does not match the exact claim");
    }
    const candidate = this.#candidates.get(claimOperationId);
    if (!candidate || !sameClaim(candidate.exactClaim, exactClaim)) {
      throw new Error("promotion has no exact staged candidate");
    }
    if (
      external?.runtimeSessionId?.value !== candidate.entry.runtimeSessionId ||
      external.generation?.value !== BigInt(candidate.entry.session.generation) ||
      external.deploymentScope !== candidate.entry.deploymentScope
    ) {
      throw new Error("promotion runtime differs from the staged candidate");
    }
    const prior = this.#byLogicalTarget.get(logicalTargetId);
    if (prior) this.#deactivate(prior);
    const promoted = this.#ownedEntry(
      candidate.entry,
      candidate.entry.session,
      candidate.observeTranscript,
      candidate.observeModelChange,
      candidate.observeLifecycle,
      candidate.observePersistedEntry,
      claimOperationId,
    );
    this.#candidates.delete(claimOperationId);
    this.#installCurrent(promoted);
    return promoted;
  }

  resolve(runtimeSessionId: string): RuntimeSessionEntry | undefined {
    return this.#entries.get(runtimeSessionId);
  }

  resolvePrior(prior: RuntimeGenerationRef): RuntimeSessionEntry | undefined {
    const external = prior.externalRuntime;
    const logicalTargetId = prior.logicalTargetId?.value;
    if (!external || !logicalTargetId) return undefined;
    const entry = this.#byLogicalTarget.get(logicalTargetId);
    if (
      !entry ||
      entry.runtimeSessionId !== external.runtimeSessionId?.value ||
      entry.deploymentScope !== external.deploymentScope ||
      BigInt(entry.session.generation) !== external.generation?.value
    ) {
      return undefined;
    }
    return entry;
  }

  candidate(claimOperationId: string): RuntimeSessionEntry | undefined {
    return this.#candidates.get(claimOperationId)?.entry;
  }

  discardCandidate(claimOperationId: string): RuntimeSessionEntry | undefined {
    const candidate = this.#candidates.get(claimOperationId);
    if (!candidate) return undefined;
    this.#candidates.delete(claimOperationId);
    return candidate.entry;
  }

  entries(): IterableIterator<[string, RuntimeSessionEntry]> {
    return this.#entries.entries();
  }

  async dispose(): Promise<void> {
    const sessions = new Set<PiSession>();
    for (const entry of this.#entries.values()) {
      this.#deactivate(entry);
      sessions.add(entry.session);
    }
    for (const candidate of this.#candidates.values()) {
      sessions.add(candidate.entry.session);
    }
    this.#entries.clear();
    this.#byLogicalTarget.clear();
    this.#candidates.clear();
    await Promise.all([...sessions].map((session) => session.dispose()));
  }

  #ownedEntry(
    config: RuntimeSessionConfig,
    session: PiSession,
    observeTranscript: TranscriptObserver,
    observeModelChange: ModelChangeObserver,
    observeLifecycle: LifecycleObserver,
    observePersistedEntry: PersistedEntryObserver,
    promotionClaimOperationId?: string,
  ): OwnedRuntimeSessionEntry {
    if (!config.runtimeSessionId || session.runtimeSessionId !== config.runtimeSessionId) {
      throw new Error("registry key does not match Pi runtime identity");
    }
    const attachmentToken = Symbol(config.runtimeSessionId);
    const entry: OwnedRuntimeSessionEntry = {
      ...config,
      session,
      attachmentToken,
      ...(promotionClaimOperationId ? { promotionClaimOperationId } : {}),
      active: true,
      unsubscribeTranscript: () => undefined,
      unsubscribeModelChange: () => undefined,
      unsubscribeLifecycle: () => undefined,
      unsubscribePersistedEntry: () => undefined,
    };
    Object.assign(entry, {
      unsubscribeTranscript: session.onTranscript((event) => {
        if (this.#callbackIsCurrent(entry, attachmentToken)) observeTranscript(entry, event);
      }),
      unsubscribeModelChange: session.onModelChange((model) => {
        if (this.#callbackIsCurrent(entry, attachmentToken)) observeModelChange(entry, model);
      }),
      unsubscribeLifecycle: session.onLifecycle((event) => {
        if (this.#callbackIsCurrent(entry, attachmentToken)) observeLifecycle(entry, event);
      }),
      unsubscribePersistedEntry: session.onPersistedEntry(() => {
        if (this.#callbackIsCurrent(entry, attachmentToken)) observePersistedEntry(entry);
      }),
    });
    return entry;
  }

  #installCurrent(entry: OwnedRuntimeSessionEntry): void {
    if (this.#entries.has(entry.runtimeSessionId)) {
      throw new Error("runtime session is already registered");
    }
    const logicalTargetId = entry.logicalTargetId;
    if (logicalTargetId && this.#byLogicalTarget.has(logicalTargetId)) {
      throw new Error("logical target already has a current runtime");
    }
    entry.active = true;
    this.#entries.set(entry.runtimeSessionId, entry);
    if (logicalTargetId) this.#byLogicalTarget.set(logicalTargetId, entry);
  }

  #deactivate(entry: OwnedRuntimeSessionEntry): void {
    if (!entry.active) return;
    entry.active = false;
    entry.unsubscribeTranscript();
    entry.unsubscribeModelChange();
    entry.unsubscribeLifecycle();
    entry.unsubscribePersistedEntry();
    if (this.#entries.get(entry.runtimeSessionId) === entry) this.#entries.delete(entry.runtimeSessionId);
    if (entry.logicalTargetId && this.#byLogicalTarget.get(entry.logicalTargetId) === entry) {
      this.#byLogicalTarget.delete(entry.logicalTargetId);
    }
  }

  #callbackIsCurrent(entry: OwnedRuntimeSessionEntry, token: symbol): boolean {
    const gate = entry.logicalTargetId ? this.#gates.get(entry.logicalTargetId) : undefined;
    const activeReplacementClaim = gate?.fencedClaimOperationId;
    const promotionOwnsFence = activeReplacementClaim !== undefined
      && activeReplacementClaim === entry.promotionClaimOperationId;
    return (!activeReplacementClaim || promotionOwnsFence)
      && !gate?.observationsFenced
      && entry.active
      && entry.attachmentToken === token
      && this.#entries.get(entry.runtimeSessionId) === entry;
  }
}

function sameClaim(left: SpawnGenerationClaim, right: SpawnGenerationClaim): boolean {
  return (
    left.authorityDomainId?.value === right.authorityDomainId?.value &&
    left.claimOperationId?.value === right.claimOperationId?.value &&
    left.logicalTargetId?.value === right.logicalTargetId?.value &&
    left.claimedGeneration?.value === right.claimedGeneration?.value &&
    left.expectedPrior?.logicalTargetId?.value === right.expectedPrior?.logicalTargetId?.value &&
    left.expectedPrior?.externalRuntime?.adapterId?.value === right.expectedPrior?.externalRuntime?.adapterId?.value &&
    left.expectedPrior?.externalRuntime?.deploymentScope === right.expectedPrior?.externalRuntime?.deploymentScope &&
    left.expectedPrior?.externalRuntime?.runtimeSessionId?.value === right.expectedPrior?.externalRuntime?.runtimeSessionId?.value &&
    left.expectedPrior?.externalRuntime?.generation?.value === right.expectedPrior?.externalRuntime?.generation?.value
  );
}
