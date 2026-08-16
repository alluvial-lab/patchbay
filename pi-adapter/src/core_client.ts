import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
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
  AdapterAssuranceManifestSchema,
  AdapterAssuranceManifestV1Schema,
  AdapterCapabilitySchema,
  AdapterDiagnosticReportingCapabilitySchema,
  AdapterControlService,
  AdapterIdSchema,
  AdapterRegistrationSchema,
  AdapterReconciliationStrength,
  AdapterSnapshotSupport,
  AdapterTargetCategory,
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
  PiControlProofKind,
  PiControlProofSchema,
  PiCursorDurabilityCondition,
  PiCursorMechanism,
  PiCursorSemanticsSchema,
  PiCwdProofKind,
  PiEventSemanticsSchema,
  PiLiveEventCaveat,
  PiPreMaterializationState,
  PiProcessReplacementOnlyKind,
  PiProjectContextResolution,
  PiProjectContextSemanticsSchema,
  PiReloadAdmission,
  PiReloadBoundarySchema,
  PiReloadMechanism,
  PiReloadableResourceKind,
  PiRuntimeProfileSchema,
  PiSessionDurabilitySchema,
  PiSessionMaterializationPolicy,
  PiTransportMechanism,
  ReceiveRequestSchema,
  ReconciliationAction,
  RuntimeSessionIdSchema,
  SessionActivityState,
  SessionConnectivityState,
  SessionReportSchema,
  SessionReportSourceCursorSchema,
  SpawnExecutionEvidenceSchema,
  SpawnExecutionPhase,
  ExternalEffectDisposition,
  NoExternalEffectProofSchema,
  SupervisorPreLaunchFailureProofSchema,
  SpawnExecutionEvidenceProducer,
  ContinuationContextStatus,
  TargetScopeKind,
  TargetScopeSchema,
  TypedCorrelationSchema,
  type AdapterCapability,
  type AdapterDiagnosticReport,
  type AdapterDiagnosticReportResult,
  type Delivery,
  type EventId,
  type Operation,
  type PiRuntimeProfile,
  type PayloadEnvelope,
  type RuntimeGenerationRef,
  type SpawnGenerationClaim,
} from "@patchbay/contracts";
import {
  diagnosticError,
  NOOP_ADAPTER_DIAGNOSTICS,
  type AdapterDiagnostics,
} from "./adapter_diagnostics.js";
import type { TranscriptEvent } from "./transcript_event.js";
import { PI_FORWARDED_DIAGNOSTIC_CODES } from "./core_diagnostics_forwarder.js";
import type { SessionReportOrder } from "./session_report_sequencer.js";

const encoder = new TextEncoder();
const attachmentTokenHeader = "x-patchbay-adapter-attachment-token";
export const PI_RUNTIME_PROFILE_SCHEMA_REF = "patchbay.PiRuntimeProfile.v1";

type AdapterClient = Client<typeof AdapterControlService>;

export interface CoreClientOptions {
  coreAddress: string;
  adapterId: string;
  authorityDomainId: string;
  attachmentEvidence: string;
}

export interface SessionIdentity {
  readonly runtimeSessionId: string;
  readonly deploymentScope: string;
  readonly generation: number;
  readonly project: string;
  readonly cwd: string;
  readonly name: string;
  readonly model: string;
}

export class PatchbayCoreClient {
  readonly #client: AdapterClient;
  readonly #options: CoreClientOptions;
  #attachmentToken: string | undefined;
  #adapterGeneration: number | undefined;
  #reattachPromise: Promise<void> | undefined;
  #diagnostics: AdapterDiagnostics;

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

  setDiagnostics(diagnostics: AdapterDiagnostics): void {
    this.#diagnostics = diagnostics;
  }

  get adapterId(): string {
    return this.#options.adapterId;
  }

