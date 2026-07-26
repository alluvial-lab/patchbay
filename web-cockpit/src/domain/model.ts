import { fromBinary } from "@bufbuild/protobuf";
import {
  ApprovalDecision,
  ApprovalResponsePayloadSchema,
  CommandTransitionSchema,
  ElicitationResponsePayloadSchema,
  ElicitationSchema,
  ElicitationState,
  ObservationSchema,
  OperationKind,
  OperationSchema,
  OperationState,
  PayloadContentType,
  ResponseContractKind,
  SessionActivityState,
  SessionConnectivityState,
  SessionStateEventSchema,
  StoredEventKind,
  type Elicitation,
  type FailureCode,
  type Observation,
  type Operation,
  type ResponseContract,
  type Session,
  type SessionSnapshot,
  type SubscribeEvent,
  type TargetScope,
  type AdapterStatus,
  type AdapterDiagnosticSeverity,
} from "@patchbay/contracts";

import type { ReconcileProjection } from "./reconcile.js";
import { foldAdapterDiagnosticObservation } from "./adapter-diagnostics.js";

export interface SessionIdentity {
  adapterId: string;
  deploymentScope: string;
  runtimeSessionId: string;
  generation: bigint;
}

export interface SessionView {
  identity: SessionIdentity;
  label: { project?: string; cwd?: string; name?: string };
  model?: string;
  connectivity: SessionConnectivityState;
  activity: SessionActivityState;
  activityDetail?: string;
  needsYou: boolean;
  lastLsn: bigint;
  lastUpdate?: Date;
  tombstoned: boolean;
  reconciled: boolean;
}

export interface CommandHistoryEntry {
  state: OperationState;
  lsn: bigint;
  failureCode?: FailureCode;
  race?: string;
}

export interface CommandView {
  id: string;
  state: OperationState;
  lsn: bigint;
  failureCode?: FailureCode;
  race?: string;
  target?: SessionIdentity;
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

export interface ObservationView {
  id: string;
  session?: SessionIdentity;
  role: "operator" | "agent" | "tool" | "system";
  kind: string;
  markdown: string;
  lsn: bigint;
  messageId?: string;
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

export interface AdapterView {
  adapterId: string;
  status?: AdapterStatus;
  asOfLsn: bigint;
  recentDiagnostics: readonly AdapterDiagnosticView[];
}

export interface PresentationModel {
  authorityDomainId?: string;
  cursor: bigint;
  reconciled: boolean;
  sessions: Map<string, SessionView>;
  commands: Map<string, CommandView>;
  elicitations: Map<string, ElicitationView>;
  adapters: Map<string, AdapterView>;
  observations: ObservationView[];
}

export function emptyPresentationModel(): PresentationModel {
  return {
    cursor: 0n,
    reconciled: false,
    sessions: new Map(),
    commands: new Map(),
    elicitations: new Map(),
    adapters: new Map(),
    observations: [],
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

/**
 * Rebuilds the projection atomically. SessionSnapshot owns the session baseline;
 * the durable prefix owns commands, Observations, and Elicitations because the
 * snapshot wire shape does not contain those axes.
 */
export function replaceFromSnapshot(
  snapshot: SessionSnapshot,
  replayEvents: readonly SubscribeEvent[],
): PresentationModel {
  const authorityDomainId = required(snapshot.authorityDomainId?.value, "snapshot authority domain");
  const snapshotLsn = requiredBigint(snapshot.snapshotLsn?.value, "snapshot LSN");
  const sessions = new Map<string, SessionView>();
  for (const session of snapshot.sessions) {
    const view = sessionFromSnapshot(session, snapshotLsn);
    sessions.set(sessionKey(view.identity), { ...view, reconciled: false });
  }
  let model: PresentationModel = {
    authorityDomainId,
    cursor: 0n,
    reconciled: false,
    sessions,
    commands: new Map(),
    elicitations: new Map(),
    adapters: new Map(),
    observations: [],
  };

  let replayCursor = 0n;
  for (const event of replayEvents) {
    const lsn = requiredBigint(event.eventId?.lsn?.value, "replay event LSN");
    const eventDomain = required(event.eventId?.authorityDomainId?.value, "replay event authority domain");
    if (eventDomain !== authorityDomainId) throw new Error("cross-domain replay event rejected");
    if (lsn <= replayCursor || lsn > snapshotLsn) {
      throw new Error(`snapshot replay is not a strictly ordered visible prefix at LSN ${lsn}`);
    }
    replayCursor = lsn;
    if (!event.payload) throw new Error("snapshot replay event payload is missing");

    if (event.payload.kind === StoredEventKind.SESSION_STATE) {
      // The snapshot is the newer authority for sessions at snapshot_lsn.
      model = { ...model, cursor: lsn };
    } else {
      model = fold(model, event);
      model.reconciled = false;
      model.sessions = new Map(
        [...model.sessions].map(([key, session]) => [key, { ...session, reconciled: false }]),
      );
    }
  }

  // Invisible authority records can trail the final operator-facing event.
  // The snapshot is authoritative through snapshot_lsn even when the visible
  // replay cursor ends earlier.
  model.cursor = snapshotLsn;
  model.reconciled = true;
  model.sessions = new Map(
    [...model.sessions].map(([key, session]) => [key, { ...session, reconciled: true }]),
  );
  deriveNeedsYou(model);
  return model;
}

/** Marks cached axes honestly while a stream/snapshot gap is unresolved. */
export function markUnreconciled(model: PresentationModel): PresentationModel {
  const next = cloneModel(model);
  next.reconciled = false;
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
        needsYou: false,
        reconciled: false,
      },
    ]),
  );
  return next;
}

