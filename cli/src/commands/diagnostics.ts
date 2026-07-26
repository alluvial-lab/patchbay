import { create, toBinary } from "@bufbuild/protobuf";
import { timestampFromDate, type Timestamp } from "@bufbuild/protobuf/wkt";
import {
  AdapterDiagnosticSeverity,
  AdapterDiagnosticState,
  AdapterSnapshotSupport,
  AuditEventKind,
  AuditPageSchema,
  AuthorityDomainIdSchema,
  DiagnosticsQuerySchema,
  EventIdSchema,
  FailureCode,
  IdempotencyStrength,
  LsnSchema,
  OperationKind,
  OperationState,
  SubmissionOutcome,
  PayloadContentType,
  PayloadEnvelopeSchema,
  PayloadContentType as PayloadContentTypeRegistry,
  TargetScopeKind,
  TargetScopeSchema,
  type AdapterStatusPage,
  type AuditRecord,
  type CommandInspection,
  type DiagnosticsQuery,
  type EventId,
  type TargetScope,
} from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";
import type { CredentialStore } from "../credentials.js";
import type { CliOutput } from "../main.js";
import { enumLabel, exitCodeForSubmission, submissionView } from "../output.js";
import { eventIdView, printTableSection, targetScopeView, timestampView, type TableSection } from "../output.js";
import {
  operationBase,
  operationContext,
  operationIds,
} from "./operations.js";
import { authorityDomainTarget, parseCanonicalSessionTarget } from "./sessions.js";
export { authorityDomainTarget, parseCanonicalSessionTarget } from "./sessions.js";

const MAX_U64 = (1n << 64n) - 1n;

export type DiagnosticsResultCase = "audit" | "command" | "adapters";
export type DiagnosticsResultFor<K extends DiagnosticsResultCase> =
  K extends "audit" ? import("@patchbay/contracts").AuditPage :
  K extends "command" ? import("@patchbay/contracts").CommandInspectionResult :
  AdapterStatusPage;

export interface DiagnosticsMeta {
  submission: NonNullable<import("@patchbay/contracts").QueryDiagnosticsResponse["submission"]>;
  resultEventId: EventId;
  asOfLsn: string;
}

export interface HumanDiagnosticsView {
  sections: readonly TableSection[];
  notices?: readonly string[];
}

export interface DiagnosticsCommandSpec<K extends DiagnosticsResultCase> {
  query: DiagnosticsQuery;
  resultCase: K;
  json: boolean;
  jsonResult(value: DiagnosticsResultFor<K>): unknown;
  humanResult(value: DiagnosticsResultFor<K>): HumanDiagnosticsView;
}

export async function runDiagnosticsCommand<K extends DiagnosticsResultCase>(
  client: Pick<ControlClient, "queryDiagnostics">,
  store: CredentialStore,
  authorityDomainId: string,
  spec: DiagnosticsCommandSpec<K>,
  output: CliOutput,
): Promise<number> {
  const context = await operationContext(store, authorityDomainId);
  const target = authorityDomainTarget(authorityDomainId);
  const operation = operationBase(
    context,
    target,
    OperationKind.QUERY,
    operationIds(target, {}),
  );
  operation.payload = create(PayloadEnvelopeSchema, {
    contentType: PayloadContentType.PROTOBUF,
    schemaRef: "patchbay.DiagnosticsQuery",
    payload: toBinary(DiagnosticsQuerySchema, spec.query),
  });

  const response = await client.queryDiagnostics({ operation });
  if (!response.submission) throw new Error("core returned a diagnostics response without submission");
  const submission = response.submission;
  if (submission.outcome !== SubmissionOutcome.ACCEPTED) {
    emitSubmissionFailure(submission, spec.json, output);
    return exitCodeForSubmission(submission.outcome);
  }

  // An accepted query still has a lifecycle. Only a completed query owns a
  // result envelope; an execution failure must retain its submission detail.
  if (submission.operationState === OperationState.FAILED) {
    emitSubmissionFailure(submission, spec.json, output);
    return 3;
  }
  if (submission.operationState !== OperationState.COMPLETED) {
    throw new Error(`core returned an unexpected diagnostics operation state: ${enumLabel(OperationState, submission.operationState)}`);
  }

  const resultEventId = response.resultEventId;
  const asOfLsn = response.asOfLsn;
  if (!resultEventId?.authorityDomainId?.value || resultEventId.authorityDomainId.value !== authorityDomainId) {
    throw new Error("core returned a diagnostics result event from another authority domain");
  }
  if (!resultEventId.lsn || !asOfLsn) throw new Error("core returned an incomplete diagnostics result envelope");
  if (spec.resultCase !== response.result.case) {
    throw new Error(`core returned diagnostics result ${response.result.case ?? "missing"}; expected ${spec.resultCase}`);
  }
  if (!response.result.value) throw new Error("core returned an empty diagnostics result");

  const meta: DiagnosticsMeta = {
    submission,
    resultEventId,
    asOfLsn: asOfLsn.value.toString(),
  };
  const value = response.result.value as DiagnosticsResultFor<K>;
  if (spec.json) {
    output.stdout(JSON.stringify({
      submission: submissionView(meta.submission),
      resultEventId: eventIdView(meta.resultEventId),
      asOfLsn: meta.asOfLsn,
      result: spec.jsonResult(value),
    }));
  } else {
    const view = spec.humanResult(value);
    for (const section of view.sections) printTableSection(section, output);
    for (const notice of view.notices ?? []) output.stderr(notice);
  }
  return 0;
}

