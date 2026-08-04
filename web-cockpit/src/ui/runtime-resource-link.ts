import { ResourceFreshnessState } from "@patchbay/contracts";

import type { ResourceIdentityView, ResourceView } from "../domain/model.js";

export interface RuntimeResourceLinkOptions {
  resource: ResourceView | undefined;
  onOpen(identity: ResourceIdentityView): void;
}

export function renderRuntimeResourceLink(
  document: Document,
  options: RuntimeResourceLinkOptions,
): HTMLElement {
  const cell = document.createElement("div");
  cell.className = "runtime-resource-link ctx-cell";
  cell.append(textElement(document, "span", "ctx-cell__label", "usage window"));

  const resource = options.resource;
  const pool = resource?.projection.status === "decoded"
      && resource.projection.value.kind === "pooled-provider-pool"
    ? resource.projection.value
    : undefined;
  const effectiveFreshness = resource && !resource.reconciled
    ? resource.hasCachedPayload ? ResourceFreshnessState.STALE : ResourceFreshnessState.UNKNOWN
    : resource?.freshness;
  const navigable = Boolean(
    resource
    && pool
    && !resource.tombstoned
    && (effectiveFreshness === ResourceFreshnessState.CURRENT
      || effectiveFreshness === ResourceFreshnessState.STALE),
  );

  const button = document.createElement("button");
  button.type = "button";
  button.className = "runtime-resource-link__button ctx-pill";
  button.disabled = !navigable;
  if (!resource || !pool || !navigable) {
    button.textContent = "Usage unavailable";
    button.title = "The linked pooled-provider resource is missing, retired, unknown, or invalid.";
    cell.append(button);
    return cell;
  }

  const freshness = effectiveFreshness === ResourceFreshnessState.CURRENT
    ? "current"
    : "stale · last reported";
  const main = document.createElement("span");
  main.className = "ctx-cell__value";
  if (pool.remainingPercent !== undefined) {
    const meter = document.createElement("span");
    meter.className = "ctx-mini-meter";
    const fill = document.createElement("span");
    fill.className = "ctx-mini-fill";
    fill.style.width = `${pool.remainingPercent}%`;
    meter.append(fill);
    main.append(meter, document.createTextNode(` ${pool.remainingPercent}%`));
  } else {
    main.textContent = pool.health;
  }
  const sub = textElement(
    document,
    "span",
    "ctx-cell__sub",
    `${pool.displayName}${pool.resetLabel ? ` · ${pool.resetLabel}` : ""} · ${freshness} →`,
  );
  button.append(main, sub);
  button.addEventListener("click", () => options.onOpen(resource.identity));
  cell.append(button);
  return cell;
}

function textElement(
  document: Document,
  tag: keyof HTMLElementTagNameMap,
  className: string,
  text: string,
): HTMLElement {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = text;
  return element;
}
