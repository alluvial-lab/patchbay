import { create } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  AdapterStatusQuerySchema,
  DiagnosticsQuerySchema,
} from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";
import type { CredentialStore } from "../credentials.js";
import type { CliOutput } from "../main.js";
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
  const query = create(DiagnosticsQuerySchema, {
    query: {
      case: "adapters",
      value: create(AdapterStatusQuerySchema, {
        adapterIds: adapterIds.map((value) => create(AdapterIdSchema, { value })),
        afterAdapterId: options.afterAdapterId,
        limit,
      }),
    },
  });
  const spec: DiagnosticsCommandSpec<"adapters"> = {
    query,
    resultCase: "adapters",
    json: options.json,
    jsonResult: adapterStatusPageView,
    humanResult: adapterTables,
  };
  return runDiagnosticsCommand(client, store, authorityDomainId, spec, output);
}