export function parsePositiveLimit(raw: string | undefined, maximum: number, option: string): number | undefined {
  if (raw === undefined) return undefined;
  if (!/^\d+$/.test(raw)) throw new Error(`${option} must be a positive integer`);
  const value = Number(raw);
  if (!Number.isSafeInteger(value) || value < 1 || value > maximum) {
    throw new Error(`${option} must be between 1 and ${maximum}`);
  }
  return value;
}

export function eventCursor(authorityDomainId: string, raw: string | undefined, option: string): EventId | undefined {
  if (raw === undefined) return undefined;
  if (!/^[1-9]\d*$/.test(raw)) throw new Error(`${option} must be a positive decimal LSN`);
  const lsn = BigInt(raw);
  if (lsn > MAX_U64) throw new Error(`${option} exceeds the uint64 LSN range`);
  return create(EventIdSchema, {
    authorityDomainId: create(AuthorityDomainIdSchema, { value: authorityDomainId }),
    lsn: create(LsnSchema, { value: lsn }),
  });
}

export function parseGeneratedEnumList(
  registry: Record<string | number, string | number>,
  raw: string | undefined,
  option: string,
): number[] {
  if (raw === undefined) return [];
  const names = new Map<string, number>();
  for (const [key, value] of Object.entries(registry)) {
    if (!/^\d+$/.test(key) && typeof value === "number") {
      names.set(normalizeEnumName(key), value);
    }
  }
  const values: number[] = [];
  for (const part of raw.split(",")) {
    const normalized = normalizeEnumName(part);
    if (!normalized) throw new Error(`${option} contains an empty value`);
    const value = names.get(normalized);
    if (value === undefined || value === 0) throw new Error(`${option} contains an unknown or unspecified value: ${part}`);
    if (values.includes(value)) throw new Error(`${option} contains a duplicate value: ${part}`);
    values.push(value);
  }
  return values;
}

export function parseRfc3339(raw: string | undefined, option: string): Timestamp | undefined {
  if (raw === undefined) return undefined;
  if (!/^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?(?:Z|[+-]\d\d:\d\d)$/.test(raw)) {
    throw new Error(`${option} must be an RFC 3339 timestamp with a timezone`);
  }
  const date = new Date(raw);
  if (!Number.isFinite(date.getTime())) throw new Error(`${option} is not a valid RFC 3339 timestamp`);
  const withoutFraction = raw.replace(/\.\d+(?=Z|[+-]\d\d:\d\d)$/, "");
  const timestamp = timestampFromDate(new Date(withoutFraction));
  const fraction = raw.match(/\.(\d+)(?=Z|[+-]\d\d:\d\d)$/)?.[1];
  if (fraction) timestamp.nanos = Number(fraction.slice(0, 9).padEnd(9, "0"));
  return timestamp;
}

export function parseAuditTarget(raw: string | undefined, authorityDomainId: string): TargetScope | undefined {
  if (raw === undefined) return undefined;
  if (!raw) throw new Error("--target must not be empty");
  if (raw.includes(";")) return parseCanonicalSessionTarget(raw);
  if (raw === "authority-domain") return authorityDomainTarget(authorityDomainId);
  if (raw === "fleet") return create(TargetScopeSchema, { kind: TargetScopeKind.FLEET_SUPERVISOR });
  const separator = raw.indexOf("=");
  if (separator <= 0 || separator !== raw.lastIndexOf("=")) throw new Error("--target must be authority-domain, fleet, actor=, adapter=, group=, resource=, or a canonical runtime identity");
  const key = raw.slice(0, separator);
  let value: string;
  try { value = decodeURIComponent(raw.slice(separator + 1)); } catch { throw new Error("--target contains invalid percent-encoding"); }
  if (!value) throw new Error("--target value must not be empty");
  switch (key) {
    case "actor": return create(TargetScopeSchema, { kind: TargetScopeKind.ACTOR, actorId: { value } });
    case "adapter": return create(TargetScopeSchema, { kind: TargetScopeKind.ADAPTER, adapterId: { value } });
    case "group": return create(TargetScopeSchema, { kind: TargetScopeKind.PROJECT_SESSION_GROUP, projectOrGroup: value });
    case "resource": return create(TargetScopeSchema, { kind: TargetScopeKind.RESOURCE, resourceId: value });
    default: throw new Error(`invalid --target kind: ${key}`);
  }
}

