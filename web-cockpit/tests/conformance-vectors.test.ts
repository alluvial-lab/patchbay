import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import test from "node:test";

import { create, toBinary } from "@bufbuild/protobuf";
import {
  ActorEndpointRefSchema,
  ActorIdSchema,
  AdapterIdSchema,
  AdapterSnapshotSupport,
  AuthorityDomainIdSchema,
  EventIdSchema,
  GenerationSchema,
  LsnSchema,
  ObservationKind,
  ObservationSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  PiPersistedPresentationItemSchema,
  PiPersistedProjectionEntrySchema,
  PiPersistedProjectionReplacementSchema,
  ResourceFreshnessState,
  ResourceSnapshotSchema,
  RuntimeSessionIdSchema,
  SessionActivityState,
  SessionConnectivityState,
  SessionRegisteredSchema,
  SessionSchema,
  SessionSnapshotSchema,
  SessionStateEventSchema,
  SessionStateSchema,
  StoredEventKind,
  StoredEventPayloadSchema,
  SubscribeEventSchema,
  TargetScopeKind,
  TargetScopeSchema,
} from "@patchbay/contracts";
import {
  TOKEN_COMMUNE_PRESENTATION_CONTRACT, composeTokenCommunePools, decodeTokenCommuneProjection,
  type TokenCommunePoolSummary, type TokenCommuneResourceInput,
} from "@patchbay/operator-domain";
import { JSDOM } from "jsdom";

import {
  PresentationProjection,
  emptyPresentationModel,
  fold,
  rendersLive,
  rendersResourceCurrent,
  resourceCollectionKey,
  resourceKey,
  sessionKey,
  type ResourceView,
} from "../src/domain/model.js";
import type { ProviderPoolProjection } from "../src/domain/resource-projection.js";
import { renderResourceDestination } from "../src/ui/resource-view.js";
import { renderTokenCommunePanel } from "../src/ui/token-commune-panel.js";
import {
  ProductionMutantHarnessError, withProductionMutant, type ProductionReplacement,
} from "./production-mutant.js";

const RUNNER = "web-cockpit" as const;

interface ImplementationCheck {
  runner: string;
  case: string;
}

interface MutationWitness { mutation_id: string; runner: string; invariant: string }
interface ConformanceVector {
  vector_id: string;
  property_id: string;
  promotion_status: string;
  implementation_checks?: readonly ImplementationCheck[];
  mutation_witnesses?: readonly MutationWitness[];
  input: unknown;
  expected_outcome: unknown;
}

interface RequestedCheck {
  vector_id: string;
  case: string;
}
interface RequestedMutation { vector_id: string; mutation_id: string }

function vectorsForRunner(): ReadonlyMap<string, ConformanceVector> {
  const directory = path.resolve(process.cwd(), "../contracts/vectors");
  const vectors = readdirSync(directory)
    .filter((filename) => filename.endsWith(".json"))
    .sort()
    .map((filename) => JSON.parse(readFileSync(path.join(directory, filename), "utf8")) as ConformanceVector);
  return new Map(vectors.map((vector) => [vector.vector_id, vector]));
}

function requestedChecks(): readonly RequestedCheck[] {
  return process.env.PATCHBAY_CONFORMANCE_REQUESTS
    ? JSON.parse(process.env.PATCHBAY_CONFORMANCE_REQUESTS) as RequestedCheck[]
    : [];
}
function requestedMutations(): readonly RequestedMutation[] {
  return process.env.PATCHBAY_CONFORMANCE_MUTATIONS
    ? JSON.parse(process.env.PATCHBAY_CONFORMANCE_MUTATIONS) as RequestedMutation[]
    : [];
}

function object(value: unknown, name: string): Record<string, unknown> {
  assert.ok(value && typeof value === "object" && !Array.isArray(value), `${name} must be an object`);
  return value as Record<string, unknown>;
}

function text(value: unknown, name: string): string {
  assert.equal(typeof value, "string", `${name} must be a string`);
  return value as string;
}

function bool(value: unknown, name: string): boolean {
  assert.equal(typeof value, "boolean", `${name} must be a boolean`);
  return value as boolean;
}

function freshness(value: unknown): ResourceFreshnessState {
  switch (text(value, "freshness")) {
    case "RESOURCE_FRESHNESS_STATE_CURRENT": return ResourceFreshnessState.CURRENT;
    case "RESOURCE_FRESHNESS_STATE_STALE": return ResourceFreshnessState.STALE;
    case "RESOURCE_FRESHNESS_STATE_UNKNOWN": return ResourceFreshnessState.UNKNOWN;
    default: throw new Error(`unknown vector freshness ${String(value)}`);
  }
}

