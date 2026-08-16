import { Code, ConnectError } from "@connectrpc/connect";
import { create } from "@bufbuild/protobuf";
import { randomBytes } from "node:crypto";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  CommandIdSchema,
  FailureCode,
  GenerationSchema,
  OperationKind,
  OperationSchema,
  RuntimeSessionIdSchema,
  SessionActivityState,
  SessionConnectivityState,
  SpawnPriorWorkDisposition,
  TargetScopeKind,
  TargetScopeSchema,
  type ContinuationContextStatus,
  type Delivery,
  type Operation,
  type RuntimeGenerationRef,
  type SpawnClaimAccepted,
} from "@patchbay/contracts";
import { PatchbayCoreClient, type SessionIdentity } from "./core_client.js";
import {
  diagnosticError,
  NOOP_ADAPTER_DIAGNOSTICS,
  openAdapterDiagnostics,
  resolveAdapterLogPath,
  type AdapterDiagnosticError,
  type AdapterDiagnosticInput,
  type AdapterDiagnostics,
  type AdapterDiagnosticSessionRef,
} from "./adapter_diagnostics.js";
import { DeliveryTranslator, UnsupportedCommandError } from "./delivery.js";
import {
  authorizeDeploymentIfRequired,
  DEPLOYMENT_AUTHORITY_ERROR_CODES,
  DeploymentAuthorityError,
  type DeploymentAuthorityRequest,
  type DeploymentAuthorityResolver,
} from "./deployment_authority.js";
import { composeAdapterDiagnostics, CoreDiagnosticsForwarder } from "./core_diagnostics_forwarder.js";
import { RpcPiSession, type PiSession } from "./pi_session.js";
import { PiRpcTransportError } from "./rpc_client.js";
import {
  buildPiRpcArgv,
  RpcManagedPiRuntimePort,
  type ManagedPiRuntimePort,
} from "./pi_process.js";
import { FileSpawnEffectJournal } from "./spawn_journal.js";
import {
  ClaimAwareSpawnSupervisor,
  LocalStagedPiReconciler,
  type ManagedPiTargetConfig,
  type SpawnSupervisorCorePort,
} from "./spawn_supervisor.js";
import {
  SessionRegistry,
  type RuntimeSessionEntry,
} from "./session_registry.js";
import { projectSessionEntries } from "./transcript_projection.js";
import type { TranscriptEvent } from "./transcript_event.js";
import {
  nextSessionReportSequence,
  type SessionReportOrder,
  type SessionReportSequence,
} from "./session_report_sequencer.js";

export interface PreprovisionedSession {
  readonly runtimeSessionId: string;
  readonly deploymentScope: string;
  readonly cwd: string;
  readonly project?: string;
  readonly name?: string;
  readonly model?: string;
  readonly generation?: number;
  /** Managed logical targets are intentionally absent; they recover via accepted claims. */
  readonly logicalTargetId?: never;
  readonly sessionPath?: string;
  readonly sessionRoot?: string;
  readonly sessionDirectory?: string;
}

export interface AdapterProcessOptions {
  coreAddress: string;
  adapterId: string;
  authorityDomainId: string;
  attachmentEvidence: string;
  adapterGeneration: number;
  sessions: PreprovisionedSession[];
  createSession?: (options: PreprovisionedSession) => Promise<PiSession>;
  diagnostics?: AdapterDiagnostics;
  forwardDiagnostics?: boolean;
  deploymentAuthorityResolver?: DeploymentAuthorityResolver;
  managedTargets?: readonly ManagedPiTargetConfig[];
  spawnJournalDirectory?: string;
  managedRuntimePort?: ManagedPiRuntimePort;
}

interface StartedDelivery {
  completion: Promise<void>;
}

interface ObservationDiagnosticContext {
  session: AdapterDiagnosticSessionRef;
  observationKind: "transcript" | "session-report";
}

/** Composition root: gRPC client + complete runtime-session registry. */
export class AdapterProcess {
  readonly #options: AdapterProcessOptions;
  readonly #core: PatchbayCoreClient;
  readonly #registry = new SessionRegistry();
  readonly #translator = new DeliveryTranslator();
  readonly #activeCommands = new Map<string, { readonly commandId: string; readonly operation: Operation }>();
  readonly #replacementResolvedCommands = new Set<string>();
  readonly #runtimePort: ManagedPiRuntimePort;
  readonly #spawnSupervisor: ClaimAwareSpawnSupervisor;
  readonly #pendingObservations = new Set<Promise<void>>();
  // Per-session report chains: transcript events (hundreds of deltas per turn)
  // must reach the core in stream order. Firing each ingestTranscript
  // concurrently lets them race, and the core appends in arrival order —
  // scrambling the delta order the cockpit folds (found in live use: agent
  // messages rendered word fragments out of order until the commit replaced
  // them). Chaining each report after the previous one preserves order at the
  // cost of one round-trip per delta, which is negligible for streaming.
  readonly #observationTails = new Map<string, Promise<void>>();
  readonly #sessionReportSequences = new Map<string, SessionReportSequence>();
  #observationError: unknown;
  #cursor = 0n;
  #started = false;
  #disposed = false;
  #runController: AbortController | undefined;
  #diagnostics: AdapterDiagnostics;
  #observationErrorContext: ObservationDiagnosticContext | undefined;