  get adapterGeneration(): number {
    if (this.#adapterGeneration === undefined) {
      throw new Error("adapter has not attached");
    }
    return this.#adapterGeneration;
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
          capability: requiredPiCapabilityManifest(),
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

  /** Diagnostics deliberately bypass #postAttach: they cannot refresh auth or
   * compete with control traffic, and a failed report is best effort. */
  reportDiagnostic(
    report: AdapterDiagnosticReport,
    signal?: AbortSignal,
  ): Promise<AdapterDiagnosticReportResult> {
    return this.#client.reportDiagnostics(report, signal ? { signal } : undefined);
  }

  async reportSession(
    identity: SessionIdentity,
    activity: SessionActivityState,
    connectivity: SessionConnectivityState,
    sourceOrder: SessionReportOrder,
    spawn?: {
      readonly claimOperationId: string;
      readonly continuationContextStatus: ContinuationContextStatus;
    },
  ): Promise<EventId | undefined> {
    if (sourceOrder.revision <= 0n) {
      throw new Error("session report revision must be positive");
    }
    if (sourceOrder.adapterGeneration !== this.#adapterGeneration) {
      throw new Error("session report adapter generation does not match the active attachment");
    }
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
              // Raw project/cwd/path labels remain adapter-local. Core-visible
              // successor/current identity uses only redacted presentation text.
              project: "",
              cwd: "",
              name: identity.name,
              model: identity.model,
              ...(spawn
                ? {
                    spawnOrigin: create(TypedCorrelationSchema, {
                      ref: {
                        case: "commandId",
                        value: create(CommandIdSchema, { value: spawn.claimOperationId }),
                      },
                    }),
                    continuationContextStatus: spawn.continuationContextStatus,
                  }
                : {}),
              sourceCursor: create(SessionReportSourceCursorSchema, {
                adapterGeneration: create(GenerationSchema, {
                  value: BigInt(sourceOrder.adapterGeneration),
                }),
                revision: sourceOrder.revision,
              }),
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

  async reportSpawnResult(operation: Operation, payload: Uint8Array): Promise<EventId | undefined> {
    return this.#ingestLifecycleEnvelope(
      operation,
      ObservationKind.RESULT,
      create(PayloadEnvelopeSchema, {
        payload,
        contentType: PayloadContentType.PROTOBUF,
        schemaRef: "patchbay.PiSpawnResult.v1",
      }),
      FailureCode.UNSPECIFIED,
    );
  }

  async reportSpawnEvidence(input: {
    readonly operation: Operation;
    readonly exactClaim: SpawnGenerationClaim;
    readonly phase: SpawnExecutionPhase;
    readonly disposition: ExternalEffectDisposition;
    readonly failureCode: FailureCode;
    readonly externalRuntime?: RuntimeGenerationRef;
    readonly supervisorNoEffectProof?: boolean;
  }): Promise<EventId | undefined> {
    const proof = input.supervisorNoEffectProof
      ? create(NoExternalEffectProofSchema, {
          proof: {
            case: "exactSupervisorPreLaunchFailure",
            value: create(SupervisorPreLaunchFailureProofSchema, {
              adapterId: this.#adapterId(),
              adapterGeneration: create(GenerationSchema, {
                value: BigInt(this.adapterGeneration),
              }),
            }),
          },
        })
      : undefined;
    const evidence = create(SpawnExecutionEvidenceSchema, {
      authorityDomainId: this.#authorityDomainId(),
      exactClaim: input.exactClaim,
      phase: input.phase,
      externalEffectDisposition: input.disposition,
      // The server replaces both fields from the authenticated attachment.
      producer: SpawnExecutionEvidenceProducer.UNSPECIFIED,
      failureCode: input.failureCode,
      ...(proof ? { noExternalEffectProof: proof } : {}),
      ...(input.externalRuntime ? { externalRuntime: input.externalRuntime } : {}),
    });
    const result = await this.#postAttach(() =>
      this.#client.ingestObservation(
        create(ObservationRequestSchema, {
          authorityDomainId: this.#authorityDomainId(),
          observation: { case: "spawnExecutionEvidence", value: evidence },
        }),
      ),
    );
    return result.eventId;
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
    return this.#ingestLifecycleEnvelope(
      operation,
      kind,
      create(PayloadEnvelopeSchema, {
        payload: encoder.encode(jsonStringify(payload)),
        contentType: PayloadContentType.JSON,
        schemaRef,
      }),
      failureCode,
    );
  }

  async #ingestLifecycleEnvelope(
    operation: Operation,
    kind: ObservationKind,
    payload: PayloadEnvelope,
    failureCode: FailureCode,
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
      payload,
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

export interface PiCapabilityEvidence {
  readonly supervisor: boolean;
  readonly controlHandshake: boolean;
  readonly strictSessionTreeValidation: boolean;
  readonly authoritativeCursorReplacement: boolean;
  readonly idleMaterializedReload: boolean;
  readonly conformanceVersion?: string;
}

