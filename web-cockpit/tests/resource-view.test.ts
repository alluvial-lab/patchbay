import assert from "node:assert/strict";
import test from "node:test";

import { create } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  AdapterSnapshotSupport,
  AuthorityDomainIdSchema,
  CommandIdSchema,
  OperationKind,
  OperationSchema,
  OperationState,
  ResourceFreshnessState,
  ResourceIdSchema,
  ResourceIdentitySchema,
  ResourceKindSchema,
  TargetScopeKind,
  TargetScopeSchema,
} from "@patchbay/contracts";
import fc from "fast-check";
import { JSDOM } from "jsdom";

import {
  emptyPresentationModel,
  rendersResourceCurrent,
  resourceCollectionKey,
  resourceKey,
  type CommandView,
  type PresentationModel,
  type ResourceIdentityView,
  type ResourceView,
} from "../src/domain/model.js";
import { renderResourceDestination } from "../src/ui/resource-view.js";
import { renderRuntimeResourceLink } from "../src/ui/runtime-resource-link.js";
import { formatTargetScope, scopeMayContainResource } from "../src/ui/target-scope.js";

const DOMAIN = create(AuthorityDomainIdSchema, { value: "operator-domain" });

function identity(resourceKind: string, resourceId: string, adapterId = "token-commune"): ResourceIdentityView {
  return { adapterId, resourceKind, resourceId };
}

function pool(
  resourceId: string,
  freshness = ResourceFreshnessState.CURRENT,
  overrides: Partial<ResourceView> = {},
): ResourceView {
  return {
    identity: identity("provider_pool", resourceId),
    freshness,
    sourceAdapterGeneration: 2n,
    revisionLsn: 12n,
    observedAt: new Date("2026-08-04T12:00:00Z"),
    tombstoned: false,
    hasCachedPayload: freshness !== ResourceFreshnessState.UNKNOWN,
    reconciled: true,
    projection: freshness === ResourceFreshnessState.UNKNOWN
      ? { status: "unavailable" }
      : {
          status: "decoded",
          value: {
            kind: "pooled-provider-pool",
            displayName: resourceId,
            providerLabel: "Anthropic",
            health: "serving",
            remainingPercent: 42,
            resetLabel: "resets in 2h",
            contributionCount: 3,
            serviceLabel: "token-commune",
            controlPosture: "administration-capable",
          },
        },
    ...overrides,
  };
}

function usage(resourceId: string): ResourceView {
  return {
    identity: identity("usage_window", resourceId, "direct-provider"),
    freshness: ResourceFreshnessState.CURRENT,
    sourceAdapterGeneration: 1n,
    revisionLsn: 10n,
    observedAt: new Date("2026-08-04T11:00:00Z"),
    tombstoned: false,
    hasCachedPayload: true,
    reconciled: true,
    projection: {
      status: "decoded",
      value: {
        kind: "direct-provider-usage-window",
        displayName: "Claude Code",
        providerLabel: "Anthropic",
        health: "low",
        remainingPercent: 8,
        resetLabel: "resets in 1h",
        accountLabel: "operator",
        planLabel: "Max",
        windowLabel: "5 hour",
        burnRateLabel: "12% / hour",
        activeSessionCount: 2,
        controlPosture: "read-only",
      },
    },
  };
}

function modelWith(...resources: ResourceView[]): PresentationModel {
  const model = emptyPresentationModel();
  model.authorityDomainId = DOMAIN.value;
  model.reconciled = true;
  for (const resource of resources) {
    model.resources.set(resourceKey(resource.identity), resource);
    model.resourceCollections.set(
      resourceCollectionKey(resource.identity.adapterId, resource.identity.resourceKind),
      {
        adapterId: resource.identity.adapterId,
        resourceKind: resource.identity.resourceKind,
        completeness: AdapterSnapshotSupport.AUTHORITATIVE,
        sourceAdapterGeneration: resource.sourceAdapterGeneration,
        revisionLsn: resource.revisionLsn,
        observedAt: resource.observedAt,
        reconciled: resource.reconciled,
      },
    );
  }
  return model;
}

