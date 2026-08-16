import { createHash, randomBytes } from "node:crypto";
import { realpath } from "node:fs/promises";
import { isAbsolute, normalize } from "node:path";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  ContinuationContextStatus,
  ExternalEffectDisposition,
  ExternalRuntimeRefSchema,
  FailureCode,
  GenerationSchema,
  LogicalTargetIdSchema,
  NoExternalEffectProofSchema,
  PayloadContentType,
  PiContinuationMode,
  PiSpawnPersistence,
  PiSpawnResultSchema,
  PiSpawnTargetSpecSchema,
  RuntimeGenerationRefSchema,
  SpawnExecutionPhase,
  SpawnRequestSchema,
  SupervisorPreLaunchFailureProofSchema,
  type SpawnClaimAccepted,
  type SpawnGenerationClaim,
  type SpawnPromotionCommitted,
  type RuntimeGenerationRef,
} from "@patchbay/contracts";
import type { ConfiguredDeploymentTarget } from "./deployment_authority.js";
import {
  buildPiRpcArgv,
  type ManagedPiRuntimePort,
  type PiLaunchSpec,
  type PiRpcRuntime,
} from "./pi_process.js";
import {
  RpcPiSession,
  type PiSession,
  type SessionLifecycleEvent,
} from "./pi_session.js";
import {
  classifyPiSessionMaterialization,
  verifyMaterializedSessionSeal,
  verifyResumedSessionExtension,
  type MaterializedSessionSeal,
} from "./session_file.js";
import { SessionRegistry, type RuntimeSessionEntry } from "./session_registry.js";
import {
  spawnTargetFingerprint,
  type SpawnEffectJournal,
} from "./spawn_journal.js";
import type { RuntimeReplacementLease } from "./runtime_action_gate.js";

export const PI_SPAWN_TARGET_SCHEMA_REF = "patchbay.PiSpawnTargetSpec.v1";
export const PI_SPAWN_RESULT_SCHEMA_REF = "patchbay.PiSpawnResult.v1";
export const PI_RPC_TARGET_SHAPE = "pi-rpc";

const DEFAULT_SETTLE_TIMEOUT_MS = 10_000;

export interface ManagedPiTargetConfig {
  readonly projectContextRef: string;
  readonly deploymentTarget: ConfiguredDeploymentTarget;
  readonly cwd: string;
  readonly sessionRoot: string;
  readonly executable: string;
  readonly cliPath: string;
  readonly controlExtensionPath: string;
  readonly sessionDirectory?: string;
  readonly model?: string;
  readonly environment?: Readonly<Record<string, string>>;
  readonly additionalArguments?: readonly string[];
}

export interface SpawnSupervisorCorePort {
  readonly adapterId: string;
  readonly adapterGeneration: number;
  authorizeDeployment(
    acceptedSpawn: SpawnClaimAccepted,
    target: ConfiguredDeploymentTarget,
    now: Date,
  ): Promise<void>;
  flushObservations(): Promise<void>;
  reportSpawnEvidence(input: {
    readonly operation: NonNullable<SpawnClaimAccepted["acceptedOperation"]>["operation"];
    readonly exactClaim: SpawnGenerationClaim;
    readonly phase: SpawnExecutionPhase;
    readonly disposition: ExternalEffectDisposition;
    readonly failureCode: FailureCode;
    readonly externalRuntime?: RuntimeGenerationRef;
    readonly supervisorNoEffectProof?: boolean;
  }): Promise<void>;
  reportSessionState(
    entry: RuntimeSessionEntry,
    connectivity: "live" | "offline" | "stale" | "failed",
    activity: "idle" | "working" | "unknown",
  ): Promise<void>;
  stageSuccessor(input: {
    readonly acceptedSpawn: SpawnClaimAccepted;
    readonly runtime: RuntimeGenerationRef;
    readonly entry: RuntimeSessionEntry;
    readonly continuationContextStatus: ContinuationContextStatus;
  }): Promise<void>;
  reportSpawnResult(
    operation: NonNullable<SpawnClaimAccepted["acceptedOperation"]>["operation"],
    payload: Uint8Array,
  ): Promise<void>;
  reportSpawnFailure(
    operation: NonNullable<SpawnClaimAccepted["acceptedOperation"]>["operation"],
    failureCode: FailureCode,
  ): Promise<void>;
}

export interface StagedPiProjection {
  readonly readinessDigest: string;
  readonly entryCount: number;
}

export interface PiAuthoritativeReconciler {
  stageClaimedSuccessor(
    runtime: RuntimeGenerationRef,
    entries: readonly unknown[],
    leafId: string | null,
  ): Promise<StagedPiProjection>;
  publishAfterPromotion(staged: StagedPiProjection, session: PiSession): Promise<void>;
}