export function piCapabilityManifest(): AdapterCapability {
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
    sessionSnapshotSupport: AdapterSnapshotSupport.PARTIAL,
    cancellationSupport: true,
    // The lifecycle-conformance checkpoint owns activation of managed spawn,
    // continuation, replacement, cursor, and reload claims. A generated
    // profile alone is not activation evidence.
    sessionReplacementSupport: false,
    assurance: create(AdapterAssuranceManifestSchema, {
      contract: {
        case: "v1",
        value: create(AdapterAssuranceManifestV1Schema, {
          deduplicationStrength: IdempotencyStrength.AT_PATCHBAY_BOUNDARY,
          continuationProofSupport: false,
          cursorSupport: false,
          generationFenceSupport: false,
          reconciliationStrength: AdapterReconciliationStrength.NONE,
          unprovenOutcomeAction: ReconciliationAction.MANUAL_REQUIRED,
        }),
      },
    }),
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
    diagnosticReporting: create(AdapterDiagnosticReportingCapabilitySchema, {
      diagnosticCodes: Object.values(PI_FORWARDED_DIAGNOSTIC_CODES),
    }),
    targetCategories: [AdapterTargetCategory.RUNTIME_SESSION],
    resourceCapabilities: [],
    adapterProfile: create(PayloadEnvelopeSchema, {
      payload: toBinary(PiRuntimeProfileSchema, piRuntimeProfile()),
      contentType: PayloadContentType.PROTOBUF,
      schemaRef: PI_RUNTIME_PROFILE_SCHEMA_REF,
    }),
  });
}

function requiredPiCapabilityManifest(): AdapterCapability {
  const manifest = piCapabilityManifest();
  decodePiRuntimeProfile(manifest);
  return manifest;
}

export function decodePiRuntimeProfile(manifest: AdapterCapability): PiRuntimeProfile {
  const envelope = manifest.adapterProfile;
  if (!envelope) throw new Error("Pi capability manifest is missing its runtime profile");
  if (
    envelope.schemaRef !== PI_RUNTIME_PROFILE_SCHEMA_REF
    || envelope.contentType !== PayloadContentType.PROTOBUF
    || envelope.payload.length === 0
  ) {
    throw new Error("Pi capability manifest has an invalid runtime profile envelope");
  }

  let profile: PiRuntimeProfile;
  try {
    profile = fromBinary(PiRuntimeProfileSchema, envelope.payload);
  } catch {
    throw new Error("Pi capability manifest runtime profile is malformed");
  }
  validatePiRuntimeProfile(profile);
  return profile;
}

function piRuntimeProfile(): PiRuntimeProfile {
  return create(PiRuntimeProfileSchema, {
    transport: PiTransportMechanism.RPC_JSONL_SUBPROCESS,
    events: create(PiEventSemanticsSchema, {
      liveEventCaveats: [
        PiLiveEventCaveat.PARTIAL_ORDER,
        PiLiveEventCaveat.PERSISTED_ENTRIES_REQUIRED_FOR_RECONCILIATION,
      ],
    }),
    sessionDurability: create(PiSessionDurabilitySchema, {
      materializationPolicy: PiSessionMaterializationPolicy.AFTER_FIRST_ASSISTANT_MESSAGE,
      preMaterializationState: PiPreMaterializationState.MEMORY_ONLY_NOT_RESUMABLE,
    }),
    cursor: create(PiCursorSemanticsSchema, {
      mechanism: PiCursorMechanism.PERSISTED_ENTRY_ID_WITH_EXACT_SET_REPLACEMENT,
      durabilityCondition: PiCursorDurabilityCondition.MATERIALIZED_SESSION_ONLY,
    }),
    controlProof: create(PiControlProofSchema, {
      kind: PiControlProofKind.CHALLENGED_EXTENSION_CUSTOM_ENTRY,
    }),
    reload: create(PiReloadBoundarySchema, {
      mechanism: PiReloadMechanism.CONTROL_EXTENSION_CTX_RELOAD,
      admission: PiReloadAdmission.IDLE_MATERIALIZED_SESSION,
      processReplacementOnly: [
        PiProcessReplacementOnlyKind.ARBITRARY_EXTENSION_DEPENDENCY_GRAPH,
        PiProcessReplacementOnlyKind.PI_RUNTIME_PACKAGE_DIST,
        PiProcessReplacementOnlyKind.NATIVE_DEPENDENCY,
        PiProcessReplacementOnlyKind.EXECUTABLE,
        PiProcessReplacementOnlyKind.UNKNOWN_SCOPE,
      ],
    }),
    enumeratedResources: [
      PiReloadableResourceKind.EXTENSION_ENTRYPOINT,
      PiReloadableResourceKind.SKILL,
      PiReloadableResourceKind.PROMPT,
      PiReloadableResourceKind.THEME,
      PiReloadableResourceKind.CONTEXT_FILE,
    ],
    projectContext: create(PiProjectContextSemanticsSchema, {
      resolution:
        PiProjectContextResolution.ADAPTER_RESOLVED_CWD_PROJECT_TRUST_AND_RESOURCE_ROOTS,
      cwdProof: PiCwdProofKind.CHALLENGED_CONTROL_EXTENSION,
    }),
  });
}

