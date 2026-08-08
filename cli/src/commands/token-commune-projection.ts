import { create, fromBinary } from "@bufbuild/protobuf";
import {
  AdapterSnapshotSupport,
  AuthorityDomainIdSchema,
  LoadSecuritySnapshotRequestSchema,
  LsnSchema,
  ObservationSchema,
  OperationKind,
  ResourceFreshnessState,
  ResourceSnapshotSchema,
  SnapshotViewKind,
  StoredEventKind,
  SubscribeRequestSchema,
  TargetScopeKind,
  type GrantSummary,
  type Resource,
  type TargetScope,
} from "@patchbay/contracts";
import {
  composeTokenCommunePools,
  decodeTokenCommuneProjection,
  decodeTokenCommuneResourceObservation,
  type SurfaceResourceIdentity,
  type TokenCommunePoolSummary,
  type TokenCommuneResourceObservation,
  type TokenCommuneResourceInput,
} from "@patchbay/operator-domain";

import type { ControlClient } from "../core-client.js";
import { timestampView } from "../output.js";
import { parseCanonicalResourceTarget } from "./diagnostics.js";

export interface CanonicalResourceWrapper {
  identity: SurfaceResourceIdentity;
  revisionLsn: string;
  completeness: AdapterSnapshotSupport;
  freshness: ResourceFreshnessState;
  observedAt: string | null;
  tombstoned: boolean;
}

export interface LoadedTokenCommuneProjection {
  summaries: readonly TokenCommunePoolSummary[];
  wrappers: ReadonlyMap<string, CanonicalResourceWrapper>;
  snapshotLsn: string;
  recentEvents: readonly TokenCommuneResourceObservation[];
}

export async function loadTokenCommuneProjection(
  client: Pick<ControlClient, "loadSnapshot" | "loadSecuritySnapshot" | "subscribe">,
  authorityDomainId: string,
): Promise<LoadedTokenCommuneProjection> {
  const domain = create(AuthorityDomainIdSchema, { value: authorityDomainId });
  const [resourceResponse, securityResponse] = await Promise.all([
    client.loadSnapshot({ authorityDomainId: domain, viewKind: SnapshotViewKind.RESOURCE }),
    client.loadSecuritySnapshot(create(LoadSecuritySnapshotRequestSchema, { authorityDomainId: domain })),
  ]);
  if (!resourceResponse.present) return { summaries: [], wrappers: new Map(), snapshotLsn: "0", recentEvents: [] };
  if (resourceResponse.viewKind !== SnapshotViewKind.RESOURCE) throw new Error("core returned a non-resource snapshot view");
  if (resourceResponse.snapshotPayload.length === 0) throw new Error("core returned an empty resource snapshot payload");
  const snapshot = fromBinary(ResourceSnapshotSchema, resourceResponse.snapshotPayload);
  if (snapshot.authorityDomainId?.value !== authorityDomainId) throw new Error("core returned a resource snapshot from another authority domain");
  if (resourceResponse.eventId?.authorityDomainId?.value !== authorityDomainId) throw new Error("core returned a resource snapshot event from another authority domain");
  if (snapshot.snapshotLsn?.value !== resourceResponse.eventId?.lsn?.value) throw new Error("resource snapshot LSN does not match its response event LSN");
  const security = securityResponse.snapshot;
  if (!security || security.authorityDomainId?.value !== authorityDomainId) throw new Error("core returned an invalid security snapshot domain");
  if (security.snapshotLsn?.value === undefined) throw new Error("security snapshot LSN is missing");

  const completeness = new Map(snapshot.viewRevisions.map((view) => [
    collectionKey(view.adapterId?.value ?? "", view.resourceKind?.value ?? ""),
    view.completeness,
  ]));
  const inputs: TokenCommuneResourceInput[] = [];
  const wrappers = new Map<string, CanonicalResourceWrapper>();
  const now = new Date();
  for (const resource of snapshot.resources) {
    const identity = resourceIdentity(resource);
    if (!hasLocalQueryGrant(security.grants, identity, now)) continue;
    const projection = decodeTokenCommuneProjection(identity, resource.resourcePayload, resource.projectionPayload);
    if (!projection) continue;
    const wrapper: CanonicalResourceWrapper = {
      identity,
      revisionLsn: requiredBigint(resource.revisionLsn?.value, "resource revision LSN").toString(),
      completeness: completeness.get(collectionKey(identity.adapterId, identity.resourceKind)) ?? AdapterSnapshotSupport.UNSPECIFIED,
      freshness: resource.freshness,
      observedAt: timestampView(resource.observedAt),
      tombstoned: resource.tombstoned,
    };
    wrappers.set(identityKey(identity), wrapper);
    inputs.push({
      identity,
      freshness: resource.freshness,
      completeness: wrapper.completeness,
      ...(wrapper.observedAt ? { observedAt: new Date(wrapper.observedAt) } : {}),
      reconciled: true,
      tombstoned: resource.tombstoned,
      projection,
    });
  }
  const recentEvents: TokenCommuneResourceObservation[] = [];
  for await (const event of client.subscribe(create(SubscribeRequestSchema, {
    authorityDomainId: domain,
    cursor: create(LsnSchema, { value: 0n }),
  }))) {
    if (event.payload?.kind !== StoredEventKind.OBSERVATION) continue;
    const decoded = decodeTokenCommuneResourceObservation(fromBinary(ObservationSchema, event.payload.payload));
    if (!decoded || !wrappers.has(identityKey(decoded.poolIdentity))) continue;
    recentEvents.push(decoded);
    if (recentEvents.length > 20) recentEvents.shift();
  }
  return {
    summaries: composeTokenCommunePools(inputs),
    wrappers,
    snapshotLsn: requiredBigint(snapshot.snapshotLsn?.value, "resource snapshot LSN").toString(),
    recentEvents: recentEvents.reverse(),
  };
}

