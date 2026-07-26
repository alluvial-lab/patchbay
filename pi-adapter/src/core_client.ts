import { create } from "@bufbuild/protobuf";
import {
  Code,
  ConnectError,
  createClient,
  type Client,
  type Interceptor,
} from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import {
  ActorEndpointRefSchema,
  ActorIdSchema,
  AdapterCapabilitySchema,
  AdapterControlService,
  AdapterIdSchema,
  AdapterRegistrationSchema,
  AdapterSnapshotSupport,
  AttachmentMethodSchema,
  AttachRequestSchema,
  AuthorityDomainIdSchema,
  CommandIdSchema,
  EndpointIdSchema,
  FailureCode,
  GenerationSchema,
  IdempotencyStrength,
  LsnSchema,
  ObservationKind,
  ObservationRequestSchema,
  ObservationSchema,
  OperationKind,
  OperationState,
  PayloadContentType,
  PayloadEnvelopeSchema,
  ReceiveRequestSchema,
  RuntimeSessionIdSchema,
  SessionActivityState,
  SessionConnectivityState,
  SessionReportSchema,
  TargetScopeKind,
  TargetScopeSchema,
  TypedCorrelationSchema,
  type Delivery,
  type EventId,
  type Operation,
} from "@patchbay/contracts";
import {
  diagnosticError,
  NOOP_ADAPTER_DIAGNOSTICS,
  type AdapterDiagnostics,
} from "./adapter_diagnostics.js";
import type { TranscriptEvent } from "./transcript_event.js";

const encoder = new TextEncoder();
const attachmentTokenHeader = "x-patchbay-adapter-attachment-token";

type AdapterClient = Client<typeof AdapterControlService>;

export interface CoreClientOptions {
  coreAddress: string;
  adapterId: string;
  authorityDomainId: string;
  attachmentEvidence: string;
}

export interface SessionIdentity {
  runtimeSessionId: string;
  deploymentScope: string;
  generation: number;
  project: string;
  cwd: string;
  name: string;
  model: string;
}

export class PatchbayCoreClient {
  readonly #client: AdapterClient;
  readonly #options: CoreClientOptions;
  #attachmentToken: string | undefined;
  #adapterGeneration: number | undefined;
  #reattachPromise: Promise<void> | undefined;
  readonly #diagnostics: AdapterDiagnostics;

  constructor(options: CoreClientOptions, diagnostics: AdapterDiagnostics = NOOP_ADAPTER_DIAGNOSTICS) {
    if (!options.adapterId || !options.authorityDomainId || !options.attachmentEvidence) {
      throw new Error("adapter id, authority domain, and attachment evidence are required");
    }
    this.#options = options;
    this.#diagnostics = diagnostics;
    const authenticate: Interceptor = (next) => async (request) => {
      request.header.set("x-patchbay-adapter-id", options.adapterId);
      request.header.set("x-patchbay-adapter-evidence", options.attachmentEvidence);
      if (this.#attachmentToken) {
        request.header.set(attachmentTokenHeader, this.#attachmentToken);
      }
      const response = await next(request);
      const issuedToken = response.header.get(attachmentTokenHeader);
      if (issuedToken) this.#attachmentToken = issuedToken;
      return response;
    };
    this.#client = createClient(
      AdapterControlService,
      createGrpcTransport({ baseUrl: options.coreAddress, interceptors: [authenticate] }),
    );
  }

