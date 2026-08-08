import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import test from "node:test";

import { create } from "@bufbuild/protobuf";
import {
  AdapterSnapshotSupport, PayloadContentType, PayloadEnvelopeSchema, ResourceFreshnessState,
} from "@patchbay/contracts";
import {
  TOKEN_COMMUNE_PRESENTATION_CONTRACT, composeTokenCommunePools, decodeTokenCommuneProjection,
  type TokenCommunePoolSummary, type TokenCommuneResourceInput,
} from "@patchbay/operator-domain";
import { JSDOM } from "jsdom";

import {
  emptyPresentationModel,
  rendersResourceCurrent,
  resourceCollectionKey,
  resourceKey,
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

function tokenPanel(summary: TokenCommunePoolSummary) {
  const dom = new JSDOM("<!doctype html><html><body><main></main></body></html>", { runScripts: "dangerously" });
  const panel = renderTokenCommunePanel(dom.window.document, {
    summaries: [summary], partial: true,
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
  assert.equal(current.draw.state, "current", "only the exact adapter/provider member draw joins");
  if (current.draw.state === "current") assert.equal(current.draw.limitFraction, Number(expected.current_draw_limit_fraction));
  assert.equal(current.drawIdentity?.adapterId, text(object(vector.input, "input").adapter_id, "adapter id"));
  const currentRendered = tokenPanel(current);
  assertHonestTokenPanel(currentRendered.panel, expected);

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