export function parseCanonicalResourceIdentity(value: string): SurfaceResourceIdentity {
  const target = parseCanonicalResourceTarget(value);
  const resource = target.resource;
  if (
    target.kind !== TargetScopeKind.RESOURCE
    || !resource?.adapterId?.value
    || !resource.resourceKind?.value
    || !resource.resourceId?.value
  ) throw new Error("canonical resource identity is incomplete");
  return {
    adapterId: resource.adapterId.value,
    resourceKind: resource.resourceKind.value,
    resourceId: resource.resourceId.value,
  };
}

export function canonicalResourceIdentity(identity: SurfaceResourceIdentity): string {
  return `adapter=${encodeURIComponent(identity.adapterId)};resource-kind=${encodeURIComponent(identity.resourceKind)};resource=${encodeURIComponent(identity.resourceId)}`;
}

export function summaryForIdentity(
  summaries: readonly TokenCommunePoolSummary[],
  identity: SurfaceResourceIdentity,
): TokenCommunePoolSummary | undefined {
  const key = identityKey(identity);
  return summaries.find((summary) =>
    identityKey(summary.poolIdentity) === key
    || (summary.drawIdentity && identityKey(summary.drawIdentity) === key),
  );
}

export function tokenCommuneSummaryView(
  summary: TokenCommunePoolSummary,
  recentEvents: readonly TokenCommuneResourceObservation[] = [],
) {
  return {
    provider: summary.provider,
    poolIdentity: canonicalResourceIdentity(summary.poolIdentity),
    draw: summary.draw.state === "current" || summary.draw.state === "stale" ? {
      state: summary.draw.state,
      limitFraction: decimal(summary.draw.limitFraction),
      consumedUnits: decimal(summary.draw.consumedUnits),
      resetsAt: summary.draw.resetsAt,
    } : { state: summary.draw.state, limitFraction: null, consumedUnits: null, resetsAt: null },
    credentials: {
      state: summary.credentials.state,
      fresh: summary.credentials.fresh,
      exhausted: summary.credentials.exhausted,
      authBroken: summary.credentials.authBroken,
      contributionCount: summary.credentials.contributionCount,
      totalDeclaredShare: summary.totalDeclaredShare === null ? null : decimal(summary.totalDeclaredShare),
    },
    fingerprint: summary.fingerprint,
    capacity5h: summary.capacity5h.state === "current" || summary.capacity5h.state === "stale" ? {
      state: summary.capacity5h.state,
      usedFraction: decimal(summary.capacity5h.usedFraction),
      observedAt: summary.capacity5h.observedAt,
      resetsAt: summary.capacity5h.resetsAt,
    } : { state: summary.capacity5h.state, usedFraction: null, observedAt: null, resetsAt: null },
    verdict: summary.verdict,
    freshness: summary.poolState,
    wrapperFreshness: summary.poolState,
    readingFreshness: capacityTelemetryState(summary),
    models: summary.models.map((model) => ({
      id: model.id,
      provider: model.provider,
      surface: model.surface,
      upstreamModel: model.upstreamModel,
      contextWindow: model.contextWindow,
      maxTokens: model.maxTokens,
      reasoning: model.reasoning,
      available: model.available,
    })),
    completeness: completenessLabel(summary.completeness),
    recentEvents: recentEvents
      .filter((event) => sameIdentity(event.poolIdentity, summary.poolIdentity))
      .slice(0, 5)
      .map(({ kind, code, occurredAt }) => ({ kind, code, occurredAt })),
  };
}

