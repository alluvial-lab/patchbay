import {
  AdapterSnapshotSupport,
  ObservationKind,
  PayloadContentType,
  ResourceFreshnessState,
  TargetScopeKind,
  type Observation,
  type PayloadEnvelope,
} from "@patchbay/contracts";

export const TOKEN_COMMUNE_PRESENTATION_CONTRACT = {
  providerPool: {
    resourceKind: "token-commune.provider-pool",
    payloadSchema: "patchbay.token_commune.provider_pool.payload.v1",
    projectionSchema: "patchbay.token_commune.provider_pool.projection.v1",
  },
  memberDraw: {
    resourceKind: "token-commune.member-draw",
    payloadSchema: "patchbay.token_commune.member_draw.payload.v1",
    projectionSchema: "patchbay.token_commune.member_draw.projection.v1",
  },
} as const;

const MAX_TEXT = 512;
const RFC3339 = /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?(?:Z|[+-]\d\d:\d\d)$/;

export interface SurfaceResourceIdentity {
  adapterId: string;
  resourceKind: string;
  resourceId: string;
}

export interface CapacityReading {
  window: string;
  usedFraction: number | null;
  resetsAt: string | null;
  observedAt: string;
}

export interface AnonymousContribution {
  health: "fresh" | "exhausted" | "auth_broken";
  telemetryState: "readings" | "no-readings";
  capacityReadings: readonly CapacityReading[];
}

export type ContributionListing =
  | { status: "reported"; contributions: readonly AnonymousContribution[] }
  | { status: "not-reported" | "unavailable"; contributions: readonly [] };

export interface CredentialHealthCounts {
  fresh: number;
  exhausted: number;
  authBroken: number;
}

export interface ProviderModel {
  id: string;
  provider: string;
  surface: string;
  upstreamModel: string | null;
  contextWindow: number;
  maxTokens: number;
  reasoning: boolean;
  available: boolean;
}

export type ProviderModelCatalog =
  | { status: "reported"; models: readonly ProviderModel[] }
  | { status: "unavailable" | "provider-mismatch"; models: readonly [] };

export type ProviderFingerprint =
  | {
      status: "reported";
      probe: "anthropic" | "openai-codex";
      templateSource: string | null;
      capturedAt: string | null;
      capturePresent: boolean;
      held: boolean;
      heldAt: string | null;
      diffPresent: boolean;
    }
  | {
      status: "unknown";
      probe: "anthropic" | "openai-codex" | null;
      reason: "probe-unavailable" | "not-probed";
    };

export interface MemberDrawReport {
  provider: string;
  limitFraction: number;
  consumedUnits: number;
  resetsAt: string | null;
}

export type TokenCommuneProjection =
  | {
      kind: "token-commune-provider-pool";
      provider: string;
      contributionListing: ContributionListing;
      credentialHealthCounts: CredentialHealthCounts;
      totalDeclaredShare: number;
      modelCatalog: ProviderModelCatalog;
      fingerprint: ProviderFingerprint;
      capacityAggregation: "none";
    }
  | {
      kind: "token-commune-member-draw";
      memberDisplayName: string;
      provider: string;
      reports: readonly MemberDrawReport[];
    };

export type TokenCommuneDecodeResult =
  | { status: "decoded"; value: TokenCommuneProjection }
  | { status: "invalid"; reason: "projection_decode_failed" }
  | { status: "unsupported" }
  | { status: "unavailable" };

export interface TokenCommuneResourceObservation {
  poolIdentity: SurfaceResourceIdentity;
  kind: "pool-event" | "event-gap";
  code: string;
  occurredAt: string;
}

const TOKEN_COMMUNE_OBSERVATION_SCHEMAS = {
  poolEvent: "patchbay.token_commune.pool_event.v1",
  eventGap: "patchbay.token_commune.event_gap.v1",
} as const;

