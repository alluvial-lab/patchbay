import { create } from "@bufbuild/protobuf";
import { Code, ConnectError, createClient, type Client, type Interceptor } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import {
  ActorEndpointRefSchema, ActorIdSchema, AdapterControlService, AdapterIdSchema,
  AdapterRegistrationSchema, AttachRequestSchema, AuthorityDomainIdSchema,
  CommandIdSchema, EndpointIdSchema, FailureCode, GenerationSchema, LsnSchema,
  ObservationKind, ObservationRequestSchema, ObservationSchema, PayloadContentType,
  PayloadEnvelopeSchema, ReceiveRequestSchema, TypedCorrelationSchema,
  type AdapterDiagnosticReport, type AdapterDiagnosticReportResult, type Delivery,
  type EventId, type ObservationRequest, type ObservationResult, type Operation,
} from "@patchbay/contracts";
import { diagnosticError, NOOP_ADAPTER_DIAGNOSTICS, type AdapterDiagnostics } from "./adapter_diagnostics.js";
import { tokenCommuneCapabilityManifest } from "./manifest.js";

const encoder = new TextEncoder();
const attachmentTokenHeader = "x-patchbay-adapter-attachment-token";
type AdapterClient = Client<typeof AdapterControlService>;

export interface CoreClientOptions {
  coreAddress: string;
  adapterId: string;
  authorityDomainId: string;
  attachmentEvidence: string;
  /** Test seam for the generated Connect client; production leaves this unset. */
  testClient?: AdapterClient;
  /** Test seam mirroring the token response header captured by the interceptor. */
  testAttachmentToken?: () => string | undefined;
}

export class PatchbayCoreClient {
  readonly #client: AdapterClient;
  readonly #options: CoreClientOptions;
  #attachmentToken: string | undefined;
  #adapterGeneration: number | undefined;
  #reattachPromise: Promise<void> | undefined;
  #diagnostics: AdapterDiagnostics;

