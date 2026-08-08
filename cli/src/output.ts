import {
  FailureCode,
  OperationState,
  SubmissionOutcome,
  type EventId,
  type SubmissionResult,
  type TargetScope,
} from "@patchbay/contracts";
import { TargetScopeKind } from "@patchbay/contracts";
import type { Timestamp } from "@bufbuild/protobuf/wkt";
import type { CliOutput } from "./main.js";

export interface SubmissionView {
  outcome: string;
  commandId: string | null;
  operationState: string;
  failureCode: string | null;
  diagnosticMessage: string | null;
  acceptedLsn: string | null;
  deduplicated: boolean;
}

export function exitCodeForSubmission(outcome: SubmissionOutcome): number {
  switch (outcome) {
    case SubmissionOutcome.ACCEPTED:
      return 0;
    case SubmissionOutcome.REJECTED:
      return 2;
    case SubmissionOutcome.FAILED:
      return 3;
    case SubmissionOutcome.UNKNOWN:
      return 4;
    case SubmissionOutcome.UNSPECIFIED:
    default:
      return 1;
  }
}

export function printSubmissionResult(
  result: SubmissionResult,
  json: boolean,
  output: CliOutput,
): number {
  const view = submissionView(result);
  if (json) {
    output.stdout(JSON.stringify(view));
  } else {
    const fields = [
      `outcome=${view.outcome}`,
      `command=${view.commandId ?? "-"}`,
      `state=${view.operationState}`,
      `failure=${view.failureCode ?? "-"}`,
      `lsn=${view.acceptedLsn ?? "-"}`,
      `deduplicated=${String(view.deduplicated)}`,
    ];
    output.stdout(fields.join(" "));
    if (view.diagnosticMessage) output.stderr(view.diagnosticMessage);
  }

  if (result.outcome === SubmissionOutcome.UNKNOWN) {
    output.stderr("Submission outcome is UNKNOWN; reconcile via the core's command records.");
  }
  return exitCodeForSubmission(result.outcome);
}

export function submissionView(result: SubmissionResult): SubmissionView {
  return {
    outcome: enumLabel(SubmissionOutcome, result.outcome),
    commandId: result.commandId?.value || null,
    operationState: enumLabel(OperationState, result.operationState),
    failureCode:
      result.failureCode === FailureCode.UNSPECIFIED
        ? null
        : enumLabel(FailureCode, result.failureCode),
    diagnosticMessage: result.diagnosticMessage || null,
    acceptedLsn: result.acceptedLsn?.value.toString() ?? null,
    deduplicated: result.deduplicated,
  };
}

export function enumLabel(
  registry: Record<number, string>,
  value: number,
): string {
  return (registry[value] ?? `UNRECOGNIZED_${value}`).toLowerCase();
}

export interface TableSection {
  title?: string;
  headers: readonly string[];
  rows: readonly (readonly string[])[];
}

export function printTableSection(section: TableSection, output: CliOutput): void {
  const headers = section.headers.map(escapeTerminalControls);
  const rows = section.rows.map((row) => row.map(escapeTerminalControls));
  if (section.title) output.stdout(escapeTerminalControls(section.title));
  const widths = headers.map((header, index) =>
    Math.max(header.length, ...rows.map((row) => row[index]?.length ?? 0)),
  );
  output.stdout(headers.map((header, index) => header.padEnd(widths[index]!)).join("  "));
  for (const row of rows) {
    output.stdout(row.map((value, index) => value.padEnd(widths[index]!)).join("  "));
  }
}

function escapeTerminalControls(value: string): string {
  return value.replace(/[\u0000-\u001f\u007f-\u009f]/gu, (character) => {
    switch (character) {
      case "\n": return "\\n";
      case "\r": return "\\r";
      case "\t": return "\\t";
      default: return `\\x${character.charCodeAt(0).toString(16).padStart(2, "0")}`;
    }
  });
}

export interface EventIdView {
  authorityDomainId: string | null;
  lsn: string | null;
}

export function eventIdView(eventId: EventId | undefined): EventIdView | null {
  if (!eventId) return null;
  return {
    authorityDomainId: eventId.authorityDomainId?.value || null,
    lsn: eventId.lsn?.value.toString() ?? null,
  };
}

export function timestampView(value: Timestamp | undefined): string | null {
  if (!value) return null;
  const milliseconds = Number(value.seconds) * 1_000 + value.nanos / 1_000_000;
  if (!Number.isFinite(milliseconds)) return null;
  return new Date(milliseconds).toISOString();
}

export function targetScopeView(value: TargetScope | undefined): unknown {
  if (!value) return null;
  return {
    kind: enumLabel(TargetScopeKind, value.kind),
    actorId: value.actorId?.value || null,
    adapterId: value.adapterId?.value || null,
    runtimeSessionId: value.runtimeSessionId?.value || null,
    sessionGeneration: value.sessionGeneration?.value.toString() ?? null,
    deploymentScope: value.deploymentScope || null,
    projectOrGroup: value.projectOrGroup || null,
    legacyAuditResourceId: value.legacyAuditResourceId || null,
    resource: value.resource ? {
      adapterId: value.resource.adapterId?.value || null,
      resourceKind: value.resource.resourceKind?.value || null,
      resourceId: value.resource.resourceId?.value || null,
    } : null,
  };
}
