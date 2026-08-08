import {
  AdapterSnapshotSupport,
  OperationKind,
  ResourceFreshnessState,
} from "@patchbay/contracts";
import {
  TOKEN_COMMUNE_PRESENTATION_CONTRACT,
  composeTokenCommunePools,
  type TokenCommuneProjection,
  type TokenCommuneResourceInput,
} from "@patchbay/operator-domain";

import {
  resourceCollectionKey,
  resourceKey,
  rendersResourceCurrent,
  type CommandView,
  type PresentationModel,
  type ResourceIdentityView,
  type ResourceView,
} from "../domain/model.js";
import { renderIcon } from "./icons.js";
import { operationKindLabel, renderOperationDelivery } from "./operation-delivery.js";
import { formatTargetScope, scopeMayContainResource } from "./target-scope.js";
import {
  renderTokenCommunePanel,
  type TokenCommunePanelInput,
} from "./token-commune-panel.js";

export interface ResourceDestinationOptions {
  selectedKey?: string;
  mobileDetailOpen: boolean;
  lockdownActive: boolean;
  onSelect(resource: ResourceView): void;
  onBack(): void;
}

export interface ResourceDestinationComponent {
  readonly element: HTMLElement;
  setMobile(mobile: boolean): void;
}

export function renderResourceDestination(
  document: Document,
  model: PresentationModel,
  options: ResourceDestinationOptions,
): ResourceDestinationComponent {
  const element = document.createElement("section");
  element.className = "resources-view";
  const active = [...model.resources.values()].filter((resource) => !resource.tombstoned);
  const tokenResources = active.filter((resource) => isTokenCommuneKind(resource.identity.resourceKind));
  const genericResources = active.filter((resource) => !isTokenCommuneKind(resource.identity.resourceKind));
  const selected = options.selectedKey
    ? model.resources.get(options.selectedKey)
    : preferredResource(genericResources);

  const list = renderResourceList(document, genericResources, selected, options.onSelect);
  const detail = renderResourceDetail(document, model, selected, options);
  if (tokenResources.length > 0) {
    element.classList.add("resources-view--token-commune");
    const panelInput = tokenCommunePanelInput(model, options.selectedKey, new Date());
    element.append(renderTokenCommunePanel(document, {
      ...panelInput,
      onSelect(summary) {
        const resource = model.resources.get(resourceKey(summary.poolIdentity));
        if (resource) options.onSelect(resource);
      },
    }));
    if (genericResources.length > 0) {
      const generic = document.createElement("div");
      generic.className = "generic-resource-plane";
      generic.append(list, detail);
      element.append(generic);
    }
  } else {
    element.append(list, detail);
  }

  return {
    element,
    setMobile(mobile) {
      element.dataset.presentation = mobile ? "mobile-drill-in" : "desktop-two-pane";
      if (tokenResources.length === 0 || genericResources.length > 0) {
        list.hidden = mobile && options.mobileDetailOpen;
        detail.hidden = mobile && !options.mobileDetailOpen;
      }
    },
  };
}

export function resourceHasLocalQueryAffordance(
  model: PresentationModel,
  identity: ResourceIdentityView,
  now: Date,
): boolean {
  return model.security.grants.some((grant) =>
    !grant.revoked
    && (!grant.expiresAt || grant.expiresAt.getTime() > now.getTime())
    && grant.allowedOperationKinds.includes(OperationKind.QUERY)
    && scopeMayContainResource(grant.targetScope, identity),
  );
}