  constructor(options: AdapterProcessOptions) {
    this.#options = options;
    const localDiagnostics = options.diagnostics ?? NOOP_ADAPTER_DIAGNOSTICS;
    this.#diagnostics = localDiagnostics;
    this.#core = new PatchbayCoreClient(options, localDiagnostics);
    if (options.forwardDiagnostics) {
      const forwarder = new CoreDiagnosticsForwarder(
        (report, signal) => this.#core.reportDiagnostic(report, signal),
        {
          authorityDomainId: options.authorityDomainId,
          adapterId: options.adapterId,
          adapterGeneration: options.adapterGeneration,
        },
      );
      this.#diagnostics = composeAdapterDiagnostics([localDiagnostics, forwarder]);
      this.#core.setDiagnostics(this.#diagnostics);
    }
    this.#runtimePort = options.managedRuntimePort ?? new RpcManagedPiRuntimePort();
    const corePort: SpawnSupervisorCorePort = {
      adapterId: options.adapterId,
      adapterGeneration: options.adapterGeneration,
      authorizeDeployment: async (acceptedSpawn, target, now) => {
        await this.authorizeDeployment({ acceptedSpawn, target }, now);
      },
      flushObservations: () => this.flushObservations(),
      reportSpawnEvidence: async (input) => {
        if (!input.operation) throw new Error("spawn evidence operation is missing");
        await this.#core.reportSpawnEvidence({ ...input, operation: input.operation });
      },
      reportSessionState: async (entry, connectivity, activity) => {
        await this.#queueSessionReport(
          entry,
          sessionActivity(activity),
          sessionConnectivity(connectivity),
        );
      },
      reportRecoveredSessionState: async (runtime, connectivity, activity) => {
        await this.#queueRecoveredSessionReport(
          runtime,
          sessionActivity(activity),
          sessionConnectivity(connectivity),
        );
      },
      resolvePriorWorkEffects: async ({ exactPrior, effects }) => {
        for (const effect of effects) {
          if (effect.disposition === SpawnPriorWorkDisposition.SUPERSEDED_BEFORE_OFFER) {
            // The accepted decision atomically terminalized never-offered work.
            continue;
          }
          if (effect.disposition !== SpawnPriorWorkDisposition.QUIESCE_OUTCOME_RECONCILIATION) {
            throw new Error("validated prior-work effect changed before reconciliation");
          }
          const commandId = effect.commandId!.value;
          this.#replacementResolvedCommands.add(commandId);
          const active = [...this.#activeCommands.values()]
            .find((candidate) => candidate.commandId === commandId);
          const operation = active?.operation ?? operationForPriorWork(commandId, exactPrior);
          await this.#core.ingestFailure(
            operation,
            FailureCode.EXECUTION_OUTCOME_UNKNOWN,
            "replacement_quiesce_outcome_unknown",
          );
          if (!active) this.#replacementResolvedCommands.delete(commandId);
        }
      },
      stageSuccessor: async ({ acceptedSpawn, entry, continuationContextStatus }) => {
        const claimOperationId = acceptedSpawn.claim?.claimOperationId?.value;
        if (!claimOperationId) throw new Error("staged successor has no exact claim operation id");
        await this.#queueSessionReport(
          entry,
          SessionActivityState.UNKNOWN,
          SessionConnectivityState.LIVE,
          undefined,
          { claimOperationId, continuationContextStatus },
        );
      },
      reportSpawnResult: async (operation, payload) => {
        if (!operation) throw new Error("spawn Result operation is missing");
        await this.#core.reportSpawnResult(operation, payload);
      },
      reportSpawnFailure: async (operation, failureCode) => {
        if (!operation) throw new Error("spawn failure operation is missing");
        await this.#core.ingestFailure(operation, failureCode, "managed_spawn_failed");
      },
    };
    this.#spawnSupervisor = new ClaimAwareSpawnSupervisor({
      runtimePort: this.#runtimePort,
      journal: new FileSpawnEffectJournal(
        options.spawnJournalDirectory ?? resolve(process.cwd(), ".patchbay", "pi-spawn-journal"),
      ),
      registry: this.#registry,
      core: corePort,
      targets: options.managedTargets ?? [],
      reconciler: new LocalStagedPiReconciler(
        (runtime, entries) => this.#publishRecoveredProjection(runtime, entries),
      ),
      observeTranscript: (entry, event) => this.#observeTranscript(entry, event),
      observeModelChange: (entry, model) => this.#observeModelChange(entry, model),
      observeLifecycle: (entry, event) => this.#observeLifecycle(entry, event),
    });
  }

  async start(): Promise<void> {
    if (this.#disposed) throw new Error("adapter process has been disposed");
    if (this.#started) return;
    this.#record({ event: "adapter.starting", level: "info" });
    await this.#core.attach(this.#options.adapterGeneration);
    this.#started = true;
    try {
      await this.#spawnSupervisor.recoverOnStart();
      for (const configured of this.#options.sessions) {
        await this.registerSession(configured);
      }
      this.#record({ event: "adapter.started", level: "info" });
    } catch (error) {
      this.#started = false;
      await this.#registry.dispose();
      throw error;
    }
  }

  /** The same complete registration path used by pre-provisioning and future spawn. */
  async registerSession(configured: PreprovisionedSession): Promise<void> {
    if (!this.#started) throw new Error("adapter process has not started");
    if (configured.logicalTargetId && !this.#options.createSession) {
      throw new Error("managed logical targets recover only from the spawn journal and exact core promotion");
    }
    const createSession = this.#options.createSession ?? ((options) => this.#createProductionSession(options));
    this.#record({ event: "session.register.started", level: "info" });
    let session: PiSession;
    try {
      session = await createSession(configured);
    } catch (error) {
      this.#record({
        event: "session.register.failed",
        level: "error",
        error: diagnosticError(error),
      });
      throw error;
    }
    const sessionRef = (): AdapterDiagnosticSessionRef => ({
      runtimeSessionId: session.runtimeSessionId,
      deploymentScope: configured.deploymentScope,
      generation: session.generation,
    });
    let entry: RuntimeSessionEntry;
    try {
      entry = this.#registry.register(
        configured,
        session,
        (observedEntry, event) => this.#observeTranscript(observedEntry, event),
        (observedEntry, model) => this.#observeModelChange(observedEntry, model),
        (observedEntry, event) => this.#observeLifecycle(observedEntry, event),
      );
    } catch (error) {
      this.#record({
        event: "session.register.failed",
        level: "error",
        session: sessionRef(),
        error: diagnosticError(error),
      });
      try {
        await session.dispose();
      } catch {
        // Preserve the registration failure that caused cleanup.
      }
      throw error;
    }

    // Reconcile Pi's persisted getEntries() snapshot before claiming a current
    // activity state. Unknown-runtime evidence is durably quarantined by the
    // core until the following authenticated report establishes the session.
    try {
      for (const event of await session.snapshotTranscript()) {
        await this.#core.ingestTranscript(this.#identity(entry), event);
      }
      await this.#queueSessionReport(
        entry,
        this.#options.adapterGeneration > 1
          ? SessionActivityState.UNKNOWN
          : SessionActivityState.IDLE,
        SessionConnectivityState.LIVE,
      );
      this.#record({
        event: "session.register.succeeded",
        level: "info",
        session: this.#sessionRef(entry),
      });
    } catch (error) {
      this.#record({
        event: "session.register.failed",
        level: "error",
        session: this.#sessionRef(entry),
        error: diagnosticError(error),
      });
      throw error;
    }
  }

  async #createProductionSession(configured: PreprovisionedSession): Promise<PiSession> {
    const launchNonce = randomBytes(32).toString("base64url");
    const piIndexPath = fileURLToPath(import.meta.resolve("@earendil-works/pi-coding-agent"));
    const cliPath = join(dirname(piIndexPath), "cli.js");
    const controlExtensionPath = fileURLToPath(
      new URL("../extensions/patchbay-control.js", import.meta.url),
    );
    const runtime = await this.#runtimePort.launch({
      executable: process.execPath,
      argv: buildPiRpcArgv({
        cliPath,
        controlExtensionPath,
        ...(configured.sessionPath ? { sessionPath: configured.sessionPath } : {}),
        ...(configured.sessionDirectory ? { sessionDirectory: configured.sessionDirectory } : {}),
        ...(configured.model ? { model: configured.model } : {}),
        ...(configured.name ? { name: configured.name } : {}),
      }),
      cwd: configured.cwd,
      launchNonce,
    });
    try {
      await this.#runtimePort.handshake(runtime, {
        expectedProjectCwd: configured.cwd,
        expectedExtensionPath: controlExtensionPath,
      });
      return await RpcPiSession.bind({
        runtimeSessionId: configured.runtimeSessionId,
        generation: configured.generation ?? 1,
        runtime,
        runtimePort: this.#runtimePort,
        actionGate: this.#registry.gateFor(configured.runtimeSessionId),
        publication: "current",
      });
    } catch (error) {
      await this.#runtimePort.terminate(runtime).catch(() => undefined);
      throw error;
    }
  }

  /** Launch-time precondition used by the downstream spawn supervisor. */
  async authorizeDeployment(
    request: DeploymentAuthorityRequest,
    now: Date,
  ): Promise<{ readonly credentialHandle: string } | undefined> {
    try {
      return await authorizeDeploymentIfRequired(
        this.#options.deploymentAuthorityResolver,
        request,
        now,
      );
    } catch (error) {
      this.#record({
        event: "deployment.authority.denied",
        level: "warn",
        error: deploymentAuthorityDiagnosticError(error),
      });
      throw error;
    }
  }

  async flushObservations(): Promise<void> {
    await Promise.all([...this.#pendingObservations]);
    if (this.#observationError !== undefined) {
      const error = this.#observationError;
      const context = this.#observationErrorContext;
      this.#observationError = undefined;
      this.#observationErrorContext = undefined;
      this.#record({
        event: "observation.flush_failed",
        level: "error",
        ...(context ? context : {}),
        error: diagnosticError(error),
      });
      throw error;
    }
  }

  async run(signal?: AbortSignal): Promise<void> {
    await this.start();
    if (this.#runController) throw new Error("adapter delivery loop is already running");

    const controller = new AbortController();
    this.#runController = controller;
    const abort = () => controller.abort(signal?.reason);
    if (signal?.aborted) abort();
    else signal?.addEventListener("abort", abort, { once: true });

    try {
      while (!controller.signal.aborted) {
        try {
          await this.#consumeDeliveries(controller.signal);
          if (!controller.signal.aborted) {
            throw new ConnectError(
              "delivery subscription ended without shutdown",
              Code.Unavailable,
            );
          }
        } catch (error) {
          if (controller.signal.aborted) return;
          const retryable = isRetryableTransportFailure(error);
          this.#record({
            event: retryable
              ? "delivery.subscription.retrying"
              : "delivery.subscription.failed",
            level: retryable ? "warn" : "error",
            error: diagnosticError(error),
          });
          if (!retryable) throw error;
          await delay(100, controller.signal);
        }
      }
    } finally {
      signal?.removeEventListener("abort", abort);
      controller.abort();
      if (this.#runController === controller) this.#runController = undefined;
    }
  }

  async dispose(): Promise<void> {
    if (this.#disposed) return;
    this.#disposed = true;
    this.#runController?.abort();
    this.#record({ event: "adapter.stopping", level: "info" });
    const entries = [...this.#registry.entries()].map(([, entry]) => entry);
    for (const entry of entries) {
      this.#record({
        event: "session.dispose.started",
        level: "info",
        session: this.#sessionRef(entry),
      });
    }
    try {
      await this.#registry.dispose();
      for (const entry of entries) {
        this.#record({
          event: "session.dispose.succeeded",
          level: "info",
          session: this.#sessionRef(entry),
        });
      }
      this.#started = false;
      this.#record({ event: "adapter.stopped", level: "info" });
    } catch (error) {
      for (const entry of entries) {
        this.#record({
          event: "session.dispose.failed",
          level: "error",
          session: this.#sessionRef(entry),
          error: diagnosticError(error),
        });
      }
      this.#started = false;
      throw error;
    } finally {
      try {
        await this.#diagnostics.flush();
      } catch {
        // A broken diagnostics implementation cannot change disposal semantics.
      }
      try {
        await this.#diagnostics.close();
      } catch {
        // A broken diagnostics implementation cannot change disposal semantics.
      }
    }
  }

  async #consumeDeliveries(signal?: AbortSignal): Promise<void> {
    if (!this.#started) throw new Error("adapter process has not started");
    const inFlight = new Set<Promise<void>>();
    let completionError: unknown;
    try {
      for await (const delivery of this.#core.receiveDeliveries(this.#cursor, signal)) {
        this.#cursor = delivery.deliveryEventId?.lsn?.value ?? this.#cursor;
        if (delivery.promotionCommitted) {
          if (!await this.#spawnSupervisor.acceptPromotion(delivery.promotionCommitted)) {
            throw new Error("spawn promotion delivery is not exactly correlated");
          }
          continue;
        }
        const operation = requiredOperation(delivery);
        const started = await this.#beginDelivery(delivery, operation);

        // Instruction completion remains in flight so the live subscription
        // can receive cancellation. Spawn remains in flight so this same
        // authenticated stream can receive the authority-bearing promotion.
        if (operation.kind === OperationKind.INSTRUCT || operation.kind === OperationKind.SPAWN) {
          let tracked: Promise<void>;
          tracked = started.completion
            .catch((error: unknown) => {
              completionError ??= error;
            })
            .finally(() => inFlight.delete(tracked));
          inFlight.add(tracked);
        } else {
          await started.completion;
        }
      }
    } finally {
      await Promise.all(inFlight);
      await this.flushObservations();
      if (completionError !== undefined) throw completionError;
    }
  }

  async #beginDelivery(delivery: Delivery, operation: Operation): Promise<StartedDelivery> {
    const commandId = operation.commandId?.value;
    const operationKind = operation.kind;
    const target = operation.targetScope;
    const runtimeSessionId = target?.runtimeSessionId?.value;
    const entry = runtimeSessionId ? this.#registry.resolve(runtimeSessionId) : undefined;
    this.#record({
      event: "delivery.received",
      level: "info",
      ...(commandId ? { commandId } : {}),
      operationKind,
      ...(entry ? { session: this.#sessionRef(entry) } : {}),
    });
    // This is the durable delivery checkpoint. ReceiveDeliveries filters on the
    // resulting command state, so an adapter restart from cursor 0 cannot
    // re-offer or re-execute acknowledged history.
    await this.#core.acknowledgeDelivery(operation, delivery.deliveryEventId);
    this.#record({
      event: "delivery.acknowledged",
      level: "info",
      ...(commandId ? { commandId } : {}),
      operationKind,
      ...(entry ? { session: this.#sessionRef(entry) } : {}),
    });

    if (operation.kind === OperationKind.SPAWN) {
      const acceptedSpawn = requiredAcceptedSpawn(delivery, operation);
      await this.#core.reportRunning(operation);
      this.#record({
        event: "delivery.running",
        level: "info",
        ...(commandId ? { commandId } : {}),
        operationKind,
      });
      return {
        completion: this.#spawnSupervisor.handleAcceptedSpawn(acceptedSpawn)
          .then(() => {
            this.#record({
              event: "delivery.completed",
              level: "info",
              ...(commandId ? { commandId } : {}),
              operationKind,
              outcome: "STAGED_AND_PROMOTED",
            });
          })
          .catch(async (error: unknown) => {
            const failureCode = error instanceof Error && "failureCode" in error
              ? error.failureCode as FailureCode
              : FailureCode.EXECUTION_FAILED;
            // Validation can fail before the supervisor has a validated
            // operation/evidence context. This terminalization is idempotent
            // with the supervisor's post-journal failure report.
            if (!(error instanceof Error && "terminalReported" in error && error.terminalReported === true)) {
              await this.#core.ingestFailure(operation, failureCode, "managed_spawn_failed")
                .catch(() => undefined);
            }
            this.#record({
              event: "delivery.failed",
              level: "error",
              ...(commandId ? { commandId } : {}),
              operationKind,
              failureCode,
              error: diagnosticError(error),
            });
          }),
      };
    }

    const targetError = this.#validateTarget(operation, entry);
    if (targetError) {
      return {
        completion: this.#core
          .ingestFailure(operation, FailureCode.DELIVERY_REJECTED, targetError)
          .then(() => {
            this.#record({
              event: "delivery.rejected",
              level: "warn",
              ...(commandId ? { commandId } : {}),
              operationKind,
              failureCode: FailureCode.DELIVERY_REJECTED,
              ...(entry ? { session: this.#sessionRef(entry) } : {}),
              reason: targetError,
            });
          }),
      };
    }

    if (!entry) throw new Error("validated delivery lost its runtime entry");
    try {
      this.#translator.validate(operation);
    } catch (error) {
      if (!(error instanceof UnsupportedCommandError)) throw error;
      const diagnostic = error.message;
      return {
        completion: this.#core
          .ingestFailure(operation, FailureCode.UNSUPPORTED_COMMAND, diagnostic)
          .then(() => {
            this.#record({
              event: "delivery.rejected",
              level: "warn",
              ...(commandId ? { commandId } : {}),
              operationKind,
              failureCode: FailureCode.UNSUPPORTED_COMMAND,
              session: this.#sessionRef(entry),
              error: diagnosticError(error),
            });
          }),
      };
    }
    await this.#core.reportRunning(operation);
    this.#record({
      event: "delivery.running",
      level: "info",
      ...(commandId ? { commandId } : {}),
      operationKind,
      session: this.#sessionRef(entry),
    });
    if (commandId) {
      this.#activeCommands.set(entry.runtimeSessionId, { commandId, operation });
    }
    if (operation.kind === OperationKind.INSTRUCT) {
      await this.#queueSessionReport(entry, SessionActivityState.WORKING);
    }
    return { completion: this.#executeDelivery(operation, entry) };
  }

  async #executeDelivery(operation: Operation, entry: RuntimeSessionEntry): Promise<void> {
    const commandId = operation.commandId?.value;
    const operationKind = operation.kind;
    try {
      const outcome = await this.#translator.deliver(operation, entry.session);
      // For in-generation work, await the serialized observation tail before
      // terminal Result so command-correlated transcript cannot arrive late.
      if (operation.kind === OperationKind.INSTRUCT) {
        await this.#queueSessionReport(entry, SessionActivityState.IDLE);
      }
      if (commandId && this.#replacementResolvedCommands.has(commandId)) return;
      await this.#core.reportResult(operation, outcome.value);
      this.#record({
        event: "delivery.completed",
        level: "info",
        ...(commandId ? { commandId } : {}),
        operationKind,
        session: this.#sessionRef(entry),
        outcome: "COMPLETED",
      });
    } catch (error) {
      if (commandId && this.#replacementResolvedCommands.has(commandId)) return;
      const classification = classifyDeliveryFailure(error);
      await this.#core.ingestFailure(
        operation,
        classification.failureCode,
        classification.diagnostic,
      );
      this.#record({
        event: classification.rejected ? "delivery.rejected" : "delivery.failed",
        level: classification.rejected ? "warn" : "error",
        ...(commandId ? { commandId } : {}),
        operationKind,
        failureCode: classification.failureCode,
        session: this.#sessionRef(entry),
        error: diagnosticError(error),
      });
      if (operation.kind === OperationKind.INSTRUCT || classification.connectivity !== undefined) {
        await this.#queueSessionReport(
          entry,
          SessionActivityState.UNKNOWN,
          classification.connectivity ?? SessionConnectivityState.LIVE,
        );
      }
    } finally {
      const active = this.#activeCommands.get(entry.runtimeSessionId);
      if (commandId && active?.commandId === commandId) {
        this.#activeCommands.delete(entry.runtimeSessionId);
      }
      if (commandId) this.#replacementResolvedCommands.delete(commandId);
    }
  }

  #validateTarget(
    operation: Operation,
    entry: RuntimeSessionEntry | undefined,
  ): string | undefined {
    const target = operation.targetScope;
    if (!target?.runtimeSessionId?.value) return "delivery is missing runtime_session_id";
    if (!entry) return `target runtime session is not registered: ${target.runtimeSessionId.value}`;
    if (target.deploymentScope !== entry.deploymentScope) {
      return `delivery deployment scope ${target.deploymentScope} does not match ${entry.deploymentScope}`;
    }
    const deliveredGeneration = target.sessionGeneration?.value;
    if (deliveredGeneration === undefined) return "delivery is missing session_generation";
    if (deliveredGeneration !== BigInt(entry.session.generation)) {
      return `delivery generation ${deliveredGeneration} does not match live generation ${entry.session.generation}`;
    }
    return undefined;
  }

  #observeTranscript(entry: RuntimeSessionEntry, event: TranscriptEvent): void {
    const identity = this.#identity(entry);
    const activeCommand = this.#activeCommands.get(entry.runtimeSessionId)?.commandId;
    const tail = this.#observationTails.get(entry.runtimeSessionId) ?? Promise.resolve();
    const next = tail
      .then(() => this.#core.ingestTranscript(identity, event, activeCommand))
      .then(() => undefined);
    this.#observationTails.set(entry.runtimeSessionId, next);
    this.#trackObservation(next, {
      session: this.#sessionRef(entry),
      observationKind: "transcript",
    });
  }

  #observeModelChange(entry: RuntimeSessionEntry, model: string): void {
    const activity = entry.session.getState().idle
      ? SessionActivityState.IDLE
      : SessionActivityState.WORKING;
    this.#record({
      event: "session.model.changed",
      level: "info",
      session: this.#sessionRef(entry),
    });
    this.#trackObservation(this.#queueSessionReport(entry, activity, undefined, model), {
      session: this.#sessionRef(entry),
      observationKind: "session-report",
    });
  }

  #observeLifecycle(
    entry: RuntimeSessionEntry,
    event: Parameters<Parameters<PiSession["onLifecycle"]>[0]>[0],
  ): void {
    // Claimed successors are quarantined until promotion. Their failure is
    // correlated through SpawnExecutionEvidence by the supervisor, never an
    // ordinary SessionReport.
    if (this.#registry.resolve(entry.runtimeSessionId) !== entry) return;
    let connectivity: SessionConnectivityState;
    if (event.kind === "transport_loss" && !event.error.processExit) {
      connectivity = SessionConnectivityState.STALE;
    } else if (event.kind === "process_exit" && confirmedCleanProcessExit(event.exit)) {
      connectivity = SessionConnectivityState.OFFLINE;
    } else {
      connectivity = SessionConnectivityState.FAILED;
    }
    this.#trackObservation(
      this.#queueSessionReport(entry, SessionActivityState.UNKNOWN, connectivity),
      { session: this.#sessionRef(entry), observationKind: "session-report" },
    );
  }

  #sessionRef(entry: RuntimeSessionEntry): AdapterDiagnosticSessionRef {
    return {
      runtimeSessionId: entry.runtimeSessionId,
      deploymentScope: entry.deploymentScope,
      generation: entry.session.generation,
    };
  }

  #record(input: AdapterDiagnosticInput): void {
    try {
      this.#diagnostics.record(input);
    } catch {
      // Diagnostics must never change an adapter operation's result.
    }
  }

  #identity(entry: RuntimeSessionEntry, model?: string): SessionIdentity {
    return {
      runtimeSessionId: entry.runtimeSessionId,
      deploymentScope: entry.deploymentScope,
      generation: entry.session.generation,
      project: entry.project ?? "",
      cwd: entry.cwd,
      name: entry.name ?? entry.runtimeSessionId,
      model: model ?? normalizedModel(entry.session.getState().model),
    };
  }

  #queueSessionReport(
    entry: RuntimeSessionEntry,
    activity: SessionActivityState,
    connectivity = SessionConnectivityState.LIVE,
    model?: string,
    spawn?: {
      readonly claimOperationId: string;
      readonly continuationContextStatus: ContinuationContextStatus;
    },
  ): Promise<void> {
    // Capture both payload and order before touching the promise tail. The tail
    // serializes delivery, but it is not allowed to decide producer order or
    // observe a later mutable PiSession state.
    const identity = Object.freeze(this.#identity(entry, model));
    const sourceOrder = this.#allocateSessionReportOrder(
      entry.runtimeSessionId,
      identity.generation,
    );
    const tail = this.#observationTails.get(entry.runtimeSessionId) ?? Promise.resolve();
    const next = tail
      .then(() => this.#core.reportSession(identity, activity, connectivity, sourceOrder, spawn))
      .then(() => {
        this.#record({
          event: "session.activity.reported",
          level: "info",
          session: this.#sessionRef(entry),
          sessionActivity: activity,
          sessionConnectivity: connectivity,
        });
      });
    this.#observationTails.set(entry.runtimeSessionId, next);
    return next;
  }

  async #publishRecoveredProjection(
    runtime: RuntimeGenerationRef,
    entries: readonly unknown[],
  ): Promise<void> {
    const external = runtime.externalRuntime;
    const runtimeSessionId = external?.runtimeSessionId?.value;
    const generation = external?.generation?.value;
    if (
      !runtimeSessionId || !external?.deploymentScope || !generation ||
      generation > BigInt(Number.MAX_SAFE_INTEGER)
    ) {
      throw new Error("recovered projection runtime identity is incomplete");
    }
    const numericGeneration = Number(generation);
    const identity: SessionIdentity = {
      runtimeSessionId,
      deploymentScope: external.deploymentScope,
      generation: numericGeneration,
      project: "",
      cwd: "",
      name: runtimeSessionId,
      model: "",
    };
    const projected = projectSessionEntries(
      entries as Parameters<typeof projectSessionEntries>[0],
      `${runtimeSessionId}:${numericGeneration}`,
    );
    for (const event of projected) {
      await this.#core.ingestTranscript(identity, event);
    }
  }

  #queueRecoveredSessionReport(
    runtime: RuntimeGenerationRef,
    activity: SessionActivityState,
    connectivity: SessionConnectivityState,
  ): Promise<void> {
    const external = runtime.externalRuntime;
    const runtimeSessionId = external?.runtimeSessionId?.value;
    const generation = external?.generation?.value;
    if (
      !runtimeSessionId || !external?.deploymentScope || !generation ||
      generation > BigInt(Number.MAX_SAFE_INTEGER)
    ) {
      return Promise.reject(new Error("recovered runtime identity is incomplete"));
    }
    const numericGeneration = Number(generation);
    const identity: SessionIdentity = Object.freeze({
      runtimeSessionId,
      deploymentScope: external.deploymentScope,
      generation: numericGeneration,
      project: "",
      cwd: "",
      name: runtimeSessionId,
      model: "",
    });
    const sourceOrder = this.#allocateSessionReportOrder(runtimeSessionId, numericGeneration);
    const tail = this.#observationTails.get(runtimeSessionId) ?? Promise.resolve();
    const next = tail
      .then(() => this.#core.reportSession(identity, activity, connectivity, sourceOrder))
      .then(() => undefined);
    this.#observationTails.set(runtimeSessionId, next);
    return next;
  }

  #allocateSessionReportOrder(
    runtimeSessionId: string,
    sessionGeneration: number,
  ): SessionReportOrder {
    const next = nextSessionReportSequence(
      this.#sessionReportSequences.get(runtimeSessionId),
      this.#options.adapterGeneration,
      sessionGeneration,
    );
    this.#sessionReportSequences.set(runtimeSessionId, next);
    return Object.freeze({
      adapterGeneration: next.adapterGeneration,
      revision: next.revision,
    });
  }

  #trackObservation(
    promise: Promise<void>,
    context: ObservationDiagnosticContext,
  ): void {
    const tracked = promise
      .catch((error: unknown) => {
        this.#record({
          event: "observation.failed",
          level: "error",
          ...context,
          error: diagnosticError(error),
        });
        if (this.#observationError === undefined) {
          this.#observationError = error;
          this.#observationErrorContext = context;
        }
      })
      .finally(() => this.#pendingObservations.delete(tracked));
    this.#pendingObservations.add(tracked);
  }
}

