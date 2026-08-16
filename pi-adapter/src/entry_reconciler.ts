import { createHash } from "node:crypto";
import {
  AuthoritativeCursorReplacement,
  type ExternalCursorFetchPort,
  type ExternalCursorPublishPort,
  type KnownCursorSuffix,
  type ProjectionReplacement,
} from "@patchbay/operator-domain/reconciliation/external-cursor";
import type { RuntimeGenerationRef } from "@patchbay/contracts";
import {
  FilePiCursorStore,
  PI_CURSOR_VALUES,
  derivePiSessionContinuityKey,
  type PiCursorProjectionRecord,
  type PiExternalCursorScope,
} from "./cursor_store.js";
import {
  encodePiProjectionReplacement,
  encodePiProjectionSuffix,
  encodePiVolatileProjectionSnapshot,
  piProjectedEntriesEqual,
  piProjectionLeavesEqual,
  projectCompletePiEntries,
  projectKnownPiSuffix,
  type PiProjectedEntry,
  type PiProjectionCursor,
  type PiProjectionLeaf,
} from "./pi_projection.js";
import { PiUnknownCursorError } from "./pi_session.js";
import type { PiSessionMaterialization } from "./session_file.js";

export interface PiProjectionObservationPort {
  publish(
    runtime: RuntimeGenerationRef,
    schemaRef: string,
    payload: Uint8Array,
  ): Promise<void>;
}

export interface PiCursorReconciliationEvidence {
  readonly logicalTargetId: string;
  readonly configuredSessionRoot: string;
  readonly piSessionId: string;
  readonly declaredSessionPath: string;
  readonly materialization: PiSessionMaterialization;
  readonly completeEntries: readonly unknown[];
  readonly leafId: string | null;
  readonly fetchKnown: (cursor: string) => Promise<{
    readonly entries: readonly unknown[];
    readonly leafId: string | null;
  }>;
}

export interface PiStagedCursorPublication {
  readonly mode: "known" | "replacement" | "volatile-snapshot";
  readonly runtime: RuntimeGenerationRef;
  readonly scope: PiExternalCursorScope;
  readonly logicalTargetId: string;
  readonly replacementEpoch: bigint | null;
  readonly baseCursor?: string;
  readonly entries: readonly PiProjectedEntry[];
  readonly cursor: PiProjectionCursor;
  readonly leaf: PiProjectionLeaf;
  readonly readinessDigest: string;
  readonly restartStable: boolean;
}

interface CachedFetch {
  readonly known?: {
    readonly baseCursor: string;
    readonly suffixEntries: readonly PiProjectedEntry[];
    readonly cursor: PiProjectionCursor;
    readonly leaf: PiProjectionLeaf;
  };
  readonly complete?: {
    readonly entries: readonly PiProjectedEntry[];
    readonly leaf: PiProjectionLeaf;
  };
}

/**
 * Concrete Pi adapter over Leaf 4's sealed transition owner. This class supplies
 * only the external fetch, publication, value, and atomic-store ports.
 */
export class PiEntryReconciler {
  readonly #store: FilePiCursorStore;
  readonly #observations: PiProjectionObservationPort;

  constructor(store: FilePiCursorStore, observations: PiProjectionObservationPort) {
    this.#store = store;
    this.#observations = observations;
  }

  /** Stage exact candidate evidence without publishing an ordinary Observation. */
  async stageClaimedSuccessor(
    runtime: RuntimeGenerationRef,
    evidence: PiCursorReconciliationEvidence,
  ): Promise<PiStagedCursorPublication> {
    validateRuntime(runtime, evidence.logicalTargetId);
    const continuity = await this.#continuity(runtime, evidence);
    const exact = projectCompletePiEntries(
      evidence.completeEntries,
      evidence.leafId,
      continuity.scope.externalContinuityId,
    );
    if (evidence.materialization.kind === "materialized") {
      requireMaterializedProjection(evidence, exact);
    }
    if (evidence.materialization.kind !== "materialized") {
      return this.#stageVolatile(runtime, evidence.logicalTargetId, continuity.scope, exact);
    }

    await this.#store.bindLogicalTarget(continuity.scope, evidence.logicalTargetId);
    await this.#store.ensureReplacementBaseline(
      continuity.scope,
      evidence.logicalTargetId,
      emptyProjection(),
    );
    const current = await this.#store.load(continuity.scope);
    if (!current) throw new Error("Pi durable cursor baseline was not initialized");