const resourceViewArbitrary: fc.Arbitrary<ResourceView> = fc.record({
  freshness: fc.constantFrom(
    ResourceFreshnessState.CURRENT,
    ResourceFreshnessState.STALE,
    ResourceFreshnessState.UNKNOWN,
  ),
  reconciled: fc.boolean(),
  tombstoned: fc.boolean(),
  health: fc.constantFrom("serving" as const, "degraded" as const, "exhausted" as const, "unknown" as const),
}).map(({ freshness, reconciled, tombstoned, health }) => {
  const effectiveFreshness = tombstoned && freshness === ResourceFreshnessState.CURRENT
    ? ResourceFreshnessState.STALE
    : freshness;
  const hasCachedPayload = effectiveFreshness !== ResourceFreshnessState.UNKNOWN;
  return pool("generated", effectiveFreshness, {
    reconciled,
    tombstoned,
    hasCachedPayload,
    projection: hasCachedPayload
      ? {
          status: "decoded",
          value: {
            kind: "pooled-provider-pool",
            displayName: "Generated pool",
            providerLabel: "Provider",
            health,
            remainingPercent: 50,
            serviceLabel: "adapter",
            controlPosture: "administration-capable",
          },
        }
      : { status: "unavailable" },
  });
});

function resourceMayRenderCurrent(view: ResourceView): boolean {
  return view.reconciled
    && !view.tombstoned
    && view.freshness === ResourceFreshnessState.CURRENT;
}

test("resource freshness dominates model and DOM across generated valid views", async () => {
  await fc.assert(fc.asyncProperty(resourceViewArbitrary, async (view) => {
    const expected = resourceMayRenderCurrent(view);
    assert.equal(rendersResourceCurrent(view), expected);
    const dom = new JSDOM();
    const component = renderResourceDestination(dom.window.document, modelWith(view), {
      selectedKey: resourceKey(view.identity), mobileDetailOpen: true, lockdownActive: false,
      onSelect() {}, onBack() {},
    });
    assert.equal(Boolean(component.element.querySelector(".resource-freshness--current")), expected);
    if (!expected) {
      assert.equal(Boolean(component.element.querySelector(".resource-detail .resource-health:not(.resource-health--unknown)")?.textContent?.startsWith("domain health")), false);
    }
    if (view.freshness === ResourceFreshnessState.UNKNOWN) {
      assert.equal(component.element.querySelector(".resource-detail .resource-meter"), null);
    }
  }), { numRuns: 100 });
});

test("independent current-eligibility oracle kills presentation mutants", () => {
  const base = pool("mutant");
  const unreconciled = { ...base, reconciled: false };
  const retired = { ...base, tombstoned: true };
  const stale = { ...base, freshness: ResourceFreshnessState.STALE };
  const healthServingOnly = (_view: ResourceView) => true;
  const freshnessOnly = (view: ResourceView) => view.freshness === ResourceFreshnessState.CURRENT;
  const ignoreTombstone = (view: ResourceView) => view.reconciled && view.freshness === ResourceFreshnessState.CURRENT;
  assert.equal(resourceMayRenderCurrent(unreconciled), false);
  assert.equal(freshnessOnly(unreconciled), true);
  assert.equal(resourceMayRenderCurrent(retired), false);
  assert.equal(ignoreTombstone(retired), true);
  assert.equal(resourceMayRenderCurrent(stale), false);
  assert.equal(healthServingOnly(stale), true);
});