const UNKNOWN_DEPLOYMENT_AUTHORITY_DIAGNOSTIC: AdapterDiagnosticError = Object.freeze({
  name: "DeploymentAuthorityResolverError",
  code: "RESOLVER_FAILURE",
});

function deploymentAuthorityDiagnosticError(error: unknown): AdapterDiagnosticError {
  try {
    if (error instanceof DeploymentAuthorityError) {
      const code: unknown = error.code;
      if (
        typeof code === "string" &&
        (DEPLOYMENT_AUTHORITY_ERROR_CODES as readonly string[]).includes(code)
      ) {
        return { name: "DeploymentAuthorityError", code };
      }
    }
  } catch {
    // Resolver failures are untrusted; no exception metadata crosses this boundary.
  }
  return UNKNOWN_DEPLOYMENT_AUTHORITY_DIAGNOSTIC;
}

function normalizedModel(model: ReturnType<PiSession["getState"]>["model"]): string {
  return model ? `${model.provider}/${model.id}` : "";
}

interface DeliveryFailureClassification {
  readonly failureCode: FailureCode;
  readonly diagnostic: string;
  readonly rejected: boolean;
  readonly connectivity?: SessionConnectivityState;
}

export function classifyDeliveryFailure(error: unknown): DeliveryFailureClassification {
  if (error instanceof UnsupportedCommandError) {
    return {
      failureCode: FailureCode.UNSUPPORTED_COMMAND,
      diagnostic: "unsupported_command",
      rejected: true,
    };
  }
  if (error instanceof PiRpcTransportError) {
    const connectivity = error.processExit
      ? confirmedCleanProcessExit(error.processExit)
        ? SessionConnectivityState.OFFLINE
        : SessionConnectivityState.FAILED
      : SessionConnectivityState.STALE;
    const outcomeUnknown = error.requestEffect === "possibly_written";
    return {
      failureCode: outcomeUnknown
        ? FailureCode.EXECUTION_OUTCOME_UNKNOWN
        : FailureCode.EXECUTION_FAILED,
      diagnostic: outcomeUnknown
        ? "rpc_execution_outcome_unknown"
        : "rpc_request_not_written",
      rejected: false,
      connectivity,
    };
  }
  return {
    failureCode: FailureCode.EXECUTION_FAILED,
    diagnostic: "delivery_execution_failed",
    rejected: false,
  };
}