    if (
      current.freshness === "current"
      && !current.pendingReplacement
      && current.projection.cursor !== null
    ) {
      try {
        const response = await evidence.fetchKnown(current.projection.cursor);
        const suffix = projectKnownPiSuffix(
          current.projection.exactEntries,
          response.entries,
          response.leafId,
          continuity.scope.externalContinuityId,
        );
        if (!sameExactProjection(suffix, exact)) {
          throw new Error("Pi known suffix result disagrees with the validated complete tree");
        }
        return stagedPublication({
          mode: "known",
          runtime,
          scope: continuity.scope,
          logicalTargetId: evidence.logicalTargetId,
          replacementEpoch: current.projection.replacementEpoch,
          baseCursor: current.projection.cursor,
          entries: suffix.suffixEntries,
          cursor: suffix.cursor,
          leaf: suffix.leaf,
          restartStable: true,
        });
      } catch (error) {
        if (!(error instanceof PiUnknownCursorError)) throw error;
      }
    }

    const machine = this.#machine(
      continuity.scope,
      runtime,
      current.projection.replacementEpoch + 1n,
      { complete: { entries: exact.entries, leaf: exact.leaf } },
    );
    const staged = await machine.stageReplacement(continuity.scope);
    if (
      staged.replacementEpoch !== current.projection.replacementEpoch + 1n
      || !entrySequencesEqual(staged.entries, exact.entries)
      || !piProjectionLeavesEqual(staged.leaf, exact.leaf)
    ) {
      throw new Error("Pi existing staged replacement conflicts with the validated exact tree");
    }
    return stagedPublication({
      mode: "replacement",
      runtime,
      scope: continuity.scope,
      logicalTargetId: evidence.logicalTargetId,
      replacementEpoch: staged.replacementEpoch,
      entries: staged.entries,
      cursor: exact.cursor,
      leaf: staged.leaf,
      restartStable: true,
    });
  }

  /** Current runtimes use the same stage → publish → commit path. */
  async reconcileCurrent(
    runtime: RuntimeGenerationRef,
    evidence: PiCursorReconciliationEvidence,
  ): Promise<PiStagedCursorPublication> {
    const staged = await this.stageClaimedSuccessor(runtime, evidence);
    await this.publishAfterPromotion(staged);
    return staged;
  }

  /** Reload stays in-generation but must re-enter through durable materialization. */
  async reconcileAfterReload(
    runtime: RuntimeGenerationRef,
    evidence: PiCursorReconciliationEvidence,
  ): Promise<PiStagedCursorPublication> {
    if (evidence.materialization.kind !== "materialized") {
      throw new Error("Pi reload reconciliation requires a materialized session");
    }
    return this.reconcileCurrent(runtime, evidence);
  }

  /** Publish only after the caller has consumed SpawnPromotionCommitted. */
  async publishAfterPromotion(staged: PiStagedCursorPublication): Promise<void> {
    validateStagedPublication(staged);
    switch (staged.mode) {
      case "volatile-snapshot": {
        const envelope = encodePiVolatileProjectionSnapshot({
          externalContinuityId: staged.scope!.externalContinuityId,
          exactEntries: staged.entries,
          cursor: staged.cursor,
          leaf: staged.leaf,
        });
        await this.#observations.publish(staged.runtime, envelope.schemaRef, envelope.payload);
        return;
      }
      case "known": {
        const fetch: CachedFetch = {
          known: {
            baseCursor: staged.baseCursor!,
            suffixEntries: staged.entries,
            cursor: staged.cursor,
            leaf: staged.leaf,
          },
        };
        await this.#machine(staged.scope!, staged.runtime, staged.replacementEpoch!, fetch)
          .reconcileKnown(staged.scope!, staged.baseCursor!);
        return;
      }
      case "replacement": {
        const replacement: ProjectionReplacement<
          PiProjectedEntry,
          PiProjectionCursor,
          PiProjectionLeaf
        > = {
          replacementEpoch: staged.replacementEpoch!,
          exactEntries: staged.entries,
          cursor: staged.cursor,
          leaf: staged.leaf,
        };
        await this.#machine(
          staged.scope!,
          staged.runtime,
          staged.replacementEpoch!,
          { complete: { entries: staged.entries, leaf: staged.leaf } },
        ).commitReplacement(staged.scope!, replacement);
        return;
      }
    }
  }

  /** Journal replay after promotion republishes the exact same batch if needed. */
  publishRecoveredAfterPromotion(staged: PiStagedCursorPublication): Promise<void> {
    return this.publishAfterPromotion(staged);
  }

  async load(scope: PiExternalCursorScope): Promise<PiCursorProjectionRecord | undefined> {
    return this.#store.load(scope);
  }

  #machine(
    scope: PiExternalCursorScope,
    runtime: RuntimeGenerationRef,
    replacementEpoch: bigint,
    cached: CachedFetch,
  ) {
    const fetch: ExternalCursorFetchPort<
      PiExternalCursorScope,
      PiProjectedEntry,
      PiProjectionCursor,
      PiProjectionLeaf
    > = {
      fetchKnown: async (_scope, cursor) => {
        const known = cached.known;
        if (!known || cursor !== known.baseCursor) throw new Error("Pi cached known suffix is unavailable");
        return {
          entries: known.suffixEntries,
          cursor: known.cursor,
          leaf: known.leaf,
        };
      },
      fetchComplete: async () => {
        if (!cached.complete) throw new Error("Pi cached complete tree is unavailable");
        return cached.complete;
      },
    };
    const publish: ExternalCursorPublishPort<
      PiExternalCursorScope,
      PiProjectedEntry,
      PiProjectionCursor,
      PiProjectionLeaf
    > = {
      publishKnownSuffix: async (_scope, suffix) => {
        if (suffix.cursor === null) throw new Error("Pi known suffix cannot clear its cursor");
        const envelope = encodePiProjectionSuffix({
          externalContinuityId: scope.externalContinuityId,
          replacementEpoch,
          baseCursor: requireStringCursor(suffix),
          entries: suffix.entries,
          cursor: suffix.cursor,
          leaf: suffix.leaf,
        });
        await this.#observations.publish(runtime, envelope.schemaRef, envelope.payload);
      },
      publishReplacement: async (_scope, replacement) => {
        const envelope = encodePiProjectionReplacement({
          externalContinuityId: scope.externalContinuityId,
          replacementEpoch: replacement.replacementEpoch,
          exactEntries: replacement.exactEntries,
          cursor: replacement.cursor,
          leaf: replacement.leaf,
        });
        await this.#observations.publish(runtime, envelope.schemaRef, envelope.payload);
      },
    };
    return AuthoritativeCursorReplacement.create(this.#store, fetch, publish, PI_CURSOR_VALUES);
  }

  async #continuity(
    runtime: RuntimeGenerationRef,
    evidence: PiCursorReconciliationEvidence,
  ) {
    const canonicalSessionPath = evidence.materialization.kind === "materialized"
      ? evidence.materialization.seal.canonicalPath
      : evidence.declaredSessionPath;
    return derivePiSessionContinuityKey({
      adapterId: runtime.externalRuntime!.adapterId!.value,
      deploymentScope: runtime.externalRuntime!.deploymentScope,
      piSessionId: evidence.piSessionId,
      configuredSessionRoot: evidence.configuredSessionRoot,
      canonicalSessionPath,
    });
  }

  #stageVolatile(
    runtime: RuntimeGenerationRef,
    logicalTargetId: string,
    scope: PiExternalCursorScope,
    exact: ReturnType<typeof projectCompletePiEntries>,
  ): PiStagedCursorPublication {
    return stagedPublication({
      mode: "volatile-snapshot",
      runtime,
      scope,
      logicalTargetId,
      replacementEpoch: null,
      entries: exact.entries,
      cursor: exact.cursor,
      leaf: exact.leaf,
      restartStable: false,
    });
  }

}