/** Decode only bounded, identity-free event summaries for operator surfaces. */
export function decodeTokenCommuneResourceObservation(
  observation: Observation,
): TokenCommuneResourceObservation | undefined {
  const envelope = observation.payload;
  if (
    observation.kind !== ObservationKind.STATUS
    || !envelope
    || envelope.contentType !== PayloadContentType.JSON
    || (envelope.schemaRef !== TOKEN_COMMUNE_OBSERVATION_SCHEMAS.poolEvent
      && envelope.schemaRef !== TOKEN_COMMUNE_OBSERVATION_SCHEMAS.eventGap)
  ) return undefined;
  const target = observation.targetScope;
  const resource = target?.resource;
  const adapterId = resource?.adapterId?.value;
  const resourceKind = resource?.resourceKind?.value;
  const resourceId = resource?.resourceId?.value;
  if (
    target?.kind !== TargetScopeKind.RESOURCE
    || target.actorId || target.adapterId || target.runtimeSessionId || target.sessionGeneration
    || target.deploymentScope || target.projectOrGroup || target.legacyAuditResourceId
    || !adapterId || resourceKind !== TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.resourceKind || !resourceId
    || observation.sender?.actorId?.value !== adapterId
  ) return undefined;
  try {
    const value = jsonObject(envelope.payload);
    const poolIdentity = { adapterId, resourceKind, resourceId };
    if (envelope.schemaRef === TOKEN_COMMUNE_OBSERVATION_SCHEMAS.poolEvent) {
      exactKeys(value, [
        "sourceEventId", "kind", "provider", "contributionId", "message", "occurredAt",
        "deliveryModel", "historyMode",
      ]);
      text(value.sourceEventId, "sourceEventId");
      const code = member(value.kind, ["capacity_shift", "auth_broken", "windfall", "fingerprint", "member"] as const, "event kind");
      text(value.provider, "provider");
      if (value.contributionId !== null) text(value.contributionId, "contributionId");
      boundedText(value.message, "message", 1024);
      const occurredAt = time(value.occurredAt, "occurredAt");
      if (value.deliveryModel !== "polling" || value.historyMode !== "latest-50-no-cursor") throw new Error("unsupported event delivery contract");
      return { poolIdentity, kind: "pool-event", code, occurredAt };
    }
    exactKeys(value, [
      "reason", "previousWindowSize", "visibleWindowSize", "overlapCount", "detectedAt",
      "deliveryModel", "historyMode", "reconstruction", "continuity",
    ]);
    const code = member(value.reason, ["initial-baseline", "window-discontinuity", "window-saturated-without-anchor", "history-became-empty"] as const, "gap reason");
    count(value.previousWindowSize, "previousWindowSize");
    count(value.visibleWindowSize, "visibleWindowSize");
    count(value.overlapCount, "overlapCount");
    const occurredAt = time(value.detectedAt, "detectedAt");
    if (
      value.deliveryModel !== "polling" || value.historyMode !== "latest-50-no-cursor"
      || value.reconstruction !== "visible-window-only" || value.continuity !== "unknown-before-visible-window"
    ) throw new Error("unsupported gap delivery contract");
    return { poolIdentity, kind: "event-gap", code, occurredAt };
  } catch {
    return undefined;
  }
}

export function decodeTokenCommuneProjection(
  identity: SurfaceResourceIdentity,
  resourcePayload: PayloadEnvelope | undefined,
  projectionPayload: PayloadEnvelope | undefined,
): TokenCommuneDecodeResult | undefined {
  const contract = identity.resourceKind === TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.resourceKind
    ? TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool
    : identity.resourceKind === TOKEN_COMMUNE_PRESENTATION_CONTRACT.memberDraw.resourceKind
      ? TOKEN_COMMUNE_PRESENTATION_CONTRACT.memberDraw
      : undefined;
  if (!contract) return undefined;
  if (!resourcePayload || !projectionPayload) return { status: "unavailable" };
  if (
    resourcePayload.schemaRef !== contract.payloadSchema
    || projectionPayload.schemaRef !== contract.projectionSchema
    || resourcePayload.contentType !== PayloadContentType.JSON
    || projectionPayload.contentType !== PayloadContentType.JSON
  ) return { status: "unsupported" };

  try {
    const value = jsonObject(projectionPayload.payload);
    return {
      status: "decoded",
      value: identity.resourceKind === TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.resourceKind
        ? decodeProviderPool(value)
        : decodeMemberDraw(value),
    };
  } catch {
    return { status: "invalid", reason: "projection_decode_failed" };
  }
}