export function tokenCommunePanelInput(
  model: PresentationModel,
  selectedKey: string | undefined,
  now: Date,
): TokenCommunePanelInput {
  const inputs: TokenCommuneResourceInput[] = [];
  for (const resource of model.resources.values()) {
    if (!isTokenCommuneKind(resource.identity.resourceKind)) continue;
    if (!resourceHasLocalQueryAffordance(model, resource.identity, now)) continue;
    const collection = model.resourceCollections.get(resourceCollectionKey(
      resource.identity.adapterId,
      resource.identity.resourceKind,
    ));
    inputs.push({
      identity: resource.identity,
      freshness: resource.freshness,
      completeness: collection?.completeness ?? AdapterSnapshotSupport.UNSPECIFIED,
      ...(resource.observedAt ? { observedAt: resource.observedAt } : {}),
      reconciled: resource.reconciled && (collection?.reconciled ?? false),
      tombstoned: resource.tombstoned,
      projection: tokenProjection(resource),
    });
  }
  const summaries = composeTokenCommunePools(inputs);
  const selectedSummary = summaries.find((summary) => resourceKey(summary.poolIdentity) === selectedKey);
  const refreshedAt = summaries.reduce<Date | undefined>((latest, summary) =>
    !summary.poolObservedAt || (latest && latest >= summary.poolObservedAt) ? latest : summary.poolObservedAt,
  undefined);
  const recentEvents = model.resourceObservations
    .filter((event) => resourceHasLocalQueryAffordance(model, event.poolIdentity, now))
    .sort((left, right) => left.lsn > right.lsn ? -1 : left.lsn < right.lsn ? 1 : 0)
    .slice(0, 12)
    .map(({ poolIdentity, kind, code, occurredAt }) => ({ poolIdentity, kind, code, occurredAt }));
  return {
    summaries,
    recentEvents,
    ...(refreshedAt ? { refreshedAt } : {}),
    partial: summaries.some((summary) => summary.completeness !== AdapterSnapshotSupport.AUTHORITATIVE),
    ...(selectedSummary ? { selectedKey: selectedSummary.key } : {}),
  };
}

function tokenProjection(resource: ResourceView): TokenCommuneResourceInput["projection"] {
  if (resource.projection.status !== "decoded") {
    return resource.projection.status === "invalid"
      ? { status: "invalid", reason: "projection_decode_failed" }
      : resource.projection.status === "unsupported" ? { status: "unsupported" } : { status: "unavailable" };
  }
  return isTokenCommuneProjection(resource.projection.value)
    ? { status: "decoded", value: resource.projection.value }
    : { status: "unsupported" };
}

function renderResourceList(
  document: Document,
  resources: readonly ResourceView[],
  selected: ResourceView | undefined,
  onSelect: (resource: ResourceView) => void,
): HTMLElement {
  const list = document.createElement("aside");
  list.className = "resource-list";
  const header = document.createElement("header");
  header.className = "nav-bar";
  header.append(textElement(document, "strong", "nav-bar__brand", "Patchbay · Resources"));
  list.append(header);

  const groups: Array<{ label: string; className: string; resources: ResourceView[] }> = [
    { label: "Pooled provider pools", className: "source-tag--pooled", resources: [] },
    { label: "Direct provider usage", className: "source-tag--direct", resources: [] },
    { label: "Unavailable projections", className: "source-tag--unavailable", resources: [] },
  ];
  for (const resource of resources) {
    const projection = resource.projection;
    if (projection.status !== "decoded") groups[2]!.resources.push(resource);
    else if (projection.value.kind === "pooled-provider-pool") groups[0]!.resources.push(resource);
    else if (projection.value.kind === "direct-provider-usage-window") groups[1]!.resources.push(resource);
    else groups[2]!.resources.push(resource);
  }

  if (resources.length === 0) {
    list.append(emptyState(document, "No operational resources", "No active resource identities are present in the reconciled projection."));
    return list;
  }
  for (const group of groups) {
    if (group.resources.length === 0) continue;
    const heading = document.createElement("div");
    heading.className = "source-section";
    heading.append(textElement(document, "span", `source-tag ${group.className}`, group.label));
    list.append(heading);
    group.resources.sort((left, right) => resourceLabel(left).localeCompare(resourceLabel(right)));
    for (const resource of group.resources) {
      list.append(renderResourceRow(document, resource, selected, onSelect));
    }
  }
  return list;
}

function renderResourceRow(
  document: Document,
  resource: ResourceView,
  selected: ResourceView | undefined,
  onSelect: (resource: ResourceView) => void,
): HTMLElement {
  const row = document.createElement("button");
  row.type = "button";
  row.className = "resource-row";
  row.dataset.resourceKey = resourceKey(resource.identity);
  if (selected && resourceKey(selected.identity) === resourceKey(resource.identity)) {
    row.classList.add("resource-row--active");
    row.setAttribute("aria-current", "true");
  }
  const top = document.createElement("span");
  top.className = "resource-row__top";
  top.append(
    textElement(document, "span", "resource-row__label", resourceLabel(resource)),
    renderFreshness(document, resource),
  );
  row.append(top, textElement(document, "span", "resource-row__identity", formatResourceIdentity(resource.identity)));
  if (resource.projection.status === "decoded" && isGenericProjection(resource.projection.value)) {
    const projection = resource.projection.value;
    row.append(textElement(
      document,
      "span",
      "resource-row__summary",
      resource.freshness === ResourceFreshnessState.STALE || !resource.reconciled
        ? `last reported ${projection.health}`
        : projection.health,
    ));
    if (effectiveFreshness(resource) !== ResourceFreshnessState.UNKNOWN && projection.remainingPercent !== undefined) {
      row.append(renderMeter(document, projection.remainingPercent, projection.resetLabel));
    }
  } else {
    row.append(textElement(document, "span", "resource-row__summary", projectionStatusLabel(resource)));
  }
  row.addEventListener("click", () => onSelect(resource));
  return row;
}

