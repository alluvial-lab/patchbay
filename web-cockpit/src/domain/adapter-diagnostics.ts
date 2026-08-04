import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import {
  AdapterDiagnosticPayloadSchema,
  AdapterDiagnosticSeverity,
  AdapterDiagnosticState,
  AdapterIdSchema,
  AdapterStatusQuerySchema,
  AuthorityDomainIdSchema,
  DiagnosticsQuerySchema,
  FailureCode,
  OperationKind,
  OperationSchema,
  OperationState,
  PayloadContentType,
  PayloadEnvelopeSchema,
  SubmissionOutcome,
  TargetScopeKind,
  TargetScopeSchema,
  type AuthorityDomainId,
  type AuditRecord,
  type Operation,
  type QueryDiagnosticsResponse,
} from "@patchbay/contracts";

import {
  runtimeSessionFromScope,
  type AdapterDiagnosticView,
  type AdapterView,
  type PresentationModel,
  type SessionIdentity,
} from "./model.js";

export function adapterConnectionPresentation(
  state: AdapterDiagnosticState,
): {
  connectivity: "live" | "offline" | "failed" | "unknown";
  label: "attached" | "detached" | "failed" | "unknown";
} {
  switch (state) {
    case AdapterDiagnosticState.ATTACHED: return { connectivity: "live", label: "attached" };
    case AdapterDiagnosticState.DETACHED: return { connectivity: "offline", label: "detached" };
    case AdapterDiagnosticState.FAILED: return { connectivity: "failed", label: "failed" };
    case AdapterDiagnosticState.UNKNOWN:
    case AdapterDiagnosticState.UNSPECIFIED:
    default: return { connectivity: "unknown", label: "unknown" };
  }
}

export function buildAdapterStatusQueryOperation(
  authorityDomainId: AuthorityDomainId,
  adapterId: string,
  ids: { commandId: string; idempotencyKey: string },
): Operation {
  if (!authorityDomainId.value || !adapterId || !ids.commandId || !ids.idempotencyKey) {
    throw new Error("adapter diagnostics query identity is incomplete");
  }
  const query = create(DiagnosticsQuerySchema, {
    query: {
      case: "adapters",
      value: create(AdapterStatusQuerySchema, {
        adapterIds: [create(AdapterIdSchema, { value: adapterId })],
        limit: 1,
        recentDiagnosticLimit: 20,
      }),
    },
  });
  return create(OperationSchema, {
    commandId: { value: ids.commandId },
    authorityDomainId: create(AuthorityDomainIdSchema, { value: authorityDomainId.value }),
    kind: OperationKind.QUERY,
    targetScope: create(TargetScopeSchema, {
      kind: TargetScopeKind.AUTHORITY_DOMAIN,
    }),
    idempotencyKey: ids.idempotencyKey,
    payload: create(PayloadEnvelopeSchema, {
      contentType: PayloadContentType.PROTOBUF,
      schemaRef: "patchbay.DiagnosticsQuery",
      payload: toBinary(DiagnosticsQuerySchema, query),
    }),
  });
}