export interface TokenCommuneResourceInput {
  identity: SurfaceResourceIdentity;
  freshness: ResourceFreshnessState;
  completeness: AdapterSnapshotSupport;
  observedAt?: Date;
  reconciled: boolean;
  tombstoned: boolean;
  projection: TokenCommuneDecodeResult;
}

export type DrawAllowance =
  | { state: "current" | "stale"; limitFraction: number; consumedUnits: number; resetsAt: string | null }
  | { state: "unavailable" | "ambiguous" | "unknown" };

export type Capacity5h =
  | { state: "current" | "stale"; usedFraction: number; observedAt: string; resetsAt: string | null }
  | { state: "no-5h-readings" | "reading-unavailable" | "unknown" };

export interface CredentialHealthSummary {
  state: "current" | "stale" | "unknown";
  fresh: number;
  exhausted: number;
  authBroken: number;
  contributionCount: number;
}

export type TokenCommuneVerdict =
  | "runnable"
  | "pool-exhausted"
  | "telemetry-stale"
  | "auth-broken"
  | "model-unavailable"
  | "unknown";

export interface TokenCommunePoolSummary {
  key: string;
  provider: string;
  poolIdentity: SurfaceResourceIdentity;
  drawIdentity?: SurfaceResourceIdentity;
  completeness: AdapterSnapshotSupport;
  poolState: "current" | "stale" | "unknown";
  poolObservedAt?: Date;
  drawObservedAt?: Date;
  draw: DrawAllowance;
  credentials: CredentialHealthSummary;
  totalDeclaredShare: number | null;
  capacity5h: Capacity5h;
  fingerprint: ProviderFingerprint;
  models: readonly ProviderModel[];
  modelState: "current" | "stale" | "unknown";
  verdict: TokenCommuneVerdict;
}

