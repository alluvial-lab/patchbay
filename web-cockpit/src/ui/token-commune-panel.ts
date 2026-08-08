import { AdapterSnapshotSupport } from "@patchbay/contracts";
import type { TokenCommunePoolSummary, TokenCommuneVerdict } from "@patchbay/operator-domain";

export interface TokenCommunePanelInput {
  summaries: readonly TokenCommunePoolSummary[];
  refreshedAt?: Date;
  partial: boolean;
  selectedKey?: string;
}

export interface TokenCommunePanelOptions extends TokenCommunePanelInput {
  formatNow?: Date;
  onSelect?(summary: TokenCommunePoolSummary): void;
}

export function renderTokenCommunePanel(
  document: Document,
  options: TokenCommunePanelOptions,
): HTMLElement {
  const section = document.createElement("section");
  section.className = "token-commune-panel";
  section.setAttribute("aria-labelledby", "token-commune-pools-title");

  const header = document.createElement("header");
  header.className = "token-commune-panel__head";
  header.append(text(document, "span", "token-commune-panel__eyebrow", "Resources · token-commune"));
  const heading = text(document, "h1", "", "Pools");
  heading.id = "token-commune-pools-title";
  header.append(heading, renderVerdictSummary(document, options.summaries));
  section.append(header);

  const pools = document.createElement("div");
  pools.className = "token-commune-pools";
  if (options.summaries.length === 0) {
    pools.append(text(document, "p", "empty-state__body", "No locally query-authorized token-commune pools are visible."));
  }
  for (const summary of options.summaries) {
    pools.append(renderPoolRow(document, summary, options));
  }
  section.append(pools, renderHonestyFooter(document, options));
  return section;
}

function renderVerdictSummary(document: Document, summaries: readonly TokenCommunePoolSummary[]): HTMLElement {
  const counts = new Map<TokenCommuneVerdict, number>();
  for (const summary of summaries) counts.set(summary.verdict, (counts.get(summary.verdict) ?? 0) + 1);
  const container = document.createElement("div");
  container.className = "token-commune-summary";
  const ordered: readonly TokenCommuneVerdict[] = [
    "runnable", "pool-exhausted", "telemetry-stale", "auth-broken", "model-unavailable", "unknown",
  ];
  for (const verdict of ordered) {
    const count = counts.get(verdict) ?? 0;
    if (count === 0) continue;
    const item = document.createElement("span");
    const strong = text(document, "strong", "", String(count));
    item.append(strong, document.createTextNode(` ${verdictLabel(verdict)}`));
    if (container.childNodes.length) container.append(text(document, "span", "token-commune-summary__dot", "·"));
    container.append(item);
  }
  if (container.childNodes.length === 0) container.append(document.createTextNode("0 visible pools"));
  return container;
}

function renderPoolRow(
  document: Document,
  summary: TokenCommunePoolSummary,
  options: TokenCommunePanelOptions,
): HTMLElement {
  const row = document.createElement("button");
  row.type = "button";
  row.className = "token-commune-pool";
  row.dataset.verdict = summary.verdict;
  row.dataset.telemetry = telemetryState(summary);
  if (summary.verdict === "telemetry-stale") row.classList.add("token-commune-pool--stale");
  if (options.selectedKey === summary.key) {
    row.classList.add("token-commune-pool--selected");
    row.setAttribute("aria-current", "true");
  }
  row.setAttribute("aria-label", [
    `Pool ${summary.provider}`,
    `adapter ${summary.poolIdentity.adapterId}`,
    `resource kind ${summary.poolIdentity.resourceKind}`,
    `resource ${summary.poolIdentity.resourceId}`,
    `completeness ${completenessLabel(summary.completeness)}`,
    `telemetry ${telemetryState(summary)}`,
    `credential evidence ${summary.credentials.state}`,
    `verdict ${verdictLabel(summary.verdict)}`,
  ].join("; "));

  const left = document.createElement("span");
  left.className = "token-commune-pool__left";
  left.append(
    text(document, "span", "token-commune-pool__name", summary.provider),
    renderModels(document, summary),
  );
  row.append(
    left,
    signal(document, drawValue(summary), "draw allowance", "token-commune-draw"),
    signal(document, credentialsValue(summary), credentialsLabel(summary), "token-commune-health"),
    signal(document, capacityValue(summary), capacityLabel(summary, options.formatNow), "token-commune-capacity"),
    renderVerdict(document, summary.verdict),
  );
  row.addEventListener("click", () => options.onSelect?.(summary));
  return row;
}

function renderModels(document: Document, summary: TokenCommunePoolSummary): HTMLElement {
  const models = document.createElement("span");
  models.className = "token-commune-pool__models";
  const presentable = summary.models.filter((model) => model.id !== "gpt-5.6");
  if (presentable.length === 0) {
    models.textContent = summary.models.length > 0
      ? "rejected catalog alias withheld"
      : summary.modelState === "unknown" ? "model catalog unavailable" : "no models reported";
    models.classList.add("token-commune-pool__models--unavailable");
    return models;
  }
  presentable.forEach((model, index) => {
    if (index) models.append(document.createTextNode(" · "));
    const label = text(
      document,
      "span",
      model.available ? "" : "token-commune-pool__model--unavailable",
      model.available ? model.id : `${model.id} · unavailable`,
    );
    models.append(label);
  });
  return models;
}

