import { create } from "@bufbuild/protobuf";
import {
  CommandIdSchema,
  CommandInspectionQuerySchema,
  DiagnosticsQuerySchema,
} from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";
import type { CredentialStore } from "../credentials.js";
import type { CliOutput } from "../main.js";
import {
  commandInspectionView,
  eventCursor,
  inspectionTables,
  parsePositiveLimit,
  runDiagnosticsCommand,
  type DiagnosticsCommandSpec,
} from "./diagnostics.js";

export interface InspectCommandOptions {
  commandId: string;
  auditBeforeEvent?: string;
  auditLimit?: string;
  json: boolean;
}

export async function inspectCommandCommand(
  client: Pick<ControlClient, "queryDiagnostics">,
  store: CredentialStore,
  authorityDomainId: string,
  options: InspectCommandOptions,
  output: CliOutput,
): Promise<number> {
  if (!options.commandId.trim()) throw new Error("inspect-command command id must not be empty");
  const auditLimit = parsePositiveLimit(options.auditLimit, 200, "--audit-limit");
  const auditBeforeEventId = options.auditBeforeEvent
    ? eventCursor(authorityDomainId, options.auditBeforeEvent, "--audit-before-event")
    : undefined;
  const query = create(DiagnosticsQuerySchema, {
    query: {
      case: "command",
      value: create(CommandInspectionQuerySchema, {
        commandId: create(CommandIdSchema, { value: options.commandId }),
        auditBeforeEventId,
        auditLimit,
      }),
    },
  });
  const spec: DiagnosticsCommandSpec<"command"> = {
    query,
    resultCase: "command",
    json: options.json,
    jsonResult: commandInspectionView,
    humanResult: inspectionTables,
  };
  return runDiagnosticsCommand(client, store, authorityDomainId, spec, output);
}
