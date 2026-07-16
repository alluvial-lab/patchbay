import {
  FailureCode,
  OperationKind,
  SessionActivityState,
  SessionConnectivityState,
  type Delivery,
  type Operation,
} from "@patchbay/contracts";
import { PatchbayCoreClient, deliveryCursor, type SessionIdentity } from "./core_client.js";
import { DeliveryTranslator, UnsupportedCommandError } from "./delivery.js";
import { PiSession, type PiSessionOptions } from "./pi_session.js";
import { SessionRegistry } from "./session_registry.js";

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

/** Composition root: gRPC client + pre-provisioned Pi session registry. */
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
    const createSession = this.#options.createSession ?? PiSession.create;
    for (const configured of this.#options.sessions) {
      const session = await createSession(configured);
      this.#registry.register(configured.runtimeSessionId, session);
      session.onTranscript((event) => {
        const promise = this.#core
          .ingestTranscript(
            this.#identity(configured, session),
            event,
            this.#activeCommands.get(configured.runtimeSessionId),
          )
          .then(() => undefined);
        this.#trackObservation(promise);
      });
      await this.#core.reportSession(
        this.#identity(configured, session),
        this.#options.adapterGeneration > 1
          ? SessionActivityState.UNKNOWN
          : SessionActivityState.IDLE,
        SessionConnectivityState.LIVE,
      );
    }
    this.#started = true;
  }

  async pollOnce(): Promise<number> {
    if (!this.#started) throw new Error("adapter process has not started");
    const inFlight: Promise<void>[] = [];
    let delivered = 0;
    for await (const delivery of this.#core.receiveDeliveries(this.#cursor)) {
      this.#cursor = deliveryCursor(delivery.deliveryEventId, this.#cursor);
      delivered += 1;
      const operation = requiredOperation(delivery);
      // Instruct runs are allowed to remain in flight so a later cancel in the
      // same durable tail can abort them. Other mappings remain ordered.
      const work = this.#processDelivery(operation);
      if (operation.kind === OperationKind.INSTRUCT) inFlight.push(work);
      else await work;
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

  dispose(): void {
    this.#registry.dispose();
    this.#started = false;
  }

  async #processDelivery(operation: Operation): Promise<void> {
    const target = operation.targetScope;
    const runtimeSessionId = target?.runtimeSessionId?.value;
    if (!runtimeSessionId) throw new Error("delivery is missing runtime_session_id");
    const configured = this.#options.sessions.find(
      (candidate) => candidate.runtimeSessionId === runtimeSessionId,
    );
    const session = this.#registry.resolve(runtimeSessionId);
    if (!configured || !session) throw new Error(`target runtime session is not registered: ${runtimeSessionId}`);
    const commandId = operation.commandId?.value;
    if (commandId) this.#activeCommands.set(runtimeSessionId, commandId);

    try {
      if (operation.kind === OperationKind.INSTRUCT) {
        await this.#core.reportSession(
          this.#identity(configured, session),
          SessionActivityState.WORKING,
        );
      }
      const outcome = await this.#translator.deliver(operation, session);
      if (outcome.sessionGenerationChanged) {
        await this.#core.reportSession(
          this.#identity(configured, session),
          SessionActivityState.IDLE,
        );
      } else if (operation.kind === OperationKind.INSTRUCT || operation.kind === OperationKind.CANCEL) {
        await this.#core.reportSession(
          this.#identity(configured, session),
          SessionActivityState.IDLE,
        );
      }
    } catch (error) {
      if (error instanceof UnsupportedCommandError && commandId) {
        await this.#core.ingestFailure(
          this.#identity(configured, session),
          commandId,
          FailureCode.UNSUPPORTED_COMMAND,
          error.message,
        );
        return;
      }
      throw error;
    } finally {
      if (commandId && this.#activeCommands.get(runtimeSessionId) === commandId) {
        this.#activeCommands.delete(runtimeSessionId);
      }
    }
  }

  #identity(configured: PreprovisionedSession, session: PiSession): SessionIdentity {
    return {
      runtimeSessionId: configured.runtimeSessionId,
      deploymentScope: configured.deploymentScope,
      generation: session.generation,
      project: configured.project ?? "",
      cwd: configured.cwd,
      name: configured.name ?? configured.runtimeSessionId,
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
    processHost.dispose();
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
