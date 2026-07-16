import { create } from "@bufbuild/protobuf";
import { createClient, type Client, type Interceptor } from "@connectrpc/connect";
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
} from "@patchbay/contracts";
import type { TranscriptEvent } from "./transcript_event.js";

const encoder = new TextEncoder();

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
}

export class PatchbayCoreClient {
  readonly #client: AdapterClient;
  readonly #options: CoreClientOptions;

  constructor(options: CoreClientOptions) {
    if (!options.adapterId || !options.authorityDomainId || !options.attachmentEvidence) {
      throw new Error("adapter id, authority domain, and attachment evidence are required");
    }
    this.#options = options;
    const authenticate: Interceptor = (next) => async (request) => {
      request.header.set("x-patchbay-adapter-id", options.adapterId);
      request.header.set("x-patchbay-adapter-evidence", options.attachmentEvidence);
      return next(request);
    };
    this.#client = createClient(
      AdapterControlService,
      createGrpcTransport({ baseUrl: options.coreAddress, interceptors: [authenticate] }),
    );
  }

  async attach(adapterGeneration: number): Promise<EventId> {
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
    return result.attachEventId;
  }

  async reportSession(
    identity: SessionIdentity,
    activity: SessionActivityState,
    connectivity = SessionConnectivityState.LIVE,
  ): Promise<EventId | undefined> {
    const result = await this.#client.ingestObservation(
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
          }),
        },
      }),
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
    const result = await this.#client.ingestObservation(
      create(ObservationRequestSchema, {
        authorityDomainId: this.#authorityDomainId(),
        observation: { case: "event", value: observation },
      }),
    );
    return result.eventId;
  }

  async ingestFailure(
    identity: SessionIdentity,
    commandId: string,
    failureCode: FailureCode,
    diagnostic: string,
  ): Promise<EventId | undefined> {
    const observation = create(ObservationSchema, {
      authorityDomainId: this.#authorityDomainId(),
      sender: create(ActorEndpointRefSchema, {
        actorId: create(ActorIdSchema, { value: this.#options.adapterId }),
      }),
      kind: ObservationKind.RESULT,
      correlations: [
        create(TypedCorrelationSchema, {
          ref: {
            case: "commandId",
            value: create(CommandIdSchema, { value: commandId }),
          },
        }),
      ],
      targetScope: create(TargetScopeSchema, {
        kind: TargetScopeKind.RUNTIME_SESSION,
        adapterId: this.#adapterId(),
        deploymentScope: identity.deploymentScope,
        runtimeSessionId: create(RuntimeSessionIdSchema, { value: identity.runtimeSessionId }),
        sessionGeneration: create(GenerationSchema, { value: BigInt(identity.generation) }),
      }),
      payload: create(PayloadEnvelopeSchema, {
        payload: encoder.encode(JSON.stringify({ diagnostic })),
        contentType: PayloadContentType.JSON,
        schemaRef: "patchbay.pi.DeliveryFailure.v1",
      }),
      failureCode,
    });
    const result = await this.#client.ingestObservation(
      create(ObservationRequestSchema, {
        authorityDomainId: this.#authorityDomainId(),
        observation: { case: "event", value: observation },
      }),
    );
    return result.eventId;
  }

  receiveDeliveries(cursor: bigint): AsyncIterable<Delivery> {
    return this.#client.receiveDeliveries(
      create(ReceiveRequestSchema, {
        adapterId: this.#adapterId(),
        cursor: create(LsnSchema, { value: cursor }),
      }),
    );
  }

  #adapterId() {
    return create(AdapterIdSchema, { value: this.#options.adapterId });
  }

  #authorityDomainId() {
    return create(AuthorityDomainIdSchema, { value: this.#options.authorityDomainId });
  }
}

export function deliveryCursor(eventId: EventId | undefined, fallback: bigint): bigint {
  return eventId?.lsn?.value ?? fallback;
}

function piCapabilityManifest() {
  return create(AdapterCapabilitySchema, {
    supportedOperationKinds: [
      OperationKind.ATTACH,
      OperationKind.INSTRUCT,
      OperationKind.CANCEL,
      OperationKind.INTERRUPT,
      OperationKind.QUERY,
      OperationKind.APPROVAL_RESPONSE,
      OperationKind.ELICITATION_RESPONSE,
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