function validatePiRuntimeProfile(profile: PiRuntimeProfile): void {
  if (profile.transport !== PiTransportMechanism.RPC_JSONL_SUBPROCESS) {
    throw new Error("Pi runtime profile has an unsupported transport mechanism");
  }
  if (!profile.events) throw new Error("Pi runtime profile is missing event semantics");
  requireExactEnumSet(
    profile.events.liveEventCaveats,
    [
      PiLiveEventCaveat.PARTIAL_ORDER,
      PiLiveEventCaveat.PERSISTED_ENTRIES_REQUIRED_FOR_RECONCILIATION,
    ],
    "live event caveats",
  );
  if (
    profile.sessionDurability?.materializationPolicy
      !== PiSessionMaterializationPolicy.AFTER_FIRST_ASSISTANT_MESSAGE
    || profile.sessionDurability.preMaterializationState
      !== PiPreMaterializationState.MEMORY_ONLY_NOT_RESUMABLE
  ) {
    throw new Error("Pi runtime profile has invalid session durability semantics");
  }
  if (
    profile.cursor?.mechanism
      !== PiCursorMechanism.PERSISTED_ENTRY_ID_WITH_EXACT_SET_REPLACEMENT
    || profile.cursor.durabilityCondition
      !== PiCursorDurabilityCondition.MATERIALIZED_SESSION_ONLY
  ) {
    throw new Error("Pi runtime profile has invalid cursor semantics");
  }
  if (profile.controlProof?.kind !== PiControlProofKind.CHALLENGED_EXTENSION_CUSTOM_ENTRY) {
    throw new Error("Pi runtime profile has invalid control proof semantics");
  }
  if (
    profile.reload?.mechanism !== PiReloadMechanism.CONTROL_EXTENSION_CTX_RELOAD
    || profile.reload.admission !== PiReloadAdmission.IDLE_MATERIALIZED_SESSION
  ) {
    throw new Error("Pi runtime profile has invalid reload semantics");
  }
  requireExactEnumSet(
    profile.reload.processReplacementOnly,
    [
      PiProcessReplacementOnlyKind.ARBITRARY_EXTENSION_DEPENDENCY_GRAPH,
      PiProcessReplacementOnlyKind.PI_RUNTIME_PACKAGE_DIST,
      PiProcessReplacementOnlyKind.NATIVE_DEPENDENCY,
      PiProcessReplacementOnlyKind.EXECUTABLE,
      PiProcessReplacementOnlyKind.UNKNOWN_SCOPE,
    ],
    "process-replacement exclusions",
  );
  requireExactEnumSet(
    profile.enumeratedResources,
    [
      PiReloadableResourceKind.EXTENSION_ENTRYPOINT,
      PiReloadableResourceKind.SKILL,
      PiReloadableResourceKind.PROMPT,
      PiReloadableResourceKind.THEME,
      PiReloadableResourceKind.CONTEXT_FILE,
    ],
    "enumerated resources",
  );
  if (
    profile.projectContext?.resolution
      !== PiProjectContextResolution.ADAPTER_RESOLVED_CWD_PROJECT_TRUST_AND_RESOURCE_ROOTS
    || profile.projectContext.cwdProof !== PiCwdProofKind.CHALLENGED_CONTROL_EXTENSION
  ) {
    throw new Error("Pi runtime profile has invalid project context semantics");
  }
}

function requireExactEnumSet(actual: readonly number[], expected: readonly number[], name: string): void {
  const values = new Set(actual);
  if (
    values.size !== actual.length
    || values.size !== expected.length
    || expected.some((value) => value === 0 || !values.has(value))
  ) {
    throw new Error(`Pi runtime profile has invalid ${name}`);
  }
}
