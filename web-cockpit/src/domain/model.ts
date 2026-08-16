import { fromBinary } from "@bufbuild/protobuf";
import {
  ApprovalDecision,
  ApprovalResponsePayloadSchema,
  CommandTransitionSchema,
  ContinuationContextStatus,
  ElicitationResponsePayloadSchema,
  ElicitationSchema,
  ElicitationState,
  ObservationKind,
  ObservationSchema,
  OperationKind,
  OperationSchema,
  OperationState,
  PayloadContentType,
  ResponseContractKind,
  ResourceStateEventSchema,
  ResourceFreshnessState,
  AdapterSnapshotSupport,
  type Resource,
  type ResourceSnapshot,
  type ResourceStateEvent,
  SessionActivityState,
  SessionConnectivityState,
  SessionStateEventSchema,
  SecurityLockdownEventSchema,
  SpawnClaimDisposition,
  SpawnClaimEventSchema,
  SpawnPromotionCommittedSchema,
  SpawnRequestSchema,
  StoredEventKind,
  type Elicitation,
  FailureCode,
  type Observation,
  type Operation,
  type ResponseContract,
  type Session,
  type SessionReport,
  type SessionSnapshot,
  type SpawnClaimEvent,
  type SpawnPromotionCommitted,
  type SubscribeEvent,
  type TargetScope,
  type AdapterStatus,
  type AdapterDiagnosticSeverity,
  type SecurityLockdownState,
  type SecuritySnapshot,
  TargetScopeKind,
} from "@patchbay/contracts";

import {
  decodeTokenCommuneResourceObservation,
  foldPiPersistedProjectionObservation,
  piPersistedProjectionContinuityId,
  type PiPersistedProjectionState,
} from "@patchbay/operator-domain";

import type { ReconcileProjection } from "./reconcile.js";
import { foldAdapterDiagnosticObservation } from "./adapter-diagnostics.js";
import {
  decodeResourceProjection,
  type ResourceIdentityView,
  type ResourceProjectionResult,
} from "./resource-projection.js";

export type { ResourceIdentityView } from "./resource-projection.js";

export interface SessionIdentity {
  adapterId: string;
  deploymentScope: string;
  runtimeSessionId: string;
  generation: bigint;
}

export interface SessionView {
  identity: SessionIdentity;
  /** Stable managed-spawn identity needed to construct an exact continuation. */
  logicalTargetId?: string;
  label: { project?: string; cwd?: string; name?: string };
  model?: string;
  connectivity: SessionConnectivityState;
  activity: SessionActivityState;
  activityDetail?: string;
  /** Typed presentation provenance; tool details can be hidden without guessing from copy. */
  activityDetailProvenance?: "runtime" | "tool";
  needsYou: boolean;
  lastLsn: bigint;
  lastUpdate?: Date;
  tombstoned: boolean;
  reconciled: boolean;
  lockdownActive?: boolean;
  resourceLinkage?: SessionResourceLinkage;
}

export interface ResourceCollectionView {
  adapterId: string;
  resourceKind: string;
  completeness: AdapterSnapshotSupport;
  sourceAdapterGeneration: bigint;
  revisionLsn: bigint;
  observedAt?: Date;
  reconciled: boolean;
}

export interface ResourceView {
  identity: ResourceIdentityView;
  freshness: ResourceFreshnessState;
  sourceAdapterGeneration: bigint;
  revisionLsn: bigint;
  observedAt?: Date;
  tombstoned: boolean;
  replacedBy?: ResourceIdentityView;
  hasCachedPayload: boolean;
  reconciled: boolean;
  projection: ResourceProjectionResult;
}

export interface SessionResourceLinkage {
  usageResource: ResourceIdentityView;
}

export interface CommandHistoryEntry {
  state: OperationState;
  lsn: bigint;
  failureCode?: FailureCode;
  race?: string;
}

export type OperationTargetView =
  | { kind: "runtime-session"; identity: SessionIdentity }
  | { kind: "operational-resource"; identity: ResourceIdentityView };

/** Presentation-only correlation to an accepted cancel/interrupt Operation. */
export interface PendingControlRequest {
  commandId: string;
  kind: OperationKind.CANCEL | OperationKind.INTERRUPT;
  lsn: bigint;
}

export interface CommandView {
  id: string;
  state: OperationState;
  lsn: bigint;
  failureCode?: FailureCode;
  race?: string;
  /** Stable managed target selected by the durable spawn claim. */
  spawnLogicalTargetId?: string;
  /** Durable spawn-claim lifecycle, independent from the command lifecycle and failure. */
  spawnClaimDisposition?: SpawnClaimDisposition;
  /** Adapter-reported continuation outcome, folded only from promotion evidence. */
  continuationContextStatus?: ContinuationContextStatus;
  pendingControlRequest?: PendingControlRequest;
  target?: OperationTargetView;
  operation: Operation;
  history: CommandHistoryEntry[];
}

export interface ElicitationAnswer {
  selectedOptionId?: string;
  freeText?: string;
  clarification?: string;
  approvalDecision?: ApprovalDecision;
}

export interface ElicitationView {
  id: string;
  kind: "approval" | "question";
  state: ElicitationState;
  contract: ResponseContract;
  prompt: string;
  target?: SessionIdentity;
  /** Same opener + authoritative correlation identifies one visual question batch. */
  groupingKey?: string;
  lsn: bigint;
  answer?: ElicitationAnswer;
}

export interface ResourceObservationView {
  poolIdentity: ResourceIdentityView;
  kind: "pool-event" | "event-gap";
  code: string;
  occurredAt: Date;
  lsn: bigint;
}

export interface ObservationView {
  id: string;
  session?: SessionIdentity;
  role: "operator" | "agent" | "tool" | "system";
  kind: string;
  markdown: string;
  lsn: bigint;
  messageId?: string;
  /** Exact typed CommandId correlation carried by the source Observation. */
  commandId?: string;
  /** Plain-text preview of tool args/result; rendered as text, never markdown. */
  detail?: string;
  /** Exact Pi-persisted projection membership; absent for transient/audit observations. */
  piProjection?: { readonly continuityId: string; readonly membershipId: string };
}

export interface AdapterDiagnosticView {
  sourceEventId: string;
  lsn: bigint;
  adapterId: string;
  adapterGeneration: bigint;
  target?: SessionIdentity;
  commandId?: string;
  severity: AdapterDiagnosticSeverity;
  code: string;
  failureCode?: FailureCode;
  operationKind?: OperationKind;
  count: number;
  observedAt?: Date;
}

export interface LockdownView {
  /** Local presentation state only; never an authoritative posture claim. */
  submitting?: boolean;
  active: boolean;
  reasonCode?: string;
  enteredAt?: Date;
  enteredEventLsn?: bigint;
}

export interface OperatorSessionSummaryView {
  actorId: string;
  endpointId: string;
  deviceId: string;
  generation: bigint;
  active: boolean;
  revoked: boolean;
  expired: boolean;
}

export interface ControlSurfaceSummaryView {
  principalId: string;
  endpointId: string;
  deviceId: string;
  generation: bigint;
  revoked: boolean;
}

export interface GrantSummaryView {
  grantId: string;
  subjectActorId: string;
  targetScope?: TargetScope;
  allowedOperationKinds: number[];
  expiresAt?: Date;
  revoked: boolean;
  revocationPolicy: number;
}

export interface SecurityInventoryView {
  snapshotLsn: bigint;
  operatorSessions: OperatorSessionSummaryView[];
  controlSurfaces: ControlSurfaceSummaryView[];
  grants: GrantSummaryView[];
}

export interface AdapterView {
  adapterId: string;
  status?: AdapterStatus;
  asOfLsn: bigint;
  recentDiagnostics: readonly AdapterDiagnosticView[];
}

export interface PresentationModel {
  authorityDomainId?: string;
  /** Persisted core/storage continuity anchor carried by authoritative snapshots. */
  coreGeneration?: bigint;
  cursor: bigint;
  reconciled: boolean;
  sessions: Map<string, SessionView>;
  resources: Map<string, ResourceView>;
  resourceCollections: Map<string, ResourceCollectionView>;
  commands: Map<string, CommandView>;
  elicitations: Map<string, ElicitationView>;
  adapters: Map<string, AdapterView>;
  observations: ObservationView[];
  /** Authoritative exact-set state for known Pi continuity scopes. */
  piPersistedProjections: Map<string, PiPersistedProjectionState>;
  resourceObservations: ResourceObservationView[];
  lockdown: LockdownView;
  security: SecurityInventoryView;
}

export function emptyPresentationModel(): PresentationModel {
  return {
    cursor: 0n,
    reconciled: false,
    sessions: new Map(),
    resources: new Map(),
    resourceCollections: new Map(),
    commands: new Map(),
    elicitations: new Map(),
    adapters: new Map(),
    observations: [],
    piPersistedProjections: new Map(),
    resourceObservations: [],
    lockdown: { active: false, submitting: false },
    security: emptySecurityInventory(),
  };
}

function emptySecurityInventory(): SecurityInventoryView {
  return {
    snapshotLsn: 0n,
    operatorSessions: [],
    controlSurfaces: [],
    grants: [],
  };
}

