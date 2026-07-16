import {
  FailureCode,
  OperationKind,
  SessionActivityState,
  SessionConnectivityState,
  type Delivery,
  type Operation,
} from "@patchbay/contracts";
import { PatchbayCoreClient, type SessionIdentity } from "./core_client.js";
import { DeliveryTranslator, UnsupportedCommandError } from "./delivery.js";
import { PiSession, type PiSessionOptions } from "./pi_session.js";
import {
  SessionRegistry,
  type RuntimeSessionEntry,
} from "./session_registry.js";

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
}

interface StartedDelivery {
  completion: Promise<void>;
}

/** Composition root: gRPC client + complete runtime-session registry. */
export class AdapterProcess {
  readonly #options: AdapterProcessOptions;
  readonly #core: PatchbayCoreClient;
  readonly #registry = new SessionRegistry();
  readonly #translator = new DeliveryTranslator();
  readonly #activeCommands = new Map<string, string>();
  readonly #pendingObservations = new Set<Promise<void>>();
  #observationError: unknown;
  #cursor = 0n;
  #started = false;

  constructor(options: AdapterProcessOptions) {
    this.#options = options;
    this.#core = new PatchbayCoreClient(options);
  }

  async start(): Promise<void> {
    if (this.#started) return;
    await this.#core.attach(this.#options.adapterGeneration);
    this.#started = true;
    try {
      for (const configured of this.#options.sessions) {
        await this.registerSession(configured);
      }
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
    const session = await createSession(configured);
    let entry: RuntimeSessionEntry;
    try {
      entry = this.#registry.register(configured, session, (observedEntry, event) => {
        const promise = this.#core
          .ingestTranscript(
            this.#identity(observedEntry),
            event,
            this.#activeCommands.get(observedEntry.runtimeSessionId),
          )
          .then(() => undefined);
        this.#trackObservation(promise);
      });
    } catch (error) {
      await session.dispose();
      throw error;
    }

    // Reconcile Pi's persisted getEntries()/TranscriptEventLog snapshot before
    // claiming a current activity state. Stable transcript event ids make this
    // replay a partial snapshot rather than command re-execution.
    for (const event of session.snapshotTranscript()) {
      await this.#core.ingestTranscript(this.#identity(entry), event);
    }
    await this.#core.reportSession(
      this.#identity(entry),
      this.#options.adapterGeneration > 1
        ? SessionActivityState.UNKNOWN
        : SessionActivityState.IDLE,
      SessionConnectivityState.LIVE,
    );
  }

  async pollOnce(): Promise<number> {
    if (!this.#started) throw new Error("adapter process has not started");
    const inFlight: Promise<void>[] = [];
    let delivered = 0;
    for await (const delivery of this.#core.receiveDeliveries(this.#cursor)) {
      const operation = requiredOperation(delivery);
      const started = await this.#beginDelivery(delivery, operation);
      this.#cursor = delivery.deliveryEventId?.lsn?.value ?? this.#cursor;
      delivered += 1;
      // Instruct runs remain in flight so a later cancel in the same durable
      // tail can abort them. Delivery acknowledgement and running status have
      // already committed before the next Operation starts.
      if (operation.kind === OperationKind.INSTRUCT) inFlight.push(started.completion);
      else await started.completion;
    }
    await Promise.all(inFlight);
    await this.flushObservations();
    return delivered;
  }

  async flushObservations(): Promise<void> {
    await Promise.all([...this.#pendingObservations]);
    if (this.#observationError !== undefined) {
      const error = this.#observationError;
      this.#observationError = undefined;
      throw error;
    }
  }

  async run(signal?: AbortSignal): Promise<void> {
    await this.start();
    while (!signal?.aborted) {
      const delivered = await this.pollOnce();
      if (delivered === 0) await delay(100, signal);
    }
  }

  async dispose(): Promise<void> {
    await this.#registry.dispose();
    this.#started = false;
  }

  async #beginDelivery(delivery: Delivery, operation: Operation): Promise<StartedDelivery> {
    // This is the durable delivery checkpoint. ReceiveDeliveries filters on the
    // resulting command state, so an adapter restart from cursor 0 cannot
    // re-offer or re-execute acknowledged history.
    await this.#core.acknowledgeDelivery(operation, delivery.deliveryEventId);

    const target = operation.targetScope;
    const runtimeSessionId = target?.runtimeSessionId?.value;
    const entry = runtimeSessionId ? this.#registry.resolve(runtimeSessionId) : undefined;
    const targetError = this.#validateTarget(operation, entry);
    if (targetError) {
      return {
        completion: this.#core
          .ingestFailure(operation, FailureCode.DELIVERY_REJECTED, targetError)
          .then(() => undefined),
      };
    }

    if (!entry) throw new Error("validated delivery lost its runtime entry");
    await this.#core.reportRunning(operation);
    const commandId = operation.commandId?.value;
    if (commandId) this.#activeCommands.set(entry.runtimeSessionId, commandId);
    if (operation.kind === OperationKind.INSTRUCT) {
      await this.#core.reportSession(
        this.#identity(entry),
        SessionActivityState.WORKING,
      );
    }
    return { completion: this.#executeDelivery(operation, entry) };
  }

  async #executeDelivery(operation: Operation, entry: RuntimeSessionEntry): Promise<void> {
    const commandId = operation.commandId?.value;
    try {
      const outcome = await this.#translator.deliver(operation, entry.session);
      if (outcome.sessionGenerationChanged) {
        await this.#core.reportSession(
          this.#identity(entry),
          SessionActivityState.IDLE,
        );
      } else if (operation.kind === OperationKind.INSTRUCT) {
        await this.#core.reportSession(
          this.#identity(entry),
          SessionActivityState.IDLE,
        );
      }
      await this.#core.reportResult(operation, outcome.value);
    } catch (error) {
      const failureCode =
        error instanceof UnsupportedCommandError
          ? FailureCode.UNSUPPORTED_COMMAND
          : FailureCode.EXECUTION_FAILED;
      const diagnostic = error instanceof Error ? error.message : String(error);
      await this.#core.ingestFailure(operation, failureCode, diagnostic);
      if (operation.kind === OperationKind.INSTRUCT) {
        await this.#core.reportSession(
          this.#identity(entry),
          SessionActivityState.UNKNOWN,
        );
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

  #identity(entry: RuntimeSessionEntry): SessionIdentity {
    return {
      runtimeSessionId: entry.runtimeSessionId,
      deploymentScope: entry.deploymentScope,
      generation: entry.session.generation,
      project: entry.project ?? "",
      cwd: entry.cwd,
      name: entry.name ?? entry.runtimeSessionId,
    };
  }

  #trackObservation(promise: Promise<void>): void {
    const tracked = promise
      .catch((error: unknown) => {
        this.#observationError ??= error;
      })
      .finally(() => this.#pendingObservations.delete(tracked));
    this.#pendingObservations.add(tracked);
  }
}

function requiredOperation(delivery: Delivery): Operation {
  if (!delivery.operation) throw new Error("delivery is missing operation");
  return delivery.operation;
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
  const processHost = new AdapterProcess({
    coreAddress: requiredEnv("PATCHBAY_CORE_ADDR"),
    adapterId: process.env["PATCHBAY_ADAPTER_ID"] ?? "pi",
    authorityDomainId: process.env["PATCHBAY_AUTHORITY_DOMAIN_ID"] ?? "default",
    attachmentEvidence: requiredEnv("PATCHBAY_ADAPTER_ATTACHMENT_SECRET"),
    adapterGeneration: Number.parseInt(process.env["PATCHBAY_ADAPTER_GENERATION"] ?? "1", 10),
    sessions,
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