export function composeTokenCommunePools(
  resources: readonly TokenCommuneResourceInput[],
): readonly TokenCommunePoolSummary[] {
  const draws = resources.filter((resource) =>
    resource.projection.status === "decoded"
    && resource.projection.value.kind === "token-commune-member-draw",
  );
  const pools: TokenCommunePoolSummary[] = [];

  for (const pool of resources) {
    if (pool.identity.resourceKind !== TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.resourceKind) continue;
    if (
      pool.projection.status !== "decoded"
      || pool.projection.value.kind !== "token-commune-provider-pool"
    ) {
      pools.push({
        key: identityKey(pool.identity),
        provider: "provider unavailable",
        poolIdentity: pool.identity,
        completeness: pool.completeness,
        poolState: "unknown",
        ...(pool.observedAt ? { poolObservedAt: pool.observedAt } : {}),
        draw: { state: "unknown" },
        credentials: { state: "unknown", fresh: 0, exhausted: 0, authBroken: 0, contributionCount: 0 },
        totalDeclaredShare: null,
        capacity5h: { state: "unknown" },
        fingerprint: { status: "unknown", probe: null, reason: "not-probed" },
        models: [],
        modelState: "unknown",
        verdict: "unknown",
      });
      continue;
    }
    const value = pool.projection.value;
    const exactDraws = draws.filter((draw) =>
      draw.identity.adapterId === pool.identity.adapterId
      && draw.projection.status === "decoded"
      && draw.projection.value.kind === "token-commune-member-draw"
      && draw.projection.value.provider === value.provider,
    );
    const matchingReports = exactDraws.flatMap((draw) => {
      if (draw.projection.status !== "decoded" || draw.projection.value.kind !== "token-commune-member-draw") return [];
      return draw.projection.value.reports
        .filter((report) => report.provider === value.provider)
        .map((report) => ({ draw, report }));
    });
    const poolStale = pool.tombstoned
      || !pool.reconciled
      || pool.freshness === ResourceFreshnessState.STALE;
    const poolUnknown = pool.freshness === ResourceFreshnessState.UNKNOWN;
    const draw = composeDraw(matchingReports);
    const credentials = composeCredentials(value.contributionListing, value.credentialHealthCounts, poolStale, poolUnknown);
    const { capacity, facts } = composeCapacity(value.contributionListing, poolStale, poolUnknown);
    const modelState = value.modelCatalog.status !== "reported" || poolUnknown
      ? "unknown"
      : poolStale ? "stale" : "current";
    const models = value.modelCatalog.status === "reported" ? value.modelCatalog.models : [];
    const sourceEvidenceComplete = value.contributionListing.status === "reported"
      && value.modelCatalog.status === "reported"
      && !poolUnknown;
    const verdict = synthesizeTokenCommuneVerdict({
      poolCurrent: !poolStale,
      sourceEvidenceComplete,
      credentials,
      capacity5h: capacity,
      contributionCapacityFacts: facts,
      modelState,
      availableModelCount: models.filter((model) => model.provider === value.provider && model.available).length,
    });
    const uniqueDraw = matchingReports.length === 1 ? matchingReports[0] : undefined;
    pools.push({
      key: `${pool.identity.adapterId}\u0000${value.provider}`,
      provider: value.provider,
      poolIdentity: pool.identity,
      ...(uniqueDraw ? { drawIdentity: uniqueDraw.draw.identity } : {}),
      completeness: pool.completeness,
      poolState: poolUnknown ? "unknown" : poolStale ? "stale" : "current",
      ...(pool.observedAt ? { poolObservedAt: pool.observedAt } : {}),
      ...(uniqueDraw?.draw.observedAt ? { drawObservedAt: uniqueDraw.draw.observedAt } : {}),
      draw,
      credentials,
      totalDeclaredShare: value.totalDeclaredShare,
      capacity5h: capacity,
      fingerprint: value.fingerprint,
      models,
      modelState,
      verdict,
    });
  }
  return pools.sort((left, right) =>
    left.poolIdentity.adapterId.localeCompare(right.poolIdentity.adapterId)
    || left.provider.localeCompare(right.provider),
  );
}

export function synthesizeTokenCommuneVerdict(input: {
  poolCurrent: boolean;
  sourceEvidenceComplete: boolean;
  credentials: CredentialHealthSummary;
  capacity5h: Capacity5h;
  contributionCapacityFacts: readonly {
    health: "fresh" | "exhausted" | "auth_broken";
    fiveHourUsedFraction: number | null | undefined;
  }[];
  modelState: "current" | "stale" | "unknown";
  availableModelCount: number;
}): TokenCommuneVerdict {
  if (!input.poolCurrent) return "telemetry-stale";
  if (!input.sourceEvidenceComplete || input.credentials.state === "unknown" || input.modelState === "unknown") return "unknown";
  if (input.credentials.fresh === 0 && input.credentials.authBroken > 0) return "auth-broken";
  if (input.modelState === "current" && input.availableModelCount === 0) return "model-unavailable";
  const facts = input.contributionCapacityFacts;
  if (
    facts.length > 0
    && (
      facts.every((fact) => fact.health === "exhausted")
      || facts.every((fact) => fact.fiveHourUsedFraction === 1)
    )
  ) return "pool-exhausted";
  if (
    input.credentials.fresh > 0
    && input.availableModelCount > 0
    && facts.some((fact) => fact.fiveHourUsedFraction !== null
      && fact.fiveHourUsedFraction !== undefined
      && fact.fiveHourUsedFraction < 1)
  ) return "runnable";
  return "unknown";
}

function composeDraw(
  matches: readonly { draw: TokenCommuneResourceInput; report: MemberDrawReport }[],
): DrawAllowance {
  if (matches.length === 0) return { state: "unavailable" };
  if (matches.length > 1) return { state: "ambiguous" };
  const match = matches[0]!;
  if (match.draw.freshness === ResourceFreshnessState.UNKNOWN) return { state: "unknown" };
  const stale = match.draw.tombstoned
    || !match.draw.reconciled
    || match.draw.freshness === ResourceFreshnessState.STALE;
  return {
    state: stale ? "stale" : "current",
    limitFraction: match.report.limitFraction,
    consumedUnits: match.report.consumedUnits,
    resetsAt: match.report.resetsAt,
  };
}