/** Pure event projection. It never mutates the input model or generated messages. */
export function fold(model: PresentationModel, event: SubscribeEvent): PresentationModel {
  const lsn = requiredBigint(event.eventId?.lsn?.value, "event LSN");
  const authorityDomainId = required(event.eventId?.authorityDomainId?.value, "event authority domain");
  const payload = event.payload;
  if (!payload) throw new Error("subscription event payload is missing");
  if (model.authorityDomainId && model.authorityDomainId !== authorityDomainId) {
    throw new Error("cross-domain event rejected by presentation fold");
  }
  if (lsn <= model.cursor) return model;

  const next = cloneModel(model);
  next.authorityDomainId = authorityDomainId;
  next.cursor = lsn;
  next.reconciled = true;

  switch (payload.kind) {
    case StoredEventKind.OPERATION:
      foldOperation(next, fromBinary(OperationSchema, payload.payload), lsn);
      break;
    case StoredEventKind.OBSERVATION:
      foldObservation(next, fromBinary(ObservationSchema, payload.payload), lsn);
      break;
    case StoredEventKind.ELICITATION:
      foldElicitation(next, fromBinary(ElicitationSchema, payload.payload), lsn);
      break;
    case StoredEventKind.COMMAND_TRANSITION:
      foldCommandTransition(next, fromBinary(CommandTransitionSchema, payload.payload), lsn);
      break;
    case StoredEventKind.SESSION_STATE:
      foldSessionState(next, fromBinary(SessionStateEventSchema, payload.payload), lsn);
      break;
    case StoredEventKind.RESOURCE_STATE: {
      const resourceEvent = fromBinary(ResourceStateEventSchema, payload.payload);
      if (resourceEvent.authorityDomainId?.value !== authorityDomainId) {
        throw new Error("cross-domain resource event rejected by presentation fold");
      }
      foldResourceState(next, resourceEvent, lsn);
      break;
    }
    case StoredEventKind.SECURITY_LOCKDOWN:
      foldSecurityLockdown(next, fromBinary(SecurityLockdownEventSchema, payload.payload), lsn);
      break;
    case StoredEventKind.SPAWN_CLAIM:
      foldSpawnClaim(next, fromBinary(SpawnClaimEventSchema, payload.payload), lsn);
      break;
    case StoredEventKind.SPAWN_PROMOTION_COMMITTED:
      foldSpawnPromotion(next, fromBinary(SpawnPromotionCommittedSchema, payload.payload), lsn);
      break;
    case StoredEventKind.GRANT:
    case StoredEventKind.DESCENDANT_GRANT:
    case StoredEventKind.REVOCATION:
      break;
    case StoredEventKind.UNSPECIFIED:
    default:
      throw new Error(`unsupported stored event kind ${payload.kind}`);
  }

  deriveNeedsYou(next);
  return next;
}

/** Snapshot baselines for the two independently materialized presentation axes. */
export interface SnapshotBaselines {
  session: SessionSnapshot;
  resource: ResourceSnapshot;
}

/**
 * Rebuilds the whole projection off to the side through the larger snapshot
 * horizon. Each snapshot suppresses only its own durable state axis.
 */
export function replaceFromSnapshots(
  snapshots: SnapshotBaselines,
  replayEvents: readonly SubscribeEvent[],
): PresentationModel {
  const sessionDomain = required(snapshots.session.authorityDomainId?.value, "session snapshot authority domain");
  const resourceDomain = required(snapshots.resource.authorityDomainId?.value, "resource snapshot authority domain");
  if (sessionDomain !== resourceDomain) throw new Error("cross-domain snapshot baselines rejected");
  const sessionGeneration = positiveBigint(
    snapshots.session.coreGeneration?.value,
    "session snapshot core generation",
  );
  const resourceGeneration = positiveBigint(
    snapshots.resource.coreGeneration?.value,
    "resource snapshot core generation",
  );
  if (sessionGeneration !== resourceGeneration) {
    throw new Error("cross-generation snapshot baselines rejected");
  }
  const sessionLsn = requiredBigint(snapshots.session.snapshotLsn?.value, "session snapshot LSN");
  const resourceLsn = requiredBigint(snapshots.resource.snapshotLsn?.value, "resource snapshot LSN");
  const horizon = sessionLsn > resourceLsn ? sessionLsn : resourceLsn;

  const sessions = new Map<string, SessionView>();
  for (const session of snapshots.session.sessions) {
    const view = sessionFromSnapshot(session, sessionDomain, sessionLsn);
    const key = sessionKey(view.identity);
    if (sessions.has(key)) throw new Error(`session snapshot repeats identity ${key}`);
    sessions.set(key, { ...view, reconciled: false });
  }

  const resources = new Map<string, ResourceView>();
  for (const resource of snapshots.resource.resources) {
    const view = resourceFromSnapshot(resource, resourceDomain, resourceLsn);
    const key = resourceKey(view.identity);
    if (resources.has(key)) throw new Error(`resource snapshot repeats identity ${key}`);
    resources.set(key, { ...view, reconciled: false });
  }
  const resourceCollections = new Map<string, ResourceCollectionView>();
  for (const revision of snapshots.resource.viewRevisions) {
    const adapterId = required(revision.adapterId?.value, "resource snapshot collection adapter");
    const resourceKind = required(revision.resourceKind?.value, "resource snapshot collection kind");
    const completeness = validCompleteness(revision.completeness, "resource snapshot collection completeness");
    const sourceAdapterGeneration = requiredBigint(
      revision.sourceAdapterGeneration?.value,
      "resource snapshot collection generation",
    );
    const revisionLsn = requiredBigint(revision.revisionLsn?.value, "resource snapshot collection revision");
    if (revisionLsn > resourceLsn) throw new Error("resource collection revision exceeds snapshot horizon");
    const key = resourceCollectionKey(adapterId, resourceKind);
    if (resourceCollections.has(key)) throw new Error(`resource snapshot repeats collection ${key}`);
    resourceCollections.set(key, {
      adapterId,
      resourceKind,
      completeness,
      sourceAdapterGeneration,
      revisionLsn,
      observedAt: optionalTimestampDate(revision.observedAt, "resource snapshot collection observed time"),
      reconciled: false,
    });
  }
  for (const resource of resources.values()) {
    if (!resourceCollections.has(resourceCollectionKey(resource.identity.adapterId, resource.identity.resourceKind))) {
      throw new Error("resource snapshot record has no matching collection revision");
    }
  }

  let model: PresentationModel = {
    authorityDomainId: sessionDomain,
    coreGeneration: sessionGeneration,
    cursor: 0n,
    reconciled: false,
    sessions,
    resources,
    resourceCollections,
    commands: new Map(),
    elicitations: new Map(),
    adapters: new Map(),
    observations: [],
    piPersistedProjections: new Map(),
    resourceObservations: [],
    lockdown: lockdownViewFromState(snapshots.session.lockdown),
    security: emptySecurityInventory(),
  };

  let replayCursor = 0n;
  for (const event of replayEvents) {
    const lsn = requiredBigint(event.eventId?.lsn?.value, "replay event LSN");
    const eventDomain = required(event.eventId?.authorityDomainId?.value, "replay event authority domain");
    if (eventDomain !== sessionDomain) throw new Error("cross-domain replay event rejected");
    if (lsn <= replayCursor || lsn > horizon) {
      throw new Error(`snapshot replay is not a strictly ordered visible prefix at LSN ${lsn}`);
    }
    replayCursor = lsn;
    if (!event.payload) throw new Error("snapshot replay event payload is missing");

    const coveredByBaseline =
      (event.payload.kind === StoredEventKind.SESSION_STATE && lsn <= sessionLsn)
      || (event.payload.kind === StoredEventKind.RESOURCE_STATE && lsn <= resourceLsn);
    const coveredPromotion =
      event.payload.kind === StoredEventKind.SPAWN_PROMOTION_COMMITTED && lsn <= sessionLsn;
    if (coveredPromotion) {
      // The session snapshot already contains later state/metadata for this
      // runtime. Fold lifecycle and managed identity only; replaying the
      // embedded promotion report must not roll the snapshot backward.
      foldSpawnPromotion(
        model,
        fromBinary(SpawnPromotionCommittedSchema, event.payload.payload),
        lsn,
        false,
      );
      model = { ...model, cursor: lsn };
    } else if (coveredByBaseline) {
      model = { ...model, cursor: lsn };
    } else {
      model = fold(model, event);
      model.reconciled = false;
    }
  }

  model.cursor = horizon;
  model.reconciled = true;
  model.sessions = new Map(
    [...model.sessions].map(([key, session]) => [key, { ...session, reconciled: true }]),
  );
  model.resources = new Map(
    [...model.resources].map(([key, resource]) => [key, { ...resource, reconciled: true }]),
  );
  model.resourceCollections = new Map(
    [...model.resourceCollections].map(([key, collection]) => [key, { ...collection, reconciled: true }]),
  );
  deriveNeedsYou(model);
  return model;
}

/**
 * Installs the redacted security inventory returned by the dedicated snapshot
 * RPC. It is separate from SessionSnapshot because the two projections have
 * different authority and redaction boundaries.
 */