export class LocalStagedPiReconciler implements PiAuthoritativeReconciler {
  async stageClaimedSuccessor(
    runtime: RuntimeGenerationRef,
    entries: readonly unknown[],
    leafId: string | null,
  ): Promise<StagedPiProjection> {
    const hash = createHash("sha256");
    hash.update(runtime.logicalTargetId?.value ?? "");
    hash.update("\0");
    hash.update(runtime.externalRuntime?.runtimeSessionId?.value ?? "");
    hash.update("\0");
    hash.update(JSON.stringify(entries));
    hash.update("\0");
    hash.update(leafId ?? "");
    return Object.freeze({ readinessDigest: hash.digest("hex"), entryCount: entries.length });
  }

  async publishAfterPromotion(_staged: StagedPiProjection, session: PiSession): Promise<void> {
    session.publishStagedTranscript();
  }
}

interface ValidatedSpawn {
  readonly acceptedSpawn: SpawnClaimAccepted;
  readonly operation: NonNullable<SpawnClaimAccepted["acceptedOperation"]>["operation"];
  readonly claim: SpawnGenerationClaim;
  readonly target: ManagedPiTargetConfig;
  readonly continuationMode: "fresh" | "require_resume" | "allow_new_context";
}

interface PromotionWaiter {
  readonly exactClaim: SpawnGenerationClaim;
  readonly runtime: RuntimeGenerationRef;
  readonly resolve: (promotion: SpawnPromotionCommitted) => void;
  readonly reject: (error: Error) => void;
}

export interface StagedPiSuccessor {
  readonly runtime: RuntimeGenerationRef;
  readonly entry: RuntimeSessionEntry;
  readonly projection: StagedPiProjection;
  readonly continuationContextStatus: ContinuationContextStatus;
}

export class SpawnSupervisorError extends Error {
  readonly failureCode: FailureCode;
  readonly ambiguous: boolean;
  terminalReported = false;

  constructor(message: string, failureCode: FailureCode, ambiguous = false) {
    super(message);
    this.name = "SpawnSupervisorError";
    this.failureCode = failureCode;
    this.ambiguous = ambiguous;
  }
}

/** Claim-aware implementation of the fixed ten-step continuation order. */
export class ClaimAwareSpawnSupervisor {
  readonly #runtimePort: ManagedPiRuntimePort;
  readonly #journal: SpawnEffectJournal;
  readonly #registry: SessionRegistry;
  readonly #core: SpawnSupervisorCorePort;
  readonly #reconciler: PiAuthoritativeReconciler;
  readonly #targets: ReadonlyMap<string, ManagedPiTargetConfig>;
  readonly #waiters = new Map<string, PromotionWaiter>();
  readonly #earlyPromotions = new Map<string, SpawnPromotionCommitted>();
  readonly #promotionEligibleClaims = new Set<string>();
  readonly #observeTranscript: Parameters<SessionRegistry["stageCandidate"]>[3];
  readonly #observeModelChange: Parameters<SessionRegistry["stageCandidate"]>[4];
  readonly #observeLifecycle: Parameters<SessionRegistry["stageCandidate"]>[5];

  constructor(options: {
    readonly runtimePort: ManagedPiRuntimePort;
    readonly journal: SpawnEffectJournal;
    readonly registry: SessionRegistry;
    readonly core: SpawnSupervisorCorePort;
    readonly targets: readonly ManagedPiTargetConfig[];
    readonly reconciler?: PiAuthoritativeReconciler;
    readonly observeTranscript?: Parameters<SessionRegistry["stageCandidate"]>[3];
    readonly observeModelChange?: Parameters<SessionRegistry["stageCandidate"]>[4];
    readonly observeLifecycle?: Parameters<SessionRegistry["stageCandidate"]>[5];
  }) {
    this.#runtimePort = options.runtimePort;
    this.#journal = options.journal;
    this.#registry = options.registry;
    this.#core = options.core;
    this.#reconciler = options.reconciler ?? new LocalStagedPiReconciler();
    this.#observeTranscript = options.observeTranscript ?? (() => undefined);
    this.#observeModelChange = options.observeModelChange ?? (() => undefined);
    this.#observeLifecycle = options.observeLifecycle ?? (() => undefined);
    const targets = new Map<string, ManagedPiTargetConfig>();
    for (const target of options.targets) {
      if (!target.projectContextRef || targets.has(target.projectContextRef)) {
        throw new Error("managed Pi project-context references must be unique");
      }
      targets.set(target.projectContextRef, Object.freeze({ ...target }));
    }
    this.#targets = targets;
  }