function operationForPriorWork(
  commandId: string,
  prior: RuntimeGenerationRef,
): Operation {
  const external = prior.externalRuntime;
  if (
    !external?.adapterId?.value || !external.deploymentScope ||
    !external.runtimeSessionId?.value || !external.generation?.value
  ) {
    throw new Error("prior-work runtime identity is incomplete");
  }
  return create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: commandId }),
    targetScope: create(TargetScopeSchema, {
      kind: TargetScopeKind.RUNTIME_SESSION,
      adapterId: external.adapterId,
      deploymentScope: external.deploymentScope,
      runtimeSessionId: create(RuntimeSessionIdSchema, {
        value: external.runtimeSessionId.value,
      }),
      sessionGeneration: create(GenerationSchema, {
        value: external.generation.value,
      }),
    }),
  });
}

function requiredOperation(delivery: Delivery): Operation {
  if (!delivery.operation) throw new Error("delivery is missing operation");
  return delivery.operation;
}

function requiredAcceptedSpawn(delivery: Delivery, operation: Operation): SpawnClaimAccepted {
  const accepted = delivery.acceptedSpawn;
  if (
    !accepted?.acceptedOperation?.operation ||
    accepted.acceptedOperation.operation.commandId?.value !== operation.commandId?.value ||
    accepted.claim?.claimOperationId?.value !== operation.commandId?.value
  ) {
    throw new Error("managed spawn delivery is missing its exact accepted envelope");
  }
  return accepted;
}