export function replaceSecuritySnapshot(
  model: PresentationModel,
  snapshot: SecuritySnapshot,
): PresentationModel {
  const authorityDomainId = required(snapshot.authorityDomainId?.value, "security snapshot authority domain");
  if (model.authorityDomainId && model.authorityDomainId !== authorityDomainId) {
    throw new Error("cross-domain security snapshot rejected");
  }
  const snapshotLsn = requiredBigint(snapshot.snapshotLsn?.value, "security snapshot LSN");
  // A startup/reconnect read can complete after the live stream has already
  // folded a newer visible event. Never let that older inventory response
  // roll the presentation posture or summaries backward.
  if (snapshotLsn < model.cursor) return model;
  const next = cloneModel(model);
  next.authorityDomainId = authorityDomainId;
  next.security = {
    snapshotLsn,
    operatorSessions: snapshot.operatorSessions.map((summary) => ({
      actorId: required(summary.actorId?.value, "operator session actor"),
      endpointId: required(summary.endpointId?.value, "operator session endpoint"),
      deviceId: required(summary.deviceId?.value, "operator session device"),
      generation: requiredBigint(summary.operatorSessionGeneration?.value, "operator session generation"),
      active: summary.active,
      revoked: summary.revoked,
      expired: summary.expired,
    })),
    controlSurfaces: snapshot.controlSurfaces.map((summary) => ({
      principalId: required(summary.principalId, "control-surface principal"),
      endpointId: required(summary.endpointId?.value, "control-surface endpoint"),
      deviceId: required(summary.deviceId?.value, "control-surface device"),
      generation: requiredBigint(summary.endpointGeneration?.value, "control-surface generation"),
      revoked: summary.revoked,
    })),
    grants: snapshot.grants.map((summary) => ({
      grantId: required(summary.grantId?.value, "grant id"),
      subjectActorId: required(summary.subjectActorId?.value, "grant subject actor"),
      targetScope: summary.targetScope,
      allowedOperationKinds: [...summary.allowedOperationKinds],
      expiresAt: timestampDate(summary.expiresAt),
      revoked: summary.revoked,
      revocationPolicy: summary.revocationPolicy,
    })),
  };
  next.lockdown = lockdownViewFromState(snapshot.lockdown);
  if (next.lockdown.active) {
    next.sessions = new Map(
      [...next.sessions].map(([key, session]) => [key, {
        ...session,
        connectivity: SessionConnectivityState.STALE,
        activity: SessionActivityState.UNKNOWN,
        activityDetail: undefined,
        activityDetailProvenance: undefined,
        lockdownActive: true,
        needsYou: false,
      }]),
    );
  }
  return next;
}

/** Establishes the persisted storage-lineage fence before subscription events
 * can become authoritative. A non-empty streamed prefix with no prior anchor
 * cannot be retroactively attributed to whichever snapshot arrives later. */
export function bindCoreLineage(
  model: PresentationModel,
  authorityDomainId: string,
  coreGeneration: bigint,
): PresentationModel {
  const domain = required(authorityDomainId, "core lineage authority domain");
  if (coreGeneration <= 0n) throw new Error("core lineage generation must be positive");
  if (model.authorityDomainId && model.authorityDomainId !== domain) {
    throw new Error("cross-domain core lineage rejected");
  }
  if (model.coreGeneration !== undefined && model.coreGeneration !== coreGeneration) {
    throw new Error("cross-generation core lineage rejected");
  }
  if (model.cursor > 0n && model.coreGeneration === undefined) {
    throw new Error("cached core authority has no storage-lineage anchor");
  }
  if (model.authorityDomainId === domain && model.coreGeneration === coreGeneration) return model;
  return { ...model, authorityDomainId: domain, coreGeneration };
}

/** Marks cached axes honestly while a stream/snapshot gap is unresolved. */
export function markUnreconciled(model: PresentationModel): PresentationModel {
  const next = cloneModel(model);
  next.reconciled = false;
  next.adapters = new Map(
    [...model.adapters].map(([key, adapter]) => [key, { ...adapter, status: undefined }]),
  );
  next.sessions = new Map(
    [...model.sessions].map(([key, session]) => [
      key,
      {
        ...session,
        connectivity:
          session.connectivity === SessionConnectivityState.UNKNOWN
            ? SessionConnectivityState.UNKNOWN
            : SessionConnectivityState.STALE,
        activity: SessionActivityState.UNKNOWN,
        activityDetail: undefined,
        activityDetailProvenance: undefined,
        needsYou: false,
        reconciled: false,
      },
    ]),
  );
  next.resources = new Map(
    [...model.resources].map(([key, resource]) => [key, {
      ...resource,
      freshness: resource.hasCachedPayload
        ? ResourceFreshnessState.STALE
        : ResourceFreshnessState.UNKNOWN,
      reconciled: false,
    }]),
  );
  next.resourceCollections = new Map(
    [...model.resourceCollections].map(([key, collection]) => [key, { ...collection, reconciled: false }]),
  );
  return next;
}

export class PresentationProjection implements ReconcileProjection {
  constructor(public model: PresentationModel = emptyPresentationModel()) {}

  markUnreconciled(): void {
    this.model = markUnreconciled(this.model);
  }

  replaceFromSnapshots(snapshots: SnapshotBaselines, replayEvents: readonly SubscribeEvent[]): void {
    const replacement = replaceFromSnapshots(snapshots, replayEvents);
    assertNewerCoreAuthority(this.model, replacement);
    this.model = replacement;
  }

  replaceSecuritySnapshot(snapshot: SecuritySnapshot): void {
    this.model = replaceSecuritySnapshot(this.model, snapshot);
  }

  bindCoreLineage(authorityDomainId: string, coreGeneration: bigint): void {
    this.model = bindCoreLineage(this.model, authorityDomainId, coreGeneration);
  }

  foldEvent(event: SubscribeEvent): void {
    this.model = fold(this.model, event);
  }
}

export function resourceKey(identity: ResourceIdentityView): string {
  return [identity.adapterId, identity.resourceKind, identity.resourceId]
    .map((part) => `${part.length}:${part}`)
    .join("|");
}

export function resourceCollectionKey(adapterId: string, resourceKind: string): string {
  return [adapterId, resourceKind]
    .map((part) => `${part.length}:${part}`)
    .join("|");
}

export function rendersResourceCurrent(resource: ResourceView): boolean {
  return resource.reconciled
    && !resource.tombstoned
    && resource.freshness === ResourceFreshnessState.CURRENT;
}

export function sessionKey(identity: SessionIdentity): string {
  return [
    identity.adapterId,
    identity.deploymentScope,
    identity.runtimeSessionId,
    identity.generation.toString(),
  ]
    .map((part) => `${part.length}:${part}`)
    .join("|");
}

export function stableTarget(session: SessionView | undefined): session is SessionView {
  return Boolean(
    session &&
      session.reconciled &&
      !session.tombstoned &&
      session.identity.adapterId &&
      session.identity.deploymentScope &&
      session.identity.runtimeSessionId &&
      session.identity.generation > 0n,
  );
}

/** Presentation-only tool filtering uses typed provenance, never activity-detail copy. */
export function presentedActivityDetail(
  session: SessionView,
  showToolCalls = true,
): string | undefined {
  return showToolCalls || session.activityDetailProvenance !== "tool"
    ? session.activityDetail
    : undefined;
}

/** The view-binding dominance rule: only a reconciled, non-tombstoned LIVE axis is live. */
export function rendersLive(session: SessionView): boolean {
  return stableTarget(session)
    && !session.lockdownActive
    && session.connectivity === SessionConnectivityState.LIVE;
}

function foldOperation(model: PresentationModel, operation: Operation, lsn: bigint): void {
  const id = required(operation.commandId?.value, "operation command id");
  if (!model.commands.has(id)) {
    const target = operationTargetFromOperation(operation);
    model.commands.set(id, {
      id,
      state: OperationState.ACCEPTED,
      lsn,
      target,
      operation,
      history: [{ state: OperationState.ACCEPTED, lsn }],
    });
  }

  if (operation.kind !== OperationKind.CANCEL && operation.kind !== OperationKind.INTERRUPT) return;
  const targetCommandId = exactCommandCorrelation(operation.correlations);
  if (!targetCommandId) return;
  const targetCommand = model.commands.get(targetCommandId);
  if (!targetCommand || targetCommand.race || lsn <= targetCommand.lsn) return;

  const action = controlRequestLabel(operation.kind);
  if (isTerminalCommand(targetCommand.state)) {
    model.commands.set(targetCommandId, {
      ...targetCommand,
      race: `${terminalStateLabel(targetCommand.state)} before ${action} arrived`,
    });
    return;
  }
  if (targetCommand.pendingControlRequest) return;

  model.commands.set(targetCommandId, {
    ...targetCommand,
    pendingControlRequest: {
      commandId: id,
      kind: operation.kind,
      lsn,
    },
  });
}

function foldSpawnClaim(model: PresentationModel, event: SpawnClaimEvent, lsn: bigint): void {
  switch (event.mutation.case) {
    case "accepted": {
      const operation = event.mutation.value.acceptedOperation?.operation;
      if (!operation) throw new Error("accepted spawn claim is missing its Operation");
      foldOperation(model, operation, lsn);
      const commandId = required(operation.commandId?.value, "accepted spawn claim command id");
      const claim = event.mutation.value.claim;
      const logicalTargetId = required(claim?.logicalTargetId?.value, "accepted spawn claim logical target id");
      const command = model.commands.get(commandId)!;
      model.commands.set(commandId, {
        ...command,
        spawnLogicalTargetId: logicalTargetId,
        spawnClaimDisposition: SpawnClaimDisposition.ACTIVE,
        target: claim?.expectedPrior
          ? { kind: "runtime-session", identity: identityFromRuntimeRef(claim.expectedPrior) }
          : command.target,
      });
      return;
    }
    case "dispositionChanged": {
      const change = event.mutation.value;
      const commandId = required(change.claimOperationId?.value, "spawn claim command id");
      const command = model.commands.get(commandId);
      if (!command) return;
      if (change.toDisposition === SpawnClaimDisposition.UNSPECIFIED) {
        throw new Error("spawn claim disposition change has unspecified target");
      }
      model.commands.set(commandId, {
        ...command,
        // Claim poison/release is retry-risk evidence, not a CommandState or command failure.
        spawnClaimDisposition: change.toDisposition,
      });
      if (change.toDisposition === SpawnClaimDisposition.TARGET_ABANDONED) {
        const targetId = change.evidence.case === "targetAbandonment"
          ? required(change.evidence.value.logicalTargetId?.value, "abandoned logical target id")
          : undefined;
        if (!targetId || (command.spawnLogicalTargetId && command.spawnLogicalTargetId !== targetId)) {
          throw new Error("target-abandoned claim has mismatched typed evidence");
        }
        model.sessions = new Map(
          [...model.sessions].map(([key, session]) => [key, session.logicalTargetId === targetId
            ? {
                ...session,
                connectivity: SessionConnectivityState.STALE,
                activity: SessionActivityState.UNKNOWN,
                activityDetail: undefined,
                activityDetailProvenance: undefined,
                tombstoned: true,
                needsYou: false,
                lastLsn: lsn,
              }
            : session]),
        );
      }
      return;
    }
    case undefined:
      throw new Error("spawn claim event is missing mutation");
  }
}