function renderResourceDetail(
  document: Document,
  model: PresentationModel,
  resource: ResourceView | undefined,
  options: ResourceDestinationOptions,
): HTMLElement {
  const detail = document.createElement("article");
  detail.className = "resource-detail";
  const header = document.createElement("header");
  header.className = "view-header";
  const back = document.createElement("button");
  back.type = "button";
  back.className = "btn btn-ghost btn--sm btn--icon-only resource-detail__back";
  back.setAttribute("aria-label", "Back to resources");
  back.append(renderIcon(document, "arrow-left"));
  back.addEventListener("click", options.onBack);
  header.append(back);

  if (!resource) {
    header.append(textElement(document, "h1", "", "Select a resource"));
    detail.append(header, emptyState(document, "No resource selected", "Choose an exact operational-resource identity from the list."));
    return detail;
  }

  const identity = textElement(document, "h1", "resource-detail__identity", formatResourceIdentity(resource.identity));
  header.append(identity, renderFreshness(document, resource));
  const body = document.createElement("div");
  body.className = "resource-detail__body";
  body.append(renderCanonicalWrapper(document, model, resource));
  if (resource.projection.status === "decoded" && isGenericProjection(resource.projection.value)) {
    body.append(renderAdapterProjection(document, resource));
  } else {
    body.append(emptyState(
      document,
      "Adapter projection unavailable",
      `${projectionStatusLabel(resource)}. Canonical identity and reconciliation context remain visible; raw adapter bytes are not exposed.`,
    ));
  }
  detail.append(header, body);
  return detail;
}

function renderCanonicalWrapper(
  document: Document,
  model: PresentationModel,
  resource: ResourceView,
): HTMLElement {
  const card = document.createElement("section");
  card.className = "card resource-wrapper";
  card.append(textElement(document, "h2", "", "Canonical Patchbay resource"));
  const collection = model.resourceCollections.get(resourceCollectionKey(
    resource.identity.adapterId,
    resource.identity.resourceKind,
  ));
  const fields: Array<[string, string]> = [
    ["Identity", formatResourceIdentity(resource.identity)],
    ["Source adapter generation", String(resource.sourceAdapterGeneration)],
    ["Resource revision LSN", String(resource.revisionLsn)],
    ["Collection revision LSN", collection ? String(collection.revisionLsn) : "unavailable"],
    ["Snapshot completeness", collection ? completenessLabel(collection.completeness) : "unavailable"],
    ["Freshness", freshnessLabel(resource)],
    ["Resource observed", resource.observedAt?.toISOString() ?? "unavailable"],
    ["Collection observed", collection?.observedAt?.toISOString() ?? "unavailable"],
    ["Lifecycle", resource.tombstoned ? "retired" : "active"],
  ];
  if (resource.replacedBy) fields.push(["Replaced by", formatResourceIdentity(resource.replacedBy)]);
  const dl = document.createElement("dl");
  dl.className = "resource-kv";
  for (const [term, value] of fields) {
    dl.append(textElement(document, "dt", "", term), textElement(document, "dd", "", value));
  }
  card.append(dl);

  const grants = model.security.grants.filter((grant) =>
    scopeMayContainResource(grant.targetScope, resource.identity),
  );
  const grantSection = document.createElement("section");
  grantSection.className = "resource-grants";
  grantSection.append(textElement(document, "h3", "", "Visible grant context · core enforced"));
  grantSection.append(textElement(
    document,
    "p",
    "identity",
    "These labels explain the visible snapshot. They do not authorize controls; the core evaluates every Operation.",
  ));
  if (grants.length === 0) {
    grantSection.append(textElement(document, "p", "identity", "No matching visible grant scopes."));
  } else {
    const list = document.createElement("ul");
    for (const grant of grants) {
      const item = document.createElement("li");
      item.textContent = `${grant.grantId} · ${formatTargetScope(grant.targetScope)} · ${grant.revoked ? "revoked" : "visible"}`;
      list.append(item);
    }
    grantSection.append(list);
  }
  card.append(grantSection, renderResourceOperations(document, model, resource));
  return card;
}