function executeStalePresentation(vector: ConformanceVector): void {
  const input = object(vector.input, "input");
  const expected = object(vector.expected_outcome, "expected_outcome");
  const tuple = input.resource_identity;
  assert.ok(Array.isArray(tuple) && tuple.length === 3 && tuple.every((part) => typeof part === "string"));
  const projection = object(input.current_projection, "current_projection");
  const snapshot = object(expected.snapshot_record, "snapshot_record");
  const identity = { adapterId: tuple[0] as string, resourceKind: tuple[1] as string, resourceId: tuple[2] as string };
  const base: ResourceView = {
    identity,
    freshness: freshness(snapshot.freshness),
    sourceAdapterGeneration: BigInt(input.adapter_generation as number),
    revisionLsn: 2n,
    tombstoned: bool(snapshot.tombstoned, "snapshot tombstoned"),
    hasCachedPayload: bool(snapshot.has_cached_payload, "snapshot cache presence"),
    reconciled: true,
    projection: {
      status: "decoded",
      value: {
        kind: "pooled-provider-pool",
        displayName: text(projection.displayName, "projection displayName"),
        providerLabel: text(projection.providerLabel, "projection providerLabel"),
        health: text(projection.health, "projection health") as ProviderPoolProjection["health"],
        remainingPercent: Number(projection.remainingPercent),
        serviceLabel: text(projection.serviceLabel, "projection serviceLabel"),
        controlPosture: text(projection.controlPosture, "projection controlPosture") as ProviderPoolProjection["controlPosture"],
      },
    },
  };
  assert.equal(rendersResourceCurrent(base), false);
  const model = emptyPresentationModel();
  model.reconciled = true;
  model.resources.set(resourceKey(identity), base);
  model.resourceCollections.set(resourceCollectionKey(identity.adapterId, identity.resourceKind), {
    adapterId: identity.adapterId,
    resourceKind: identity.resourceKind,
    completeness: AdapterSnapshotSupport.PARTIAL,
    sourceAdapterGeneration: base.sourceAdapterGeneration,
    revisionLsn: base.revisionLsn,
    reconciled: true,
  });
  const dom = new JSDOM();
  const rendered = renderResourceDestination(dom.window.document, model, {
    selectedKey: resourceKey(identity), mobileDetailOpen: true, lockdownActive: false,
    onSelect() {}, onBack() {},
  });
  assert.equal(rendered.element.querySelector(".resource-freshness--current"), null);
  assert.match(rendered.element.textContent ?? "", new RegExp(text(expected.stale_wording, "stale wording"), "i"));
  assert.equal(bool(expected.adapter_health_overrides_freshness, "health override"), false);

  const eligibility = object(expected.current_eligibility, "current_eligibility");
  const current = {
    ...base,
    freshness: freshness(eligibility.freshness),
    reconciled: bool(eligibility.reconciled, "current reconciled"),
    tombstoned: bool(eligibility.tombstoned, "current tombstoned"),
  };
  assert.equal(rendersResourceCurrent(current), true);
  const disallowedExpected = bool(expected.disallowed_states_render_current, "disallowed current");
  assert.equal(rendersResourceCurrent({ ...current, reconciled: false }), disallowedExpected);
  assert.equal(rendersResourceCurrent({ ...current, tombstoned: true }), disallowedExpected);
  assert.equal(rendersResourceCurrent({ ...current, freshness: ResourceFreshnessState.STALE }), disallowedExpected);
  assert.equal(rendersResourceCurrent({ ...current, freshness: ResourceFreshnessState.UNKNOWN }), disallowedExpected);

  const unknown = { ...base, freshness: ResourceFreshnessState.UNKNOWN, hasCachedPayload: false, projection: { status: "unavailable" as const } };
  const unknownModel = emptyPresentationModel();
  unknownModel.resources.set(resourceKey(identity), unknown);
  unknownModel.resourceCollections.set(resourceCollectionKey(identity.adapterId, identity.resourceKind), model.resourceCollections.values().next().value!);
  const unknownRendered = renderResourceDestination(dom.window.document, unknownModel, {
    selectedKey: resourceKey(identity), mobileDetailOpen: true, lockdownActive: false,
    onSelect() {}, onBack() {},
  });
  assert.equal(Boolean(unknownRendered.element.querySelector(".resource-meter, .resource-health")), bool(expected.unknown_exposes_domain_health_or_meter, "unknown domain claims"));
}

function tokenProjectionEnvelope(schemaRef: string, value: unknown) {
  return create(PayloadEnvelopeSchema, {
    schemaRef, contentType: PayloadContentType.JSON,
    payload: new TextEncoder().encode(JSON.stringify(value)),
  });
}

function tokenInput(
  vector: ConformanceVector,
  options: { freshness?: ResourceFreshnessState; projection?: unknown } = {},
): TokenCommuneResourceInput {
  const input = object(vector.input, "input");
  const identity = {
    adapterId: text(input.adapter_id, "adapter id"),
    resourceKind: TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.resourceKind,
    resourceId: "local:provider-pool:conformance:openai-codex",
  };
  const decoded = options.projection === undefined
    ? decodeTokenCommuneProjection(
      identity,
      tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, {}),
      tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, input.expected_projection),
    )!
    : options.projection;
  return {
    identity,
    freshness: options.freshness ?? ResourceFreshnessState.CURRENT,
    completeness: AdapterSnapshotSupport.PARTIAL,
    observedAt: new Date("2026-08-08T12:00:00.000Z"),
    reconciled: true,
    tombstoned: false,
    projection: decoded as TokenCommuneResourceInput["projection"],
  };
}

function tokenDrawInput(vector: ConformanceVector, adapterId: string): TokenCommuneResourceInput {
  const input = object(vector.input, "input");
  const identity = {
    adapterId,
    resourceKind: TOKEN_COMMUNE_PRESENTATION_CONTRACT.memberDraw.resourceKind,
    resourceId: `local:member-draw:conformance:${adapterId}:openai-codex`,
  };
  return {
    identity,
    freshness: ResourceFreshnessState.CURRENT,
    completeness: AdapterSnapshotSupport.PARTIAL,
    observedAt: new Date("2026-08-08T12:00:00.000Z"),
    reconciled: true,
    tombstoned: false,
    projection: decodeTokenCommuneProjection(
      identity,
      tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.memberDraw.payloadSchema, {}),
      tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.memberDraw.projectionSchema, input.expected_draw_projection),
    )!,
  };
}