  async attach(adapterGeneration: number): Promise<EventId> {
    this.#record({
      event: "adapter.attach.started",
      level: "info",
    });
    this.#adapterGeneration = adapterGeneration;
    this.#attachmentToken = undefined;
    try {
      const result = await this.#client.attach(
        create(AttachRequestSchema, {
        registration: create(AdapterRegistrationSchema, {
          adapterId: this.#adapterId(),
          endpointId: create(EndpointIdSchema, {
            value: `${this.#options.adapterId}-endpoint`,
          }),
          authorityDomainId: this.#authorityDomainId(),
          adapterGeneration: create(GenerationSchema, { value: BigInt(adapterGeneration) }),
          capability: piCapabilityManifest(),
        }),
          attachmentEvidence: encoder.encode(this.#options.attachmentEvidence),
        }),
      );
      if (!result.accepted || !result.attachEventId) {
        throw new Error(`core rejected adapter attachment: ${result.failureCode || "unknown"}`);
      }
      if (!this.#attachmentToken) {
        throw new Error("core attachment response is missing the adapter attachment token");
      }
      this.#record({
        event: "adapter.attach.succeeded",
        level: "info",
      });
      return result.attachEventId;
    } catch (error) {
      this.#record({
        event: "adapter.attach.failed",
        level: "error",
        error: diagnosticError(error),
      });
      throw error;
    }
  }

  #record(input: Parameters<AdapterDiagnostics["record"]>[0]): void {
    try {
      this.#diagnostics.record(input);
    } catch {
      // Diagnostics must never change an adapter operation's result.
    }
  }

  async reportSession(
    identity: SessionIdentity,
    activity: SessionActivityState,
    connectivity = SessionConnectivityState.LIVE,
  ): Promise<EventId | undefined> {
    const result = await this.#postAttach(() =>
      this.#client.ingestObservation(
        create(ObservationRequestSchema, {
          authorityDomainId: this.#authorityDomainId(),
          observation: {
            case: "sessionReport",
            value: create(SessionReportSchema, {
              adapterId: this.#adapterId(),
              deploymentScope: identity.deploymentScope,
              runtimeSessionId: create(RuntimeSessionIdSchema, { value: identity.runtimeSessionId }),
              sessionGeneration: create(GenerationSchema, { value: BigInt(identity.generation) }),
              connectivity,
              activity,
              project: identity.project,
              cwd: identity.cwd,
              name: identity.name,
              model: identity.model,
            }),
          },
        }),
      ),
    );
    return result.eventId;
  }

  async ingestTranscript(
    identity: SessionIdentity,
    event: TranscriptEvent,
    commandId?: string,
  ): Promise<EventId | undefined> {
    const correlations = commandId
      ? [
          create(TypedCorrelationSchema, {
            ref: {
              case: "commandId",
              value: create(CommandIdSchema, { value: commandId }),
            },
          }),
        ]
      : [];
    const observation = create(ObservationSchema, {
      authorityDomainId: this.#authorityDomainId(),
      sender: create(ActorEndpointRefSchema, {
        actorId: create(ActorIdSchema, { value: this.#options.adapterId }),
      }),
      kind: ObservationKind.EVENT,
      correlations,
      targetScope: create(TargetScopeSchema, {
        kind: TargetScopeKind.RUNTIME_SESSION,
        adapterId: this.#adapterId(),
        deploymentScope: identity.deploymentScope,
        runtimeSessionId: create(RuntimeSessionIdSchema, { value: identity.runtimeSessionId }),
        sessionGeneration: create(GenerationSchema, { value: BigInt(identity.generation) }),
      }),
      payload: create(PayloadEnvelopeSchema, {
        payload: encoder.encode(JSON.stringify(event)),
        contentType: PayloadContentType.JSON,
        schemaRef: "patchbay.pi.TranscriptEvent.v1",
      }),
      failureCode: FailureCode.UNSPECIFIED,
    });
    const result = await this.#postAttach(() =>
      this.#client.ingestObservation(
        create(ObservationRequestSchema, {
          authorityDomainId: this.#authorityDomainId(),
          observation: { case: "event", value: observation },
        }),
      ),
    );
    return result.eventId;
  }

  async acknowledgeDelivery(operation: Operation, deliveryEventId?: EventId): Promise<EventId | undefined> {
    return this.#ingestLifecycle(
      operation,
      ObservationKind.EVENT,
      "patchbay.adapter.DeliveryAcknowledgement.v1",
      {
        acceptedLsn: deliveryEventId?.lsn?.value.toString() ?? null,
      },
    );
  }

  async reportRunning(operation: Operation): Promise<EventId | undefined> {
    return this.#ingestLifecycle(
      operation,
      ObservationKind.STATUS,
      "patchbay.pi.DeliveryStatus.v1",
      { state: operationStateName(OperationState.RUNNING) },
    );
  }

  async reportResult(operation: Operation, value?: unknown): Promise<EventId | undefined> {
    return this.#ingestLifecycle(
      operation,
      ObservationKind.RESULT,
      "patchbay.pi.DeliveryResult.v1",
      { value: value ?? null },
    );
  }

  async ingestFailure(
    operation: Operation,
    failureCode: FailureCode,
    diagnostic: string,
  ): Promise<EventId | undefined> {
    return this.#ingestLifecycle(
      operation,
      ObservationKind.RESULT,
      "patchbay.pi.DeliveryFailure.v1",
      { diagnostic },
      failureCode,
    );
  }

  receiveDeliveries(cursor: bigint, signal?: AbortSignal): AsyncIterable<Delivery> {
    return this.#receiveDeliveries(cursor, signal);
  }

  async *#receiveDeliveries(
    cursor: bigint,
    signal?: AbortSignal,
  ): AsyncGenerator<Delivery> {
    for (let attempt = 0; attempt < 2; attempt += 1) {
      const failedToken = this.#attachmentToken;
      try {
        for await (const delivery of this.#client.receiveDeliveries(
          create(ReceiveRequestSchema, {
            adapterId: this.#adapterId(),
            cursor: create(LsnSchema, { value: cursor }),
          }),
          signal ? { signal } : {},
        )) {
          yield delivery;
        }
        return;
      } catch (error) {
        if (attempt !== 0 || !isUnauthenticated(error)) throw error;
        await this.#refreshAttachment(failedToken);
      }
    }
  }

  async #postAttach<T>(call: () => Promise<T>): Promise<T> {
    const failedToken = this.#attachmentToken;
    try {
      return await call();
    } catch (error) {
      if (!isUnauthenticated(error)) throw error;
      await this.#refreshAttachment(failedToken);
      return call();
    }
  }

  async #refreshAttachment(failedToken: string | undefined): Promise<void> {
    if (this.#attachmentToken && this.#attachmentToken !== failedToken) return;
    if (this.#adapterGeneration === undefined) {
      throw new Error("cannot reattach before the initial adapter attachment");
    }
    this.#reattachPromise ??= this.attach(this.#adapterGeneration)
      .then(() => undefined)
      .finally(() => {
        this.#reattachPromise = undefined;
      });
    await this.#reattachPromise;
  }

  #adapterId() {
    return create(AdapterIdSchema, { value: this.#options.adapterId });
  }

  #authorityDomainId() {
    return create(AuthorityDomainIdSchema, { value: this.#options.authorityDomainId });
  }

  async #ingestLifecycle(
    operation: Operation,
    kind: ObservationKind,
    schemaRef: string,
    payload: unknown,
    failureCode = FailureCode.UNSPECIFIED,
  ): Promise<EventId | undefined> {
    const commandId = operation.commandId?.value;
    if (!commandId) throw new Error("delivery operation is missing command_id");
    if (!operation.targetScope) throw new Error("delivery operation is missing target_scope");
    const observation = create(ObservationSchema, {
      authorityDomainId: this.#authorityDomainId(),
      sender: create(ActorEndpointRefSchema, {
        actorId: create(ActorIdSchema, { value: this.#options.adapterId }),
      }),
      kind,
      correlations: [
        create(TypedCorrelationSchema, {
          ref: {
            case: "commandId",
            value: create(CommandIdSchema, { value: commandId }),
          },
        }),
      ],
      targetScope: operation.targetScope,
      payload: create(PayloadEnvelopeSchema, {
        payload: encoder.encode(jsonStringify(payload)),
        contentType: PayloadContentType.JSON,
        schemaRef,
      }),
      failureCode,
    });
    const result = await this.#postAttach(() =>
      this.#client.ingestObservation(
        create(ObservationRequestSchema, {
          authorityDomainId: this.#authorityDomainId(),
          observation: { case: "event", value: observation },
        }),
      ),
    );
    return result.eventId;
  }
}

