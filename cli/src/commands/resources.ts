import { AdapterSnapshotSupport, ResourceFreshnessState } from "@patchbay/contracts";
import type { TokenCommunePoolSummary, TokenCommuneResourceObservation } from "@patchbay/operator-domain";

import type { ControlClient } from "../core-client.js";
import type { CliOutput } from "../main.js";
import { printTableSection } from "../output.js";
import {
  canonicalResourceIdentity,
  derivationNote,
  loadTokenCommuneProjection,
  parseCanonicalResourceIdentity,
  summaryForIdentity,
  tokenCommuneSummaryView,
} from "./token-commune-projection.js";

export interface ResourceQueryOptions {
  adapterId?: string;
  provider?: string;
  json: boolean;
}

export interface ResourceInspectOptions {
  identity: string;
  json: boolean;
}

export async function resourceQueryCommand(
  client: Pick<ControlClient, "loadSnapshot" | "loadSecuritySnapshot" | "subscribe">,
  authorityDomainId: string,
  options: ResourceQueryOptions,
  output: CliOutput,
): Promise<number> {
  validateFilter(options.adapterId, "--adapter-id");
  validateFilter(options.provider, "--provider");
  const loaded = await loadTokenCommuneProjection(client, authorityDomainId);
  const summaries = loaded.summaries.filter((summary) =>
    (!options.adapterId || summary.poolIdentity.adapterId === options.adapterId)
    && (!options.provider || summary.provider === options.provider),
  );
  if (options.json) {
    output.stdout(JSON.stringify({
      snapshotLsn: loaded.snapshotLsn,
      summaries: summaries.map((summary) => tokenCommuneSummaryView(summary, loaded.recentEvents)),
      derivation: derivationNote(),
    }));
  } else if (summaries.length === 0) {
    output.stdout("No locally query-authorized token-commune pools matched.");
  } else {
    printSummaryTable(summaries, loaded.recentEvents, output);
    output.stdout(derivationNote());
  }
  return 0;
}

export async function resourceInspectCommand(
  client: Pick<ControlClient, "loadSnapshot" | "loadSecuritySnapshot" | "subscribe">,
  authorityDomainId: string,
  options: ResourceInspectOptions,
  output: CliOutput,
): Promise<number> {
  const identity = parseCanonicalResourceIdentity(options.identity);
  const loaded = await loadTokenCommuneProjection(client, authorityDomainId);
  const wrapper = loaded.wrappers.get(identityKey(identity));
  if (!wrapper) throw new Error(`authorized token-commune resource not found: ${canonicalResourceIdentity(identity)}`);
  const summary = summaryForIdentity(loaded.summaries, identity);
  if (!summary) throw new Error("resource has no composable provider-pool summary");
  if (options.json) {
    output.stdout(JSON.stringify({
      snapshotLsn: loaded.snapshotLsn,
      resource: {
        identity: canonicalResourceIdentity(wrapper.identity),
        revisionLsn: wrapper.revisionLsn,
        completeness: completenessLabel(wrapper.completeness),
        freshness: freshnessLabel(wrapper.freshness),
        observedAt: wrapper.observedAt,
        tombstoned: wrapper.tombstoned,
      },
      summary: tokenCommuneSummaryView(summary, loaded.recentEvents),
      derivation: derivationNote(),
    }));
  } else {
    printTableSection({
      title: "RESOURCE",
      headers: ["IDENTITY", "REVISION", "COMPLETENESS", "FRESHNESS", "OBSERVED", "LIFECYCLE"],
      rows: [[
        canonicalResourceIdentity(wrapper.identity),
        wrapper.revisionLsn,
        completenessLabel(wrapper.completeness),
        freshnessLabel(wrapper.freshness),
        wrapper.observedAt ?? "unavailable",
        wrapper.tombstoned ? "retired" : "active",
      ]],
    }, output);
    printSummaryTable([summary], loaded.recentEvents, output);
    output.stdout(derivationNote());
  }
  return 0;
}

function printSummaryTable(
  summaries: readonly TokenCommunePoolSummary[],
  events: readonly TokenCommuneResourceObservation[],
  output: CliOutput,
): void {
  printTableSection({
    title: "TOKEN-COMMUNE POOLS",
    headers: ["PROVIDER", "DRAW", "CONTRIBUTIONS", "5H CAPACITY", "FINGERPRINT", "VERDICT", "FRESHNESS", "MODELS", "EVENTS"],
    rows: summaries.map((summary) => [
      summary.provider,
      drawLabel(summary),
      credentialsLabel(summary),
      capacityLabel(summary),
      fingerprintLabel(summary),
      summary.verdict,
      telemetryLabel(summary),
      modelsLabel(summary),
      eventsLabel(summary, events),
    ]),
  }, output);
}