export interface AuditRecordView {
  auditEventId: ReturnType<typeof eventIdView>;
  occurredAt: string | null;
  kind: string;
  actorId: string | null;
  deviceId: string | null;
  endpointId: string | null;
  operatorSessionHash: string | null;
  commandId: string | null;
  targetScope: unknown;
  failureCode: string | null;
  reasonCode: string | null;
  correlationId: string | null;
  sourceEventId: ReturnType<typeof eventIdView>;
  sourceNetwork: string | null;
  adapterDiagnostic: {
    adapterId: string | null;
    adapterGeneration: string | null;
    severity: string;
    operationKind: string;
    count: number;
    adapterObservedAt: string | null;
  } | null;
}

export function auditRecordView(record: AuditRecord): AuditRecordView {
  return {
    auditEventId: eventIdView(record.auditEventId),
    occurredAt: timestampView(record.occurredAt),
    kind: enumLabel(AuditEventKind, record.kind),
    actorId: record.actorId?.value || null,
    deviceId: record.deviceId?.value || null,
    endpointId: record.endpointId?.value || null,
    operatorSessionHash: record.operatorSessionHash.length ? bytesHex(record.operatorSessionHash) : null,
    commandId: record.commandId?.value || null,
    targetScope: targetScopeView(record.targetScope),
    failureCode: record.failureCode === FailureCode.UNSPECIFIED ? null : enumLabel(FailureCode, record.failureCode),
    reasonCode: record.reasonCode || null,
    correlationId: record.correlationId || null,
    sourceEventId: eventIdView(record.sourceEventId),
    sourceNetwork: record.sourceNetwork || null,
    adapterDiagnostic: record.adapterDiagnostic ? {
      adapterId: record.adapterDiagnostic.adapterId?.value || null,
      adapterGeneration: record.adapterDiagnostic.adapterGeneration?.value.toString() ?? null,
      severity: enumLabel(AdapterDiagnosticSeverity, record.adapterDiagnostic.severity),
      operationKind: enumLabel(OperationKind, record.adapterDiagnostic.operationKind),
      count: record.adapterDiagnostic.count,
      adapterObservedAt: timestampView(record.adapterDiagnostic.adapterObservedAt),
    } : null,
  };
}

export function auditPageView(page: import("@patchbay/contracts").AuditPage) {
  return {
    records: page.records.map(auditRecordView),
    page: { hasMore: page.hasMore, nextBeforeEvent: eventIdView(page.nextBeforeEventId) },
  };
}

export function commandInspectionView(result: import("@patchbay/contracts").CommandInspectionResult) {
  return {
    found: result.found,
    inspection: result.inspection ? inspectionView(result.inspection) : null,
  };
}

function inspectionView(inspection: CommandInspection) {
  return {
    command: inspection.command ? commandSummaryView(inspection.command) : null,
    acceptedEventId: eventIdView(inspection.acceptedEventId),
    currentState: enumLabel(OperationState, inspection.currentState),
    failureCode: inspection.failureCode === FailureCode.UNSPECIFIED ? null : enumLabel(FailureCode, inspection.failureCode),
    terminalEventId: eventIdView(inspection.terminalEventId),
    history: inspection.history.map((entry) => ({
      eventId: eventIdView(entry.eventId),
      state: enumLabel(OperationState, entry.state),
      failureCode: entry.failureCode === FailureCode.UNSPECIFIED ? null : enumLabel(FailureCode, entry.failureCode),
      occurredAt: timestampView(entry.occurredAt),
      correlations: entry.correlations.map(correlationView),
    })),
    audit: inspection.audit ? auditPageView(inspection.audit) : null,
  };
}