function sessionConnectivity(
  value: "live" | "offline" | "stale" | "failed",
): SessionConnectivityState {
  switch (value) {
    case "live": return SessionConnectivityState.LIVE;
    case "offline": return SessionConnectivityState.OFFLINE;
    case "stale": return SessionConnectivityState.STALE;
    case "failed": return SessionConnectivityState.FAILED;
  }
}

function confirmedCleanProcessExit(exit: {
  readonly expected: boolean;
  readonly code: number | null;
  readonly signal: NodeJS.Signals | null;
}): boolean {
  return exit.expected && (
    (exit.code === 0 && exit.signal === null) ||
    (exit.code === 143 && exit.signal === null) ||
    (exit.code === null && exit.signal === "SIGTERM")
  );
}

function sessionActivity(value: "idle" | "working" | "unknown"): SessionActivityState {
  switch (value) {
    case "idle": return SessionActivityState.IDLE;
    case "working": return SessionActivityState.WORKING;
    case "unknown": return SessionActivityState.UNKNOWN;
  }
}

function isRetryableTransportFailure(error: unknown): boolean {
  return (
    error instanceof ConnectError &&
    [
      Code.Canceled,
      Code.Aborted,
      Code.DeadlineExceeded,
      Code.ResourceExhausted,
      Code.Unavailable,
    ].includes(
      error.code,
    )
  );
}

