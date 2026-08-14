import { Code, ConnectError } from "@connectrpc/connect";
import {
  FailureCode,
  OperationKind,
  SessionActivityState,
  SessionConnectivityState,
  type Delivery,
  type Operation,
} from "@patchbay/contracts";
import { PatchbayCoreClient, type SessionIdentity } from "./core_client.js";
import {
  diagnosticError,
  NOOP_ADAPTER_DIAGNOSTICS,
  openAdapterDiagnostics,
  resolveAdapterLogPath,
  type AdapterDiagnosticInput,
  type AdapterDiagnostics,
  type AdapterDiagnosticSessionRef,
} from "./adapter_diagnostics.js";
import { DeliveryTranslator, UnsupportedCommandError } from "./delivery.js";
import { composeAdapterDiagnostics, CoreDiagnosticsForwarder } from "./core_diagnostics_forwarder.js";
import { PiSession, type PiSessionOptions } from "./pi_session.js";
import {
  SessionRegistry,
  type RuntimeSessionEntry,
} from "./session_registry.js";
import {
  nextSessionReportSequence,
  type SessionReportOrder,
  type SessionReportSequence,
} from "./session_report_sequencer.js";

export interface PreprovisionedSession extends PiSessionOptions {
  runtimeSessionId: string;
  deploymentScope: string;
  project?: string;
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
  readonly #activeCommands = new Map<string, string>();
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
  }

  async start(): Promise<void> {
    if (this.#disposed) throw new Error("adapter process has been disposed");
    if (this.#started) return;
    this.#record({ event: "adapter.starting", level: "info" });
    await this.#core.attach(this.#options.adapterGeneration);
    this.#started = true;
    try {
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
    const createSession = this.#options.createSession ?? PiSession.create;
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
        (observedEntry, event) => {
          const identity = this.#identity(observedEntry);
          const activeCommand = this.#activeCommands.get(observedEntry.runtimeSessionId);
          const tail = this.#observationTails.get(observedEntry.runtimeSessionId) ?? Promise.resolve();
          const next = tail
            .then(() => this.#core.ingestTranscript(identity, event, activeCommand))
            .then(() => undefined);
          this.#observationTails.set(observedEntry.runtimeSessionId, next);
          this.#trackObservation(next, {
            session: this.#sessionRef(observedEntry),
            observationKind: "transcript",
          });
        },
        (observedEntry, model) => {
          const activity = observedEntry.session.getState().idle
            ? SessionActivityState.IDLE
            : SessionActivityState.WORKING;
          this.#record({
            event: "session.model.changed",
            level: "info",
            session: this.#sessionRef(observedEntry),
          });
          this.#trackObservation(this.#queueSessionReport(observedEntry, activity, undefined, model), {
            session: this.#sessionRef(observedEntry),
            observationKind: "session-report",
          });
        },
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
      for (const event of session.snapshotTranscript()) {
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
        const operation = requiredOperation(delivery);
        const started = await this.#beginDelivery(delivery, operation);
        this.#cursor = delivery.deliveryEventId?.lsn?.value ?? this.#cursor;

        // Instruction completion remains in flight so the live subscription
        // can receive a later cancellation for the same session.
        if (operation.kind === OperationKind.INSTRUCT) {
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
    if (commandId) this.#activeCommands.set(entry.runtimeSessionId, commandId);
    if (operation.kind === OperationKind.INSTRUCT) {
      await this.#queueSessionReport(entry, SessionActivityState.WORKING);
    }
    return { completion: this.#executeDelivery(operation, entry) };
  }

  async #executeDelivery(operation: Operation, entry: RuntimeSessionEntry): Promise<void> {
    const commandId = operation.commandId?.value;
    const operationKind = operation.kind;
    const fromGeneration = entry.session.generation;
    try {
      const outcome = await this.#translator.deliver(operation, entry.session);
      if (outcome.sessionGenerationChanged) {
        this.#record({
          event: "session.generation.changed",
          level: "info",
          session: this.#sessionRef(entry),
          fromGeneration,
          toGeneration: entry.session.generation,
        });
      }
      if (outcome.sessionGenerationChanged) {
        // This Result is bound to the accepted target (generation N). Commit
        // it before the ordinary N+1 report; otherwise the core correctly
        // quarantines the now-stale Result instead of completing the command.
        await this.#core.reportResult(operation, outcome.value);
        await this.#queueSessionReport(entry, SessionActivityState.IDLE);
      } else {
        // For in-generation work, await the serialized observation tail before
        // terminal Result so command-correlated transcript cannot arrive late.
        if (operation.kind === OperationKind.INSTRUCT) {
          await this.#queueSessionReport(entry, SessionActivityState.IDLE);
        }
        await this.#core.reportResult(operation, outcome.value);
      }
      this.#record({
        event: "delivery.completed",
        level: "info",
        ...(commandId ? { commandId } : {}),
        operationKind,
        session: this.#sessionRef(entry),
        outcome: "COMPLETED",
      });
    } catch (error) {
      const failureCode =
        error instanceof UnsupportedCommandError
          ? FailureCode.UNSUPPORTED_COMMAND
          : FailureCode.EXECUTION_FAILED;
      const diagnostic = error instanceof Error ? error.message : String(error);
      await this.#core.ingestFailure(operation, failureCode, diagnostic);
      this.#record({
        event: error instanceof UnsupportedCommandError ? "delivery.rejected" : "delivery.failed",
        level: error instanceof UnsupportedCommandError ? "warn" : "error",
        ...(commandId ? { commandId } : {}),
        operationKind,
        failureCode,
        session: this.#sessionRef(entry),
        error: diagnosticError(error),
      });
      if (operation.kind === OperationKind.INSTRUCT) {
        await this.#queueSessionReport(entry, SessionActivityState.UNKNOWN);
      }
    } finally {
      if (commandId && this.#activeCommands.get(entry.runtimeSessionId) === commandId) {
        this.#activeCommands.delete(entry.runtimeSessionId);
      }
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
      .then(() => this.#core.reportSession(identity, activity, connectivity, sourceOrder))
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

function normalizedModel(model: ReturnType<PiSession["getState"]>["model"]): string {
  return model ? `${model.provider}/${model.id}` : "";
}

function requiredOperation(delivery: Delivery): Operation {
  if (!delivery.operation) throw new Error("delivery is missing operation");
  return delivery.operation;
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