function foldSpawnPromotion(
  model: PresentationModel,
  promotion: SpawnPromotionCommitted,
  lsn: bigint,
  publishSession = true,
): void {
  const accepted = promotion.acceptedClaim;
  const operation = accepted?.acceptedOperation?.operation;
  const claim = accepted?.claim;
  const staged = promotion.stagedSuccessor?.staged;
  const report = staged?.report;
  const promoted = promotion.promotedRuntime;
  if (!operation || !claim || !report || !promoted?.externalRuntime) {
    throw new Error("spawn promotion is missing accepted, staged, or promoted evidence");
  }
  const continuationContextStatus = staged.continuationContextStatus;
  if (claim.expectedPrior) {
    if (continuationContextStatus === ContinuationContextStatus.UNSPECIFIED
        || !Object.values(ContinuationContextStatus).includes(continuationContextStatus)) {
      throw new Error("continuation promotion has invalid adapter context status");
    }
  } else if (continuationContextStatus !== ContinuationContextStatus.UNSPECIFIED) {
    throw new Error("fresh promotion carries continuation-only context status");
  }
  const commandId = required(operation.commandId?.value, "promoted spawn command id");
  if (!model.commands.has(commandId)) foldOperation(model, operation, lsn);
  const currentCommand = model.commands.get(commandId)!;
  if (!isTerminalCommand(currentCommand.state)) {
    model.commands.set(commandId, {
      ...currentCommand,
      state: OperationState.COMPLETED,
      lsn,
      failureCode: undefined,
      spawnClaimDisposition: SpawnClaimDisposition.PROMOTED,
      continuationContextStatus: claim.expectedPrior ? continuationContextStatus : undefined,
      pendingControlRequest: undefined,
      history: [
        ...currentCommand.history,
        { state: OperationState.COMPLETED, lsn },
      ],
    });
  }

  const logicalTargetId = required(promoted.logicalTargetId?.value, "promoted logical target id");
  if (claim.logicalTargetId?.value !== logicalTargetId
      || staged.classifiedTarget?.logicalTargetId?.value !== logicalTargetId) {
    throw new Error("spawn promotion logical target evidence disagrees");
  }
  const candidate = sessionFromPromotedReport(report, logicalTargetId, lsn, model.lockdown.active);
  if (candidate.identity.adapterId !== promoted.externalRuntime.adapterId?.value
      || candidate.identity.deploymentScope !== promoted.externalRuntime.deploymentScope
      || candidate.identity.runtimeSessionId !== promoted.externalRuntime.runtimeSessionId?.value
      || candidate.identity.generation !== promoted.externalRuntime.generation?.value) {
    throw new Error("spawn promotion report and promoted runtime disagree");
  }
  const candidateKey = sessionKey(candidate.identity);
  if (publishSession) {
    if (claim.expectedPrior) {
      const priorIdentity = identityFromRuntimeRef(claim.expectedPrior);
      const oldKey = sessionKey(priorIdentity);
      const old = model.sessions.get(oldKey);
      if (old) {
        model.sessions.set(oldKey, {
          ...old,
          connectivity: SessionConnectivityState.STALE,
          activity: SessionActivityState.UNKNOWN,
          activityDetail: undefined,
          activityDetailProvenance: undefined,
          tombstoned: true,
          needsYou: false,
          lastLsn: lsn,
        });
      }
    }
    model.sessions.set(candidateKey, candidate);
  } else {
    const snapshotted = model.sessions.get(candidateKey);
    if (snapshotted) {
      model.sessions.set(candidateKey, { ...snapshotted, logicalTargetId });
    }
  }
  let completed = model.commands.get(commandId)!;
  if (completed.spawnClaimDisposition !== SpawnClaimDisposition.PROMOTED) {
    completed = {
      ...completed,
      spawnClaimDisposition: SpawnClaimDisposition.PROMOTED,
    };
    model.commands.set(commandId, completed);
  }
  if (!completed.target) {
    model.commands.set(commandId, {
      ...completed,
      target: { kind: "runtime-session", identity: candidate.identity },
    });
  }
}

function sessionFromPromotedReport(
  report: SessionReport,
  logicalTargetId: string,
  lsn: bigint,
  lockdownActive: boolean,
): SessionView {
  const identity = identityFromParts(report);
  return {
    identity,
    logicalTargetId,
    label: labels(report),
    model: report.model || undefined,
    connectivity: lockdownActive
      ? SessionConnectivityState.STALE
      : normalizeConnectivity(report.connectivity),
    activity: lockdownActive
      ? SessionActivityState.UNKNOWN
      : normalizeActivity(report.activity),
    needsYou: false,
    lastLsn: lsn,
    tombstoned: false,
    reconciled: true,
    lockdownActive,
  };
}

function identityFromRuntimeRef(runtime: NonNullable<SpawnPromotionCommitted["promotedRuntime"]>): SessionIdentity {
  const external = runtime.externalRuntime;
  if (!external) throw new Error("runtime generation reference is missing external runtime");
  return {
    adapterId: required(external.adapterId?.value, "runtime adapter id"),
    deploymentScope: required(external.deploymentScope, "runtime deployment scope"),
    runtimeSessionId: required(external.runtimeSessionId?.value, "runtime session id"),
    generation: requiredBigint(external.generation?.value, "runtime generation"),
  };
}

function foldCommandTransition(
  model: PresentationModel,
  transition: ReturnType<typeof decodeCommandTransition>,
  lsn: bigint,
): void {
  const id = required(transition.commandId?.value, "command transition id");
  const current = model.commands.get(id);
  if (!current) throw new Error(`command transition references unknown command ${id}`);
  if (isTerminalCommand(current.state)) return;
  if (transition.fromState !== current.state) {
    throw new Error(`command transition ${id} expected ${current.state}, got ${transition.fromState}`);
  }
  if (transition.toState === OperationState.UNSPECIFIED) {
    throw new Error(`command transition ${id} has unspecified target state`);
  }

  const terminal = isTerminalCommand(transition.toState);
  const race = terminal && current.pendingControlRequest && !current.race
    ? `${terminalStateLabel(transition.toState)} after ${controlRequestLabel(current.pendingControlRequest.kind)} requested`
    : current.race;
  const updated: CommandView = {
    ...current,
    state: transition.toState,
    lsn,
    failureCode: transition.failureCode || undefined,
    race,
    pendingControlRequest: terminal ? undefined : current.pendingControlRequest,
    history: [
      ...current.history,
      {
        state: transition.toState,
        lsn,
        failureCode: transition.failureCode || undefined,
      },
    ],
  };
  model.commands.set(id, updated);
  if (transition.toState === OperationState.COMPLETED) applyCompletedResponse(model, updated);
}

function decodeCommandTransition(payload: Uint8Array) {
  return fromBinary(CommandTransitionSchema, payload);
}

function foldElicitation(model: PresentationModel, elicitation: Elicitation, lsn: bigint): void {
  const id = required(elicitation.elicitationId?.value, "elicitation id");
  const contract = elicitation.responseContract;
  if (!contract) throw new Error(`elicitation ${id} is missing response contract`);
  const kind = contractKind(contract.contractKind);
  const existing = model.elicitations.get(id);
  if (existing && isTerminalElicitation(existing.state)) return;
  if (elicitation.state === ElicitationState.UNSPECIFIED) {
    throw new Error(`elicitation ${id} has unspecified state`);
  }
  model.elicitations.set(id, {
    id,
    kind,
    state: elicitation.state,
    contract,
    prompt: elicitationPrompt(elicitation),
    target: runtimeSessionFromScope(elicitation.targetContext),
    groupingKey: elicitationGroupingKey(elicitation) ?? existing?.groupingKey,
    lsn,
    answer: existing?.answer,
  });
}