function tokenResources(vector: ConformanceVector): readonly TokenCommuneResourceInput[] {
  const adapterId = text(object(vector.input, "input").adapter_id, "adapter id");
  return [tokenInput(vector), tokenDrawInput(vector, adapterId), tokenDrawInput(vector, `${adapterId}-competitor`)];
}

function tokenPanel(
  summary: TokenCommunePoolSummary,
  recentEvents: Parameters<typeof renderTokenCommunePanel>[1]["recentEvents"] = [],
) {
  const dom = new JSDOM("<!doctype html><html><body><main></main></body></html>", { runScripts: "dangerously" });
  const panel = renderTokenCommunePanel(dom.window.document, {
    summaries: [summary], recentEvents, partial: true,
    refreshedAt: new Date("2026-08-08T12:00:00.000Z"),
    formatNow: new Date("2026-08-08T12:05:00.000Z"),
  });
  dom.window.document.querySelector("main")!.append(panel);
  return { dom, panel };
}

function assertHonestTokenPanel(panel: HTMLElement, expected: Record<string, unknown>): void {
  const markup = panel.outerHTML;
  assert.equal(Boolean(panel.querySelector(".token-commune-pool--stale .token-commune-verdict--run")), false);
  assert.equal(markup.includes("gpt-5.6"), bool(expected.forbidden_alias_visible, "forbidden alias visible"));
  assert.equal(/private contributor|private member|private-sub-key|anonymous-contribution/i.test(markup), bool(expected.private_fields_visible, "private fields visible"));
  assert.equal((panel.textContent ?? "").includes("Verdicts are a Patchbay synthesis"), expected.verdict_owner === "Patchbay");
  assert.equal((panel.ownerDocument.defaultView as any)?.__pwned === true, bool(expected.dynamic_renderer_executed, "renderer executed"));
}

function executeTokenCommunePresentation(vector: ConformanceVector): void {
  const expected = object(vector.expected_outcome, "expected outcome");
  const current = composeTokenCommunePools(tokenResources(vector))[0]!;
  assert.equal(current.provider, text(expected.current_provider, "current provider"));
  assert.equal(current.capacity5h.state, "current");
  if (current.capacity5h.state === "current") assert.equal(current.capacity5h.usedFraction, Number(expected.current_capacity_used_fraction));
  assert.equal(current.verdict, "runnable");
  assert.equal(current.totalDeclaredShare, 1.5);
  assert.equal(current.fingerprint.status, "unknown");
  assert.equal(current.draw.state, "current", "only the exact adapter/provider member draw joins");
  if (current.draw.state === "current") assert.equal(current.draw.limitFraction, Number(expected.current_draw_limit_fraction));
  assert.equal(current.drawIdentity?.adapterId, text(object(vector.input, "input").adapter_id, "adapter id"));
  const recentEvents = [
    { poolIdentity: current.poolIdentity, kind: "event-gap" as const, code: "window-discontinuity", occurredAt: new Date("2026-08-08T12:04:00.000Z") },
    { poolIdentity: current.poolIdentity, kind: "pool-event" as const, code: "capacity_shift", occurredAt: new Date("2026-08-08T12:03:00.000Z") },
  ];
  const currentRendered = tokenPanel(current, recentEvents);
  assertHonestTokenPanel(currentRendered.panel, expected);
  const currentText = currentRendered.panel.textContent ?? "";
  assert.equal(currentText.includes("fingerprint unknown"), bool(expected.safe_fingerprint_visible, "fingerprint visible"));
  assert.equal(currentText.includes("150% total declared share"), bool(expected.total_declared_share_visible, "declared share visible"));
  assert.equal(currentText.includes("10 units consumed"), bool(expected.draw_consumed_units_visible, "draw consumed units visible"));
  assert.equal(currentText.includes("reset unavailable"), bool(expected.draw_reset_visible, "draw reset visible"));
  assert.equal((currentText.match(/reset unavailable/g) ?? []).length >= 2, bool(expected.capacity_reset_visible, "capacity reset visible"));
  assert.equal(currentText.includes("reading 9m ago"), bool(expected.old_reading_age_visible_under_current_wrapper, "reading age visible"));
  assert.equal(currentText.includes("gap window-discontinuity") && currentText.includes("pool capacity_shift"), bool(expected.resource_events_visible, "resource events visible"));

  const stale = composeTokenCommunePools([tokenInput(vector, { freshness: ResourceFreshnessState.STALE })])[0]!;
  assert.equal(stale.verdict, "telemetry-stale");
  const staleRendered = tokenPanel(stale);
  assert.ok(staleRendered.panel.querySelector(".token-commune-pool--stale"));
  assert.equal(staleRendered.panel.querySelector(".token-commune-verdict--run"), null);

  const unknown = composeTokenCommunePools([tokenInput(vector, { projection: { status: "unavailable" } })]);
  assert.equal(unknown.length > 0, bool(expected.unknown_rows_visible, "unknown rows visible"));
  assert.equal(unknown[0]?.verdict, "unknown");
  assert.match(tokenPanel(unknown[0]!).panel.textContent ?? "", /unknown/);

  const input = object(vector.input, "input");
  const crossProjection = structuredClone(input.expected_projection) as any;
  crossProjection.modelCatalog.models[0].provider = "anthropic";
  const cross = decodeTokenCommuneProjection(
    tokenInput(vector).identity,
    tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, {}),
    tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, crossProjection),
  )!;
  const crossSummary = composeTokenCommunePools([tokenInput(vector, { projection: cross })])[0]!;
  assert.equal(crossSummary.verdict === "runnable", bool(expected.cross_provider_model_runnable, "cross-provider runnable"));
  assert.equal(crossSummary.models.some((model) => model.provider === "anthropic"), bool(expected.cross_provider_model_visible, "cross-provider visible"));
  assert.equal(tokenPanel(crossSummary).panel.textContent?.includes("claude-cross-pool") ?? false, false);

  const oldProjection = structuredClone(input.expected_projection) as any;
  oldProjection.contributionListing.contributions[1].capacityReadings[0].observedAt = "2026-08-08T06:00:00.000Z";
  const oldSummary = composeTokenCommunePools([tokenInput(vector, { projection: decodeTokenCommuneProjection(
    tokenInput(vector).identity,
    tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, {}),
    tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, oldProjection),
  )! })])[0]!;
  const oldRow = tokenPanel(oldSummary).panel.querySelector<HTMLElement>(".token-commune-pool")!;
  assert.equal(oldRow.textContent?.includes("reading 6h ago") ?? false, bool(expected.old_reading_age_visible_under_current_wrapper, "old reading age visible"));
  assert.match(oldRow.textContent ?? "", /wrapper 5m ago · current/);

  for (const [state, expectedTelemetry] of [
    ["no-5h-readings", expected.no_5h_readings_telemetry],
    ["all-null-5h-readings", expected.all_null_5h_readings_telemetry],
  ] as const) {
    const missingProjection = structuredClone(input.expected_projection) as any;
    for (const contribution of missingProjection.contributionListing.contributions) {
      if (state === "no-5h-readings") {
        contribution.telemetryState = "no-readings";
        contribution.capacityReadings = [];
      } else {
        for (const reading of contribution.capacityReadings) reading.usedFraction = null;
      }
    }
    const missingSummary = composeTokenCommunePools([tokenInput(vector, { projection: decodeTokenCommuneProjection(
      tokenInput(vector).identity,
      tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, {}),
      tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, missingProjection),
    )! })])[0]!;
    const row = tokenPanel(missingSummary).panel.querySelector<HTMLElement>(".token-commune-pool")!;
    assert.equal(row.dataset.telemetry, expectedTelemetry);
  }

  const aliasProjection = structuredClone(input.expected_projection) as any;
  aliasProjection.modelCatalog.models[0].id = "gpt-5.6";
  const alias = decodeTokenCommuneProjection(
    tokenInput(vector).identity,
    tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, {}),
    tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, aliasProjection),
  )!;
  assert.equal(alias.status, "invalid");

  const hostileProjection = { ...(input.expected_projection as object), ...(input.hostile_fields as object) };
  const hostile = decodeTokenCommuneProjection(
    tokenInput(vector).identity,
    tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, {}),
    tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, hostileProjection),
  )!;
  assert.equal(hostile.status, "invalid");
  const hostileRendered = tokenPanel(composeTokenCommunePools([tokenInput(vector, { projection: hostile })])[0]!);
  assertHonestTokenPanel(hostileRendered.panel, expected);
}