function signal(document: Document, value: string, label: string, className: string): HTMLElement {
  const wrapper = document.createElement("span");
  wrapper.className = `token-commune-signal ${className}`;
  wrapper.append(
    text(document, "span", "token-commune-signal__value", value),
    text(document, "span", "token-commune-signal__label", label),
  );
  return wrapper;
}

function renderVerdict(document: Document, verdict: TokenCommuneVerdict): HTMLElement {
  return text(
    document,
    "span",
    `token-commune-verdict token-commune-verdict--${verdictTone(verdict)}`,
    verdictLabel(verdict),
  );
}

function renderHonestyFooter(document: Document, options: TokenCommunePanelOptions): HTMLElement {
  const footer = document.createElement("footer");
  footer.className = "token-commune-honesty";
  const refreshed = options.refreshedAt ? age(options.refreshedAt, options.formatNow) : "unavailable";
  footer.append(
    text(
      document,
      "p",
      "",
      "Patchbay summaries derive from per-contribution token-commune readings. Draw allowance is native limitFraction, formatted as a percentage. Capacity is the highest real anonymous 5h-window usedFraction; 5h is Patchbay's display window, not necessarily the provider's binding window. No native pool aggregate exists.",
    ),
    text(
      document,
      "p",
      "",
      "Verdicts are a Patchbay synthesis. Patchbay verdict rule: freshness → unknown evidence → auth broken → model unavailable → pool exhausted → runnable. Draw does not affect the verdict.",
    ),
    text(
      document,
      "p",
      "",
      `Polled, not streamed · Patchbay completeness: ${options.partial ? "partial" : "reported per row"} · panel refreshed ${refreshed}; wrapper and underlying reading ages may differ · credential freshness and capacity telemetry freshness are independent axes. Contributor identities and stable contribution IDs are not exposed; counts and readings are anonymous. Per-contribution drill-down is omitted from MVP by Patchbay choice.`,
    ),
  );
  return footer;
}

function drawValue(summary: TokenCommunePoolSummary): string {
  if (summary.draw.state === "current" || summary.draw.state === "stale") {
    return `${formatPercent(summary.draw.limitFraction)}${summary.draw.state === "stale" ? " · stale" : ""}`;
  }
  return summary.draw.state;
}
function credentialsValue(summary: TokenCommunePoolSummary): string {
  if (summary.credentials.state === "unknown") return "unknown";
  const parts = [
    summary.credentials.fresh > 0 ? `${summary.credentials.fresh} fresh` : "",
    summary.credentials.exhausted > 0 ? `${summary.credentials.exhausted} exhausted` : "",
    summary.credentials.authBroken > 0 ? `${summary.credentials.authBroken} auth broken` : "",
  ].filter(Boolean);
  return parts.join(" · ") || "0 reported";
}
function credentialsLabel(summary: TokenCommunePoolSummary): string {
  const noun = summary.credentials.contributionCount === 1 ? "contribution" : "contributions";
  return `${summary.credentials.contributionCount} ${noun} · credentials ${summary.credentials.state}`;
}
function capacityValue(summary: TokenCommunePoolSummary): string {
  if (summary.capacity5h.state === "current" || summary.capacity5h.state === "stale") {
    return `5h · ${formatPercent(summary.capacity5h.usedFraction)} used`;
  }
  if (summary.capacity5h.state === "no-5h-readings") return "no 5h readings";
  if (summary.capacity5h.state === "reading-unavailable") return "5h reading unavailable";
  return "capacity unknown";
}
function capacityLabel(summary: TokenCommunePoolSummary, now?: Date): string {
  if (summary.capacity5h.state === "current" || summary.capacity5h.state === "stale") {
    const prefix = summary.capacity5h.state === "stale"
      ? `${age(new Date(summary.capacity5h.observedAt), now)} · stale`
      : "highest 5h utilization";
    return prefix;
  }
  return "highest 5h utilization";
}
function telemetryState(summary: TokenCommunePoolSummary): string {
  return summary.verdict === "telemetry-stale" || summary.capacity5h.state === "stale" ? "stale"
    : summary.capacity5h.state === "unknown" ? "unknown" : "current";
}
function verdictTone(verdict: TokenCommuneVerdict): "run" | "warn" | "stop" | "unknown" {
  if (verdict === "runnable") return "run";
  if (verdict === "auth-broken") return "stop";
  if (verdict === "unknown") return "unknown";
  return "warn";
}
function verdictLabel(verdict: TokenCommuneVerdict): string {
  return verdict.replaceAll("-", " ");
}
function formatPercent(fraction: number): string {
  return `${Math.round(fraction * 1000) / 10}%`;
}
function completenessLabel(value: AdapterSnapshotSupport): string {
  if (value === AdapterSnapshotSupport.AUTHORITATIVE) return "authoritative";
  if (value === AdapterSnapshotSupport.PARTIAL) return "partial";
  if (value === AdapterSnapshotSupport.NONE) return "none";
  return "unknown";
}
function age(value: Date, now = new Date()): string {
  const milliseconds = Math.max(0, now.getTime() - value.getTime());
  const seconds = Math.floor(milliseconds / 1000);
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  return `${hours}h ago`;
}
function text<K extends keyof HTMLElementTagNameMap>(
  document: Document,
  tag: K,
  className: string,
  value: string,
): HTMLElementTagNameMap[K] {
  const element = document.createElement(tag);
  element.className = className;
  element.textContent = value;
  return element;
}