function foldObservation(model: PresentationModel, observation: Observation, lsn: bigint): void {
  if (observation.payload?.schemaRef === "patchbay.AdapterDiagnosticPayload") {
    foldAdapterDiagnosticObservation(model, observation, lsn);
    return;
  }
  const resourceObservation = decodeTokenCommuneResourceObservation(observation);
  if (resourceObservation) {
    model.resourceObservations.push({
      ...resourceObservation,
      occurredAt: new Date(resourceObservation.occurredAt),
      lsn,
    });
    model.resourceObservations.sort((left, right) => left.lsn < right.lsn ? -1 : left.lsn > right.lsn ? 1 : 0);
    if (model.resourceObservations.length > 100) model.resourceObservations.splice(0, model.resourceObservations.length - 100);
    return;
  }
  const piContinuityId = piPersistedProjectionContinuityId(observation);
  if (piContinuityId) {
    const projection = foldPiPersistedProjectionObservation(
      model.piPersistedProjections.get(piContinuityId),
      observation,
    );
    if (!projection) throw new Error("Pi projection continuity decoded without a fold");
    model.piPersistedProjections.set(piContinuityId, projection.state);
    const removed = new Set(projection.removedMembershipIds);
    if (removed.size > 0) {
      model.observations = model.observations.filter(
        (item) => !item.piProjection || !removed.has(item.piProjection.membershipId),
      );
    }
    const target = runtimeSessionFromScope(observation.targetScope);
    const replacingMessageIds = new Set(
      projection.addedItems
        .map((item) => item.transcriptEvent.messageId)
        .filter((messageId): messageId is string => Boolean(messageId)),
    );
    if (target && replacingMessageIds.size > 0) {
      const targetKey = sessionKey(target);
      model.observations = model.observations.filter((item) => !(
        !item.piProjection &&
        item.messageId &&
        replacingMessageIds.has(item.messageId) &&
        item.session &&
        sessionKey(item.session) === targetKey
      ));
    }
    for (const item of projection.addedItems) {
      foldTranscriptObservation(model, item.transcriptEvent, target, lsn, undefined, {
        continuityId: piContinuityId,
        membershipId: item.membershipId,
      });
    }
    return;
  }

  const commandId = exactCommandCorrelation(observation.correlations);
  deriveTerminalRace(model, observation, commandId, lsn);

  const target = runtimeSessionFromScope(observation.targetScope);
  const transcript = decodeTranscriptEvent(observation);
  if (!transcript) return;

  const key = target && sessionKey(target);
  const session = key ? model.sessions.get(key) : undefined;
  if (session) {
    const detail = activityDetail(transcript);
    model.sessions.set(key!, {
      ...session,
      activityDetail: detail?.text,
      activityDetailProvenance: detail?.provenance,
      lastLsn: lsn,
    });
  }

  foldTranscriptObservation(model, transcript, target, lsn, commandId);
}

function foldSecurityLockdown(
  model: PresentationModel,
  event: ReturnType<typeof decodeSecurityLockdown>,
  lsn: bigint,
): void {
  switch (event.transition.case) {
    case "entered": {
      model.lockdown = {
        active: true,
        submitting: false,
        reasonCode: event.transition.value.reasonCode || undefined,
        enteredAt: timestampDate(event.transition.value.occurredAt),
        enteredEventLsn: lsn,
      };
      model.sessions = new Map(
        [...model.sessions].map(([key, session]) => [key, {
          ...session,
          connectivity: SessionConnectivityState.STALE,
          activity: SessionActivityState.UNKNOWN,
          activityDetail: undefined,
          activityDetailProvenance: undefined,
          lockdownActive: true,
          lastLsn: lsn,
          needsYou: false,
        }]),
      );
      return;
    }
    case "exited":
      // Exit clears only the posture. Existing sessions remain stale until a
      // later authoritative adapter signal arrives.
      model.lockdown = { active: false, submitting: false };
      model.sessions = new Map(
        [...model.sessions].map(([key, session]) => [key, { ...session, lockdownActive: false }]),
      );
      return;
    case undefined:
      throw new Error("security lockdown event is missing transition");
  }
}

function decodeSecurityLockdown(payload: Uint8Array) {
  return fromBinary(SecurityLockdownEventSchema, payload);
}

export function lockdownViewFromState(state: SecurityLockdownState | undefined): LockdownView {
  return {
    active: state?.active ?? false,
    submitting: false,
    reasonCode: state?.reasonCode || undefined,
    enteredAt: timestampDate(state?.enteredAt),
    enteredEventLsn: state?.enteredEventId?.lsn?.value,
  };
}

export function foldResourceState(
  model: PresentationModel,
  event: ResourceStateEvent,
  lsn: bigint,
): void {
  const eventDomain = required(event.authorityDomainId?.value, "resource event authority domain");
  if (!model.authorityDomainId || eventDomain !== model.authorityDomainId) {
    throw new Error("cross-domain resource event rejected by presentation fold");
  }
  const sourceAdapterId = required(event.sourceAdapterId?.value, "resource event source adapter");
  const sourceAdapterGeneration = requiredBigint(
    event.sourceAdapterGeneration?.value,
    "resource event source generation",
  );
  const observedAt = requiredTimestampDate(event.observedAt, "resource event observed time");
  const projectedGeneration = [...model.resourceCollections.values()]
    .filter((collection) => collection.adapterId === sourceAdapterId)
    .reduce(
      (maximum, collection) => collection.sourceAdapterGeneration > maximum
        ? collection.sourceAdapterGeneration
        : maximum,
      0n,
    );
  if (sourceAdapterGeneration < projectedGeneration) {
    throw new Error("resource event lowers projected adapter generation");
  }

  const viewKinds = new Set<string>();
  const validatedViews = event.views.map((view) => {
    const resourceKind = required(view.resourceKind?.value, "resource view update kind");
    if (viewKinds.has(resourceKind)) throw new Error("resource event repeats a view update");
    viewKinds.add(resourceKind);
    return {
      resourceKind,
      completeness: validCompleteness(view.completeness, "resource view completeness"),
    };
  });

  const identities = new Set<string>();
  const validatedMutations = event.mutations.map((mutation) => {
    const identity = resourceIdentityFromWire(mutation.identity, "resource mutation identity");
    if (identity.adapterId !== sourceAdapterId) throw new Error("resource mutation does not match source adapter");
    if (!viewKinds.has(identity.resourceKind)) throw new Error("resource mutation has no matching view update");
    const key = resourceKey(identity);
    if (identities.has(key)) throw new Error("resource event mutates one identity more than once");
    identities.add(key);
    const current = model.resources.get(key);
    const fromRevision = mutation.fromRevisionLsn?.value;
    if ((!current && fromRevision !== undefined) || (current && fromRevision !== current.revisionLsn)) {
      throw new Error("resource mutation prior revision does not match projection");
    }
    if (current && fromRevision === undefined) throw new Error("resource mutation omits prior revision");
    if (!mutation.mutation.case) throw new Error("resource mutation variant is missing");
    return { mutation, identity, key, current };
  });

  for (const { mutation, identity } of validatedMutations) {
    if (mutation.mutation.case !== "tombstone" || !mutation.mutation.value.replacedBy) continue;
    const replacement = resourceIdentityFromWire(
      mutation.mutation.value.replacedBy,
      "resource replacement identity",
    );
    if (replacement.adapterId !== sourceAdapterId || resourceKey(replacement) === resourceKey(identity)) {
      throw new Error("resource tombstone has invalid replacement identity");
    }
    if (!validatedMutations.some((candidate) =>
      resourceKey(candidate.identity) === resourceKey(replacement)
      && candidate.mutation.mutation.case === "upsert")) {
      throw new Error("resource tombstone replacement has no matching upsert");
    }
  }

  for (const view of validatedViews) {
    model.resourceCollections.set(resourceCollectionKey(sourceAdapterId, view.resourceKind), {
      adapterId: sourceAdapterId,
      resourceKind: view.resourceKind,
      completeness: view.completeness,
      sourceAdapterGeneration,
      revisionLsn: lsn,
      observedAt,
      reconciled: true,
    });
  }

  for (const { mutation, identity, key, current } of validatedMutations) {
    switch (mutation.mutation.case) {
      case "upsert": {
        if (current?.tombstoned) throw new Error("resource upsert targets a terminal tombstone");
        const value = mutation.mutation.value;
        validateResourceEnvelope(value.resourcePayload, "resource upsert payload");
        validateResourceEnvelope(value.projectionPayload, "resource upsert projection");
        model.resources.set(key, {
          identity,
          freshness: ResourceFreshnessState.CURRENT,
          sourceAdapterGeneration,
          revisionLsn: lsn,
          observedAt,
          tombstoned: false,
          hasCachedPayload: true,
          reconciled: true,
          projection: decodeResourceProjection(identity, value.resourcePayload, value.projectionPayload),
        });
        break;
      }
      case "unknown":
        if (current?.tombstoned) throw new Error("resource unknown targets a terminal tombstone");
        model.resources.set(key, {
          identity,
          freshness: ResourceFreshnessState.UNKNOWN,
          sourceAdapterGeneration,
          revisionLsn: lsn,
          observedAt,
          tombstoned: false,
          hasCachedPayload: false,
          reconciled: true,
          projection: { status: "unavailable" },
        });
        break;
      case "tombstone": {
        if (!current || current.tombstoned) throw new Error("resource tombstone targets an unknown or retired identity");
        const replacedBy = mutation.mutation.value.replacedBy
          ? resourceIdentityFromWire(mutation.mutation.value.replacedBy, "resource replacement identity")
          : undefined;
        model.resources.set(key, {
          ...current,
          freshness: current.hasCachedPayload
            ? ResourceFreshnessState.STALE
            : ResourceFreshnessState.UNKNOWN,
          sourceAdapterGeneration,
          revisionLsn: lsn,
          observedAt,
          tombstoned: true,
          replacedBy,
          reconciled: true,
        });
        break;
      }
      case "freshnessChanged": {
        if (!current || current.tombstoned) throw new Error("resource freshness change targets an unknown or retired identity");
        const change = mutation.mutation.value;
        const from = validFreshness(change.from, "resource freshness from");
        const to = validFreshness(change.to, "resource freshness to");
        if (from === to || current.freshness !== from) throw new Error("resource freshness transition is invalid");
        if (to !== ResourceFreshnessState.UNKNOWN && !current.hasCachedPayload) {
          throw new Error("resource freshness transition marks an empty payload current or stale");
        }
        model.resources.set(key, {
          ...current,
          freshness: to,
          sourceAdapterGeneration,
          revisionLsn: lsn,
          observedAt,
          hasCachedPayload: to === ResourceFreshnessState.UNKNOWN ? false : current.hasCachedPayload,
          projection: to === ResourceFreshnessState.UNKNOWN ? { status: "unavailable" } : current.projection,
          reconciled: true,
        });
        break;
      }
    }
  }
}