async function expectWebProductionMutationKilled(
  vector: ConformanceVector,
  packageRoot: string,
  replacements: readonly ProductionReplacement[],
  entry: string,
  mutant: (module: Record<string, any>) => Promise<void> | void,
  mutationId: string,
): Promise<void> {
  executeTokenCommunePresentation(vector);
  let moduleLoaded = false;
  let killed = false;
  try {
    await withProductionMutant(packageRoot, replacements, entry, async (module) => {
      moduleLoaded = true;
      await mutant(module);
    });
  } catch (error) {
    if (error instanceof ProductionMutantHarnessError) throw error;
    assert.equal(moduleLoaded, true, `production mutation ${mutationId} failed before module loading completed`);
    killed = true;
  }
  assert.equal(moduleLoaded, true, `production mutation ${mutationId} did not load`);
  assert.equal(killed, true, `production mutation ${mutationId} survived the presentation oracle`);
}

async function killWebMutation(vector: ConformanceVector, mutationId: string): Promise<void> {
  const input = object(vector.input, "input");
  const expected = object(vector.expected_outcome, "expected outcome");
  const operatorRoot = path.resolve(process.cwd(), "../operator-domain");
  const webRoot = process.cwd();
  const operatorMutation = async (
    replacements: readonly ProductionReplacement[],
    mutant: (module: Record<string, any>) => Promise<void> | void,
  ) => expectWebProductionMutationKilled(vector, operatorRoot, replacements, "token-commune.js", mutant, mutationId);
  const rendererMutation = async (
    replacements: readonly ProductionReplacement[],
    mutant: (module: Record<string, any>) => Promise<void> | void,
  ) => expectWebProductionMutationKilled(vector, webRoot, replacements, "ui/token-commune-panel.js", mutant, mutationId);

  switch (mutationId) {
    case "style-stale-as-live":
      await operatorMutation([{
        file: "token-commune.js", from: "if (!input.poolCurrent)\n        return \"telemetry-stale\";", to: "if (!input.poolCurrent)\n        return \"runnable\";",
      }], (module) => {
        const stale = module.composeTokenCommunePools([tokenInput(vector, { freshness: ResourceFreshnessState.STALE })])[0];
        const panel = tokenPanel(stale).panel;
        assert.ok(panel.querySelector(".token-commune-pool--stale"));
        assert.equal(panel.querySelector(".token-commune-verdict--run"), null);
      });
      return;
    case "drop-unknown-row":
      await operatorMutation([{
        file: "token-commune.js", from: "            pools.push({\n                key: identityKey(pool.identity),", to: "            if (true) continue;\n            pools.push({\n                key: identityKey(pool.identity),",
      }], (module) => {
        const rows = module.composeTokenCommunePools([tokenInput(vector, { projection: { status: "unavailable" } })]);
        assert.equal(rows.length > 0, bool(expected.unknown_rows_visible, "unknown rows visible"));
      });
      return;
    case "join-draw-by-provider-only":
      await operatorMutation([{
        file: "token-commune.js", from: "const exactDraws = draws.filter((draw) => draw.identity.adapterId === pool.identity.adapterId\n            && draw.projection.status", to: "const exactDraws = draws.filter((draw) => draw.projection.status",
      }], (module) => {
        const summary = module.composeTokenCommunePools(tokenResources(vector))[0];
        assert.equal(summary.draw.state, "current", "exact adapter/provider join must select one native draw");
        assert.equal(summary.drawIdentity?.adapterId, text(input.adapter_id, "adapter id"));
      });
      return;
    case "accept-forbidden-gpt-5-6-alias":
      await operatorMutation([{
        file: "token-commune.js", from: "if (id === \"gpt-5.6\")\n                throw new Error(\"rejected model alias\");", to: "if (false)\n                throw new Error(\"rejected model alias\");",
      }], (module) => {
        const aliasProjection = structuredClone(input.expected_projection) as any;
        aliasProjection.modelCatalog.models[0].id = "gpt-5.6";
        const decoded = module.decodeTokenCommuneProjection(
          tokenInput(vector).identity,
          tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, {}),
          tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, aliasProjection),
        );
        assert.equal(decoded.status, "invalid");
      });
      return;
    case "display-cross-provider-model":
      await operatorMutation([{
        file: "token-commune.js",
        from: "if (modelCatalog.status === \"reported\" && modelCatalog.models.some((model) => model.provider !== provider)) {",
        to: "if (false && modelCatalog.status === \"reported\" && modelCatalog.models.some((model) => model.provider !== provider)) {",
      }], (module) => {
        const projection = structuredClone(input.expected_projection) as any;
        projection.modelCatalog.models[0].provider = "anthropic";
        const decoded = module.decodeTokenCommuneProjection(
          tokenInput(vector).identity,
          tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, {}),
          tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, projection),
        );
        const summary = module.composeTokenCommunePools([tokenInput(vector, { projection: decoded })])[0];
        assert.equal(summary.models.some((model: any) => model.provider === "anthropic"), bool(expected.cross_provider_model_visible, "cross-provider visible"));
      });
      return;
    case "drop-carried-capabilities":
      await rendererMutation([{
        file: "ui/token-commune-panel.js", from: "`${fingerprintLabel(summary)} · wrapper", to: "`fingerprint omitted · wrapper",
      }], (module) => {
        const summary = composeTokenCommunePools(tokenResources(vector))[0]!;
        const panel = module.renderTokenCommunePanel(tokenPanel(summary).dom.window.document, { summaries: [summary], recentEvents: [], partial: true });
        assert.equal((panel.textContent ?? "").includes("fingerprint unknown"), bool(expected.safe_fingerprint_visible, "fingerprint visible"));
      });
      return;
    case "hide-current-wrapper-old-reading-age":
      await rendererMutation([{
        file: "ui/token-commune-panel.js",
        from: "return `reading ${age(new Date(summary.capacity5h.observedAt), now)} · ${summary.capacity5h.state} · reset ${resetLabel(summary.capacity5h.resetsAt)}`;",
        to: "return `highest 5h utilization · ${summary.capacity5h.state}`;",
      }], (module) => {
        const projection = structuredClone(input.expected_projection) as any;
        projection.contributionListing.contributions[1].capacityReadings[0].observedAt = "2026-08-08T06:00:00.000Z";
        const decoded = decodeTokenCommuneProjection(
          tokenInput(vector).identity,
          tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, {}),
          tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, projection),
        )!;
        const summary = composeTokenCommunePools([tokenInput(vector, { projection: decoded })])[0]!;
        const panel = module.renderTokenCommunePanel(tokenPanel(summary).dom.window.document, { summaries: [summary], recentEvents: [], partial: true, formatNow: new Date("2026-08-08T12:05:00Z") });
        assert.equal((panel.textContent ?? "").includes("reading 6h ago"), bool(expected.old_reading_age_visible_under_current_wrapper, "old reading age visible"));
      });
      return;
    case "label-missing-reading-current":
      await rendererMutation([{
        file: "ui/token-commune-panel.js",
        from: "if (summary.capacity5h.state === \"no-5h-readings\" || summary.capacity5h.state === \"reading-unavailable\")\n        return \"unavailable\";",
        to: "if (false)\n        return \"unavailable\";",
      }], (module) => {
        const projection = structuredClone(input.expected_projection) as any;
        for (const contribution of projection.contributionListing.contributions) {
          contribution.telemetryState = "no-readings";
          contribution.capacityReadings = [];
        }
        const decoded = decodeTokenCommuneProjection(
          tokenInput(vector).identity,
          tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, {}),
          tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, projection),
        )!;
        const summary = composeTokenCommunePools([tokenInput(vector, { projection: decoded })])[0]!;
        const panel = module.renderTokenCommunePanel(tokenPanel(summary).dom.window.document, { summaries: [summary], recentEvents: [], partial: true });
        assert.equal(panel.querySelector(".token-commune-pool")?.dataset.telemetry, expected.no_5h_readings_telemetry);
      });
      return;
    case "drop-resource-events":
      await rendererMutation([{
        file: "ui/token-commune-panel.js",
        from: "const visible = events.filter((event) => sameIdentity(event.poolIdentity, summary.poolIdentity)).slice(0, 3);",
        to: "const visible = [];",
      }], (module) => {
        const summary = composeTokenCommunePools(tokenResources(vector))[0]!;
        const events = [{ poolIdentity: summary.poolIdentity, kind: "event-gap", code: "window-discontinuity", occurredAt: new Date("2026-08-08T12:04:00Z") }];
        const panel = module.renderTokenCommunePanel(tokenPanel(summary).dom.window.document, { summaries: [summary], recentEvents: events, partial: true });
        assert.equal((panel.textContent ?? "").includes("gap window-discontinuity"), bool(expected.resource_events_visible, "resource events visible"));
      });
      return;
    case "expose-contributor-member-subkey":
      await operatorMutation([
        { file: "token-commune.js", from: "function exactKeys(value, expected) {\n    const actual", to: "function exactKeys(value, expected) {\n    return;\n    const actual" },
        { file: "token-commune.js", from: "kind: \"token-commune-provider-pool\",\n        provider,", to: "kind: \"token-commune-provider-pool\",\n        provider: value.member ?? value.subKey ?? provider," },
      ], (module) => {
        const hostileProjection = { ...(input.expected_projection as object), ...(input.hostile_fields as object) };
        const decoded = module.decodeTokenCommuneProjection(
          tokenInput(vector).identity,
          tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, {}),
          tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, hostileProjection),
        );
        const summary = module.composeTokenCommunePools([tokenInput(vector, { projection: decoded })])[0];
        assertHonestTokenPanel(tokenPanel(summary).panel, expected);
      });
      return;
    case "trust-adapter-verdict":
      await operatorMutation([
        { file: "token-commune.js", from: "function exactKeys(value, expected) {\n    const actual", to: "function exactKeys(value, expected) {\n    return;\n    const actual" },
        { file: "token-commune.js", from: "const verdict = synthesizeTokenCommuneVerdict({", to: "const verdict = value.verdict ?? synthesizeTokenCommuneVerdict({" },
      ], (module) => {
        const hostileProjection = { ...(input.expected_projection as object), ...(input.hostile_fields as object) };
        const decoded = module.decodeTokenCommuneProjection(
          tokenInput(vector).identity,
          tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.payloadSchema, {}),
          tokenProjectionEnvelope(TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.projectionSchema, hostileProjection),
        );
        const summary = module.composeTokenCommunePools([tokenInput(vector, { projection: decoded })])[0];
        assert.notEqual(summary.verdict, object(input.hostile_fields, "hostile").verdict);
      });
      return;
    case "load-dynamic-renderer":
      await rendererMutation([{
        file: "ui/token-commune-panel.js", from: "function renderPoolRow(document, summary, options) {\n    const row", to: "function renderPoolRow(document, summary, options) {\n    if (summary.rendererUrl) document.defaultView.__pwned = true;\n    const row",
      }], (module) => {
        const summary = { ...composeTokenCommunePools(tokenResources(vector))[0]!, rendererUrl: object(input.hostile_fields, "hostile").rendererUrl };
        const dom = new JSDOM("<!doctype html><html><body></body></html>", { runScripts: "dangerously" });
        const panel = module.renderTokenCommunePanel(dom.window.document, { summaries: [summary], partial: true });
        assertHonestTokenPanel(panel, expected);
      });
      return;
    case "render-hostile-html-script":
      await rendererMutation([{
        file: "ui/token-commune-panel.js", from: "element.textContent = value;", to: "element.innerHTML = value;",
      }], (module) => {
        const summary = { ...composeTokenCommunePools(tokenResources(vector))[0]!, provider: text(object(input.hostile_fields, "hostile").html, "hostile html") };
        const dom = new JSDOM("<!doctype html><html><body></body></html>", { runScripts: "dangerously" });
        const panel = module.renderTokenCommunePanel(dom.window.document, { summaries: [summary], partial: true });
        assert.equal(panel.querySelector("img, script"), null);
      });
      return;
    default: throw new Error(`unhandled web mutation ${vector.vector_id}:${mutationId}`);
  }
}

