import { create } from "@bufbuild/protobuf";
import {
  ActorIdSchema,
  AuditEventKind,
  AuditQuerySchema,
  CommandIdSchema,
  DiagnosticsQuerySchema,
  EndpointIdSchema,
  FailureCode,
} from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";
import type { CredentialStore } from "../credentials.js";
import type { CliOutput } from "../main.js";
import {
  auditPageView,
  auditTable,
  eventCursor,
  parseAuditTarget,
  parseGeneratedEnumList,
  parsePositiveLimit,
  parseRfc3339,
  runDiagnosticsCommand,
  type DiagnosticsCommandSpec,
} from "./diagnostics.js";

export interface AuditQueryOptions {
  kinds?: string;
  actorId?: string;
  endpointId?: string;
  commandId?: string;
  target?: string;
  failureCodes?: string;
  reasonCodes?: string;
  since?: string;
  until?: string;
  beforeEvent?: string;
  limit?: string;
  json: boolean;
}

export async function auditQueryCommand(
  client: Pick<ControlClient, "queryDiagnostics">,
  store: CredentialStore,
  authorityDomainId: string,
  options: AuditQueryOptions,
  output: CliOutput,
): Promise<number> {
  const kinds = parseGeneratedEnumList(AuditEventKind, options.kinds, "--kind");
  const failureCodes = parseGeneratedEnumList(FailureCode, options.failureCodes, "--failure-code");
  const reasonCodes = csvStrings(options.reasonCodes, "--reason-code");
  const actorId = options.actorId === undefined ? undefined : createRequiredId(options.actorId, "--actor-id", ActorIdSchema);
  const endpointId = options.endpointId === undefined ? undefined : createRequiredId(options.endpointId, "--endpoint-id", EndpointIdSchema);
  const commandId = options.commandId === undefined ? undefined : createRequiredId(options.commandId, "--command-id", CommandIdSchema);
  const since = parseRfc3339(options.since, "--since");
  const until = parseRfc3339(options.until, "--until");
  if (since && until && compareTimestamp(since, until) >= 0) {
    throw new Error("--since must be before --until");
  }
  const limit = parsePositiveLimit(options.limit, 500, "--limit");
  const beforeEventId = options.beforeEvent === undefined
    ? undefined
    : eventCursor(authorityDomainId, options.beforeEvent, "--before-event");

  const query = create(DiagnosticsQuerySchema, {
    query: {
      case: "audit",
      value: create(AuditQuerySchema, {
        kinds,
        actorId,
        endpointId,
        commandId,
        targetScope: parseAuditTarget(options.target, authorityDomainId),
        failureCodes,
        reasonCodes,
        occurredFromInclusive: since,
        occurredBeforeExclusive: until,
        beforeEventId,
        limit,
      }),
    },
  });
  const spec: DiagnosticsCommandSpec<"audit"> = {
    query,
    resultCase: "audit",
    json: options.json,
    jsonResult: auditPageView,
    humanResult: auditTable,
  };
  return runDiagnosticsCommand(client, store, authorityDomainId, spec, output);
}

function csvStrings(raw: string | undefined, option: string): string[] {
  if (raw === undefined) return [];
  const values = raw.split(",").map((value) => value.trim());
  if (values.some((value) => !value)) throw new Error(`${option} contains an empty value`);
  return values;
}

function createRequiredId(raw: string, option: string, schema: any): any {
  if (!raw) throw new Error(`${option} must not be empty`);
  return create(schema, { value: raw });
}

function compareTimestamp(left: { seconds: bigint; nanos: number }, right: { seconds: bigint; nanos: number }): number {
  if (left.seconds < right.seconds) return -1;
  if (left.seconds > right.seconds) return 1;
  return left.nanos - right.nanos;
}