function resourceFromSnapshot(resource: Resource, authorityDomainId: string, snapshotLsn: bigint): ResourceView {
  if (required(resource.authorityDomainId?.value, "resource snapshot record domain") !== authorityDomainId) {
    throw new Error("cross-domain resource snapshot record rejected");
  }
  const identity = resourceIdentityFromWire(resource.identity, "resource snapshot identity");
  const freshness = validFreshness(resource.freshness, "resource snapshot freshness");
  const sourceAdapterGeneration = requiredBigint(
    resource.sourceAdapterGeneration?.value,
    "resource snapshot source generation",
  );
  const revisionLsn = requiredBigint(resource.revisionLsn?.value, "resource snapshot revision");
  if (revisionLsn > snapshotLsn) throw new Error("resource revision exceeds snapshot horizon");
  const observedAt = requiredTimestampDate(resource.observedAt, "resource snapshot observed time");
  const hasResourcePayload = Boolean(resource.resourcePayload);
  const hasProjectionPayload = Boolean(resource.projectionPayload);
  if (hasResourcePayload !== hasProjectionPayload) throw new Error("resource snapshot retains only one payload envelope");
  const hasCachedPayload = hasResourcePayload && hasProjectionPayload;
  if (hasCachedPayload) {
    validateResourceEnvelope(resource.resourcePayload, "resource snapshot payload");
    validateResourceEnvelope(resource.projectionPayload, "resource snapshot projection");
  }
  if (freshness === ResourceFreshnessState.UNKNOWN && hasCachedPayload) {
    throw new Error("unknown resource snapshot retains payload");
  }
  if (freshness !== ResourceFreshnessState.UNKNOWN && !hasCachedPayload) {
    throw new Error("current or stale resource snapshot has no payload");
  }
  if (resource.tombstoned && freshness === ResourceFreshnessState.CURRENT) {
    throw new Error("tombstoned resource snapshot is current");
  }
  const replacedBy = resource.replacedBy
    ? resourceIdentityFromWire(resource.replacedBy, "resource snapshot replacement")
    : undefined;
  if (replacedBy && (!resource.tombstoned || replacedBy.adapterId !== identity.adapterId || resourceKey(replacedBy) === resourceKey(identity))) {
    throw new Error("resource snapshot replacement is invalid");
  }
  return {
    identity,
    freshness,
    sourceAdapterGeneration,
    revisionLsn,
    observedAt,
    tombstoned: resource.tombstoned,
    replacedBy,
    hasCachedPayload,
    reconciled: true,
    projection: hasCachedPayload
      ? decodeResourceProjection(identity, resource.resourcePayload, resource.projectionPayload)
      : { status: "unavailable" },
  };
}

function resourceIdentityFromWire(
  identity: { adapterId?: { value: string }; resourceKind?: { value: string }; resourceId?: { value: string } } | undefined,
  name: string,
): ResourceIdentityView {
  return {
    adapterId: required(identity?.adapterId?.value, `${name} adapter`),
    resourceKind: required(identity?.resourceKind?.value, `${name} kind`),
    resourceId: required(identity?.resourceId?.value, `${name} id`),
  };
}

function validateResourceEnvelope(
  envelope: { schemaRef: string; contentType: PayloadContentType } | undefined,
  name: string,
): void {
  if (!envelope || !envelope.schemaRef) throw new Error(`${name} is missing or incomplete`);
  if (envelope.contentType < PayloadContentType.BINARY || envelope.contentType > PayloadContentType.PROTOBUF) {
    throw new Error(`${name} has unknown content type`);
  }
}

function validCompleteness(value: AdapterSnapshotSupport, name: string): AdapterSnapshotSupport {
  if (value < AdapterSnapshotSupport.AUTHORITATIVE || value > AdapterSnapshotSupport.NONE) {
    throw new Error(`${name} is unknown or unspecified`);
  }
  return value;
}

function validFreshness(value: ResourceFreshnessState, name: string): ResourceFreshnessState {
  if (value < ResourceFreshnessState.CURRENT || value > ResourceFreshnessState.UNKNOWN) {
    throw new Error(`${name} is unknown or unspecified`);
  }
  return value;
}

function requiredTimestampDate(
  timestamp: { seconds: bigint; nanos: number } | undefined,
  name: string,
): Date {
  const result = optionalTimestampDate(timestamp, name);
  if (!result) throw new Error(`${name} is missing`);
  return result;
}

function optionalTimestampDate(
  timestamp: { seconds: bigint; nanos: number } | undefined,
  name: string,
): Date | undefined {
  if (!timestamp) return undefined;
  if (
    timestamp.seconds < -62_135_596_800n
    || timestamp.seconds > 253_402_300_799n
    || timestamp.nanos < 0
    || timestamp.nanos >= 1_000_000_000
  ) {
    throw new Error(`${name} is invalid`);
  }
  return timestampDate(timestamp);
}

function foldSessionState(model: PresentationModel, event: ReturnType<typeof decodeSessionState>, lsn: bigint): void {
  const mutation = event.mutation;
  switch (mutation.case) {
    case "registered": {
      const value = mutation.value;
      const identity = identityFromParts(value);
      const key = sessionKey(identity);
      if (model.sessions.has(key)) return;
      model.sessions.set(key, {
        identity,
        label: labels(value),
        model: value.model || undefined,
        connectivity: model.lockdown.active
          ? SessionConnectivityState.STALE
          : normalizeConnectivity(value.initialState?.connectivity),
        activity: model.lockdown.active
          ? SessionActivityState.UNKNOWN
          : normalizeActivity(value.initialState?.activity),
        needsYou: false,
        lastLsn: lsn,
        tombstoned: false,
        reconciled: true,
        lockdownActive: model.lockdown.active,
      });
      return;
    }
    case "generationBumped": {
      const value = mutation.value;
      const from = identityFromParts({ ...value, sessionGeneration: value.fromGeneration });
      const nextGeneration = requiredBigint(value.toGeneration?.value, "next session generation");
      if (nextGeneration <= from.generation) throw new Error("session generation did not increase");
      const oldKey = sessionKey(from);
      const old = model.sessions.get(oldKey);
      if (!old) throw new Error("session generation bump references unknown generation");
      model.sessions.set(oldKey, {
        ...old,
        connectivity: SessionConnectivityState.STALE,
        activity: SessionActivityState.UNKNOWN,
        activityDetail: undefined,
        activityDetailProvenance: undefined,
        tombstoned: true,
        needsYou: false,
        lastLsn: lsn,
      });
      const identity = { ...from, generation: nextGeneration };
      model.sessions.set(sessionKey(identity), {
        identity,
        label: labels(value),
        model: value.model || undefined,
        connectivity: model.lockdown.active
          ? SessionConnectivityState.STALE
          : normalizeConnectivity(value.initialState?.connectivity),
        activity: model.lockdown.active
          ? SessionActivityState.UNKNOWN
          : normalizeActivity(value.initialState?.activity),
        needsYou: false,
        lastLsn: lsn,
        tombstoned: false,
        reconciled: true,
        lockdownActive: model.lockdown.active,
      });
      return;
    }
    case "connectivityChanged": {
      const value = mutation.value;
      const key = sessionKey(identityFromParts(value));
      const current = model.sessions.get(key);
      if (!current || current.tombstoned) return;
      model.sessions.set(key, {
        ...current,
        connectivity: model.lockdown.active
          ? SessionConnectivityState.STALE
          : normalizeConnectivity(value.to),
        lockdownActive: model.lockdown.active,
        reconciled: true,
        lastLsn: lsn,
      });
      return;
    }
    case "activityChanged": {
      const value = mutation.value;
      const key = sessionKey(identityFromParts(value));
      const current = model.sessions.get(key);
      if (!current || current.tombstoned) return;
      model.sessions.set(key, {
        ...current,
        activity: normalizeActivity(value.to),
        lastLsn: lsn,
      });
      return;
    }
    case "modelChanged": {
      const value = mutation.value;
      const key = sessionKey(identityFromParts(value));
      const current = model.sessions.get(key);
      if (!current || current.tombstoned) return;
      model.sessions.set(key, { ...current, model: value.to || undefined, lastLsn: lsn });
      return;
    }
    case "relabeled": {
      const value = mutation.value;
      const key = sessionKey(identityFromParts(value));
      const current = model.sessions.get(key);
      if (!current || current.tombstoned) return;
      model.sessions.set(key, { ...current, label: labels(value), lastLsn: lsn });
      return;
    }
    case undefined:
      throw new Error("session state event is missing mutation");
  }
}

function decodeSessionState(payload: Uint8Array) {
  return fromBinary(SessionStateEventSchema, payload);
}