function isUnauthenticated(error: unknown): boolean {
  return error instanceof ConnectError && error.code === Code.Unauthenticated;
}

function jsonStringify(value: unknown): string {
  return JSON.stringify(value, (_key, nested) =>
    typeof nested === "bigint" ? nested.toString() : nested,
  );
}

function operationStateName(state: OperationState): string {
  return OperationState[state] ?? String(state);
}

function piCapabilityManifest() {
  return create(AdapterCapabilitySchema, {
    supportedOperationKinds: [
      OperationKind.ATTACH,
      OperationKind.INSTRUCT,
      OperationKind.CANCEL,
      OperationKind.INTERRUPT,
      OperationKind.QUERY,
      OperationKind.RECONFIGURE,
      OperationKind.SESSION_MANAGEMENT,
    ],
    supportedTargetSpecShapes: [],
    streamingSupport: true,
    snapshotSupport: AdapterSnapshotSupport.PARTIAL,
    cancellationSupport: true,
    sessionReplacementSupport: true,
    idempotencyStrength: IdempotencyStrength.AT_PATCHBAY_BOUNDARY,
    attachmentMethod: create(AttachmentMethodSchema, {
      kind: "configured-local-material",
      descriptor: new Uint8Array(),
      descriptorContentType: PayloadContentType.BINARY,
    }),
    knownFailureModes: [
      FailureCode.UNSUPPORTED_COMMAND,
      FailureCode.EXECUTION_FAILED,
      FailureCode.EXECUTION_OUTCOME_UNKNOWN,
    ],
  });
}