function renderResourceOperations(
  document: Document,
  model: PresentationModel,
  resource: ResourceView,
): HTMLElement {
  const section = document.createElement("section");
  section.className = "resource-operations";
  section.append(textElement(document, "h3", "", "Operation delivery"));
  const commands = [...model.commands.values()]
    .filter((command) => sameResourceTarget(command, resource.identity))
    .sort((left, right) => acceptedLsn(left) < acceptedLsn(right) ? -1 : 1);
  if (commands.length === 0) {
    section.append(textElement(document, "p", "identity", "No Operations target this exact resource identity."));
    return section;
  }
  for (const command of commands) {
    const row = document.createElement("div");
    row.className = "resource-operation";
    row.append(textElement(document, "strong", "", `${operationKindLabel(command.operation.kind)} · ${command.id}`));
    row.append(renderOperationDelivery(document, command, undefined, model.lockdown.active));
    section.append(row);
  }
  return section;
}

function renderAdapterProjection(document: Document, resource: ResourceView): HTMLElement {
  if (resource.projection.status !== "decoded" || !isGenericProjection(resource.projection.value)) {
    throw new Error("generic decoded projection required");
  }
  const projection = resource.projection.value;
  const card = document.createElement("section");
  card.className = `card adapter-projection adapter-projection--${projection.kind}`;
  const stale = effectiveFreshness(resource) === ResourceFreshnessState.STALE || resource.tombstoned;
  card.append(textElement(
    document,
    "h2",
    "",
    stale ? `Last reported · ${projection.displayName}` : projection.displayName,
  ));
  card.append(textElement(document, "p", "identity", `${projection.providerLabel} · ${projection.controlPosture}`));
  if (effectiveFreshness(resource) === ResourceFreshnessState.UNKNOWN) {
    card.append(textElement(document, "p", "identity", "Domain health unavailable while resource freshness is unknown."));
    return card;
  }
  card.append(renderDomainHealth(document, projection.health, stale));
  if (projection.remainingPercent !== undefined) {
    card.append(renderMeter(document, projection.remainingPercent, projection.resetLabel, true));
  }
  const details = projection.kind === "pooled-provider-pool"
    ? [
        projection.serviceLabel && `service ${projection.serviceLabel}`,
        projection.contributionCount !== undefined && `${projection.contributionCount} contributions`,
      ]
    : [
        projection.accountLabel && `account ${projection.accountLabel}`,
        projection.planLabel && `plan ${projection.planLabel}`,
        projection.windowLabel && `window ${projection.windowLabel}`,
        projection.burnRateLabel && `burn ${projection.burnRateLabel}`,
        projection.activeSessionCount !== undefined && `${projection.activeSessionCount} active sessions`,
      ];
  for (const detail of details.filter((value): value is string => Boolean(value))) {
    card.append(textElement(document, "p", "adapter-projection__fact", detail));
  }
  return card;
}

function renderFreshness(document: Document, resource: ResourceView): HTMLElement {
  const freshness = effectiveFreshness(resource);
  const name = freshness === ResourceFreshnessState.CURRENT
    ? "current"
    : freshness === ResourceFreshnessState.STALE ? "stale" : "unknown";
  const badge = document.createElement("span");
  badge.className = `resource-freshness resource-freshness--${name}`;
  badge.setAttribute("aria-label", `Resource freshness ${name}`);
  badge.append(textElement(document, "span", "resource-freshness__dot", ""), document.createTextNode(name));
  return badge;
}

function renderDomainHealth(document: Document, health: string, stale: boolean): HTMLElement {
  const tone = health === "serving" || health === "ok"
    ? "ok"
    : health === "degraded" || health === "low" || health === "paused"
      ? "warning"
      : health === "exhausted" ? "danger" : "unknown";
  return textElement(
    document,
    "p",
    `resource-health resource-health--${tone}`,
    stale ? `last reported health · ${health}` : `domain health · ${health}`,
  );
}

