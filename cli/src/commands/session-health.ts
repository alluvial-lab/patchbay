import {
  SessionActivityState,
  SessionConnectivityState,
  type Session,
} from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";
import type { CliOutput } from "../main.js";
import { printTableSection } from "../output.js";
import { canonicalSessionIdentity, loadSessions, resolveSession } from "./sessions.js";

export interface SessionHealthOptions {
  sessionId?: string;
  json: boolean;
}

export async function sessionHealthCommand(
  client: Pick<ControlClient, "loadSnapshot">,
  authorityDomainId: string,
  options: SessionHealthOptions,
  output: CliOutput,
): Promise<number> {
  const loaded = await loadSessions(client, authorityDomainId);
  const sessions = options.sessionId
    ? [resolveSession(loaded, options.sessionId)]
    : loaded.filter((session) => !session.tombstoned);

  if (sessions.length === 0) {
    output.stderr("No sessions found in the authoritative snapshot.");
    return 1;
  }

  const rows = sessions.map(sessionHealthView);
  if (options.json) {
    output.stdout(JSON.stringify(rows));
  } else {
    printTable(rows, output);
  }
  return 0;
}

export function sessionHealthView(session: Session) {
  const connectivity = session.state?.connectivity ?? SessionConnectivityState.UNKNOWN;
  const activity = session.state?.activity ?? SessionActivityState.UNKNOWN;
  return {
    identity: canonicalSessionIdentity(session),
    adapterId: session.adapterId?.value ?? "",
    deploymentScope: session.deploymentScope,
    runtimeSessionId: session.runtimeSessionId?.value ?? "",
    generation: session.sessionGeneration?.value.toString() ?? "",
    connectivity: connectivityLabel(connectivity),
    activity: activityLabel(activity),
    model: session.model || null,
    name: session.name || null,
    lastAuthoritativeLsn: session.lastAuthoritativeLsn?.value.toString() ?? null,
  };
}

function connectivityLabel(state: SessionConnectivityState): string {
  if (state === SessionConnectivityState.UNSPECIFIED) return "unknown";
  return enumName(SessionConnectivityState, state);
}

function activityLabel(state: SessionActivityState): string {
  if (state === SessionActivityState.UNSPECIFIED) return "unknown";
  return enumName(SessionActivityState, state);
}

function enumName(registry: Record<number, string>, value: number): string {
  return (registry[value] ?? "unknown").toLowerCase();
}

function printTable(
  rows: ReturnType<typeof sessionHealthView>[],
  output: CliOutput,
): void {
  printTableSection({
    headers: ["IDENTITY", "CONNECTIVITY", "ACTIVITY", "MODEL", "NAME"],
    rows: rows.map((row) => [
      row.identity,
      row.connectivity,
      row.activity,
      row.model ?? "Model unknown",
      row.name ?? "-",
    ]),
  }, output);
}