  async handleAcceptedSpawn(acceptedSpawn: SpawnClaimAccepted): Promise<StagedPiSuccessor> {
    const validated = await this.#validate(acceptedSpawn);
    const claimOperationId = validated.claim.claimOperationId!.value;
    const existing = await this.#journal.reconcile(claimOperationId);
    if (existing) {
      if (!sameClaim(existing.exactClaim, validated.claim)) {
        throw new SpawnSupervisorError("spawn journal exact-claim correlation failed", FailureCode.DELIVERY_REJECTED);
      }
      if (existing.promoted) {
        throw new SpawnSupervisorError("spawn claim is already promoted", FailureCode.STALE_EVENT);
      }
      if (existing.phases.some((phase) => phase.phase === SpawnExecutionPhase.LAUNCH_ATTEMPTED)) {
        throw new SpawnSupervisorError(
          "spawn generation has an ambiguous prior launch attempt and cannot auto-relaunch",
          FailureCode.EXECUTION_OUTCOME_UNKNOWN,
          true,
        );
      }
    }

    await this.#core.authorizeDeployment(acceptedSpawn, validated.target.deploymentTarget, new Date());
    const launchNonce = existing?.launchNonce ?? randomBytes(32).toString("base64url");
    if (!existing) {
      await this.#journal.beginClaim({
        exactClaim: validated.claim,
        launchNonce,
        targetFingerprint: spawnTargetFingerprint([
          validated.target.projectContextRef,
          validated.target.deploymentTarget.adapterId,
          validated.target.deploymentTarget.deploymentScope,
          validated.target.deploymentTarget.logicalTargetId,
          PI_RPC_TARGET_SHAPE,
        ]),
        createdAt: new Date().toISOString(),
      });
    }

    const gate = this.#registry.gateFor(validated.claim.logicalTargetId!.value);
    const lease = await gate.acquireReplacement(claimOperationId);
    let launchAttempted = false;
    let launched: PiRpcRuntime | undefined;
    let successor: RpcPiSession | undefined;
    let unsubscribeCandidateLifecycle: (() => void) | undefined;
    let externalRuntime: RuntimeGenerationRef | undefined;
    let lastPhase = validated.continuationMode === "fresh"
      ? SpawnExecutionPhase.OFFERED
      : SpawnExecutionPhase.QUIESCING_PRIOR;
    let priorEntry: RuntimeSessionEntry | undefined;
    let priorTerminated = false;
    let lastPhaseHasNoSuccessorProof = false;
    let seal: MaterializedSessionSeal | undefined;
    let promotionCommitted = false;

    try {
      if (validated.continuationMode !== "fresh") {
        priorEntry = this.#registry.resolvePrior(validated.claim.expectedPrior!);
        if (!priorEntry) {
          throw new SpawnSupervisorError("exact continuation prior is not current", FailureCode.DELIVERY_REJECTED);
        }
        const prior = requireRpcSession(priorEntry.session);
        await this.#quiescePrior(validated, priorEntry, prior, lease);
        await this.#recordProgress(
          validated,
          SpawnExecutionPhase.QUIESCING_PRIOR,
          ExternalEffectDisposition.PROVED_NONE,
          FailureCode.EXECUTION_FAILED,
          undefined,
          false,
          true,
        );
        lastPhase = SpawnExecutionPhase.QUIESCING_PRIOR;
        lastPhaseHasNoSuccessorProof = true;

        const priorRuntime = prior.runtimeForSupervisor(lease);
        const handshake = await this.#runtimePort.handshake(priorRuntime, {
          expectedProjectCwd: validated.target.cwd,
          expectedExtensionPath: validated.target.controlExtensionPath,
        });
        const entries = await prior.getEntries(undefined, lease);
        const materialization = await classifyPiSessionMaterialization({
          sessionId: handshake.sessionId,
          declaredPath: handshake.sessionFile,
          allowedRoot: validated.target.sessionRoot,
          rpcEntries: entries.entries,
          rpcLeafId: entries.leafId,
        });
        if (validated.continuationMode === "require_resume" && materialization.kind !== "materialized") {
          throw new SpawnSupervisorError("require_resume session is not materialized and valid", FailureCode.EXECUTION_FAILED);
        }
        if (materialization.kind === "materialized") seal = materialization.seal;

        // Dispose first so process/stdout/lifecycle callbacks are fenced before
        // TERM→KILL begins; RpcPiSession delegates the termination itself.
        await prior.dispose();
        priorTerminated = true;
        await this.#recordProgress(
          validated,
          SpawnExecutionPhase.PRIOR_TERMINATED,
          ExternalEffectDisposition.PROVED_NONE,
          FailureCode.EXECUTION_FAILED,
          undefined,
          false,
          true,
        );
        await this.#core.reportSessionState(priorEntry, "offline", "unknown");
        lastPhase = SpawnExecutionPhase.PRIOR_TERMINATED;
        lastPhaseHasNoSuccessorProof = true;

        if (seal) {
          const revalidated = await verifyMaterializedSessionSeal({
            seal,
            sessionId: seal.sessionId,
            declaredPath: seal.canonicalPath,
            allowedRoot: validated.target.sessionRoot,
            rpcEntries: entries.entries,
            rpcLeafId: entries.leafId,
          });
          if (revalidated.kind !== "materialized") {
            throw new SpawnSupervisorError(
              "sealed continuation session changed before successor launch",
              FailureCode.EXECUTION_FAILED,
            );
          }
        }
      }

      const resumeSelector = validated.continuationMode === "require_resume" ? seal?.canonicalPath : undefined;
      if (validated.continuationMode === "require_resume" && !resumeSelector) {
        throw new SpawnSupervisorError("require_resume has no verified selector", FailureCode.EXECUTION_FAILED);
      }
      const launchSpec = await this.#launchSpec(validated.target, launchNonce, resumeSelector);
      await this.#journal.recordPhase({
        claimOperationId,
        phase: SpawnExecutionPhase.LAUNCH_ATTEMPTED,
        externalEffectDisposition: ExternalEffectDisposition.MAY_EXIST,
        recordedAt: new Date().toISOString(),
        poisoned: true,
      });
      lastPhase = SpawnExecutionPhase.LAUNCH_ATTEMPTED;
      lastPhaseHasNoSuccessorProof = false;
      launchAttempted = true;
      launched = await this.#runtimePort.launch(launchSpec);

      // Bind enough strict RPC state to identify the external runtime. A loss
      // before this point is may-exist ambiguity and never causes relaunch.
      successor = await RpcPiSession.bind({
        generation: Number(validated.claim.claimedGeneration!.value),
        runtime: launched,
        runtimePort: this.#runtimePort,
        actionGate: gate,
        publication: "claimed_successor",
      }, lease);
      let resolveCandidateLifecycle!: (event: SessionLifecycleEvent) => void;
      const candidateLifecycle = new Promise<SessionLifecycleEvent>((resolve) => {
        resolveCandidateLifecycle = resolve;
      });
      unsubscribeCandidateLifecycle = successor.onLifecycle(resolveCandidateLifecycle);
      externalRuntime = runtimeRef(validated.claim, this.#core.adapterId, validated.target.deploymentTarget.deploymentScope, successor.runtimeSessionId);
      await this.#journal.recordExternalIdentity({
        claimOperationId,
        runtime: externalRuntime,
        processToken: launched.processToken,
        pid: launched.pid,
        recordedAt: new Date().toISOString(),
      });
      await this.#core.reportSpawnEvidence({
        operation: validated.operation,
        exactClaim: validated.claim,
        phase: SpawnExecutionPhase.LAUNCH_ATTEMPTED,
        disposition: ExternalEffectDisposition.IDENTIFIED,
        failureCode: FailureCode.UNSPECIFIED,
        externalRuntime,
      });
      await this.#recordProgress(
        validated,
        SpawnExecutionPhase.EXTERNAL_IDENTITY_KNOWN,
        ExternalEffectDisposition.IDENTIFIED,
        FailureCode.UNSPECIFIED,
        externalRuntime,
      );
      lastPhase = SpawnExecutionPhase.EXTERNAL_IDENTITY_KNOWN;

      const handshake = await this.#runtimePort.handshake(launched, {
        expectedProjectCwd: validated.target.cwd,
        expectedExtensionPath: validated.target.controlExtensionPath,
      });
      if (handshake.sessionId !== successor.runtimeSessionId) {
        throw new SpawnSupervisorError("successor handshake session id mismatched", FailureCode.EXECUTION_FAILED, true);
      }
      const successorEntries = await successor.getEntries(undefined, lease);
      let persistence = PiSpawnPersistence.MEMORY_ONLY;
      if (validated.continuationMode === "require_resume") {
        const resumed = await verifyResumedSessionExtension({
          seal: seal!,
          handshake,
          sessionId: handshake.sessionId,
          declaredPath: handshake.sessionFile,
          allowedRoot: validated.target.sessionRoot,
          rpcEntries: successorEntries.entries,
          rpcLeafId: successorEntries.leafId,
        });
        if (resumed.kind !== "materialized") {
          throw new SpawnSupervisorError("successor failed sealed-prefix verification", FailureCode.EXECUTION_FAILED, true);
        }
        persistence = PiSpawnPersistence.MATERIALIZED;
      } else {
        const materialization = await classifyPiSessionMaterialization({
          sessionId: handshake.sessionId,
          declaredPath: handshake.sessionFile,
          allowedRoot: validated.target.sessionRoot,
          rpcEntries: successorEntries.entries,
          rpcLeafId: successorEntries.leafId,
        });
        if (materialization.kind === "invalid") {
          throw new SpawnSupervisorError("successor session tree is invalid", FailureCode.EXECUTION_FAILED, true);
        }
        if (materialization.kind === "materialized") persistence = PiSpawnPersistence.MATERIALIZED;
      }
      await this.#recordProgress(
        validated,
        SpawnExecutionPhase.HANDSHAKE_RECONCILING,
        ExternalEffectDisposition.IDENTIFIED,
        FailureCode.UNSPECIFIED,
        externalRuntime,
      );
      lastPhase = SpawnExecutionPhase.HANDSHAKE_RECONCILING;

      const projection = await this.#reconciler.stageClaimedSuccessor(
        externalRuntime,
        successorEntries.entries,
        successorEntries.leafId,
      );
      const status = validated.continuationMode === "require_resume"
        ? ContinuationContextStatus.RESUMED
        : validated.continuationMode === "allow_new_context"
          ? ContinuationContextStatus.NEW_CONTEXT
          : ContinuationContextStatus.UNSPECIFIED;
      const entry = this.#registry.stageCandidate(
        validated.claim,
        {
          runtimeSessionId: successor.runtimeSessionId,
          deploymentScope: validated.target.deploymentTarget.deploymentScope,
          cwd: validated.target.cwd,
          logicalTargetId: validated.claim.logicalTargetId!.value,
        },
        successor,
        this.#observeTranscript,
        this.#observeModelChange,
        this.#observeLifecycle,
      );
      await this.#core.stageSuccessor({
        acceptedSpawn,
        runtime: externalRuntime,
        entry,
        continuationContextStatus: status,
      });
      const result = create(PiSpawnResultSchema, {
        continuationContextStatus: status,
        persistence,
        readinessDigest: projection.readinessDigest,
      });
      // Result can atomically cause core promotion; arm the exact claim before
      // the RPC so a fast promotion delivery can be held without retaining
      // unrelated historical promotions replayed from cursor zero.
      this.#promotionEligibleClaims.add(claimOperationId);
      await this.#core.reportSpawnResult(validated.operation, toBinary(PiSpawnResultSchema, result));
      await this.#recordProgress(
        validated,
        SpawnExecutionPhase.SUCCESS_EVIDENCE_REPORTED,
        ExternalEffectDisposition.IDENTIFIED,
        FailureCode.UNSPECIFIED,
        externalRuntime,
      );
      lastPhase = SpawnExecutionPhase.SUCCESS_EVIDENCE_REPORTED;

      const promotionOutcome = await Promise.race([
        this.#waitForPromotion(validated.claim, externalRuntime).then((promotion) => ({
          case: "promotion" as const,
          promotion,
        })),
        candidateLifecycle.then((event) => ({ case: "lifecycle" as const, event })),
      ]);
      if (promotionOutcome.case === "lifecycle") {
        this.#waiters.delete(claimOperationId);
        throw new SpawnSupervisorError(
          `claimed successor ended before promotion (${promotionOutcome.event.kind})`,
          FailureCode.EXECUTION_OUTCOME_UNKNOWN,
          true,
        );
      }
      const promotion = promotionOutcome.promotion;
      requireExactPromotion(promotion, validated.claim, externalRuntime);
      unsubscribeCandidateLifecycle();
      unsubscribeCandidateLifecycle = undefined;
      promotionCommitted = true;
      const promotedEntry = this.#registry.promoteCandidate(validated.claim, externalRuntime);
      try {
        await this.#reconciler.publishAfterPromotion(projection, successor);
        await this.#journal.markPromoted(claimOperationId);
        lease.promoted();
      } catch {
        if (gate.fencedClaimOperationId === claimOperationId) lease.poison();
        await this.#core.reportSessionState(promotedEntry, "stale", "unknown").catch(() => undefined);
        const publicationError = new SpawnSupervisorError(
          "successor publication failed after core promotion",
          FailureCode.EXECUTION_OUTCOME_UNKNOWN,
          true,
        );
        publicationError.terminalReported = true;
        throw publicationError;
      }
      await this.#core.reportSessionState(promotedEntry, "live", successor.getState().idle ? "idle" : "working");
      this.#promotionEligibleClaims.delete(claimOperationId);
      return Object.freeze({ runtime: externalRuntime, entry: promotedEntry, projection, continuationContextStatus: status });
    } catch (error) {
      unsubscribeCandidateLifecycle?.();
      this.#promotionEligibleClaims.delete(claimOperationId);
      this.#earlyPromotions.delete(claimOperationId);
      this.#waiters.delete(claimOperationId);
      const supervisorError = normalizeSupervisorError(error, launchAttempted);
      if (!promotionCommitted) {
        if (launchAttempted) {
          if (externalRuntime) {
            await this.#recordProgress(
              validated,
              lastPhase,
              ExternalEffectDisposition.IDENTIFIED,
              supervisorError.failureCode,
              externalRuntime,
              true,
            ).catch(() => undefined);
          } else {
            await this.#core.reportSpawnEvidence({
              operation: validated.operation,
              exactClaim: validated.claim,
              phase: SpawnExecutionPhase.LAUNCH_ATTEMPTED,
              disposition: ExternalEffectDisposition.MAY_EXIST,
              failureCode: FailureCode.EXECUTION_OUTCOME_UNKNOWN,
            }).catch(() => undefined);
          }
          const candidate = this.#registry.discardCandidate(claimOperationId);
          if (candidate) {
            await candidate.session.dispose().catch(() => undefined);
          } else if (successor) {
            await successor.dispose().catch(() => undefined);
          } else if (launched) {
            await this.#runtimePort.terminate(launched).catch(() => undefined);
          }
          if (gate.fencedClaimOperationId === claimOperationId) lease.poison();
          supervisorError.terminalReported = await this.#core
            .reportSpawnFailure(validated.operation, FailureCode.EXECUTION_OUTCOME_UNKNOWN)
            .then(() => true, () => false);
        } else if (gate.fencedClaimOperationId === claimOperationId) {
          if (!lastPhaseHasNoSuccessorProof) {
            await this.#recordNoSuccessorEffect(validated, lastPhase).catch(() => undefined);
          }
          if (priorEntry) {
            await this.#core.reportSessionState(
              priorEntry,
              priorTerminated ? "offline" : "live",
              priorTerminated ? "unknown" : "idle",
            ).catch(() => undefined);
          }
          lease.release();
          supervisorError.terminalReported = await this.#core
            .reportSpawnFailure(validated.operation, supervisorError.failureCode)
            .then(() => true, () => false);
        }
      }
      throw supervisorError;
    }
  }

  acceptPromotion(promotion: SpawnPromotionCommitted): boolean {
    const claim = promotion.acceptedClaim?.claim;
    const claimOperationId = claim?.claimOperationId?.value;
    if (!claimOperationId) return false;
    const waiter = this.#waiters.get(claimOperationId);
    if (!waiter) {
      if (this.#promotionEligibleClaims.has(claimOperationId)) {
        this.#earlyPromotions.set(claimOperationId, promotion);
      }
      return true;
    }
    try {
      requireExactPromotion(promotion, waiter.exactClaim, waiter.runtime);
      this.#waiters.delete(claimOperationId);
      waiter.resolve(promotion);
    } catch (error) {
      waiter.reject(error instanceof Error ? error : new Error(String(error)));
    }
    return true;
  }

  async #validate(acceptedSpawn: SpawnClaimAccepted): Promise<ValidatedSpawn> {
    const operation = acceptedSpawn.acceptedOperation?.operation;
    const claim = acceptedSpawn.claim;
    if (
      !operation?.commandId?.value ||
      !claim?.claimOperationId?.value ||
      claim.claimOperationId.value !== operation.commandId.value ||
      !claim.logicalTargetId?.value ||
      !claim.claimedGeneration?.value ||
      claim.claimedGeneration.value > BigInt(Number.MAX_SAFE_INTEGER)
    ) {
      throw new SpawnSupervisorError("accepted spawn envelope is incomplete", FailureCode.DELIVERY_REJECTED);
    }
    const payload = operation.payload;
    if (!payload || payload.contentType !== PayloadContentType.PROTOBUF || payload.schemaRef !== "patchbay.SpawnRequest") {
      throw new SpawnSupervisorError("spawn request envelope is invalid", FailureCode.DELIVERY_REJECTED);
    }
    let request;
    try {
      request = fromBinary(SpawnRequestSchema, payload.payload);
    } catch {
      throw new SpawnSupervisorError("spawn request protobuf is invalid", FailureCode.DELIVERY_REJECTED);
    }
    if (request.targetSpec?.shape !== PI_RPC_TARGET_SHAPE) {
      throw new SpawnSupervisorError("Pi spawn target shape is unsupported", FailureCode.UNSUPPORTED_COMMAND);
    }
    const adapterPayload = request.targetSpec.adapterPayload;
    if (
      !adapterPayload ||
      adapterPayload.contentType !== PayloadContentType.PROTOBUF ||
      adapterPayload.schemaRef !== PI_SPAWN_TARGET_SCHEMA_REF
    ) {
      throw new SpawnSupervisorError("Pi target payload is invalid", FailureCode.DELIVERY_REJECTED);
    }
    let piTarget;
    try {
      piTarget = fromBinary(PiSpawnTargetSpecSchema, adapterPayload.payload);
    } catch {
      throw new SpawnSupervisorError("Pi target protobuf is invalid", FailureCode.DELIVERY_REJECTED);
    }
    const target = this.#targets.get(piTarget.projectContextRef);
    if (!target || target.deploymentTarget.logicalTargetId !== claim.logicalTargetId.value) {
      throw new SpawnSupervisorError("Pi project context is not configured for the logical target", FailureCode.DELIVERY_REJECTED);
    }
    const continuationMode = request.intent.case === "fresh"
      ? "fresh"
      : request.intent.case === "continuation" && piTarget.continuationMode === PiContinuationMode.REQUIRE_RESUME
        ? "require_resume"
        : request.intent.case === "continuation" && piTarget.continuationMode === PiContinuationMode.ALLOW_NEW_CONTEXT
          ? "allow_new_context"
          : undefined;
    if (!continuationMode) {
      throw new SpawnSupervisorError("Pi continuation mode is invalid", FailureCode.DELIVERY_REJECTED);
    }
    if (
      continuationMode === "fresh" && (claim.expectedPrior || claim.claimedGeneration.value !== 1n) ||
      continuationMode !== "fresh" && (!claim.expectedPrior || claim.claimedGeneration.value !== claim.expectedPrior.externalRuntime!.generation!.value + 1n)
    ) {
      throw new SpawnSupervisorError("Pi spawn generation is not the exact accepted claim", FailureCode.DELIVERY_REJECTED);
    }
    return Object.freeze({ acceptedSpawn, operation, claim, target, continuationMode });
  }

  async #quiescePrior(
    validated: ValidatedSpawn,
    entry: RuntimeSessionEntry,
    prior: RpcPiSession,
    lease: RuntimeReplacementLease,
  ): Promise<void> {
    const state = await prior.refreshState(lease);
    const activity = prior.activitySnapshot();
    if (state.streaming || state.compacting || state.pendingMessageCount > 0) {
      await prior.requestUnderLease({ type: "abort" }, lease);
      await prior.waitForSettled(Math.max(activity.activityEpoch, 1), DEFAULT_SETTLE_TIMEOUT_MS);
      await prior.refreshState(lease);
    }
    await this.#core.flushObservations();
    if (!prior.getState().idle) {
      await this.#core.reportSessionState(entry, "stale", "unknown");
      throw new SpawnSupervisorError("prior runtime did not reach a settled state", FailureCode.EXECUTION_OUTCOME_UNKNOWN, true);
    }
    await this.#core.reportSessionState(entry, "live", "idle");
    if (validated.acceptedSpawn.pendingReplacement?.exactPrior &&
      !sameRuntime(validated.acceptedSpawn.pendingReplacement.exactPrior, validated.claim.expectedPrior!)) {
      throw new SpawnSupervisorError("pending replacement fence does not match the exact prior", FailureCode.DELIVERY_REJECTED);
    }
  }

  async #recordProgress(
    validated: ValidatedSpawn,
    phase: SpawnExecutionPhase,
    disposition: ExternalEffectDisposition,
    failureCode: FailureCode,
    externalRuntime?: RuntimeGenerationRef,
    poisoned = false,
    supervisorNoEffectProof = false,
  ): Promise<void> {
    await this.#journal.recordPhase({
      claimOperationId: validated.claim.claimOperationId!.value,
      phase,
      externalEffectDisposition: disposition,
      recordedAt: new Date().toISOString(),
      ...(poisoned ? { poisoned: true } : {}),
    });
    await this.#core.reportSpawnEvidence({
      operation: validated.operation,
      exactClaim: validated.claim,
      phase,
      disposition,
      failureCode,
      ...(externalRuntime ? { externalRuntime } : {}),
      ...(supervisorNoEffectProof ? { supervisorNoEffectProof: true } : {}),
    });
  }

  async #recordNoSuccessorEffect(validated: ValidatedSpawn, phase: SpawnExecutionPhase): Promise<void> {
    const allowedPhase = validated.continuationMode === "fresh"
      ? SpawnExecutionPhase.OFFERED
      : phase === SpawnExecutionPhase.PRIOR_TERMINATED
        ? SpawnExecutionPhase.PRIOR_TERMINATED
        : SpawnExecutionPhase.QUIESCING_PRIOR;
    await this.#core.reportSpawnEvidence({
      operation: validated.operation,
      exactClaim: validated.claim,
      phase: allowedPhase,
      disposition: ExternalEffectDisposition.PROVED_NONE,
      failureCode: FailureCode.EXECUTION_FAILED,
      supervisorNoEffectProof: true,
    });
  }

  async #launchSpec(
    target: ManagedPiTargetConfig,
    launchNonce: string,
    sessionPath?: string,
  ): Promise<PiLaunchSpec> {
    const cwd = await canonicalPath(target.cwd);
    const executable = await canonicalPath(target.executable);
    const cliPath = await canonicalPath(target.cliPath);
    const controlExtensionPath = await canonicalPath(target.controlExtensionPath);
    const argv = buildPiRpcArgv({
      cliPath,
      controlExtensionPath,
      ...(sessionPath ? { sessionPath } : {}),
      ...(target.sessionDirectory ? { sessionDirectory: target.sessionDirectory } : {}),
      ...(target.model ? { model: target.model } : {}),
      ...(target.additionalArguments ? { additionalArguments: target.additionalArguments } : {}),
    });
    return Object.freeze({
      executable,
      argv,
      cwd,
      launchNonce,
      ...(target.environment ? { environment: target.environment } : {}),
    });
  }

  #waitForPromotion(
    exactClaim: SpawnGenerationClaim,
    runtime: RuntimeGenerationRef,
  ): Promise<SpawnPromotionCommitted> {
    const claimOperationId = exactClaim.claimOperationId!.value;
    const early = this.#earlyPromotions.get(claimOperationId);
    if (early) {
      this.#earlyPromotions.delete(claimOperationId);
      requireExactPromotion(early, exactClaim, runtime);
      return Promise.resolve(early);
    }
    if (this.#waiters.has(claimOperationId)) {
      return Promise.reject(new Error("spawn promotion waiter already exists"));
    }
    return new Promise<SpawnPromotionCommitted>((resolve, reject) => {
      this.#waiters.set(claimOperationId, { exactClaim, runtime, resolve, reject });
    });
  }
}