export function derivationNote(): string {
  return "Patchbay synthesis: native limitFraction; highest real anonymous 5h usedFraction (Patchbay display window, not necessarily binding); freshness → unknown evidence → auth broken → model unavailable → pool exhausted → runnable; draw excluded; no native pool aggregate or contributor identity.";
}

function hasLocalQueryGrant(grants: readonly GrantSummary[], identity: SurfaceResourceIdentity, now: Date): boolean {
  return grants.some((grant) =>
    !grant.revoked
    && grant.allowedOperationKinds.includes(OperationKind.QUERY)
    && (!grant.expiresAt || timestampDate(grant.expiresAt).getTime() > now.getTime())
    && scopeContainsResource(grant.targetScope, identity),
  );
}

function scopeContainsResource(scope: TargetScope | undefined, identity: SurfaceResourceIdentity): boolean {
  if (!scope) return false;
  switch (scope.kind) {
    case TargetScopeKind.AUTHORITY_DOMAIN:
    case TargetScopeKind.FLEET_SUPERVISOR:
      return !hasAnyTargetFields(scope);
    case TargetScopeKind.ADAPTER:
      return scope.adapterId?.value === identity.adapterId && !hasFieldsOtherThan(scope, "adapter");
    case TargetScopeKind.RESOURCE:
      return !hasFieldsOtherThan(scope, "resource")
        && scope.resource?.adapterId?.value === identity.adapterId
        && scope.resource.resourceKind?.value === identity.resourceKind
        && scope.resource.resourceId?.value === identity.resourceId;
    default:
      return false;
  }
}

function hasAnyTargetFields(scope: TargetScope): boolean {
  return Boolean(scope.actorId || scope.adapterId || scope.runtimeSessionId || scope.sessionGeneration
    || scope.deploymentScope || scope.projectOrGroup || scope.legacyAuditResourceId || scope.resource);
}
function hasFieldsOtherThan(scope: TargetScope, allowed: "adapter" | "resource"): boolean {
  if (scope.actorId || scope.projectOrGroup || scope.legacyAuditResourceId) return true;
  if (allowed === "adapter") return Boolean(scope.runtimeSessionId || scope.sessionGeneration || scope.deploymentScope || scope.resource);
  return Boolean(scope.adapterId || scope.runtimeSessionId || scope.sessionGeneration || scope.deploymentScope);
}
function resourceIdentity(resource: Resource): SurfaceResourceIdentity {
  const identity = resource.identity;
  const adapterId = identity?.adapterId?.value;
  const resourceKind = identity?.resourceKind?.value;
  const resourceId = identity?.resourceId?.value;
  if (!adapterId || !resourceKind || !resourceId) throw new Error("resource snapshot contains incomplete identity");
  return { adapterId, resourceKind, resourceId };
}
function identityKey(identity: SurfaceResourceIdentity): string {
  return `${identity.adapterId}\u0000${identity.resourceKind}\u0000${identity.resourceId}`;
}
function sameIdentity(left: SurfaceResourceIdentity, right: SurfaceResourceIdentity): boolean {
  return identityKey(left) === identityKey(right);
}
function capacityTelemetryState(summary: TokenCommunePoolSummary): "current" | "stale" | "unknown" | "unavailable" {
  if (summary.poolState === "stale" || summary.capacity5h.state === "stale") return "stale";
  if (summary.poolState === "unknown" || summary.capacity5h.state === "unknown") return "unknown";
  if (summary.capacity5h.state === "no-5h-readings" || summary.capacity5h.state === "reading-unavailable") return "unavailable";
  return "current";
}
function collectionKey(adapterId: string, resourceKind: string): string {
  return `${adapterId}\u0000${resourceKind}`;
}
function timestampDate(value: Parameters<typeof timestampView>[0]): Date {
  const view = timestampView(value);
  if (!view) throw new Error("invalid grant expiration timestamp");
  return new Date(view);
}
function requiredBigint(value: bigint | undefined, field: string): bigint {
  if (value === undefined) throw new Error(`${field} is missing`);
  return value;
}
function decimal(value: number): string {
  if (!Number.isFinite(value)) throw new Error("summary number must be finite");
  return String(value);
}
function completenessLabel(value: AdapterSnapshotSupport): string {
  if (value === AdapterSnapshotSupport.AUTHORITATIVE) return "authoritative";
  if (value === AdapterSnapshotSupport.PARTIAL) return "partial";
  if (value === AdapterSnapshotSupport.NONE) return "none";
  return "unknown";
}