function composeCredentials(
  listing: ContributionListing,
  counts: CredentialHealthCounts,
  stale: boolean,
  unknown: boolean,
): CredentialHealthSummary {
  if (listing.status !== "reported" || unknown) {
    return { state: "unknown", fresh: 0, exhausted: 0, authBroken: 0, contributionCount: 0 };
  }
  return {
    state: stale ? "stale" : "current",
    ...counts,
    contributionCount: listing.contributions.length,
  };
}

function composeCapacity(
  listing: ContributionListing,
  stale: boolean,
  unknown: boolean,
): {
  capacity: Capacity5h;
  facts: readonly { health: "fresh" | "exhausted" | "auth_broken"; fiveHourUsedFraction: number | null | undefined }[];
} {
  if (listing.status !== "reported" || unknown) return { capacity: { state: "unknown" }, facts: [] };
  const facts = listing.contributions.map((contribution) => {
    const readings = contribution.capacityReadings.filter((reading) => reading.window === "5h");
    const measured = readings.filter((reading) => reading.usedFraction !== null);
    return {
      health: contribution.health,
      fiveHourUsedFraction: measured.length === 0
        ? readings.length === 0 ? undefined : null
        : Math.max(...measured.map((reading) => reading.usedFraction!)),
    };
  });
  const all5h = listing.contributions.flatMap((contribution, contributionIndex) =>
    contribution.capacityReadings
      .filter((reading) => reading.window === "5h")
      .map((reading, readingIndex) => ({ reading, contributionIndex, readingIndex })),
  );
  if (all5h.length === 0) return { capacity: { state: "no-5h-readings" }, facts };
  const measured = all5h.filter((candidate) => candidate.reading.usedFraction !== null);
  if (measured.length === 0) return { capacity: { state: "reading-unavailable" }, facts };
  measured.sort((left, right) =>
    right.reading.usedFraction! - left.reading.usedFraction!
    || Date.parse(right.reading.observedAt) - Date.parse(left.reading.observedAt)
    || left.contributionIndex - right.contributionIndex
    || left.readingIndex - right.readingIndex,
  );
  const selected = measured[0]!.reading;
  return {
    capacity: {
      state: stale ? "stale" : "current",
      usedFraction: selected.usedFraction!,
      observedAt: selected.observedAt,
      resetsAt: selected.resetsAt,
    },
    facts,
  };
}

function decodeProviderPool(value: Record<string, unknown>): TokenCommuneProjection {
  exactKeys(value, [
    "provider", "contributionListing", "credentialHealthCounts", "totalDeclaredShare",
    "statusTelemetry", "modelCatalog", "fingerprint", "capacityAggregation",
  ]);
  const provider = text(value.provider, "provider");
  const listing = contributionListing(record(value.contributionListing, "contributionListing"));
  const countsValue = record(value.credentialHealthCounts, "credentialHealthCounts");
  exactKeys(countsValue, ["fresh", "exhausted", "authBroken"]);
  const counts = {
    fresh: count(countsValue.fresh, "fresh"),
    exhausted: count(countsValue.exhausted, "exhausted"),
    authBroken: count(countsValue.authBroken, "authBroken"),
  };
  if (listing.status === "reported") {
    const actual = listing.contributions.reduce((acc, contribution) => {
      if (contribution.health === "fresh") acc.fresh += 1;
      else if (contribution.health === "exhausted") acc.exhausted += 1;
      else acc.authBroken += 1;
      return acc;
    }, { fresh: 0, exhausted: 0, authBroken: 0 });
    if (actual.fresh !== counts.fresh || actual.exhausted !== counts.exhausted || actual.authBroken !== counts.authBroken) {
      throw new Error("credential health counts disagree with anonymous rows");
    }
  } else if (counts.fresh + counts.exhausted + counts.authBroken !== 0) {
    throw new Error("unreported contribution listing cannot carry health counts");
  }
  const totalDeclaredShare = nonNegative(value.totalDeclaredShare, "totalDeclaredShare");
  validateStatusTelemetry(record(value.statusTelemetry, "statusTelemetry"));
  let modelCatalog = decodeModelCatalog(record(value.modelCatalog, "modelCatalog"));
  if (modelCatalog.status === "reported" && modelCatalog.models.some((model) => model.provider !== provider)) {
    modelCatalog = { status: "provider-mismatch", models: [] };
  }
  const fingerprint = decodeFingerprint(record(value.fingerprint, "fingerprint"));
  if (value.capacityAggregation !== "none") throw new Error("capacityAggregation must be none");
  return {
    kind: "token-commune-provider-pool",
    provider,
    contributionListing: listing,
    credentialHealthCounts: counts,
    totalDeclaredShare,
    modelCatalog,
    fingerprint,
    capacityAggregation: "none",
  };
}