export function supervisorNoEffectProof(
  adapterId: string,
  adapterGeneration: number,
) {
  return create(NoExternalEffectProofSchema, {
    proof: {
      case: "exactSupervisorPreLaunchFailure",
      value: create(SupervisorPreLaunchFailureProofSchema, {
        adapterId: create(AdapterIdSchema, { value: adapterId }),
        adapterGeneration: create(GenerationSchema, { value: BigInt(adapterGeneration) }),
      }),
    },
  });
}

function runtimeRef(
  claim: SpawnGenerationClaim,
  adapterId: string,
  deploymentScope: string,
  runtimeSessionId: string,
): RuntimeGenerationRef {
  return create(RuntimeGenerationRefSchema, {
    logicalTargetId: create(LogicalTargetIdSchema, { value: claim.logicalTargetId!.value }),
    externalRuntime: create(ExternalRuntimeRefSchema, {
      adapterId: create(AdapterIdSchema, { value: adapterId }),
      deploymentScope,
      runtimeSessionId: { $typeName: "patchbay.RuntimeSessionId", value: runtimeSessionId },
      generation: create(GenerationSchema, { value: claim.claimedGeneration!.value }),
    }),
  });
}

function requireRpcSession(session: PiSession): RpcPiSession {
  if (!(session instanceof RpcPiSession)) {
    throw new SpawnSupervisorError("managed continuation prior is not an RPC runtime", FailureCode.EXECUTION_FAILED);
  }
  return session;
}