export function serializePiStagedCursorPublication(
  staged: PiStagedCursorPublication,
): Record<string, unknown> {
  validateStagedPublication(staged);
  return {
    mode: staged.mode,
    runtime: runtimeJson(staged.runtime),
    scope: staged.scope,
    logicalTargetId: staged.logicalTargetId,
    replacementEpoch: staged.replacementEpoch?.toString() ?? null,
    ...(staged.baseCursor ? { baseCursor: staged.baseCursor } : {}),
    entries: staged.entries,
    cursor: staged.cursor,
    leaf: staged.leaf,
    readinessDigest: staged.readinessDigest,
    restartStable: staged.restartStable,
  };
}

export function restorePiStagedCursorPublication(
  value: unknown,
  runtime: RuntimeGenerationRef,
): PiStagedCursorPublication {
  if (!isRecord(value)) throw new Error("Pi staged cursor journal payload is malformed");
  const mode = value["mode"];
  if (!isMode(mode)) throw new Error("Pi staged cursor journal mode is malformed");
  const scope = parseScope(value["scope"]);
  const epochText = value["replacementEpoch"];
  if (!(epochText === null || (typeof epochText === "string" && /^[1-9][0-9]*$/u.test(epochText)))) {
    throw new Error("Pi staged cursor journal epoch is malformed");
  }
  const entries = Array.isArray(value["entries"])
    ? value["entries"] as unknown as readonly PiProjectedEntry[]
    : (() => { throw new Error("Pi staged cursor journal entries are malformed"); })();
  const leaf = value["leaf"];
  if (!isRecord(leaf) || !(leaf["entryId"] === null || typeof leaf["entryId"] === "string") || typeof leaf["treeDigest"] !== "string") {
    throw new Error("Pi staged cursor journal leaf is malformed");
  }
  const restored = stagedPublication({
    mode,
    runtime,
    scope,
    logicalTargetId: stringField(value, "logicalTargetId"),
    replacementEpoch: epochText === null ? null : BigInt(epochText),
    ...(typeof value["baseCursor"] === "string" ? { baseCursor: value["baseCursor"] } : {}),
    entries,
    cursor: value["cursor"] === null ? null : stringField(value, "cursor"),
    leaf: { entryId: leaf["entryId"] as string | null, treeDigest: leaf["treeDigest"] },
    restartStable: value["restartStable"] === true,
  });
  if (restored.readinessDigest !== value["readinessDigest"] || !sameRuntimeJson(value["runtime"], runtime)) {
    throw new Error("Pi staged cursor journal readiness evidence is inconsistent");
  }
  return restored;
}