test("resource destination groups pooled, direct, and unavailable projections without raw bytes", () => {
  const dom = new JSDOM();
  const invalid: ResourceView = {
    ...pool("invalid"),
    projection: {
      status: "invalid",
      projection: { schemaRef: "provider_pool.projection.v1", contentType: 3 },
      reason: "projection_decode_failed",
    },
  };
  const tombstone = pool("retired", ResourceFreshnessState.STALE, { tombstoned: true });
  const model = modelWith(pool("shared-pool"), usage("anthropic-5h"), invalid, tombstone);
  const component = renderResourceDestination(dom.window.document, model, {
    mobileDetailOpen: false,
    lockdownActive: false,
    onSelect() {},
    onBack() {},
  });
  dom.window.document.body.append(component.element);

  assert.deepEqual(
    [...component.element.querySelectorAll(".source-tag")].map((node) => node.textContent),
    ["Pooled provider pools", "Direct provider usage", "Unavailable projections"],
  );
  assert.equal(component.element.querySelectorAll(".resource-row").length, 3);
  assert.doesNotMatch(component.element.textContent!, /raw|\{.*displayName/);
  assert.doesNotMatch(component.element.textContent!, /retired/);
  assert.match(component.element.textContent!, /shared-pool/);
});

test("canonical wrapper precedes decoded domain cards and includes grant plus Operation context", () => {
  const dom = new JSDOM();
  const resource = pool("shared-pool");
  const model = modelWith(resource);
  const scope = resourceScope(resource.identity);
  model.security.grants.push({
    grantId: "grant-resource",
    subjectActorId: "operator",
    targetScope: scope,
    allowedOperationKinds: [OperationKind.QUERY],
    revoked: false,
    revocationPolicy: 1,
  });
  model.commands.set("resource-query", resourceCommand(resource.identity));
  const component = renderResourceDestination(dom.window.document, model, {
    selectedKey: resourceKey(resource.identity),
    mobileDetailOpen: true,
    lockdownActive: false,
    onSelect() {},
    onBack() {},
  });
  dom.window.document.body.append(component.element);

  const detail = component.element.querySelector(".resource-detail")!;
  assert.equal(detail.querySelector("h1")!.textContent, "adapter=token-commune;resource-kind=provider_pool;resource=shared-pool");
  const cards = detail.querySelectorAll(".card");
  assert.equal(cards[0]!.classList.contains("resource-wrapper"), true);
  assert.equal(cards[1]!.classList.contains("adapter-projection"), true);
  assert.match(cards[0]!.textContent!, /Resource revision LSN12/);
  assert.match(cards[0]!.textContent!, /Snapshot completenessauthoritative/);
  assert.match(cards[0]!.textContent!, /grant-resource/);
  assert.match(cards[0]!.textContent!, /adapter=token-commune;resource-kind=provider_pool;resource=shared-pool/);
  assert.match(cards[0]!.textContent!, /core enforced/);
  assert.match(cards[0]!.textContent!, /resource-query/);
  assert.ok(cards[0]!.querySelector(".command-step--running"));
});

test("freshness dominates domain health, meters, and retirement presentation", () => {
  const dom = new JSDOM();
  const stale = pool("stale-pool", ResourceFreshnessState.STALE);
  const unknown = pool("unknown-pool", ResourceFreshnessState.UNKNOWN);
  const retired = pool("retired-pool", ResourceFreshnessState.STALE, {
    tombstoned: true,
    replacedBy: identity("provider_pool", "replacement"),
  });
  let component = renderResourceDestination(dom.window.document, modelWith(stale, unknown, retired), {
    selectedKey: resourceKey(stale.identity),
    mobileDetailOpen: true,
    lockdownActive: false,
    onSelect() {},
    onBack() {},
  });
  assert.match(component.element.querySelector(".resource-detail")!.textContent!, /last reported/i);
  assert.ok(component.element.querySelector(".resource-detail .resource-meter"));

  component = renderResourceDestination(dom.window.document, modelWith(stale, unknown, retired), {
    selectedKey: resourceKey(unknown.identity),
    mobileDetailOpen: true,
    lockdownActive: false,
    onSelect() {},
    onBack() {},
  });
  assert.match(component.element.querySelector(".resource-detail")!.textContent!, /unknown · no domain health or meter is current/);
  assert.equal(component.element.querySelector(".resource-detail .resource-meter"), null);

  component = renderResourceDestination(dom.window.document, modelWith(stale, unknown, retired), {
    selectedKey: resourceKey(retired.identity),
    mobileDetailOpen: true,
    lockdownActive: false,
    onSelect() {},
    onBack() {},
  });
  assert.match(component.element.querySelector(".resource-detail")!.textContent!, /retired/);
  assert.match(component.element.querySelector(".resource-detail")!.textContent!, /replacement/);
});

test("mobile mode moves between list and the same exact detail renderer", () => {
  const dom = new JSDOM();
  const resource = pool("mobile-pool");
  const list = renderResourceDestination(dom.window.document, modelWith(resource), {
    selectedKey: resourceKey(resource.identity),
    mobileDetailOpen: false,
    lockdownActive: false,
    onSelect() {},
    onBack() {},
  });
  list.setMobile(true);
  assert.equal(list.element.querySelector<HTMLElement>(".resource-list")!.hidden, false);
  assert.equal(list.element.querySelector<HTMLElement>(".resource-detail")!.hidden, true);

  const detail = renderResourceDestination(dom.window.document, modelWith(resource), {
    selectedKey: resourceKey(resource.identity),
    mobileDetailOpen: true,
    lockdownActive: false,
    onSelect() {},
    onBack() {},
  });
  detail.setMobile(true);
  assert.equal(detail.element.querySelector<HTMLElement>(".resource-list")!.hidden, true);
  assert.equal(detail.element.querySelector<HTMLElement>(".resource-detail")!.hidden, false);
  assert.match(detail.element.querySelector(".resource-detail")!.textContent!, /mobile-pool/);
});

test("runtime resource linkage accepts only live-or-stale decoded pools", () => {
  const dom = new JSDOM();
  const opened: ResourceIdentityView[] = [];
  const current = pool("linked");
  const link = renderRuntimeResourceLink(dom.window.document, {
    resource: current,
    onOpen: (selected) => opened.push(selected),
  });
  const button = link.querySelector("button")!;
  assert.equal(button.disabled, false);
  assert.match(button.textContent!, /42%/);
  button.click();
  assert.deepEqual(opened, [current.identity]);

  const stale = renderRuntimeResourceLink(dom.window.document, {
    resource: pool("stale", ResourceFreshnessState.STALE),
    onOpen() {},
  });
  assert.equal(stale.querySelector("button")!.disabled, false);
  assert.match(stale.textContent!, /stale · last reported/);

  for (const unavailable of [usage("direct"), undefined, pool("retired", ResourceFreshnessState.STALE, { tombstoned: true })]) {
    const value = renderRuntimeResourceLink(dom.window.document, { resource: unavailable, onOpen() {} });
    assert.equal(value.querySelector("button")!.disabled, true);
    assert.match(value.textContent!, /Usage unavailable/);
  }
});

test("scope formatting and explanatory resource containment stay exact and non-authoritative", () => {
  const resource = identity("provider_pool", "shared-pool");
  const exact = resourceScope(resource);
  const adapter = create(TargetScopeSchema, {
    kind: TargetScopeKind.ADAPTER,
    adapterId: create(AdapterIdSchema, { value: resource.adapterId }),
  });
  assert.equal(formatTargetScope(exact), "adapter=token-commune;resource-kind=provider_pool;resource=shared-pool");
  assert.equal(scopeMayContainResource(exact, resource), true);
  assert.equal(scopeMayContainResource(adapter, resource), true);
  assert.equal(scopeMayContainResource(create(TargetScopeSchema, { kind: TargetScopeKind.FLEET_SUPERVISOR }), resource), true);
  assert.equal(scopeMayContainResource(create(TargetScopeSchema, { kind: TargetScopeKind.AUTHORITY_DOMAIN }), resource), true);
  assert.equal(scopeMayContainResource(resourceScope(identity("provider_pool", "other")), resource), false);
  assert.equal(scopeMayContainResource(create(TargetScopeSchema, { kind: TargetScopeKind.RUNTIME_SESSION }), resource), false);
});

function resourceScope(identity: ResourceIdentityView) {
  return create(TargetScopeSchema, {
    kind: TargetScopeKind.RESOURCE,
    resource: create(ResourceIdentitySchema, {
      adapterId: create(AdapterIdSchema, { value: identity.adapterId }),
      resourceKind: create(ResourceKindSchema, { value: identity.resourceKind }),
      resourceId: create(ResourceIdSchema, { value: identity.resourceId }),
    }),
  });
}

function resourceCommand(identity: ResourceIdentityView): CommandView {
  const operation = create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: "resource-query" }),
    authorityDomainId: DOMAIN,
    kind: OperationKind.QUERY,
    targetScope: resourceScope(identity),
    idempotencyKey: "resource-query-key",
  });
  return {
    id: "resource-query",
    state: OperationState.RUNNING,
    lsn: 15n,
    target: { kind: "operational-resource", identity },
    operation,
    history: [
      { state: OperationState.ACCEPTED, lsn: 14n },
      { state: OperationState.RUNNING, lsn: 15n },
    ],
  };
}