function requireExactPromotion(
  promotion: SpawnPromotionCommitted,
  exactClaim: SpawnGenerationClaim,
  runtime: RuntimeGenerationRef,
): void {
  if (!sameClaim(promotion.acceptedClaim?.claim, exactClaim) || !sameRuntime(promotion.promotedRuntime, runtime)) {
    throw new SpawnSupervisorError("promotion does not match the exact claim and runtime", FailureCode.STALE_EVENT);
  }
}

function sameClaim(left: SpawnGenerationClaim | undefined, right: SpawnGenerationClaim): boolean {
  return !!left &&
    left.authorityDomainId?.value === right.authorityDomainId?.value &&
    left.claimOperationId?.value === right.claimOperationId?.value &&
    left.logicalTargetId?.value === right.logicalTargetId?.value &&
    left.claimedGeneration?.value === right.claimedGeneration?.value &&
    ((!left.expectedPrior && !right.expectedPrior) ||
      (!!left.expectedPrior && !!right.expectedPrior && sameRuntime(left.expectedPrior, right.expectedPrior)));
}

function sameRuntime(left: RuntimeGenerationRef | undefined, right: RuntimeGenerationRef): boolean {
  return !!left &&
    left.logicalTargetId?.value === right.logicalTargetId?.value &&
    left.externalRuntime?.adapterId?.value === right.externalRuntime?.adapterId?.value &&
    left.externalRuntime?.deploymentScope === right.externalRuntime?.deploymentScope &&
    left.externalRuntime?.runtimeSessionId?.value === right.externalRuntime?.runtimeSessionId?.value &&
    left.externalRuntime?.generation?.value === right.externalRuntime?.generation?.value;
}

function normalizeSupervisorError(error: unknown, launched: boolean): SpawnSupervisorError {
  if (error instanceof SpawnSupervisorError) return error;
  return new SpawnSupervisorError(
    launched ? "spawn outcome is ambiguous after launch" : "spawn failed before successor launch",
    launched ? FailureCode.EXECUTION_OUTCOME_UNKNOWN : FailureCode.EXECUTION_FAILED,
    launched,
  );
}

async function canonicalPath(value: string): Promise<string> {
  if (!isAbsolute(value) || normalize(value) !== value) throw new Error("managed Pi path is not canonical absolute");
  return realpath(value);
}