function commandSummaryView(command: import("@patchbay/contracts").CommandSummary) {
  return {
    commandId: command.commandId?.value || null,
    sender: endpointView(command.sender),
    recipient: endpointView(command.recipient),
    kind: enumLabel(OperationKind, command.kind),
    targetScope: targetScopeView(command.targetScope),
    correlations: command.correlations.map(correlationView),
    validityWindow: command.validityWindow ? {
      startsAt: timestampView(command.validityWindow.startsAt),
      expiresAt: timestampView(command.validityWindow.expiresAt),
    } : null,
    submittedAt: timestampView(command.submittedAt),
  };
}

function endpointView(endpoint: import("@patchbay/contracts").ActorEndpointRef | undefined) {
  return endpoint ? {
    actorId: endpoint.actorId?.value || null,
    endpointId: endpoint.endpointId?.value || null,
    deviceId: endpoint.deviceId?.value || null,
    endpointGeneration: endpoint.endpointGeneration?.value.toString() ?? null,
  } : null;
}

function correlationView(correlation: import("@patchbay/contracts").TypedCorrelation) {
  if (correlation.ref.case === undefined) return null;
  const value = correlation.ref.value;
  if ("value" in value && correlation.ref.case !== "eventId") return { kind: correlation.ref.case, value: value.value };
  return { kind: correlation.ref.case, value: eventIdView(value as EventId) };
}

function bytesHex(value: Uint8Array): string {
  return Buffer.from(value).toString("hex");
}

function emitSubmissionFailure(
  submission: NonNullable<import("@patchbay/contracts").QueryDiagnosticsResponse["submission"]>,
  json: boolean,
  output: CliOutput,
): void {
  const view = submissionView(submission);
  if (json) {
    output.stdout(JSON.stringify({
      submission: view,
      resultEventId: null,
      asOfLsn: null,
      result: null,
    }));
    return;
  }
  output.stdout([
    `outcome=${view.outcome}`,
    `command=${view.commandId ?? "-"}`,
    `state=${view.operationState}`,
    `failure=${view.failureCode ?? "-"}`,
    `lsn=${view.acceptedLsn ?? "-"}`,
    `deduplicated=${String(view.deduplicated)}`,
  ].join(" "));
  if (view.diagnosticMessage) output.stderr(view.diagnosticMessage);
}

function normalizeEnumName(value: string): string {
  return value.trim().replace(/-/g, "_").toUpperCase();
}

export function adapterStatusPageView(page: AdapterStatusPage) {
  return {
    adapters: page.adapters.map((adapter) => ({
      adapterId: adapter.adapterId?.value || null,
      endpointId: adapter.endpointId?.value || null,
      adapterGeneration: adapter.adapterGeneration?.value.toString() ?? null,
      state: enumLabel(AdapterDiagnosticState, adapter.state),
      attachEventId: eventIdView(adapter.attachEventId),
      attachedAt: timestampView(adapter.attachedAt),
      capability: adapter.capability ? {
        supportedOperationKinds: adapter.capability.supportedOperationKinds.map((value) => enumLabel(OperationKind, value)),
        supportedTargetSpecShapes: [...adapter.capability.supportedTargetSpecShapes],
        streamingSupport: adapter.capability.streamingSupport,
        snapshotSupport: enumLabel(AdapterSnapshotSupport, adapter.capability.snapshotSupport),
        cancellationSupport: adapter.capability.cancellationSupport,
        sessionReplacementSupport: adapter.capability.sessionReplacementSupport,
        idempotencyStrength: enumLabel(IdempotencyStrength, adapter.capability.idempotencyStrength),
        attachmentMethodKind: adapter.capability.attachmentMethodKind || null,
        attachmentDescriptorContentType: enumLabel(PayloadContentTypeRegistry, adapter.capability.attachmentDescriptorContentType),
        knownFailureModes: adapter.capability.knownFailureModes.map((value) => enumLabel(FailureCode, value)),
        diagnosticReporting: adapter.capability.diagnosticReporting ? {
          diagnosticCodes: [...adapter.capability.diagnosticReporting.diagnosticCodes],
        } : null,
      } : null,
      lastLifecycleRecord: adapter.lastLifecycleRecord ? auditRecordView(adapter.lastLifecycleRecord) : null,
      recentDiagnostics: adapter.recentDiagnostics.map(auditRecordView),
      liveSessionCount: adapter.liveSessionCount,
      staleSessionCount: adapter.staleSessionCount,
      offlineSessionCount: adapter.offlineSessionCount,
      failedSessionCount: adapter.failedSessionCount,
    })),
    page: { hasMore: page.hasMore, nextAfterAdapterId: page.nextAfterAdapterId || null },
  };
}