export function mergeAdapterStatusResult(
  model: PresentationModel,
  response: QueryDiagnosticsResponse,
  requestedAdapterId?: string,
): PresentationModel {
  const next = cloneModel(model);
  const responseAsOfLsn = response.asOfLsn?.value;
  const asOfLsn = responseAsOfLsn ?? 0n;
  const submission = response.submission;
  const acceptedAndCompleted =
    submission?.outcome === SubmissionOutcome.ACCEPTED &&
    submission.operationState === OperationState.COMPLETED &&
    responseAsOfLsn !== undefined;
  const page = response.result.case === "adapters" ? response.result.value : undefined;

  // A rejected/failed/incomplete query is a normal protocol value, not a
  // transport exception. It cannot authorize retaining a cached attachment.
  // Keep newer live diagnostic evidence, but clear the status for the adapter
  // whose query failed (or all statuses for callers without a request key).
  if (!acceptedAndCompleted || !page) {
    const adapterIds = requestedAdapterId
      ? [requestedAdapterId]
      : [...next.adapters.keys()];
    for (const adapterId of adapterIds) {
      const current = next.adapters.get(adapterId);
      if (!current) continue;
      next.adapters.set(adapterId, { ...current, status: undefined });
    }
    return next;
  }

  const returned = new Set<string>();
  for (const status of page.adapters) {
    const adapterId = status.adapterId?.value;
    if (!adapterId) continue;
    returned.add(adapterId);
    const current = next.adapters.get(adapterId);
    const historical = status.recentDiagnostics
      .map((record) => diagnosticFromAudit(record))
      .filter((record): record is AdapterDiagnosticView => Boolean(record));
    const currentAsOfLsn = current?.asOfLsn ?? 0n;
    const responseIsAtLeastAsFresh = asOfLsn >= currentAsOfLsn;
    next.adapters.set(adapterId, {
      adapterId,
      // A delayed response must not roll a newer live status backward.
      status: responseIsAtLeastAsFresh ? status : current?.status,
      asOfLsn: responseIsAtLeastAsFresh ? asOfLsn : currentAsOfLsn,
      recentDiagnostics: dedupeDiagnostics([
        ...historical,
        ...(current?.recentDiagnostics ?? []),
      ]).slice(0, 20),
    });
  }

  // A successful completed response with no matching adapter is still not an
  // adapter result for the requested target. Do not leave an old ATTACHED
  // status visible in that case.
  if (requestedAdapterId && !returned.has(requestedAdapterId)) {
    const current = next.adapters.get(requestedAdapterId);
    if (current) next.adapters.set(requestedAdapterId, { ...current, status: undefined });
  }
  return next;
}

export function clearAdapterStatus(
  model: PresentationModel,
  adapterId: string,
): PresentationModel {
  const next = cloneModel(model);
  const current = next.adapters.get(adapterId);
  if (current) next.adapters.set(adapterId, { ...current, status: undefined });
  return next;
}

export function foldAdapterDiagnosticObservation(
  model: PresentationModel,
  observation: import("@patchbay/contracts").Observation,
  lsn: bigint,
): void {
  const envelope = observation.payload;
  if (!envelope || envelope.contentType !== PayloadContentType.PROTOBUF || envelope.schemaRef !== "patchbay.AdapterDiagnosticPayload") return;
  const target = observation.targetScope;
  const adapterId = target?.adapterId?.value;
  if (!adapterId || !target || (target.kind !== TargetScopeKind.ADAPTER && target.kind !== TargetScopeKind.RUNTIME_SESSION)) return;
  const payload = decodeDiagnosticPayload(envelope.payload);
  if (!payload || !Number.isInteger(payload.count) || payload.count < 1 || payload.count > 1_000) return;
  if (payload.severity < AdapterDiagnosticSeverity.INFO || payload.severity > AdapterDiagnosticSeverity.ERROR) return;
  if (payload.operationKind < OperationKind.UNSPECIFIED || payload.operationKind > OperationKind.SESSION_MANAGEMENT) return;
  if (observation.failureCode < FailureCode.UNSPECIFIED || observation.failureCode > FailureCode.EXECUTION_OUTCOME_UNKNOWN) return;
  if (payload.adapterGeneration?.value === undefined || payload.adapterGeneration.value === 0n || payload.code.length === 0 || payload.code.length > 64 || !/^[a-z0-9_]+$/.test(payload.code)) return;
  const targetSession = target.kind === TargetScopeKind.RUNTIME_SESSION
    ? runtimeSessionFromScope(target)
    : undefined;
  if (target.kind === TargetScopeKind.RUNTIME_SESSION && !targetSession) return;
  const record: AdapterDiagnosticView = {
    sourceEventId: `${model.authorityDomainId ?? ""}:${lsn}`,
    lsn,
    adapterId,
    adapterGeneration: payload.adapterGeneration.value,
    target: targetSession,
    commandId: commandCorrelation(observation),
    severity: payload.severity,
    code: payload.code,
    failureCode: observation.failureCode === FailureCode.UNSPECIFIED ? undefined : observation.failureCode,
    operationKind: payload.operationKind === OperationKind.UNSPECIFIED ? undefined : payload.operationKind,
    count: payload.count,
    observedAt: observation.observedAt ? timestampDate(observation.observedAt) : undefined,
  };
  const current = model.adapters.get(adapterId) ?? { adapterId, asOfLsn: 0n, recentDiagnostics: [] };
  model.adapters.set(adapterId, {
    ...current,
    recentDiagnostics: dedupeDiagnostics([...current.recentDiagnostics, record]).slice(0, 20),
  });
}