function whole(value: unknown, name: string): bigint {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    throw new Error(`${name} must be a non-negative safe integer`);
  }
  return BigInt(value);
}

function executeSpawnReconnectSurface(vector: ConformanceVector): void {
  const input = object(vector.input, "input");
  const expected = object(vector.expected_outcome, "expected outcome");
  const domain = create(AuthorityDomainIdSchema, {
    value: text(input.authority_domain_id, "authority domain"),
  });
  const adapterId = create(AdapterIdSchema, { value: text(input.adapter_id, "adapter id") });
  const deploymentScope = text(input.deployment_scope, "deployment scope");
  const cachedGeneration = whole(input.cached_generation, "cached generation");
  const promotedGeneration = whole(input.promoted_generation, "promoted generation");
  const cachedLsn = whole(input.cached_lsn, "cached LSN");
  const promotionLsn = whole(input.promotion_lsn, "promotion LSN");
  const coreGeneration = create(GenerationSchema, {
    value: whole(input.core_generation, "core generation"),
  });
  const priorRuntime = create(RuntimeSessionIdSchema, { value: "runtime-n" });
  const successorRuntime = create(RuntimeSessionIdSchema, { value: "runtime-n-plus-one" });

  const registrationEvent = (lsn: bigint, runtimeSessionId: typeof priorRuntime, generation: bigint) => {
    const registered = create(SessionRegisteredSchema, {
      adapterId,
      deploymentScope,
      runtimeSessionId,
      sessionGeneration: create(GenerationSchema, { value: generation }),
      initialState: create(SessionStateSchema, {
        connectivity: SessionConnectivityState.LIVE,
        activity: SessionActivityState.WORKING,
      }),
    });
    return create(SubscribeEventSchema, {
      eventId: create(EventIdSchema, {
        authorityDomainId: domain,
        lsn: create(LsnSchema, { value: lsn }),
      }),
      payload: create(StoredEventPayloadSchema, {
        kind: StoredEventKind.SESSION_STATE,
        payload: toBinary(SessionStateEventSchema, create(SessionStateEventSchema, {
          authorityDomainId: domain,
          mutation: { case: "registered", value: registered },
        })),
      }),
    });
  };

  const projection = new PresentationProjection();
  projection.bindCoreLineage(domain.value, coreGeneration.value);
  projection.foldEvent(registrationEvent(cachedLsn, priorRuntime, cachedGeneration));
  projection.markUnreconciled();
  projection.replaceFromSnapshots({
    session: create(SessionSnapshotSchema, {
      authorityDomainId: domain,
      coreGeneration,
      snapshotLsn: create(LsnSchema, { value: promotionLsn }),
      sessions: [create(SessionSchema, {
        authorityDomainId: domain,
        adapterId,
        deploymentScope,
        runtimeSessionId: successorRuntime,
        sessionGeneration: create(GenerationSchema, { value: promotedGeneration }),
        state: create(SessionStateSchema, {
          connectivity: SessionConnectivityState.LIVE,
          activity: SessionActivityState.IDLE,
        }),
        lastAuthoritativeLsn: create(LsnSchema, { value: promotionLsn }),
      })],
    }),
    resource: create(ResourceSnapshotSchema, {
      authorityDomainId: domain,
      coreGeneration,
      snapshotLsn: create(LsnSchema, { value: promotionLsn }),
    }),
  }, []);

  const priorKey = sessionKey({
    adapterId: adapterId.value,
    deploymentScope,
    runtimeSessionId: priorRuntime.value,
    generation: cachedGeneration,
  });
  const successorKey = sessionKey({
    adapterId: adapterId.value,
    deploymentScope,
    runtimeSessionId: successorRuntime.value,
    generation: promotedGeneration,
  });
  assert.equal(projection.model.sessions.has(priorKey), expected.surface_tombstoned_or_cached_generation_visible);
  assert.equal(projection.model.sessions.get(successorKey)?.identity.generation, BigInt(expected.surface_current_generation as number));
  assert.equal(rendersLive(projection.model.sessions.get(successorKey)!), true);

  const repaired = projection.model;
  projection.foldEvent(registrationEvent(promotionLsn, priorRuntime, cachedGeneration));
  assert.equal(projection.model, repaired, "remembered equal-LSN stream evidence must be inert");
  assert.equal(projection.model.sessions.has(priorKey), expected.remembered_equal_lsn_overwrites_repair);

  assert.throws(() => projection.replaceFromSnapshots({
    session: create(SessionSnapshotSchema, {
      authorityDomainId: domain,
      coreGeneration,
      snapshotLsn: create(LsnSchema, { value: promotionLsn }),
      sessions: [create(SessionSchema, {
        authorityDomainId: domain,
        adapterId,
        deploymentScope,
        runtimeSessionId: priorRuntime,
        sessionGeneration: create(GenerationSchema, { value: cachedGeneration }),
        state: create(SessionStateSchema, {
          connectivity: SessionConnectivityState.LIVE,
          activity: SessionActivityState.WORKING,
        }),
        lastAuthoritativeLsn: create(LsnSchema, { value: cachedLsn }),
      })],
    }),
    resource: create(ResourceSnapshotSchema, {
      authorityDomainId: domain,
      coreGeneration,
      snapshotLsn: create(LsnSchema, { value: promotionLsn }),
    }),
  }, []), /not newer than cached core authority/);
  assert.equal(projection.model, repaired, "rejected cached-N replacement must install nothing");
}