export function auditTable(page: import("@patchbay/contracts").AuditPage): HumanDiagnosticsView {
  const records = page.records.map(auditRecordView);
  return {
    sections: [{
      title: "AUDIT",
      headers: ["LSN", "TIME", "KIND", "ACTOR", "ENDPOINT", "COMMAND", "TARGET", "FAILURE", "REASON"],
      rows: records.map((record) => [
        record.auditEventId?.lsn ?? "-",
        record.occurredAt ?? "-",
        record.kind,
        record.actorId ?? "-",
        record.endpointId ?? "-",
        record.commandId ?? "-",
        formatTarget(record.targetScope),
        record.failureCode ?? "-",
        record.reasonCode ?? "-",
      ]),
    }],
    notices: [
      ...(page.records.length === 0 ? ["No audit records matched the query."] : []),
      ...(page.hasMore ? [`More audit records available; rerun with --before-event ${page.nextBeforeEventId?.lsn?.value.toString() ?? "<cursor>"}.`] : []),
    ],
  };
}

export function inspectionTables(result: import("@patchbay/contracts").CommandInspectionResult): HumanDiagnosticsView {
  if (!result.found || !result.inspection) {
    return {
      sections: [{ title: "COMMAND", headers: ["FIELD", "VALUE"], rows: [["FOUND", "false"]] }],
      notices: ["No command matched the supplied command id."],
    };
  }
  const inspection = result.inspection;
  const command = inspection.command;
  return {
    sections: [
      { title: "COMMAND", headers: ["FIELD", "VALUE"], rows: [
        ["COMMAND", command?.commandId?.value ?? "-"],
        ["KIND", command ? enumLabel(OperationKind, command.kind) : "-"],
        ["STATE", enumLabel(OperationState, inspection.currentState)],
        ["FAILURE", inspection.failureCode === FailureCode.UNSPECIFIED ? "-" : enumLabel(FailureCode, inspection.failureCode)],
      ] },
      { title: "HISTORY", headers: ["LSN", "TIME", "STATE", "FAILURE", "CORRELATIONS"], rows: inspection.history.map((entry) => [
        entry.eventId?.lsn?.value.toString() ?? "-", timestampView(entry.occurredAt) ?? "-",
        enumLabel(OperationState, entry.state),
        entry.failureCode === FailureCode.UNSPECIFIED ? "-" : enumLabel(FailureCode, entry.failureCode),
        String(entry.correlations.length),
      ]) },
      auditTable(inspection.audit ?? create(AuditPageSchema, {})).sections[0]!,
    ],
    notices: inspection.audit?.hasMore ? [`More audit records available; rerun with --audit-before-event ${inspection.audit.nextBeforeEventId?.lsn?.value.toString() ?? "<cursor>"}.`] : [],
  };
}

export function adapterTables(page: AdapterStatusPage): HumanDiagnosticsView {
  return {
    sections: [{
      title: "ADAPTERS",
      headers: ["ADAPTER", "ENDPOINT", "GENERATION", "STATE", "LIVE", "STALE", "OFFLINE", "FAILED", "SNAPSHOT", "IDEMPOTENCY", "ATTACHED_AT"],
      rows: page.adapters.map((adapter) => [
        adapter.adapterId?.value ?? "-", adapter.endpointId?.value ?? "-", adapter.adapterGeneration?.value.toString() ?? "-",
        enumLabel(AdapterDiagnosticState, adapter.state), String(adapter.liveSessionCount), String(adapter.staleSessionCount),
        String(adapter.offlineSessionCount), String(adapter.failedSessionCount),
        adapter.capability ? enumLabel(AdapterSnapshotSupport, adapter.capability.snapshotSupport) : "-",
        adapter.capability ? enumLabel(IdempotencyStrength, adapter.capability.idempotencyStrength) : "-",
        timestampView(adapter.attachedAt) ?? "-",
      ]),
    }],
    notices: [
      ...(page.adapters.length === 0 ? ["No adapters matched the query."] : []),
      ...(page.hasMore ? [`More adapters available; rerun with --after-adapter-id ${page.nextAfterAdapterId || "<cursor>"}.`] : []),
    ],
  };
}

function formatTarget(value: unknown): string {
  if (!value || typeof value !== "object") return "-";
  const target = value as Record<string, unknown>;
  const kind = String(target.kind ?? "unknown");
  if (kind === "runtime_session") return `adapter=${target.adapterId};scope=${target.deploymentScope};runtime=${target.runtimeSessionId};generation=${target.sessionGeneration}`;
  if (kind === "actor" || kind === "adapter" || kind === "resource") return `${kind}=${target.actorId ?? target.adapterId ?? target.resourceId}`;
  if (kind === "project_session_group") return `group=${target.projectOrGroup}`;
  return kind;
}