function delay(milliseconds: number, signal?: AbortSignal): Promise<void> {
  if (signal?.aborted) return Promise.resolve();
  return new Promise((resolve) => {
    const timer = setTimeout(resolve, milliseconds);
    signal?.addEventListener(
      "abort",
      () => {
        clearTimeout(timer);
        resolve();
      },
      { once: true },
    );
  });
}

async function runFromEnvironment(): Promise<void> {
  const sessions = JSON.parse(process.env["PATCHBAY_PI_SESSIONS"] ?? "[]") as PreprovisionedSession[];
  const managedTargets = JSON.parse(
    process.env["PATCHBAY_PI_MANAGED_TARGETS"] ?? "[]",
  ) as ManagedPiTargetConfig[];
  const adapterId = process.env["PATCHBAY_ADAPTER_ID"] ?? "pi";
  const attachmentEvidence = requiredEnv("PATCHBAY_ADAPTER_ATTACHMENT_SECRET");
  const diagnostics = await openAdapterDiagnostics({
    path: resolveAdapterLogPath(),
    adapterId,
    adapterGeneration: Number.parseInt(process.env["PATCHBAY_ADAPTER_GENERATION"] ?? "1", 10),
    secrets: [attachmentEvidence],
  });
  const processHost = new AdapterProcess({
    coreAddress: requiredEnv("PATCHBAY_CORE_ADDR"),
    adapterId,
    authorityDomainId: process.env["PATCHBAY_AUTHORITY_DOMAIN_ID"] ?? "default",
    attachmentEvidence,
    adapterGeneration: Number.parseInt(process.env["PATCHBAY_ADAPTER_GENERATION"] ?? "1", 10),
    sessions,
    managedTargets,
    ...(process.env["PATCHBAY_PI_SPAWN_JOURNAL_DIR"]
      ? { spawnJournalDirectory: process.env["PATCHBAY_PI_SPAWN_JOURNAL_DIR"] }
      : {}),
    diagnostics,
    forwardDiagnostics: true,
  });
  const controller = new AbortController();
  process.once("SIGINT", () => controller.abort());
  process.once("SIGTERM", () => controller.abort());
  try {
    await processHost.run(controller.signal);
  } finally {
    await processHost.dispose();
  }
}

function requiredEnv(name: string): string {
  const value = process.env[name];
  if (!value) throw new Error(`${name} is required`);
  return value;
}

if (process.argv[1] && import.meta.url === new URL(process.argv[1], "file:").href) {
  await runFromEnvironment();
}