function renderMeter(
  document: Document,
  percent: number,
  resetLabel?: string,
  large = false,
): HTMLElement {
  const wrapper = document.createElement("div");
  wrapper.className = large ? "resource-meter resource-meter--large" : "resource-meter";
  const bar = document.createElement("span");
  bar.className = "resource-meter__bar";
  const fill = document.createElement("span");
  fill.className = "resource-meter__fill";
  fill.style.width = `${percent}%`;
  bar.append(fill);
  wrapper.append(bar, textElement(
    document,
    "span",
    "resource-meter__label",
    `${percent}%${resetLabel ? ` · ${resetLabel}` : ""}`,
  ));
  return wrapper;
}

function sameResourceTarget(command: CommandView, identity: ResourceIdentityView): boolean {
  return Boolean(
    command.target?.kind === "operational-resource"
    && resourceKey(command.target.identity) === resourceKey(identity),
  );
}

function acceptedLsn(command: CommandView): bigint {
  return command.history[0]?.lsn ?? command.lsn;
}

function preferredResource(resources: readonly ResourceView[]): ResourceView | undefined {
  return resources.find(rendersResourceCurrent) ?? resources[0];
}

function effectiveFreshness(resource: ResourceView): ResourceFreshnessState {
  if (!resource.reconciled || resource.tombstoned) {
    return resource.hasCachedPayload ? ResourceFreshnessState.STALE : ResourceFreshnessState.UNKNOWN;
  }
  if (rendersResourceCurrent(resource)) return ResourceFreshnessState.CURRENT;
  return resource.freshness === ResourceFreshnessState.STALE
    ? ResourceFreshnessState.STALE
    : ResourceFreshnessState.UNKNOWN;
}

function freshnessLabel(resource: ResourceView): string {
  const freshness = effectiveFreshness(resource);
  if (freshness === ResourceFreshnessState.CURRENT) return "current";
  if (freshness === ResourceFreshnessState.STALE) return "stale · adapter values are last reported";
  return "unknown · no domain health or meter is current";
}

function projectionStatusLabel(resource: ResourceView): string {
  switch (resource.projection.status) {
    case "decoded": return resource.projection.value.kind;
    case "invalid": return `invalid projection (${resource.projection.reason})`;
    case "unsupported": return `unsupported projection ${resource.projection.projection.schemaRef}`;
    case "unavailable": return "projection unavailable";
  }
}

function resourceLabel(resource: ResourceView): string {
  if (resource.projection.status !== "decoded") return resource.identity.resourceId;
  return isGenericProjection(resource.projection.value)
    ? resource.projection.value.displayName
    : resource.projection.value.provider;
}

function isTokenCommuneKind(kind: string): boolean {
  return kind === TOKEN_COMMUNE_PRESENTATION_CONTRACT.providerPool.resourceKind
    || kind === TOKEN_COMMUNE_PRESENTATION_CONTRACT.memberDraw.resourceKind;
}

function isTokenCommuneProjection(value: import("../domain/resource-projection.js").DecodedResourceProjection): value is TokenCommuneProjection {
  return value.kind === "token-commune-provider-pool" || value.kind === "token-commune-member-draw";
}

function isGenericProjection(value: import("../domain/resource-projection.js").DecodedResourceProjection): value is Extract<
  import("../domain/resource-projection.js").DecodedResourceProjection,
  { kind: "pooled-provider-pool" | "direct-provider-usage-window" }
> {
  return value.kind === "pooled-provider-pool" || value.kind === "direct-provider-usage-window";
}

function formatResourceIdentity(identity: ResourceIdentityView): string {
  return `adapter=${identity.adapterId};resource-kind=${identity.resourceKind};resource=${identity.resourceId}`;
}

function completenessLabel(value: AdapterSnapshotSupport): string {
  switch (value) {
    case AdapterSnapshotSupport.AUTHORITATIVE: return "authoritative";
    case AdapterSnapshotSupport.PARTIAL: return "partial";
    case AdapterSnapshotSupport.NONE: return "none";
    case AdapterSnapshotSupport.UNSPECIFIED:
    default: return "unknown";
  }
}

function emptyState(document: Document, title: string, body: string): HTMLElement {
  const empty = document.createElement("div");
  empty.className = "empty-state";
  empty.append(
    textElement(document, "p", "empty-state__title", title),
    textElement(document, "p", "empty-state__body", body),
  );
  return empty;
}

function textElement<K extends keyof HTMLElementTagNameMap>(
  document: Document,
  tag: K,
  className: string,
  text: string,
): HTMLElementTagNameMap[K] {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = text;
  return element;
}