function stagedPublication(input: Omit<PiStagedCursorPublication, "readinessDigest">): PiStagedCursorPublication {
  const entries = Object.freeze(input.entries.map((entry) => Object.freeze({
    ...entry,
    presentationItems: Object.freeze(entry.presentationItems.map((item) => Object.freeze({ ...item }))),
  })));
  const base = Object.freeze({ ...input, entries, leaf: Object.freeze({ ...input.leaf }) });
  return Object.freeze({ ...base, readinessDigest: stagedDigest(base) });
}

function validateStagedPublication(staged: PiStagedCursorPublication): void {
  validateRuntime(staged.runtime, staged.logicalTargetId);
  if (staged.mode === "known" && !staged.baseCursor) throw new Error("Pi known publication has no base cursor");
  if (
    (staged.mode === "volatile-snapshot" && staged.replacementEpoch !== null)
    || (staged.mode !== "volatile-snapshot" && (staged.replacementEpoch === null || staged.replacementEpoch <= 0n))
  ) {
    throw new Error("Pi staged publication has an invalid authoritative epoch claim");
  }
  if (staged.restartStable !== (staged.mode === "known" || staged.mode === "replacement")) {
    throw new Error("Pi staged publication has an invalid restart-stability claim");
  }
  if (staged.readinessDigest !== stagedDigest(staged)) {
    throw new Error("Pi staged publication readiness digest changed");
  }
}

function stagedDigest(input: Omit<PiStagedCursorPublication, "readinessDigest"> | PiStagedCursorPublication): string {
  const hash = createHash("sha256");
  hash.update(JSON.stringify({
    mode: input.mode,
    runtime: runtimeJson(input.runtime),
    scope: input.scope,
    logicalTargetId: input.logicalTargetId,
    replacementEpoch: input.replacementEpoch?.toString() ?? null,
    baseCursor: input.baseCursor ?? null,
    entries: input.entries,
    cursor: input.cursor,
    leaf: input.leaf,
    restartStable: input.restartStable,
  }));
  return hash.digest("hex");
}

