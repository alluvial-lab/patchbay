import assert from "node:assert/strict";
import test from "node:test";

import axe from "axe-core";
import { AdapterSnapshotSupport } from "@patchbay/contracts";
import type { TokenCommunePoolSummary } from "@patchbay/operator-domain";
import { JSDOM } from "jsdom";

import { renderTokenCommunePanel } from "../src/ui/token-commune-panel.js";

function summary(overrides: Partial<TokenCommunePoolSummary> = {}): TokenCommunePoolSummary {
  return {
    key: "token-commune\u0000openai-codex",
    provider: "openai-codex",
    poolIdentity: {
      adapterId: "token-commune",
      resourceKind: "token-commune.provider-pool",
      resourceId: "local:provider-pool:opaque",
    },
    completeness: AdapterSnapshotSupport.PARTIAL,
    poolObservedAt: new Date("2026-08-07T10:00:00Z"),
    draw: { state: "current", limitFraction: 0.25, consumedUnits: 12, resetsAt: null },
    credentials: { state: "current", fresh: 2, exhausted: 0, authBroken: 0, contributionCount: 2 },
    capacity5h: {
      state: "current",
      usedFraction: 0.35,
      observedAt: "2026-08-07T09:59:00Z",
      resetsAt: "2026-08-07T15:00:00Z",
    },
    models: [
      { id: "gpt-5.5", available: true },
      { id: "gpt-5.3-codex-spark", available: false },
    ],
    modelState: "current",
    verdict: "runnable",
    ...overrides,
  };
}

function render(...summaries: TokenCommunePoolSummary[]) {
  const dom = new JSDOM("<!doctype html><html lang='en'><head><title>Panel</title></head><body><main></main></body></html>", {
    runScripts: "dangerously",
  });
  const panel = renderTokenCommunePanel(dom.window.document, {
    summaries,
    refreshedAt: new Date("2026-08-07T10:00:00Z"),
    partial: true,
    formatNow: new Date("2026-08-07T10:07:00Z"),
  });
  dom.window.document.querySelector("main")!.append(panel);
  return { dom, panel };
}

test("option-7 panel renders exact calm signal order and owns every derivation", () => {
  const { panel } = render(summary());
  assert.equal(panel.querySelector(".token-commune-panel__eyebrow")!.textContent, "Resources · token-commune");
  assert.equal(panel.querySelector("h1")!.textContent, "Pools");
  const row = panel.querySelector(".token-commune-pool")!;
  assert.deepEqual([...row.children].map((child) => child.className), [
    "token-commune-pool__left",
    "token-commune-signal token-commune-draw",
    "token-commune-signal token-commune-health",
    "token-commune-signal token-commune-capacity",
    "token-commune-verdict token-commune-verdict--run",
  ]);
  assert.match(row.textContent!, /gpt-5\.5/);
  assert.match(row.textContent!, /gpt-5\.3-codex-spark · unavailable/);
  assert.match(row.textContent!, /25%draw allowance/);
  assert.match(row.textContent!, /2 fresh2 contributions · credentials current/);
  assert.match(row.textContent!, /5h · 35% usedhighest 5h utilization/);

  const footer = panel.querySelector(".token-commune-honesty")!.textContent!;
  assert.match(footer, /native limitFraction/);
  assert.match(footer, /highest real anonymous 5h-window usedFraction/);
  assert.match(footer, /5h is Patchbay's display window/);
  assert.match(footer, /No native pool aggregate exists/);
  assert.match(footer, /Verdicts are a Patchbay synthesis/);
  assert.match(footer, /Patchbay verdict rule: freshness → unknown evidence → auth broken → model unavailable → pool exhausted → runnable/);
  assert.match(footer, /Polled, not streamed/);
  assert.match(footer, /wrapper and underlying reading ages may differ/);
  assert.match(footer, /credential freshness and capacity telemetry freshness are independent axes/);
  assert.match(footer, /Contributor identities and stable contribution IDs are not exposed/);
  assert.match(footer, /Per-contribution drill-down is omitted/);
});

test("stale telemetry dominates live styling without erasing fresh credential evidence", () => {
  const stale = summary({
    credentials: { state: "stale", fresh: 1, exhausted: 0, authBroken: 0, contributionCount: 1 },
    capacity5h: {
      state: "stale", usedFraction: 0.52, observedAt: "2026-08-07T10:00:00Z", resetsAt: null,
    },
    verdict: "telemetry-stale",
  });
  const { panel } = render(stale);
  const row = panel.querySelector(".token-commune-pool")!;
  assert.ok(row.classList.contains("token-commune-pool--stale"));
  assert.equal(row.getAttribute("data-telemetry"), "stale");
  assert.equal(row.querySelector(".token-commune-verdict--run"), null);
  assert.match(row.textContent!, /1 fresh/);
  assert.match(row.textContent!, /credentials stale/);
  assert.match(row.textContent!, /7m ago · stale/);
  assert.match(row.textContent!, /telemetry stale/);
});

test("unknown, null/no-reading, auth, model, exhausted, and runnable outcomes remain distinct", () => {
  const rows = [
    summary({ provider: "unknown", key: "a", capacity5h: { state: "unknown" }, verdict: "unknown" }),
    summary({ provider: "null", key: "b", capacity5h: { state: "reading-unavailable" }, verdict: "unknown" }),
    summary({ provider: "none", key: "c", capacity5h: { state: "no-5h-readings" }, verdict: "auth-broken" }),
    summary({ provider: "model", key: "d", models: [{ id: "safe-unavailable", available: false }], verdict: "model-unavailable" }),
    summary({ provider: "empty", key: "e", verdict: "pool-exhausted" }),
    summary({ provider: "run", key: "f", verdict: "runnable" }),
  ];
  const { panel } = render(...rows);
  const text = panel.textContent!;
  for (const label of ["capacity unknown", "5h reading unavailable", "no 5h readings", "auth broken", "model unavailable", "pool exhausted", "runnable"]) {
    assert.match(text, new RegExp(label));
  }
});

test("surface withholds rejected aliases and contributor/member/raw/aggregate data", () => {
  const unsafe = summary({ models: [{ id: "gpt-5.6", available: true }] });
  const { panel } = render(unsafe);
  const markup = panel.outerHTML;
  assert.doesNotMatch(markup, />gpt-5\.6</);
  assert.match(markup, /rejected catalog alias withheld/);
  assert.doesNotMatch(markup, /private member|subKey|anonymous-contribution|credential reason|raw JSON/i);
  assert.doesNotMatch(panel.textContent!, /pool remaining|average capacity|weighted capacity|aggregate %/i);
  assert.equal(panel.querySelector("[data-contribution-id], [data-member], [data-admin-role]"), null);
});

test("panel has zero critical axe-core violations", async () => {
  const { dom } = render(summary());
  dom.window.eval(axe.source);
  const result = await (dom.window as unknown as { axe: typeof axe }).axe.run(dom.window.document, {
    rules: { "color-contrast": { enabled: false } },
  });
  assert.equal(result.violations.filter((violation) => violation.impact === "critical").length, 0);
});