function decodeMemberDraw(value: Record<string, unknown>): TokenCommuneProjection {
  exactKeys(value, ["memberDisplayName", "provider", "reports"]);
  const memberDisplayName = text(value.memberDisplayName, "memberDisplayName");
  const provider = text(value.provider, "provider");
  const reports = array(value.reports, "reports").map((candidate) => {
    const report = record(candidate, "drawReport");
    exactKeys(report, [
      "provider", "limitFraction", "fromDecree", "consumedUnits", "drawUnits",
      "exceeded", "enforceable", "resetsAt",
    ]);
    const drawUnits = report.drawUnits;
    if (drawUnits !== null) nonNegative(drawUnits, "drawUnits");
    bool(report.fromDecree, "fromDecree");
    bool(report.exceeded, "exceeded");
    bool(report.enforceable, "enforceable");
    return {
      provider: text(report.provider, "report.provider"),
      limitFraction: fraction(report.limitFraction, "limitFraction"),
      consumedUnits: nonNegative(report.consumedUnits, "consumedUnits"),
      resetsAt: nullableTime(report.resetsAt, "resetsAt"),
    };
  });
  return { kind: "token-commune-member-draw", memberDisplayName, provider, reports };
}

function contributionListing(value: Record<string, unknown>): ContributionListing {
  exactKeys(value, ["status", "contributions"]);
  const status = member(value.status, ["reported", "not-reported", "unavailable"] as const, "contribution status");
  const raw = array(value.contributions, "contributions");
  if (status !== "reported") {
    if (raw.length !== 0) throw new Error("unreported contribution listing must be empty");
    return { status, contributions: [] };
  }
  return { status, contributions: raw.map(decodeContribution) };
}

function decodeContribution(candidate: unknown): AnonymousContribution {
  const value = record(candidate, "contribution");
  exactKeys(value, [
    "subKey", "subKeySource", "subKeyStability", "attribution", "declaredShare", "health",
    "telemetryState", "capacityReadings", "fingerprint",
  ]);
  if (!/^local:anonymous-contribution:[0-9a-f]{24}:[1-9][0-9]*$/.test(text(value.subKey, "subKey"))) throw new Error("invalid subKey");
  if (value.subKeySource !== "synthesized-content-hash" || value.subKeyStability !== "snapshot-local" || value.attribution !== "unavailable") {
    throw new Error("invalid anonymous contribution identity metadata");
  }
  fraction(value.declaredShare, "declaredShare");
  const healthValue = record(value.health, "health");
  const health = member(healthValue.state, ["fresh", "exhausted", "auth_broken"] as const, "health");
  if (health === "fresh") exactKeys(healthValue, ["state"]);
  if (health === "exhausted") {
    exactKeys(healthValue, ["state", "exhaustedUntil"]);
    time(healthValue.exhaustedUntil, "exhaustedUntil");
  }
  if (health === "auth_broken") {
    exactKeys(healthValue, ["state", "reason"]);
    text(healthValue.reason, "reason");
  }
  const telemetryState = member(value.telemetryState, ["readings", "no-readings"] as const, "telemetryState");
  const capacityReadings = array(value.capacityReadings, "capacityReadings").map(decodeCapacity);
  if ((telemetryState === "readings") !== (capacityReadings.length > 0)) throw new Error("telemetry state disagrees with readings");
  validatePoolFingerprint(record(value.fingerprint, "contribution fingerprint"));
  return { health, telemetryState, capacityReadings };
}

