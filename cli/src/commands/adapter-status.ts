import { create, toBinary } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  AdapterStatusQuerySchema,
  DiagnosticsQuerySchema,
  OperationKind,
  OperationState,
  PayloadContentType,
  PayloadEnvelopeSchema,
  SubmissionOutcome,
  type AdapterCapabilitySummary,
  type SubmissionResult,
  type TargetScope,
} from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";
import type { CredentialStore } from "../credentials.js";
import type { CliOutput } from "../main.js";
import {
  operationBase,
  operationContext,
  operationIds,
} from "./operations.js";
import { authorityDomainTarget } from "./sessions.js";
import {
  adapterStatusPageView,
  adapterTables,
  parsePositiveLimit,
  runDiagnosticsCommand,
  type DiagnosticsCommandSpec,
} from "./diagnostics.js";

export interface AdapterStatusOptions {
  adapterIds: readonly string[];
  afterAdapterId?: string;
  limit?: string;
  json: boolean;
}

export async function adapterStatusCommand(
  client: Pick<ControlClient, "queryDiagnostics">,
  store: CredentialStore,
  authorityDomainId: string,
  options: AdapterStatusOptions,
  output: CliOutput,
): Promise<number> {
  const adapterIds = options.adapterIds.map((id) => {
    if (!id) throw new Error("adapter ids must not be empty");
    return id;
  });
  if (new Set(adapterIds).size !== adapterIds.length) throw new Error("adapter ids must be unique");
  // Preserve an explicitly supplied opaque cursor, including an empty value;
  // core owns cursor validation and must distinguish it from omission.
  const limit = parsePositiveLimit(options.limit, 500, "--limit");
  const query = adapterStatusQuery(
    adapterIds,
    options.afterAdapterId,
    limit,
    // Core's adapter page limit does not imply a recent-diagnostics prefix;
    // request the bounded default explicitly so the projection includes it.
    100,
  );
  const spec: DiagnosticsCommandSpec<"adapters"> = {
    query,
    resultCase: "adapters",
    json: options.json,
    jsonResult: adapterStatusPageView,
    humanResult: adapterTables,
  };
  return runDiagnosticsCommand(client, store, authorityDomainId, spec, output);
}

export async function loadAdapterCapability(
  client: Pick<ControlClient, "queryDiagnostics">,
  store: CredentialStore,
  authorityDomainId: string,
  adapterId: string,
): Promise<AdapterCapabilitySummary | undefined> {
  if (!adapterId) return undefined;
  const context = await operationContext(store, authorityDomainId);
  const queryTarget = authorityDomainTarget(authorityDomainId);
  const operation = operationBase(
    context,
    queryTarget,
    OperationKind.QUERY,
    operationIds(queryTarget, {}),
  );
  operation.payload = create(PayloadEnvelopeSchema, {
    contentType: PayloadContentType.PROTOBUF,
    schemaRef: "patchbay.DiagnosticsQuery",
    payload: toBinary(DiagnosticsQuerySchema, adapterStatusQuery([adapterId], undefined, 1, 1)),
  });
  const response = await client.queryDiagnostics({ operation });
  if (response.submission?.outcome !== SubmissionOutcome.ACCEPTED
      || response.submission.operationState !== OperationState.COMPLETED
      || response.resultEventId?.authorityDomainId?.value !== authorityDomainId
      || !response.resultEventId.lsn
      || !response.asOfLsn
      || response.result.case !== "adapters") {
    return undefined;
  }
  const matches = response.result.value.adapters.filter(
    (adapter) => adapter.adapterId?.value === adapterId,
  );
  return matches.length === 1 ? matches[0]?.capability : undefined;
}

export async function capabilityForUnknownSubmission(
  client: Partial<Pick<ControlClient, "queryDiagnostics">>,
  store: CredentialStore,
  authorityDomainId: string,
  target: TargetScope,
  result: SubmissionResult,
): Promise<AdapterCapabilitySummary | undefined> {
  if (result.outcome !== SubmissionOutcome.UNKNOWN) return undefined;
  const adapterId = target.adapterId?.value;
  if (!adapterId || !client.queryDiagnostics) return undefined;

  try {
    return await loadAdapterCapability(client as Pick<ControlClient, "queryDiagnostics">, store, authorityDomainId, adapterId);
  } catch {
    return undefined;
  }
}

function adapterStatusQuery(
  adapterIds: readonly string[],
  afterAdapterId: string | undefined,
  limit: number | undefined,
  recentDiagnosticLimit: number,
) {
  return create(DiagnosticsQuerySchema, {
    query: {
      case: "adapters",
      value: create(AdapterStatusQuerySchema, {
        adapterIds: adapterIds.map((value) => create(AdapterIdSchema, { value })),
        afterAdapterId,
        limit,
        recentDiagnosticLimit,
      }),
    },
  });
}