function applyCompletedResponse(model: PresentationModel, command: CommandView): void {
  const correlation = command.operation.correlations.find(
    (candidate) => candidate.ref.case === "elicitationId",
  );
  const elicitationId = correlation?.ref.case === "elicitationId" ? correlation.ref.value.value : undefined;
  if (!elicitationId) return;
  const elicitation = model.elicitations.get(elicitationId);
  if (!elicitation || isTerminalElicitation(elicitation.state)) return;
  const envelope = command.operation.payload;
  if (!envelope || envelope.contentType !== PayloadContentType.PROTOBUF) return;

  if (command.operation.kind === OperationKind.APPROVAL_RESPONSE) {
    const payload = fromBinary(ApprovalResponsePayloadSchema, envelope.payload);
    const state =
      payload.decision === ApprovalDecision.DENIED
        ? ElicitationState.DECLINED
        : ElicitationState.ANSWERED;
    model.elicitations.set(elicitationId, {
      ...elicitation,
      state,
      lsn: command.lsn,
      answer: { approvalDecision: payload.decision },
    });
  } else if (command.operation.kind === OperationKind.ELICITATION_RESPONSE) {
    const payload = fromBinary(ElicitationResponsePayloadSchema, envelope.payload);
    model.elicitations.set(elicitationId, {
      ...elicitation,
      state: ElicitationState.ANSWERED,
      lsn: command.lsn,
      answer: {
        selectedOptionId: payload.selectedOptionId || undefined,
        freeText: payload.freeText || undefined,
        clarification: payload.clarification || undefined,
      },
    });
  }
}

function deriveNeedsYou(model: PresentationModel): void {
  const pendingTargets = new Set(
    [...model.elicitations.values()]
      .filter((elicitation) => !isTerminalElicitation(elicitation.state) && elicitation.target)
      .map((elicitation) => sessionKey(elicitation.target!)),
  );
  model.sessions = new Map(
    [...model.sessions].map(([key, session]) => [
      key,
      {
        ...session,
        needsYou:
          session.reconciled &&
          !session.tombstoned &&
          (pendingTargets.has(key) || session.activityDetail === "waiting for command"),
      },
    ]),
  );
}

function sessionFromSnapshot(
  session: Session,
  authorityDomainId: string,
  snapshotLsn: bigint,
): SessionView {
  if (session.authorityDomainId?.value !== authorityDomainId) {
    throw new Error("cross-domain session snapshot record rejected");
  }
  const lastLsn = session.lastAuthoritativeLsn?.value ?? snapshotLsn;
  if (lastLsn > snapshotLsn) {
    throw new Error("session snapshot record exceeds its authority horizon");
  }
  const identity = identityFromParts(session);
  return {
    identity,
    label: labels(session),
    model: session.model || undefined,
    connectivity: session.tombstoned
      ? SessionConnectivityState.STALE
      : normalizeConnectivity(session.state?.connectivity),
    activity: session.tombstoned
      ? SessionActivityState.UNKNOWN
      : normalizeActivity(session.state?.activity),
    needsYou: false,
    lastLsn,
    lastUpdate: timestampDate(session.observedAt),
    tombstoned: session.tombstoned,
    reconciled: true,
    lockdownActive: false,
  };
}

function operationTargetFromOperation(operation: Operation): OperationTargetView | undefined {
  const direct = operationTargetFromScope(operation.targetScope);
  if (direct || operation.kind !== OperationKind.SPAWN) return direct;
  const payload = operation.payload;
  if (!payload
      || payload.contentType !== PayloadContentType.PROTOBUF
      || payload.schemaRef !== "patchbay.SpawnRequest") return undefined;
  try {
    const request = fromBinary(SpawnRequestSchema, payload.payload);
    return request.intent.case === "continuation" && request.intent.value.prior
      ? { kind: "runtime-session", identity: identityFromRuntimeRef(request.intent.value.prior) }
      : undefined;
  } catch {
    return undefined;
  }
}

export function operationTargetFromScope(scope: TargetScope | undefined): OperationTargetView | undefined {
  const session = runtimeSessionFromScope(scope);
  if (session) return { kind: "runtime-session", identity: session };
  if (!scope || scope.kind !== TargetScopeKind.RESOURCE || hasNonResourceTargetFields(scope)) return undefined;
  try {
    return {
      kind: "operational-resource",
      identity: resourceIdentityFromWire(scope.resource, "operation target resource"),
    };
  } catch {
    return undefined;
  }
}

export function runtimeSessionFromScope(target: TargetScope | undefined): SessionIdentity | undefined {
  if (
    !target
    || target.kind !== TargetScopeKind.RUNTIME_SESSION
    || target.resource
    || target.legacyAuditResourceId
    || target.actorId
    || target.projectOrGroup
    || !target.adapterId
    || !target.runtimeSessionId
    || !target.sessionGeneration
  ) return undefined;
  try {
    return {
      adapterId: required(target.adapterId.value, "target adapter id"),
      deploymentScope: required(target.deploymentScope, "target deployment scope"),
      runtimeSessionId: required(target.runtimeSessionId.value, "target runtime session id"),
      generation: requiredBigint(target.sessionGeneration.value, "target session generation"),
    };
  } catch {
    return undefined;
  }
}

function hasNonResourceTargetFields(scope: TargetScope): boolean {
  return Boolean(
    scope.actorId
    || scope.adapterId
    || scope.runtimeSessionId
    || scope.sessionGeneration
    || scope.deploymentScope
    || scope.projectOrGroup
    || scope.legacyAuditResourceId,
  );
}

function identityFromParts(value: {
  adapterId?: { value: string };
  deploymentScope: string;
  runtimeSessionId?: { value: string };
  sessionGeneration?: { value: bigint };
}): SessionIdentity {
  return {
    adapterId: required(value.adapterId?.value, "adapter id"),
    deploymentScope: required(value.deploymentScope, "deployment scope"),
    runtimeSessionId: required(value.runtimeSessionId?.value, "runtime session id"),
    generation: requiredBigint(value.sessionGeneration?.value, "session generation"),
  };
}

function labels(value: { project: string; cwd: string; name: string }): SessionView["label"] {
  return {
    project: value.project || undefined,
    cwd: value.cwd || undefined,
    name: value.name || undefined,
  };
}

function normalizeConnectivity(value: SessionConnectivityState | undefined): SessionConnectivityState {
  return value === undefined || value === SessionConnectivityState.UNSPECIFIED
    ? SessionConnectivityState.UNKNOWN
    : value;
}

function normalizeActivity(value: SessionActivityState | undefined): SessionActivityState {
  return value === undefined || value === SessionActivityState.UNSPECIFIED
    ? SessionActivityState.UNKNOWN
    : value;
}

function contractKind(value: ResponseContractKind): "approval" | "question" {
  if (value === ResponseContractKind.APPROVAL) return "approval";
  if (value === ResponseContractKind.QUESTION) return "question";
  throw new Error(`unsupported response contract kind ${value}`);
}

function elicitationGroupingKey(elicitation: Elicitation): string | undefined {
  const opener = elicitation.opener;
  const actorId = opener?.actorId?.value;
  const correlations = elicitation.correlations
    .map(correlationKey)
    .filter((value): value is string => Boolean(value))
    .sort();
  if (!actorId || correlations.length === 0) return undefined;
  const endpoint = opener?.endpointId?.value ?? "";
  const generation = opener?.endpointGeneration?.value ?? 0n;
  return [actorId, endpoint, generation.toString(), ...correlations]
    .map((part) => `${part.length}:${part}`)
    .join("|");
}

function correlationKey(correlation: Elicitation["correlations"][number]): string | undefined {
  switch (correlation.ref.case) {
    case "commandId": return `command:${correlation.ref.value.value}`;
    case "messageId": return `message:${correlation.ref.value.value}`;
    case "replyId": return `reply:${correlation.ref.value.value}`;
    case "elicitationId": return `elicitation:${correlation.ref.value.value}`;
    case "eventId": {
      const domain = correlation.ref.value.authorityDomainId?.value;
      const lsn = correlation.ref.value.lsn?.value;
      return domain && lsn !== undefined ? `event:${domain}:${lsn}` : undefined;
    }
    case undefined: return undefined;
  }
}

function elicitationPrompt(elicitation: Elicitation): string {
  const envelope = elicitation.payload;
  if (!envelope) return "Response required";
  const text = decodeText(envelope.payload);
  if (envelope.contentType === PayloadContentType.JSON) {
    try {
      const value: unknown = JSON.parse(text);
      if (isRecord(value)) {
        for (const field of ["prompt", "question", "message"]) {
          if (typeof value[field] === "string" && value[field]) return value[field];
        }
      }
    } catch {
      return "Response required";
    }
  }
  return text || "Response required";
}

type TranscriptRecord = Record<string, unknown> & { kind: string };

function decodeTranscriptEvent(observation: Observation): TranscriptRecord | undefined {
  const envelope = observation.payload;
  if (
    !envelope ||
    envelope.contentType !== PayloadContentType.JSON ||
    envelope.schemaRef !== "patchbay.pi.TranscriptEvent.v1"
  ) {
    return undefined;
  }
  const value: unknown = JSON.parse(decodeText(envelope.payload));
  if (!isRecord(value) || typeof value.kind !== "string") {
    throw new Error("Pi transcript observation is malformed");
  }
  return value as TranscriptRecord;
}

function activityDetail(
  event: TranscriptRecord,
): { text: string; provenance: "runtime" | "tool" } | undefined {
  switch (event.kind) {
    case "turn_started":
      return { text: "thinking", provenance: "runtime" };
    case "assistant_delta":
      return { text: "responding", provenance: "runtime" };
    case "assistant_committed":
      return { text: "finishing response", provenance: "runtime" };
    case "tool_requested":
      return {
        text: typeof event.tool === "string" ? `using ${event.tool}` : "using tool",
        provenance: "tool",
      };
    case "tool_finished":
      return { text: event.error ? "tool failed" : "processing tool result", provenance: "tool" };
    case "turn_finished":
      return { text: "waiting for command", provenance: "runtime" };
    case "provider_error":
      return { text: "provider error", provenance: "runtime" };
    default:
      return undefined;
  }
}