function emptyProjection(): ProjectionReplacement<
  PiProjectedEntry,
  PiProjectionCursor,
  PiProjectionLeaf
> {
  const emptyTreeDigest = createHash("sha256").update("[]").digest("hex");
  return {
    replacementEpoch: 0n,
    exactEntries: [],
    cursor: null,
    leaf: { entryId: null, treeDigest: emptyTreeDigest },
  };
}

function sameExactProjection(
  known: ReturnType<typeof projectKnownPiSuffix>,
  complete: ReturnType<typeof projectCompletePiEntries>,
): boolean {
  return known.cursor === complete.cursor
    && piProjectionLeavesEqual(known.leaf, complete.leaf)
    && entrySequencesEqual(known.entries, complete.entries);
}

function entrySequencesEqual(left: readonly PiProjectedEntry[], right: readonly PiProjectedEntry[]): boolean {
  return left.length === right.length && left.every((entry, index) => piProjectedEntriesEqual(entry, right[index]!));
}

function requireStringCursor(
  suffix: KnownCursorSuffix<PiProjectedEntry, PiProjectionCursor, PiProjectionLeaf>,
): string {
  if (typeof suffix.baseCursor !== "string") throw new Error("Pi known suffix base cursor is absent");
  return suffix.baseCursor;
}

function requireMaterializedProjection(
  evidence: PiCursorReconciliationEvidence,
  exact: ReturnType<typeof projectCompletePiEntries>,
): void {
  if (evidence.materialization.kind !== "materialized") return;
  const seal = evidence.materialization.seal;
  if (
    seal.sessionId !== evidence.piSessionId
    || seal.canonicalPath !== evidence.declaredSessionPath
    || seal.leafId !== exact.leaf.entryId
    || seal.treeDigest !== exact.leaf.treeDigest
    || seal.orderedEntryIds.length !== exact.entries.length
    || seal.orderedEntryIds.some((id, index) => id !== exact.entries[index]!.stableEntryId)
  ) {
    throw new Error("Pi cursor projection differs from the verified materialized session seal");
  }
}

function validateRuntime(runtime: RuntimeGenerationRef, logicalTargetId: string): void {
  if (
    !logicalTargetId
    || runtime.logicalTargetId?.value !== logicalTargetId
    || !runtime.externalRuntime?.adapterId?.value
    || !runtime.externalRuntime.deploymentScope
    || !runtime.externalRuntime.runtimeSessionId?.value
    || !runtime.externalRuntime.generation?.value
  ) {
    throw new Error("Pi cursor reconciliation runtime identity is incomplete");
  }
}

function runtimeJson(runtime: RuntimeGenerationRef): Record<string, string> {
  return {
    logicalTargetId: runtime.logicalTargetId!.value,
    adapterId: runtime.externalRuntime!.adapterId!.value,
    deploymentScope: runtime.externalRuntime!.deploymentScope,
    runtimeSessionId: runtime.externalRuntime!.runtimeSessionId!.value,
    generation: runtime.externalRuntime!.generation!.value.toString(),
  };
}

function sameRuntimeJson(value: unknown, runtime: RuntimeGenerationRef): boolean {
  return isRecord(value) && JSON.stringify(value) === JSON.stringify(runtimeJson(runtime));
}

function parseScope(value: unknown): PiExternalCursorScope {
  if (!isRecord(value)) throw new Error("Pi staged cursor scope is malformed");
  return {
    adapterId: stringField(value, "adapterId"),
    deploymentScope: stringField(value, "deploymentScope"),
    externalContinuityId: stringField(value, "externalContinuityId"),
  };
}

function stringField(value: Record<string, unknown>, field: string): string {
  const candidate = value[field];
  if (typeof candidate !== "string" || candidate.length === 0) throw new Error(`Pi staged cursor ${field} is malformed`);
  return candidate;
}

function isMode(value: unknown): value is PiStagedCursorPublication["mode"] {
  return value === "known" || value === "replacement" || value === "volatile-snapshot";
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