export function diagnosticsForSession(
  adapter: AdapterView | undefined,
  session: SessionIdentity,
): readonly AdapterDiagnosticView[] {
  return adapter?.recentDiagnostics.filter((diagnostic) =>
    !diagnostic.target || identityKey(diagnostic.target) === identityKey(session),
  ) ?? [];
}

export function renderAdapterStatus(document: Document, adapter: AdapterView | undefined): HTMLElement {
  const status = document.createElement("span");
  status.className = "adapter-status";
  status.setAttribute("aria-label", "Adapter connection status");
  const state = adapter?.status?.state ?? AdapterDiagnosticState.UNKNOWN;
  const presentation = adapterConnectionPresentation(state);
  const indicator = document.createElement("span");
  indicator.className = `connectivity-indicator connectivity-indicator--${presentation.connectivity}`;
  indicator.append(document.createElement("span"));
  indicator.lastElementChild!.className = "connectivity-indicator__dot";
  indicator.append(document.createTextNode(`adapter ${presentation.label}`));
  status.append(indicator);
  return status;
}

function diagnosticFromAudit(record: AuditRecord): AdapterDiagnosticView | undefined {
  const detail = record.adapterDiagnostic;
  const source = record.sourceEventId;
  const adapterId = detail?.adapterId?.value;
  const lsn = source?.lsn?.value;
  if (!adapterId || lsn === undefined || !detail?.adapterGeneration || !source) return undefined;
  if (detail.severity < AdapterDiagnosticSeverity.INFO || detail.severity > AdapterDiagnosticSeverity.ERROR) return undefined;
  if (detail.operationKind < OperationKind.UNSPECIFIED || detail.operationKind > OperationKind.SESSION_MANAGEMENT) return undefined;
  if (detail.count < 1 || detail.count > 1_000 || !record.reasonCode || !/^[a-z0-9_]{1,64}$/.test(record.reasonCode)) return undefined;
  const target = record.targetScope;
  const targetSession = target?.kind === TargetScopeKind.RUNTIME_SESSION
    ? runtimeSessionFromScope(target)
    : undefined;
  if (target?.kind === TargetScopeKind.RUNTIME_SESSION && !targetSession) return undefined;
  return {
    sourceEventId: `${source.authorityDomainId?.value ?? ""}:${lsn}`,
    lsn,
    adapterId,
    adapterGeneration: detail.adapterGeneration.value,
    target: targetSession,
    commandId: record.commandId?.value,
    severity: detail.severity,
    code: record.reasonCode,
    failureCode: record.failureCode === FailureCode.UNSPECIFIED ? undefined : record.failureCode,
    operationKind: detail.operationKind === OperationKind.UNSPECIFIED ? undefined : detail.operationKind,
    count: detail.count,
    observedAt: detail.adapterObservedAt ? timestampDate(detail.adapterObservedAt) : undefined,
  };
}

function decodeDiagnosticPayload(bytes: Uint8Array) {
  try {
    const payload = fromBinary(AdapterDiagnosticPayloadSchema, bytes);
    const severity = payload.severity;
    if (severity === AdapterDiagnosticSeverity.UNSPECIFIED) return undefined;
    return payload;
  } catch {
    return undefined;
  }
}

function commandCorrelation(observation: import("@patchbay/contracts").Observation): string | undefined {
  for (const correlation of observation.correlations) {
    if (correlation.ref.case === "commandId") return correlation.ref.value.value;
  }
  return undefined;
}

function identityKey(identity: SessionIdentity): string {
  return [identity.adapterId, identity.deploymentScope, identity.runtimeSessionId, identity.generation.toString()]
    .map((part) => `${part.length}:${part}`)
    .join("|");
}

function timestampDate(timestamp: { seconds: bigint; nanos: number }): Date {
  return new Date(Number(timestamp.seconds) * 1_000 + Math.floor(timestamp.nanos / 1_000_000));
}

function dedupeDiagnostics(records: readonly AdapterDiagnosticView[]): AdapterDiagnosticView[] {
  const seen = new Set<string>();
  return [...records]
    .sort((left, right) => left.lsn > right.lsn ? -1 : left.lsn < right.lsn ? 1 : 0)
    .filter((record) => {
      if (seen.has(record.sourceEventId)) return false;
      seen.add(record.sourceEventId);
      return true;
    });
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

