import { Code, ConnectError } from "@connectrpc/connect";
import { FailureCode, type Delivery, type Operation } from "@patchbay/contracts";
import { loadTokenCommuneAdapterConfig, type TokenCommuneAdapterConfig } from "./config.js";
import { loadGatewayCredential } from "./credential.js";
import { createHttpTokenCommuneGatewayClient, type TokenCommuneGatewayClient } from "./gateway_client.js";
import { PatchbayCoreClient } from "./core_client.js";
import {
  diagnosticError, NOOP_ADAPTER_DIAGNOSTICS, openAdapterDiagnostics,
  type AdapterDiagnostics,
} from "./adapter_diagnostics.js";
import { composeAdapterDiagnostics, CoreDiagnosticsForwarder } from "./core_diagnostics_forwarder.js";

export interface AdapterProcessOptions extends TokenCommuneAdapterConfig {
  gateway: TokenCommuneGatewayClient;
  diagnostics?: AdapterDiagnostics;
  forwardDiagnostics?: boolean;
  coreClient?: PatchbayCoreClient;
  retryDelayMs?: number;
}

export class AdapterProcess {
  readonly #core: PatchbayCoreClient;
  readonly #gateway: TokenCommuneGatewayClient;
  readonly #retryDelayMs: number;
  #diagnostics: AdapterDiagnostics;
  #cursor = 0n;
  #started = false;
  #disposed = false;
  #runController: AbortController | undefined;

  constructor(readonly options: AdapterProcessOptions) {
    this.#gateway = options.gateway;
    void this.#gateway; // Composed now; polling is intentionally a later feature.
    this.#retryDelayMs = options.retryDelayMs ?? 100;
    const local = options.diagnostics ?? NOOP_ADAPTER_DIAGNOSTICS;
    this.#diagnostics = local;
    this.#core = options.coreClient ?? new PatchbayCoreClient(options, local);
    if (options.forwardDiagnostics) {
      const forwarder = new CoreDiagnosticsForwarder(
        (report, signal) => this.#core.reportDiagnostic(report, signal),
        { authorityDomainId: options.authorityDomainId, adapterId: options.adapterId, adapterGeneration: options.adapterGeneration },
      );
      this.#diagnostics = composeAdapterDiagnostics([local, forwarder]);
      this.#core.setDiagnostics(this.#diagnostics);
    }
  }

  async start(): Promise<void> {
    if (this.#disposed) throw new Error("adapter process has been disposed");
    if (this.#started) return;
    this.#record({ event: "adapter.starting", level: "info" });
    await this.#core.attach(this.options.adapterGeneration);
    this.#started = true;
    this.#record({ event: "adapter.started", level: "info" });
  }

  async run(signal?: AbortSignal): Promise<void> {
    await this.start();
    if (this.#runController) throw new Error("adapter delivery loop is already running");
    const controller = new AbortController();
    this.#runController = controller;
    const abort = () => controller.abort(signal?.reason);
    if (signal?.aborted) abort(); else signal?.addEventListener("abort", abort, { once: true });
    try {
      while (!controller.signal.aborted) {
        try {
          await this.#consume(controller.signal);
          if (!controller.signal.aborted) throw new ConnectError("delivery subscription ended without shutdown", Code.Unavailable);
        } catch (error) {
          if (controller.signal.aborted) return;
          const retryable = isRetryableTransportFailure(error);
          this.#record({
            event: retryable ? "delivery.subscription.retrying" : "delivery.subscription.failed",
            level: retryable ? "warn" : "error",
            error: diagnosticError(error),
            failureCode: retryable ? FailureCode.TRANSPORT_TIMEOUT : FailureCode.ADAPTER_UNAVAILABLE,
          });
          if (!retryable) throw error;
          await delay(this.#retryDelayMs, controller.signal);
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
    this.#started = false;
    this.#record({ event: "adapter.stopped", level: "info" });
    try { await this.#diagnostics.flush(); } catch { /* diagnostics are non-interfering */ }
    try { await this.#diagnostics.close(); } catch { /* diagnostics are non-interfering */ }
  }

  async #consume(signal: AbortSignal): Promise<void> {
    for await (const delivery of this.#core.receiveDeliveries(this.#cursor, signal)) {
      const operation = requiredOperation(delivery);
      const commandId = operation.commandId?.value;
      this.#record({
        event: "delivery.received", level: "info", ...(commandId ? { commandId } : {}), operationKind: operation.kind,
      });
      await this.#core.acknowledgeDelivery(operation, delivery.deliveryEventId);
      this.#cursor = delivery.deliveryEventId?.lsn?.value ?? this.#cursor;
      this.#record({
        event: "delivery.acknowledged", level: "info", ...(commandId ? { commandId } : {}), operationKind: operation.kind,
      });
      await this.#core.rejectUnsupported(operation);
      this.#record({
        event: "delivery.unsupported", level: "warn", ...(commandId ? { commandId } : {}),
        operationKind: operation.kind, failureCode: FailureCode.UNSUPPORTED_COMMAND,
      });
    }
  }

  #record(input: Parameters<AdapterDiagnostics["record"]>[0]): void {
    try { this.#diagnostics.record(input); } catch { /* observer only */ }
  }
}

function requiredOperation(delivery: Delivery): Operation {
  if (!delivery.operation) throw new Error("delivery is missing operation");
  return delivery.operation;
}
function isRetryableTransportFailure(error: unknown): boolean {
  return error instanceof ConnectError && [Code.Canceled, Code.Aborted, Code.DeadlineExceeded, Code.ResourceExhausted, Code.Unavailable].includes(error.code);
}
function delay(milliseconds: number, signal: AbortSignal): Promise<void> {
  if (signal.aborted) return Promise.resolve();
  return new Promise((resolve) => {
    const timer = setTimeout(resolve, milliseconds);
    signal.addEventListener("abort", () => { clearTimeout(timer); resolve(); }, { once: true });
  });
}

export async function runFromEnvironment(env: NodeJS.ProcessEnv = process.env): Promise<void> {
  // Configuration is deliberately complete before credential, log, or network access.
  const config = loadTokenCommuneAdapterConfig(env);
  const credential = await loadGatewayCredential(config.gatewayCredentialFile);
  const diagnostics = await openAdapterDiagnostics({
    path: config.diagnosticPath,
    adapterId: config.adapterId,
    adapterGeneration: config.adapterGeneration,
    secrets: [config.attachmentEvidence, config.gatewayCredentialFile, ...credential.redactionSecrets()],
  });
  const gateway = createHttpTokenCommuneGatewayClient({ baseUrl: config.gatewayBaseUrl, credential });
  const host = new AdapterProcess({ ...config, gateway, diagnostics, forwardDiagnostics: true });
  const controller = new AbortController();
  const stop = () => controller.abort();
  process.once("SIGINT", stop);
  process.once("SIGTERM", stop);
  try { await host.run(controller.signal); }
  finally {
    process.removeListener("SIGINT", stop);
    process.removeListener("SIGTERM", stop);
    await host.dispose();
    credential.dispose();
  }
}

if (process.argv[1] && import.meta.url === new URL(process.argv[1], "file:").href) {
  await runFromEnvironment();
}