export class PresentationProjection implements ReconcileProjection {
  constructor(public model: PresentationModel = emptyPresentationModel()) {}

  markUnreconciled(): void {
    this.model = markUnreconciled(this.model);
  }

  replaceFromSnapshot(snapshot: SessionSnapshot, replayEvents: readonly SubscribeEvent[]): void {
    this.model = replaceFromSnapshot(snapshot, replayEvents);
  }

  foldEvent(event: SubscribeEvent): void {
    this.model = fold(this.model, event);
  }
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

/** The view-binding dominance rule: only a reconciled, non-tombstoned LIVE axis is live. */
export function rendersLive(session: SessionView): boolean {
  return stableTarget(session) && session.connectivity === SessionConnectivityState.LIVE;
}

function foldOperation(model: PresentationModel, operation: Operation, lsn: bigint): void {
  const id = required(operation.commandId?.value, "operation command id");
  if (!model.commands.has(id)) {
    const target = identityFromTarget(operation.targetScope);
    model.commands.set(id, {
      id,
      state: OperationState.ACCEPTED,
      lsn,
      target,
      operation,
      history: [{ state: OperationState.ACCEPTED, lsn }],
    });
  }
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

  const updated: CommandView = {
    ...current,
    state: transition.toState,
    lsn,
    failureCode: transition.failureCode || undefined,
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
    target: identityFromTarget(elicitation.targetContext),
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
  const target = identityFromTarget(observation.targetScope);
  const transcript = decodeTranscriptEvent(observation);
  if (!transcript) return;

  const key = target && sessionKey(target);
  const session = key ? model.sessions.get(key) : undefined;
  if (session) {
    const detail = activityDetail(transcript);
    model.sessions.set(key!, { ...session, activityDetail: detail, lastLsn: lsn });
  }

  foldTranscriptObservation(model, transcript, target, lsn);
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
        connectivity: normalizeConnectivity(value.initialState?.connectivity),
        activity: normalizeActivity(value.initialState?.activity),
        needsYou: false,
        lastLsn: lsn,
        tombstoned: false,
        reconciled: true,
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
        tombstoned: true,
        needsYou: false,
        lastLsn: lsn,
      });
      const identity = { ...from, generation: nextGeneration };
      model.sessions.set(sessionKey(identity), {
        identity,
        label: labels(value),
        model: value.model || undefined,
        connectivity: normalizeConnectivity(value.initialState?.connectivity),
        activity: normalizeActivity(value.initialState?.activity),
        needsYou: false,
        lastLsn: lsn,
        tombstoned: false,
        reconciled: true,
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
        connectivity: normalizeConnectivity(value.to),
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

function sessionFromSnapshot(session: Session, snapshotLsn: bigint): SessionView {
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
    lastLsn: session.lastAuthoritativeLsn?.value ?? snapshotLsn,
    lastUpdate: timestampDate(session.observedAt),
    tombstoned: session.tombstoned,
    reconciled: true,
  };
}

function identityFromTarget(target: TargetScope | undefined): SessionIdentity | undefined {
  if (!target?.adapterId || !target.runtimeSessionId || !target.sessionGeneration) return undefined;
  return {
    adapterId: required(target.adapterId.value, "target adapter id"),
    deploymentScope: required(target.deploymentScope, "target deployment scope"),
    runtimeSessionId: required(target.runtimeSessionId.value, "target runtime session id"),
    generation: requiredBigint(target.sessionGeneration.value, "target session generation"),
  };
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

function activityDetail(event: TranscriptRecord): string | undefined {
  switch (event.kind) {
    case "turn_started":
      return "thinking";
    case "assistant_delta":
      return "responding";
    case "assistant_committed":
      return "finishing response";
    case "tool_requested":
      return typeof event.tool === "string" ? `using ${event.tool}` : "using tool";
    case "tool_finished":
      return event.error ? "tool failed" : "processing tool result";
    case "turn_finished":
      return "waiting for command";
    case "provider_error":
      return "provider error";
    default:
      return undefined;
  }
}

function foldTranscriptObservation(
  model: PresentationModel,
  event: TranscriptRecord,
  session: SessionIdentity | undefined,
  lsn: bigint,
): void {
  const messageId = typeof event.messageId === "string" ? event.messageId : undefined;
  if (event.kind === "assistant_delta" && messageId && typeof event.delta === "string") {
    const index = model.observations.findIndex((item) => item.messageId === messageId);
    if (index >= 0) {
      const current = model.observations[index]!;
      model.observations[index] = { ...current, markdown: current.markdown + event.delta, lsn };
    } else {
      model.observations.push({
        id: messageId,
        messageId,
        session,
        role: "agent",
        kind: event.kind,
        markdown: event.delta,
        lsn,
      });
    }
    return;
  }
  if (event.kind === "assistant_committed" && messageId && typeof event.text === "string") {
    const index = model.observations.findIndex((item) => item.messageId === messageId);
    const next: ObservationView = {
      id: messageId,
      messageId,
      session,
      role: "agent",
      kind: event.kind,
      markdown: event.text,
      lsn,
    };
    if (index >= 0) model.observations[index] = next;
    else model.observations.push(next);
    return;
  }
  if (event.kind === "user_confirmed" && messageId && typeof event.text === "string") {
    model.observations.push({
      id: messageId,
      messageId,
      session,
      role: "operator",
      kind: event.kind,
      markdown: event.text,
      lsn,
    });
    return;
  }
  if (event.kind === "tool_requested" || event.kind === "tool_finished") {
    const id = typeof event.toolCallId === "string" ? event.toolCallId : `tool-${lsn}`;
    const tool = typeof event.tool === "string" ? event.tool : "tool";
    const body = event.kind === "tool_requested" ? `Running **${tool}**` : event.error ? `**${tool}** failed: ${event.error}` : `**${tool}** finished`;
    model.observations.push({ id, session, role: "tool", kind: event.kind, markdown: body, lsn });
  }
}

function cloneModel(model: PresentationModel): PresentationModel {
  return {
    ...model,
    sessions: new Map(model.sessions),
    commands: new Map(model.commands),
    elicitations: new Map(model.elicitations),
    adapters: new Map(model.adapters),
    observations: [...model.observations],
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

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