function decodeCapacity(candidate: unknown): CapacityReading {
  const value = record(candidate, "capacity reading");
  exactKeys(value, ["window", "usedFraction", "usedUnits", "limitUnits", "resetsAt", "source", "observedAt"]);
  const usedFraction = value.usedFraction === null ? null : fraction(value.usedFraction, "usedFraction");
  if (value.usedUnits !== null) nonNegative(value.usedUnits, "usedUnits");
  if (value.limitUnits !== null) nonNegative(value.limitUnits, "limitUnits");
  member(value.source, ["headers", "usage_endpoint", "observed_429", "declared"] as const, "source");
  return {
    window: text(value.window, "window"),
    usedFraction,
    resetsAt: nullableTime(value.resetsAt, "resetsAt"),
    observedAt: time(value.observedAt, "observedAt"),
  };
}

function decodeModelCatalog(value: Record<string, unknown>): ProviderModelCatalog {
  exactKeys(value, ["status", "models"]);
  const status = member(value.status, ["reported", "unavailable"] as const, "model status");
  const rows = array(value.models, "models");
  if (status === "unavailable") {
    if (rows.length !== 0) throw new Error("unavailable model catalog must be empty");
    return { status, models: [] };
  }
  return {
    status,
    models: rows.map((candidate) => {
      const model = record(candidate, "model");
      exactKeys(model, ["id", "provider", "surface", "upstreamModel", "contextWindow", "maxTokens", "reasoning", "available"]);
      const id = text(model.id, "model.id");
      // The upstream catalog rejects this removed bare alias. Fail closed rather
      // than presenting it as a Patchbay-created model name.
      if (id === "gpt-5.6") throw new Error("rejected model alias");
      const provider = text(model.provider, "model.provider");
      const surface = text(model.surface, "model.surface");
      const upstreamModel = model.upstreamModel === null
        ? null
        : text(model.upstreamModel, "model.upstreamModel");
      const contextWindow = positive(model.contextWindow, "contextWindow");
      const maxTokens = positive(model.maxTokens, "maxTokens");
      const reasoning = bool(model.reasoning, "reasoning");
      return {
        id,
        provider,
        surface,
        upstreamModel,
        contextWindow,
        maxTokens,
        reasoning,
        available: bool(model.available, "available"),
      };
    }),
  };
}

function validateStatusTelemetry(value: Record<string, unknown>): void {
  const status = member(value.status, ["reported", "not-reported", "unavailable"] as const, "status telemetry");
  if (status !== "reported") {
    exactKeys(value, ["status", "contributions"]);
    if (array(value.contributions, "status contributions").length !== 0) throw new Error("unreported status telemetry must be empty");
    return;
  }
  exactKeys(value, ["status", "gatewayOk", "anthropicHealth", "joinability", "contributions"]);
  bool(value.gatewayOk, "gatewayOk");
  if (value.joinability !== "unjoinable-with-pool-rows") throw new Error("status joinability must remain unavailable");
  if (value.anthropicHealth !== null) {
    const health = record(value.anthropicHealth, "anthropicHealth");
    member(health.state, ["fresh", "exhausted", "auth_broken"] as const, "anthropicHealth.state");
  }
  for (const candidate of array(value.contributions, "status contributions")) {
    const contribution = record(candidate, "status contribution");
    exactKeys(contribution, ["contributionId", "provider", "readings"]);
    text(contribution.contributionId, "contributionId");
    text(contribution.provider, "provider");
    array(contribution.readings, "readings").forEach(decodeCapacity);
  }
}