function foldTranscriptObservation(
  model: PresentationModel,
  event: TranscriptRecord,
  session: SessionIdentity | undefined,
  lsn: bigint,
  commandId: string | undefined,
  piProjection?: ObservationView["piProjection"],
): void {
  const messageId = typeof event.messageId === "string" ? event.messageId : undefined;
  const matchingIndex = (): number => model.observations.findIndex((item) =>
    piProjection
      ? item.piProjection?.membershipId === piProjection.membershipId
      : item.messageId === messageId,
  );
  if (event.kind === "assistant_delta" && messageId && typeof event.delta === "string") {
    const index = matchingIndex();
    if (index >= 0) {
      const current = model.observations[index]!;
      model.observations[index] = {
        ...current,
        markdown: current.markdown + event.delta,
        lsn,
        commandId: commandId ?? current.commandId,
        ...(piProjection ? { piProjection } : {}),
      };
    } else {
      model.observations.push({
        id: piProjection?.membershipId ?? messageId,
        messageId,
        session,
        role: "agent",
        kind: event.kind,
        markdown: event.delta,
        lsn,
        commandId,
        ...(piProjection ? { piProjection } : {}),
      });
    }
    return;
  }
  if (event.kind === "assistant_committed" && messageId && typeof event.text === "string") {
    const index = matchingIndex();
    const next: ObservationView = {
      id: piProjection?.membershipId ?? messageId,
      messageId,
      session,
      role: "agent",
      kind: event.kind,
      markdown: event.text,
      lsn,
      commandId,
      ...(piProjection ? { piProjection } : {}),
    };
    if (index >= 0) model.observations[index] = next;
    else model.observations.push(next);
    return;
  }
  if (event.kind === "user_confirmed" && messageId && typeof event.text === "string") {
    model.observations.push({
      id: piProjection?.membershipId ?? messageId,
      messageId,
      session,
      role: "operator",
      kind: event.kind,
      markdown: event.text,
      lsn,
      commandId,
      ...(piProjection ? { piProjection } : {}),
    });
    return;
  }
  if (event.kind === "tool_requested" || event.kind === "tool_finished") {
    const baseId = typeof event.toolCallId === "string" ? event.toolCallId : `tool-${lsn}`;
    const id = piProjection?.membershipId
      ?? (event.kind === "tool_finished" ? `${baseId}:finished` : baseId);
    const tool = typeof event.tool === "string" ? event.tool : "tool";
    const body = event.kind === "tool_requested" ? `Running **${tool}**` : event.error ? `**${tool}** failed: ${event.error}` : `**${tool}** finished`;
    const detail =
      event.kind === "tool_requested"
        ? toolPreview(event.args)
        : toolPreview(event.error ?? event.result);
    model.observations.push({
      id,
      session,
      role: "tool",
      kind: event.kind,
      markdown: body,
      lsn,
      commandId,
      ...(detail ? { detail } : {}),
      ...(piProjection ? { piProjection } : {}),
    });
  }
}

function exactCommandCorrelation(
  correlations: readonly Observation["correlations"][number][],
): string | undefined {
  let commandId: string | undefined;
  for (const correlation of correlations) {
    if (correlation.ref.case !== "commandId") continue;
    const candidate = correlation.ref.value.value;
    if (!candidate || (commandId !== undefined && candidate !== commandId)) return undefined;
    commandId = candidate;
  }
  return commandId;
}

function deriveTerminalRace(
  model: PresentationModel,
  observation: Observation,
  commandId: string | undefined,
  lsn: bigint,
): void {
  if (!commandId || observation.kind !== ObservationKind.RESULT) return;
  const command = model.commands.get(commandId);
  if (!command || !isTerminalCommand(command.state) || command.race || lsn <= command.lsn) return;

  const candidate = observationTerminalCandidate(observation.failureCode);
  if (candidate === undefined || candidate === command.state) return;

  let race: string;
  if (candidate === OperationState.CANCELLED) {
    race = `${terminalStateLabel(command.state)} before cancellation arrived`;
  } else if (candidate === OperationState.COMPLETED) {
    race = command.state === OperationState.EXPIRED
      ? "Expired before adapter completion"
      : `${terminalStateLabel(command.state)} before completion arrived`;
  } else {
    race = `${terminalStateLabel(command.state)} before later ${terminalStateLabel(candidate).toLocaleLowerCase()}`;
  }
  model.commands.set(commandId, { ...command, race });
}

function controlRequestLabel(kind: PendingControlRequest["kind"]): "cancellation" | "interrupt" {
  return kind === OperationKind.CANCEL ? "cancellation" : "interrupt";
}

function observationTerminalCandidate(failureCode: FailureCode): OperationState | undefined {
  switch (failureCode) {
    case FailureCode.UNSPECIFIED:
      return OperationState.COMPLETED;
    case FailureCode.UNSUPPORTED_COMMAND:
    case FailureCode.DELIVERY_REJECTED:
      return OperationState.REJECTED;
    case FailureCode.EXPIRED:
      return OperationState.EXPIRED;
    case FailureCode.CANCELLED:
      return OperationState.CANCELLED;
    case FailureCode.SUPERSEDED:
      return OperationState.SUPERSEDED;
    case FailureCode.STALE_EVENT:
      return undefined;
    default:
      return OperationState.FAILED;
  }
}

function terminalStateLabel(state: OperationState): string {
  switch (state) {
    case OperationState.COMPLETED: return "Completed";
    case OperationState.REJECTED: return "Rejected";
    case OperationState.FAILED: return "Failed";
    case OperationState.EXPIRED: return "Expired";
    case OperationState.CANCELLED: return "Cancelled";
    case OperationState.SUPERSEDED: return "Superseded";
    default: return "Terminal outcome committed";
  }
}

/** Ordered arg keys that name what a call is doing (bash command, read path…). */
const TOOL_PREVIEW_KEYS = ["command", "path", "filePath", "file", "query", "pattern", "url", "prompt"];
const TOOL_PREVIEW_LIMIT = 240;

/**
 * Compact plain-text preview of a tool call's args (or result/error). Rendered
 * as text, never markdown — tool args are untrusted content.
 */
function toolPreview(value: unknown): string | undefined {
  let text: string | undefined;
  if (typeof value === "string" && value) {
    text = value;
  } else if (isRecord(value)) {
    for (const key of TOOL_PREVIEW_KEYS) {
      const candidate = value[key];
      if (typeof candidate === "string" && candidate) {
        text = candidate;
        break;
      }
    }
    text ??= Object.keys(value).length > 0 ? JSON.stringify(value) : undefined;
  } else if (value !== undefined && value !== null) {
    text = JSON.stringify(value);
  }
  if (!text) return undefined;
  return text.length > TOOL_PREVIEW_LIMIT ? `${text.slice(0, TOOL_PREVIEW_LIMIT - 1)}…` : text;
}

function cloneModel(model: PresentationModel): PresentationModel {
  return {
    ...model,
    sessions: new Map(model.sessions),
    resources: new Map(model.resources),
    resourceCollections: new Map(model.resourceCollections),
    commands: new Map(model.commands),
    elicitations: new Map(model.elicitations),
    adapters: new Map(model.adapters),
    observations: [...model.observations],
    piPersistedProjections: new Map(model.piPersistedProjections),
    resourceObservations: [...model.resourceObservations],
  };
}

function isTerminalCommand(state: OperationState): boolean {
  return state >= OperationState.COMPLETED;
}

export function isTerminalElicitation(state: ElicitationState): boolean {
  return state !== ElicitationState.UNSPECIFIED && state !== ElicitationState.OPENED && state !== ElicitationState.PENDING;
}

function timestampDate(timestamp: { seconds: bigint; nanos: number } | undefined): Date | undefined {
  if (!timestamp) return undefined;
  return new Date(Number(timestamp.seconds) * 1_000 + Math.floor(timestamp.nanos / 1_000_000));
}

function decodeText(bytes: Uint8Array): string {
  return new TextDecoder().decode(bytes);
}

function required(value: string | undefined, name: string): string {
  if (!value) throw new Error(`${name} is missing`);
  return value;
}

function requiredBigint(value: bigint | undefined, name: string): bigint {
  if (value === undefined || value < 0n) throw new Error(`${name} is missing or invalid`);
  return value;
}

function positiveBigint(value: bigint | undefined, name: string): bigint {
  const parsed = requiredBigint(value, name);
  if (parsed === 0n) throw new Error(`${name} must be positive`);
  return parsed;
}

/** Cached surface state is replaceable only by a strictly newer prefix of the
 * same persisted core continuity. Streams, process handles, and wall clocks do
 * not participate in this comparison. */
function assertNewerCoreAuthority(
  current: PresentationModel,
  replacement: PresentationModel,
): void {
  if (!current.authorityDomainId) return;
  if (replacement.authorityDomainId !== current.authorityDomainId) {
    throw new Error("cross-domain snapshot replacement rejected");
  }
  if (current.coreGeneration === undefined) {
    throw new Error("cached core authority has no storage-lineage anchor");
  }
  if (replacement.coreGeneration !== current.coreGeneration) {
    throw new Error("cross-generation snapshot replacement rejected");
  }
  if (replacement.cursor <= current.cursor) {
    throw new Error("snapshot replacement is not newer than cached core authority");
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
