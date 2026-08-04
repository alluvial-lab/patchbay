import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import test from "node:test";

import { AdapterSnapshotSupport, ResourceFreshnessState } from "@patchbay/contracts";
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

const RUNNER = "web-cockpit" as const;

interface ImplementationCheck {
  runner: string;
  case: string;
}

interface ConformanceVector {
  vector_id: string;
  property_id: string;
  promotion_status: string;
  implementation_checks?: readonly ImplementationCheck[];
  input: unknown;
  expected_outcome: unknown;
}

interface RequestedCheck {
  vector_id: string;
  case: string;
}

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

async function executeVectorCase(vector: ConformanceVector, caseName: string): Promise<void> {
  assert.ok(vector.property_id);
  assert.ok(vector.promotion_status === "draft" || vector.promotion_status === "promoted");
  switch (caseName) {
    case "resource_stale_presentation_dominance":
      executeStalePresentation(vector);
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
});