function textList(value: unknown, name: string): readonly string[] {
  assert.ok(Array.isArray(value) && value.every((item) => typeof item === "string"), `${name} must be a string array`);
  return value;
}

function executePiAuthoritativeReplacementPresentation(vector: ConformanceVector): void {
  const input = object(vector.input, "input");
  const expected = object(vector.expected_outcome, "expected outcome");
  const oldIds = textList(input.old_projection_ids, "old projection ids");
  const nextIds = textList(input.replacement_projection_ids, "replacement projection ids");
  const continuityId = `pi1:${"a".repeat(43)}`;
  const event = (lsn: bigint, epoch: bigint, ids: readonly string[]) => {
    const entries = ids.map((id, index) => ({
      stableEntryId: id,
      parentEntryId: index === 0 ? null : ids[0]!,
      contentDigest: createHash("sha256").update(id).digest("hex"),
      presentationItems: index === 0 ? [] : [{
        membershipId: `membership:${id}`,
        transcriptEventJson: JSON.stringify({
          kind: "user_confirmed", eventId: `event:${id}`, sessionId: continuityId,
          ts: 1, messageId: id, text: id,
        }),
      }],
    }));
    const treeDigest = createHash("sha256")
      .update(JSON.stringify(entries.map((entry) => [entry.stableEntryId, entry.parentEntryId])))
      .digest("hex");
    const cursor = ids.at(-1)!;
    const batchHash = createHash("sha256");
    for (const part of ["replacement", continuityId, epoch.toString(), canonicalPi(entries), cursor, cursor, treeDigest]) {
      batchHash.update(`${Buffer.byteLength(part)}:${part}\0`);
    }
    const payload = toBinary(PiPersistedProjectionReplacementSchema, create(PiPersistedProjectionReplacementSchema, {
      externalContinuityId: continuityId,
      replacementEpoch: epoch,
      batchId: batchHash.digest("hex"),
      exactEntries: entries.map((entry) => create(PiPersistedProjectionEntrySchema, {
        stableEntryId: entry.stableEntryId,
        parentEntryId: entry.parentEntryId ?? "",
        contentDigest: entry.contentDigest,
        presentationItems: entry.presentationItems.map((item) => create(PiPersistedPresentationItemSchema, {
          membershipId: item.membershipId,
          transcriptEventJson: new TextEncoder().encode(item.transcriptEventJson),
        })),
      })),
      cursorEntryId: cursor,
      leafEntryId: cursor,
      treeDigest,
    }));
    const observation = create(ObservationSchema, {
      sender: create(ActorEndpointRefSchema, { actorId: create(ActorIdSchema, { value: "pi" }) }),
      kind: ObservationKind.EVENT,
      targetScope: create(TargetScopeSchema, {
        kind: TargetScopeKind.RUNTIME_SESSION,
        adapterId: create(AdapterIdSchema, { value: text(input.adapter_id, "adapter id") }),
        deploymentScope: text(input.deployment_scope, "deployment scope"),
        runtimeSessionId: create(RuntimeSessionIdSchema, { value: "runtime-n-plus-one" }),
        sessionGeneration: create(GenerationSchema, { value: 2n }),
      }),
      payload: create(PayloadEnvelopeSchema, {
        contentType: PayloadContentType.PROTOBUF,
        schemaRef: "patchbay.PiPersistedProjectionReplacement.v1",
        payload,
      }),
    });
    return create(SubscribeEventSchema, {
      eventId: create(EventIdSchema, {
        authorityDomainId: create(AuthorityDomainIdSchema, { value: "authority-main" }),
        lsn: create(LsnSchema, { value: lsn }),
      }),
      payload: create(StoredEventPayloadSchema, {
        kind: StoredEventKind.OBSERVATION,
        payload: toBinary(ObservationSchema, observation),
      }),
    });
  };

  let model = fold(emptyPresentationModel(), event(1n, BigInt(input.old_epoch as number), oldIds));
  model = fold(model, event(2n, BigInt(input.replacement_epoch as number), nextIds));
  assert.deepEqual(
    [...model.piPersistedProjections.values()][0]?.exactEntries.map((entry) => entry.stableEntryId),
    expected.external_projection_ids,
  );
  assert.equal(model.observations.some((item) => item.messageId === "omitted-stale"), false);
  assert.deepEqual(model.observations.map((item) => item.messageId), ["current"]);
}