function drawLabel(summary: TokenCommunePoolSummary): string {
  if (summary.draw.state === "current" || summary.draw.state === "stale") {
    return `${percent(summary.draw.limitFraction)} (${summary.draw.state}; consumed ${summary.draw.consumedUnits}; reset ${summary.draw.resetsAt ?? "unavailable"})`;
  }
  return summary.draw.state;
}
function credentialsLabel(summary: TokenCommunePoolSummary): string {
  if (summary.credentials.state === "unknown") return "unknown";
  const share = summary.totalDeclaredShare === null ? "share unavailable" : `${percent(summary.totalDeclaredShare)} declared share`;
  return `${summary.credentials.contributionCount} anonymous / ${share}; ${summary.credentials.fresh} fresh / ${summary.credentials.exhausted} exhausted / ${summary.credentials.authBroken} auth-broken (${summary.credentials.state})`;
}
function capacityLabel(summary: TokenCommunePoolSummary): string {
  if (summary.capacity5h.state === "current" || summary.capacity5h.state === "stale") {
    return `${percent(summary.capacity5h.usedFraction)} used (${summary.capacity5h.state}; source ${age(new Date(summary.capacity5h.observedAt))}; reset ${summary.capacity5h.resetsAt ?? "unavailable"})`;
  }
  return summary.capacity5h.state;
}
function telemetryLabel(summary: TokenCommunePoolSummary): string {
  const reading = summary.capacity5h.state === "no-5h-readings" || summary.capacity5h.state === "reading-unavailable"
    ? "unavailable"
    : summary.capacity5h.state;
  return `wrapper ${summary.poolState} / reading ${reading} / credentials ${summary.credentials.state}`;
}
function fingerprintLabel(summary: TokenCommunePoolSummary): string {
  if (summary.fingerprint.status === "unknown") return `unknown (${summary.fingerprint.reason})`;
  if (summary.fingerprint.held) return `held${summary.fingerprint.diffPresent ? " / diff" : ""}`;
  return summary.fingerprint.capturePresent ? `captured${summary.fingerprint.diffPresent ? " / diff" : ""}` : "reported / no capture";
}
function eventsLabel(
  summary: TokenCommunePoolSummary,
  events: readonly TokenCommuneResourceObservation[],
): string {
  const visible = events.filter((event) => identityKey(event.poolIdentity) === identityKey(summary.poolIdentity)).slice(0, 5);
  return visible.length === 0 ? "none in bounded replay" : visible.map((event) => `${event.kind === "event-gap" ? "gap" : "pool"}:${event.code}@${event.occurredAt}`).join(", ");
}
function modelsLabel(summary: TokenCommunePoolSummary): string {
  if (summary.models.length === 0) return summary.modelState === "unknown" ? "catalog unknown" : "none reported";
  return summary.models.map((model) => {
    const provenance = `upstream ${model.upstreamModel ?? "unavailable"}`;
    return model.available ? `${model.id} (${provenance})` : `${model.id} unavailable (${provenance})`;
  }).join(", ");
}
function percent(value: number): string {
  return `${Math.round(value * 1000) / 10}%`;
}
function age(value: Date, now = new Date()): string {
  const seconds = Math.floor(Math.max(0, now.getTime() - value.getTime()) / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  return `${Math.floor(minutes / 60)}h ago`;
}
function validateFilter(value: string | undefined, option: string): void {
  if (value !== undefined && (value.length === 0 || value.length > 512)) throw new Error(`${option} must be a bounded non-empty value`);
}
function identityKey(identity: { adapterId: string; resourceKind: string; resourceId: string }): string {
  return `${identity.adapterId}\u0000${identity.resourceKind}\u0000${identity.resourceId}`;
}
function completenessLabel(value: AdapterSnapshotSupport): string {
  if (value === AdapterSnapshotSupport.AUTHORITATIVE) return "authoritative";
  if (value === AdapterSnapshotSupport.PARTIAL) return "partial";
  if (value === AdapterSnapshotSupport.NONE) return "none";
  return "unknown";
}
function freshnessLabel(value: ResourceFreshnessState): string {
  if (value === ResourceFreshnessState.CURRENT) return "current";
  if (value === ResourceFreshnessState.STALE) return "stale";
  return "unknown";
}