  constructor(options: CoreClientOptions, diagnostics: AdapterDiagnostics = NOOP_ADAPTER_DIAGNOSTICS) {
    if (!options.coreAddress || !options.adapterId || !options.authorityDomainId || !options.attachmentEvidence) {
      throw new Error("core address, adapter id, authority domain, and attachment evidence are required");
    }
    this.#options = options;
    this.#diagnostics = diagnostics;
    if (options.testClient) {
      this.#client = options.testClient;
      return;
    }
    const authenticate: Interceptor = (next) => async (request) => {
      request.header.set("x-patchbay-adapter-id", options.adapterId);
      request.header.set("x-patchbay-adapter-evidence", options.attachmentEvidence);
      if (this.#attachmentToken) request.header.set(attachmentTokenHeader, this.#attachmentToken);
      const response = await next(request);
      const issued = response.header.get(attachmentTokenHeader);
      if (issued) this.#attachmentToken = issued;
      return response;
    };
    this.#client = createClient(AdapterControlService, createGrpcTransport({
      baseUrl: options.coreAddress, interceptors: [authenticate],
    }));
  }

  setDiagnostics(diagnostics: AdapterDiagnostics): void { this.#diagnostics = diagnostics; }

  async attach(adapterGeneration: number): Promise<EventId> {
    if (!Number.isSafeInteger(adapterGeneration) || adapterGeneration <= 0) throw new Error("adapter generation must be positive");
    this.#adapterGeneration = adapterGeneration;
    this.#attachmentToken = undefined;
    this.#record({ event: "adapter.attach.started", level: "info" });
    try {
      const result = await this.#client.attach(create(AttachRequestSchema, {
        registration: create(AdapterRegistrationSchema, {
          adapterId: this.#adapterId(),
          endpointId: create(EndpointIdSchema, { value: `${this.#options.adapterId}-endpoint` }),
          authorityDomainId: this.#authorityDomainId(),
          adapterGeneration: create(GenerationSchema, { value: BigInt(adapterGeneration) }),
          capability: tokenCommuneCapabilityManifest(),
        }),
        attachmentEvidence: encoder.encode(this.#options.attachmentEvidence),
      }));
      const testToken = this.#options.testAttachmentToken?.();
      if (testToken) this.#attachmentToken = testToken;
      if (!result.accepted || !result.attachEventId) throw new Error("core rejected adapter attachment");
      if (!this.#attachmentToken) throw new Error("core attachment response is missing the adapter attachment token");
      this.#record({ event: "adapter.attach.succeeded", level: "info" });
      return result.attachEventId;
    } catch (error) {
      this.#record({ event: "adapter.attach.failed", level: "error", error: diagnosticError(error) });
      throw error;
    }
  }

  reportDiagnostic(report: AdapterDiagnosticReport, signal?: AbortSignal): Promise<AdapterDiagnosticReportResult> {
    return this.#client.reportDiagnostics(report, signal ? { signal } : undefined);
  }

  async ingestObservation(request: ObservationRequest): Promise<ObservationResult> {
    return this.#postAttach(() => this.#client.ingestObservation(request));
  }

  async acknowledgeDelivery(operation: Operation, deliveryEventId?: EventId): Promise<EventId | undefined> {
    return this.#ingestLifecycle(operation, ObservationKind.EVENT, "patchbay.adapter.DeliveryAcknowledgement.v1", {
      acceptedLsn: deliveryEventId?.lsn?.value.toString() ?? null,
    });
  }

  async rejectUnsupported(operation: Operation): Promise<EventId | undefined> {
    return this.#ingestLifecycle(operation, ObservationKind.RESULT, "patchbay.token_commune.UnsupportedDelivery.v1", {
      reason: "unsupported_command",
    }, FailureCode.UNSUPPORTED_COMMAND);
  }

  receiveDeliveries(cursor: bigint, signal?: AbortSignal): AsyncIterable<Delivery> {
    return this.#receive(cursor, signal);
  }

  async *#receive(cursor: bigint, signal?: AbortSignal): AsyncGenerator<Delivery> {
    for (let attempt = 0; attempt < 2; attempt += 1) {
      const failedToken = this.#attachmentToken;
      try {
        for await (const delivery of this.#client.receiveDeliveries(create(ReceiveRequestSchema, {
          adapterId: this.#adapterId(), cursor: create(LsnSchema, { value: cursor }),
        }), signal ? { signal } : {})) yield delivery;
        return;
      } catch (error) {
        if (attempt !== 0 || !isUnauthenticated(error)) throw error;
        await this.#refreshAttachment(failedToken);
      }
    }
  }

  async #postAttach<T>(call: () => Promise<T>): Promise<T> {
    const failedToken = this.#attachmentToken;
    try { return await call(); }
    catch (error) {
      if (!isUnauthenticated(error)) throw error;
      await this.#refreshAttachment(failedToken);
      return call();
    }
  }

  async #refreshAttachment(failedToken: string | undefined): Promise<void> {
    if (this.#attachmentToken && this.#attachmentToken !== failedToken) return;
    if (this.#adapterGeneration === undefined) throw new Error("cannot reattach before initial attachment");
    this.#reattachPromise ??= this.attach(this.#adapterGeneration).then(() => undefined).finally(() => { this.#reattachPromise = undefined; });
    await this.#reattachPromise;
  }

  async #ingestLifecycle(operation: Operation, kind: ObservationKind, schemaRef: string, payload: unknown, failureCode = FailureCode.UNSPECIFIED): Promise<EventId | undefined> {
    const commandId = operation.commandId?.value;
    if (!commandId) throw new Error("delivery operation is missing command_id");
    if (!operation.targetScope) throw new Error("delivery operation is missing target_scope");
    const observation = create(ObservationSchema, {
      authorityDomainId: this.#authorityDomainId(),
      sender: create(ActorEndpointRefSchema, { actorId: create(ActorIdSchema, { value: this.#options.adapterId }) }),
      kind,
      correlations: [create(TypedCorrelationSchema, { ref: { case: "commandId", value: create(CommandIdSchema, { value: commandId }) } })],
      targetScope: operation.targetScope,
      payload: create(PayloadEnvelopeSchema, {
        payload: encoder.encode(JSON.stringify(payload)), contentType: PayloadContentType.JSON, schemaRef,
      }),
      failureCode,
    });
    const result = await this.ingestObservation(create(ObservationRequestSchema, {
      authorityDomainId: this.#authorityDomainId(), observation: { case: "event", value: observation },
    }));
    return result.eventId;
  }

  #record(input: Parameters<AdapterDiagnostics["record"]>[0]): void {
    try { this.#diagnostics.record(input); } catch { /* diagnostics cannot veto control */ }
  }
  #adapterId() { return create(AdapterIdSchema, { value: this.#options.adapterId }); }
  #authorityDomainId() { return create(AuthorityDomainIdSchema, { value: this.#options.authorityDomainId }); }
}

function isUnauthenticated(error: unknown): boolean {
  return error instanceof ConnectError && error.code === Code.Unauthenticated;
}