function canonicalPi(value: unknown): string {
  if (value === null || typeof value === "string" || typeof value === "boolean" || typeof value === "number") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonicalPi).join(",")}]`;
  const object = value as Record<string, unknown>;
  return `{${Object.keys(object).sort().map((key) => `${JSON.stringify(key)}:${canonicalPi(object[key])}`).join(",")}}`;
}

async function executeVectorCase(vector: ConformanceVector, caseName: string): Promise<void> {
  assert.ok(vector.property_id);
  assert.ok(vector.promotion_status === "draft" || vector.promotion_status === "promoted");
  switch (caseName) {
    case "resource_stale_presentation_dominance":
      executeStalePresentation(vector);
      return;
    case "token_commune_cockpit_presentation":
      executeTokenCommunePresentation(vector);
      return;
    case "spawn_reconnect_surface_convergence":
      executeSpawnReconnectSurface(vector);
      return;
    case "pi_authoritative_replacement_presentation":
      executePiAuthoritativeReplacementPresentation(vector);
      return;
    default:
      throw new Error(`unhandled ${RUNNER} conformance case ${vector.vector_id}:${caseName}`);
  }
}

test("conformance vector runner", async () => {
  const vectors = vectorsForRunner();
  for (const request of requestedChecks()) {
    const vector = vectors.get(request.vector_id);
    assert.ok(vector, `unknown vector id ${request.vector_id}`);
    assert.ok(
      vector.implementation_checks?.some((check) => check.runner === RUNNER && check.case === request.case),
      `unregistered requested check ${request.vector_id}:${request.case}`,
    );
    await executeVectorCase(vector, request.case);
    console.log(`PATCHBAY_CONFORMANCE_EXECUTED=${request.vector_id}:${request.case}`);
  }
  for (const request of requestedMutations()) {
    const vector = vectors.get(request.vector_id);
    assert.ok(vector, `unknown mutation vector id ${request.vector_id}`);
    assert.ok(
      vector.mutation_witnesses?.some((witness) => witness.runner === RUNNER && witness.mutation_id === request.mutation_id),
      `unregistered requested mutation ${request.vector_id}:${request.mutation_id}`,
    );
    await killWebMutation(vector, request.mutation_id);
    console.log(`PATCHBAY_CONFORMANCE_MUTATION_KILLED=${request.vector_id}:${request.mutation_id}`);
  }
});