function decodeFingerprint(value: Record<string, unknown>): ProviderFingerprint {
  const status = member(value.status, ["reported", "unknown"] as const, "fingerprint status");
  if (status === "reported") {
    exactKeys(value, ["status", "probe", "value"]);
    const probe = member(value.probe, ["anthropic", "openai-codex"] as const, "fingerprint probe");
    const state = record(value.value, "fingerprint value");
    exactKeys(state, ["templateSource", "capturedAt", "capturePresent", "holdReason", "heldAt", "diffPresent"]);
    const templateSource = state.templateSource === null ? null : text(state.templateSource, "templateSource");
    const capturedAt = nullableTime(state.capturedAt, "capturedAt");
    const capturePresent = bool(state.capturePresent, "capturePresent");
    if (state.holdReason !== null) text(state.holdReason, "holdReason");
    const heldAt = nullableTime(state.heldAt, "heldAt");
    const diffPresent = bool(state.diffPresent, "diffPresent");
    return {
      status,
      probe,
      templateSource,
      capturedAt,
      capturePresent,
      held: state.holdReason !== null || heldAt !== null,
      heldAt,
      diffPresent,
    };
  } else {
    exactKeys(value, ["status", "probe", "reason"]);
    const probe = value.probe === null ? null : member(value.probe, ["anthropic", "openai-codex"] as const, "fingerprint probe");
    const reason = member(value.reason, ["probe-unavailable", "not-probed"] as const, "fingerprint reason");
    return { status, probe, reason };
  }
}

function validatePoolFingerprint(value: Record<string, unknown>): void {
  exactKeys(value, ["state", "templateSource", "since", "diffPresent"]);
  member(value.state, ["ok", "held", "unknown"] as const, "pool fingerprint state");
  member(value.templateSource, ["compiled", "override"] as const, "templateSource");
  nullableTime(value.since, "since");
  bool(value.diffPresent, "diffPresent");
}

function identityKey(identity: SurfaceResourceIdentity): string {
  return `${identity.adapterId}\u0000${identity.resourceKind}\u0000${identity.resourceId}`;
}

function jsonObject(payload: Uint8Array): Record<string, unknown> {
  return record(JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(payload)) as unknown, "projection");
}
function record(value: unknown, field: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(`${field} must be an object`);
  return value as Record<string, unknown>;
}
function array(value: unknown, field: string): unknown[] {
  if (!Array.isArray(value)) throw new Error(`${field} must be an array`);
  return value;
}
function exactKeys(value: Record<string, unknown>, expected: readonly string[]): void {
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) throw new Error("unexpected object shape");
}
function text(value: unknown, field: string): string {
  return boundedText(value, field, MAX_TEXT);
}
function boundedText(value: unknown, field: string, max: number): string {
  if (typeof value !== "string" || value.length === 0 || value.length > max) throw new Error(`${field} must be a bounded non-empty string`);
  return value;
}
function bool(value: unknown, field: string): boolean {
  if (typeof value !== "boolean") throw new Error(`${field} must be boolean`);
  return value;
}
function count(value: unknown, field: string): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) throw new Error(`${field} must be a non-negative safe integer`);
  return value;
}
function nonNegative(value: unknown, field: string): number {
  if (typeof value !== "number" || !Number.isFinite(value) || value < 0) throw new Error(`${field} must be finite and non-negative`);
  return value;
}
function positive(value: unknown, field: string): number {
  const result = nonNegative(value, field);
  if (result === 0) throw new Error(`${field} must be positive`);
  return result;
}
function fraction(value: unknown, field: string): number {
  const result = nonNegative(value, field);
  if (result > 1) throw new Error(`${field} must be a fraction`);
  return result;
}
function time(value: unknown, field: string): string {
  const result = text(value, field);
  if (!RFC3339.test(result) || !Number.isFinite(Date.parse(result))) throw new Error(`${field} must be RFC3339`);
  return result;
}
function nullableTime(value: unknown, field: string): string | null {
  return value === null ? null : time(value, field);
}
function member<const T extends string>(value: unknown, members: readonly T[], field: string): T {
  if (typeof value !== "string" || !members.includes(value as T)) throw new Error(`${field} has an unknown value`);
  return value as T;
}
