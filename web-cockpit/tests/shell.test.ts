import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { create } from "@bufbuild/protobuf";
import {
  AdapterAssuranceManifestSchema,
  AdapterAssuranceManifestV1Schema,
  AdapterCapabilitySummarySchema,
  AdapterIdSchema,
  AdapterReconciliationStrength,
  AdapterStatusSchema,
  AuthorityDomainIdSchema,
  CommandIdSchema,
  ContinuationContextStatus,
  ElicitationState,
  ExternalRuntimeRefSchema,
  FailureCode,
  GenerationSchema,
  IdempotencyStrength,
  LocalSubmissionState,
  LogicalTargetIdSchema,
  OperationKind,
  OperationSchema,
  OperationState,
  PayloadContentType,
  PayloadEnvelopeSchema,
  QuestionContractSchema,
  ReconciliationAction,
  ResourceFreshnessState,
  ResponseContractKind,
  RuntimeGenerationRefSchema,
  ResponseContractSchema,
  ResponseOptionSchema,
  RuntimeSessionIdSchema,
  SessionActivityState,
  SessionConnectivityState,
  SpawnClaimDisposition,
  SubmissionOutcome,
  SubmissionResultSchema,
  TargetScopeKind,
  TargetScopeSchema,
} from "@patchbay/contracts";
import { continuationSpawnPayload } from "@patchbay/operator-domain";
import axe from "axe-core";
import fc from "fast-check";
import { JSDOM } from "jsdom";

import {
  emptyPresentationModel,
  rendersLive,
  resourceCollectionKey,
  resourceKey,
  sessionKey,
  type CommandView,
  type ElicitationView,
  type PresentationModel,
  type ResourceView,
  type SessionIdentity,
  type SessionView,
} from "../src/domain/model.js";
import { createMobileElicitationSheet } from "../src/ui/elicitation.js";
import { renderIcon, type IconName } from "../src/ui/icons.js";
import { createMarkdownRenderer } from "../src/ui/markdown.js";
import {
  operationKindLabel,
  operationStateName,
  renderOperationDelivery,
  retrySafetyPresentation,
} from "../src/ui/operation-delivery.js";
import { renderInstructionCard, renderSessionDetail } from "../src/ui/session-detail.js";
import { renderSessionRow } from "../src/ui/session-list.js";
import { createCockpitShell } from "../src/ui/shell.js";

const DOMAIN = create(AuthorityDomainIdSchema, { value: "operator-domain" });

if (false) {
  // @ts-expect-error The public icon vocabulary rejects untyped consumer names.
  const invalidIconName: IconName = "not-a-lucide-icon";
  void invalidIconName;
}

test("typed icon factory renders Lucide outline geometry", () => {
  const dom = new JSDOM();
  const icon = renderIcon(dom.window.document, "paperclip", { size: "lg" });

  assert.equal(icon.getAttribute("viewBox"), "0 0 24 24");
  assert.equal(icon.getAttribute("stroke-width"), "2");
  assert.equal(icon.getAttribute("aria-hidden"), "true");
  assert.equal(icon.classList.contains("icon"), true);
  assert.equal(icon.classList.contains("icon--lg"), true);
  assert.equal(icon.querySelector("path")?.getAttribute("d"), "m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l8.57-8.57A4 4 0 1 1 18 8.84l-8.59 8.57a2 2 0 0 1-2.83-2.83l8.49-8.48");
});

test("shared Operation delivery renders lifecycle, failure, and contextual actions for any target", () => {
  const dom = new JSDOM();
  const command = runningCommand(session("session-1").identity);
  command.target = {
    kind: "operational-resource",
    identity: { adapterId: "token-commune", resourceKind: "provider_pool", resourceId: "pool-1" },
  };
  let cancelled = "";
  const delivery = renderOperationDelivery(dom.window.document, command, {
    cancel: (selected) => { cancelled = selected.id; },
  });
  dom.window.document.body.append(delivery);

  assert.equal(operationStateName(OperationState.RUNNING), "running");
  assert.equal(operationKindLabel(OperationKind.QUERY), "Query");
  assert.match(delivery.textContent!, /running/);
  assert.match(delivery.textContent!, /Last transition: accepted → running/);
  delivery.querySelector<HTMLButtonElement>('[aria-label="Cancel running operation"]')!.click();
  assert.equal(cancelled, command.id);
});

test("poisoned spawn claim warning renders alongside terminal outcome until disposition releases", () => {
  const dom = new JSDOM();
  const command = runningCommand(session("spawn-poison").identity, "spawn-poisoned");
  command.operation.kind = OperationKind.SPAWN;
  command.state = OperationState.CANCELLED;
  command.failureCode = FailureCode.CANCELLED;
  command.spawnClaimDisposition = SpawnClaimDisposition.POISONED_PENDING_RECONCILIATION;
  command.history = [
    { state: OperationState.DELIVERED, lsn: 2n },
    { state: OperationState.CANCELLED, lsn: 4n, failureCode: FailureCode.CANCELLED },
  ];

  const poisoned = renderOperationDelivery(dom.window.document, command);
  assert.deepEqual(
    [...poisoned.querySelectorAll<HTMLElement>(".failure-banner__term")].map((term) => term.textContent),
    ["cancelled", "execution_outcome_unknown"],
  );
  assert.match(poisoned.textContent!, /Execution may have occurred; evaluate adapter idempotency before retrying/);

  command.spawnClaimDisposition = SpawnClaimDisposition.RELEASED_NO_EXTERNAL_EFFECT;
  const released = renderOperationDelivery(dom.window.document, command);
  assert.deepEqual(
    [...released.querySelectorAll<HTMLElement>(".failure-banner__term")].map((term) => term.textContent),
    ["cancelled"],
  );
  assert.doesNotMatch(released.textContent!, /execution_outcome_unknown/);
});

test("retry safety combines canonical failure with generated assurance and never capability alone", () => {
  const dom = new JSDOM();
  const command = failedCommand(session("retry-matrix").identity);
  command.failureCode = FailureCode.EXECUTION_OUTCOME_UNKNOWN;

  for (const [strength, modifier, label] of [
    [IdempotencyStrength.END_TO_END, "safe", "safe to retry"],
    [IdempotencyStrength.AT_PATCHBAY_BOUNDARY, "maybe", "retry may double-execute"],
    [IdempotencyStrength.NONE, "unsafe", "retry will double-execute"],
  ] as const) {
    const capability = assuranceCapability(strength, ReconciliationAction.MANUAL_REQUIRED);
    const rendered = renderOperationDelivery(
      dom.window.document,
      command,
      undefined,
      false,
      capability,
    );
    const indicator = rendered.querySelector<HTMLElement>(".retry-safety-indicator");
    assert.equal(indicator?.classList.contains(`retry-safety-indicator--${modifier}`), true);
    assert.match(indicator?.textContent ?? "", new RegExp(label));
    assert.match(rendered.textContent ?? "", /Outcome qualifier: manual-required/);
  }

  const unknownQualified = renderOperationDelivery(
    dom.window.document,
    command,
    undefined,
    false,
    assuranceCapability(
      IdempotencyStrength.AT_PATCHBAY_BOUNDARY,
      ReconciliationAction.NONE,
    ),
  );
  assert.match(unknownQualified.textContent ?? "", /Outcome qualifier: unknown/);
  assert.doesNotMatch(unknownQualified.textContent ?? "", /manual-required/);

  const maximal = assuranceCapability(
    IdempotencyStrength.END_TO_END,
    ReconciliationAction.MANUAL_REQUIRED,
  );
  assert.equal(
    retrySafetyPresentation(FailureCode.CANCELLED, maximal),
    undefined,
    "capability alone cannot create a retry decision",
  );
  const noFailure = runningCommand(session("capability-alone").identity);
  const rendered = renderOperationDelivery(
    dom.window.document,
    noFailure,
    undefined,
    false,
    maximal,
  );
  assert.equal(rendered.querySelector(".retry-safety-indicator"), null);

  const preExecution = retrySafetyPresentation(
    FailureCode.TARGET_OFFLINE,
    assuranceCapability(IdempotencyStrength.NONE, ReconciliationAction.NONE),
  );
  assert.equal(preExecution?.modifier, "safe");
});

test("poisoned spawn exposes permanent target abandonment and terminal disposition removes it", () => {
  const dom = new JSDOM();
  const command = runningCommand(session("spawn-abandon").identity, "spawn-abandon");
  command.operation.kind = OperationKind.SPAWN;
  command.state = OperationState.CANCELLED;
  command.spawnLogicalTargetId = "logical-spawn-abandon";
  command.spawnClaimDisposition = SpawnClaimDisposition.POISONED_PENDING_RECONCILIATION;
  let abandoned = "";
  const actionable = renderOperationDelivery(dom.window.document, command, {
    abandonSpawnTarget: (selected) => { abandoned = selected.spawnLogicalTargetId ?? ""; },
  });
  actionable
    .querySelector<HTMLButtonElement>('[aria-label="Permanently abandon this managed target"]')!
    .click();
  assert.equal(abandoned, "logical-spawn-abandon");

  command.spawnClaimDisposition = SpawnClaimDisposition.TARGET_ABANDONED;
  const terminal = renderOperationDelivery(dom.window.document, command, {
    abandonSpawnTarget: () => undefined,
  });
  assert.equal(
    terminal.querySelector('[aria-label="Permanently abandon this managed target"]'),
    null,
  );
  assert.match(terminal.textContent!, /Claim: target_abandoned/);
});

test("delivery reserves one action slot and exposes only state-valid actions", () => {
  const dom = new JSDOM();
  const states = [
    OperationState.ACCEPTED,
    OperationState.DELIVERED,
    OperationState.RUNNING,
    OperationState.COMPLETED,
    OperationState.REJECTED,
    OperationState.FAILED,
    OperationState.EXPIRED,
    OperationState.CANCELLED,
    OperationState.SUPERSEDED,
  ];
  for (const state of states) {
    const command = runningCommand(session("delivery").identity);
    command.state = state;
    command.history = [{ state, lsn: 1n }];
    const delivery = renderOperationDelivery(dom.window.document, command, {
      cancel: () => undefined,
      interrupt: () => undefined,
    });
    const slot = delivery.querySelector<HTMLElement>(".delivery-line__actions")!;
    const expected = state === OperationState.RUNNING
      ? [`Cancel running operation`, "Interrupt running operation"]
      : state === OperationState.ACCEPTED || state === OperationState.DELIVERED
        ? [`Cancel ${operationStateName(state)} operation`]
        : [];
    assert.deepEqual(
      [...slot.querySelectorAll("button")].map((button) => button.getAttribute("aria-label")),
      expected,
    );
    assert.equal(slot.getAttribute("aria-hidden"), expected.length === 0 ? "true" : null);
  }
});

test("shell stylesheet provides responsive layout without rebinding protocol states", async () => {
  const css = await readFile(new URL("../src/ui/shell.css", import.meta.url), "utf8").catch(
    () => readFile(new URL("../../src/ui/shell.css", import.meta.url), "utf8"),
  );
  assert.match(css, /\.cockpit\s*\{[^}]*display:\s*flex/s);
  assert.match(css, /\.cockpit \.timeline\s*\{[^}]*max-width:\s*860px/s);
  assert.match(css, /\.cockpit \.msg--agent\s*\{[^}]*560px/s);
  assert.match(css, /@media \(max-width:\s*760px\)/);
  assert.match(css, /\.cockpit \.session-row\s*\{[^}]*min-width:\s*0;[^}]*overflow:\s*hidden/s);
  assert.match(
    css,
    /\.cockpit \.session-row__identity,[\s\S]*?\.cockpit \.session-row__context\s*\{[^}]*min-width:\s*0;[^}]*overflow:\s*hidden;[^}]*text-overflow:\s*ellipsis;[^}]*white-space:\s*nowrap;/,
  );
  assert.match(css, /\.cockpit \.instruction-card__delivery\s*\{[^}]*min-height:\s*56px/s);
  assert.match(css, /\.cockpit \.delivery-line__actions\s*\{[^}]*flex:\s*0 0 96px/s);
  assert.match(css, /--mobile-bottom-nav-reserve:\s*calc\(72px \+ env\(safe-area-inset-bottom, 0px\)\)/);
  assert.match(css, /\.cockpit \.cockpit__content\s*\{[^}]*padding-bottom:\s*var\(--mobile-bottom-nav-reserve\)/s);
  assert.match(css, /\.cockpit \.composer\s*\{[^}]*bottom:\s*var\(--mobile-bottom-nav-reserve\)/s);
  assert.match(css, /\.cockpit \.delivery-line__actions \.btn\s*\{[^}]*min-width:\s*44px[^}]*min-height:\s*44px/s);
  assert.match(css, /\.settings-dialog\s*\{[^}]*padding:\s*var\(--space-6\)/s);
  assert.doesNotMatch(css, /connectivity-indicator--(?:live|stale|offline|unknown|failed)/);
  assert.doesNotMatch(css, /command-step--(?:accepted|delivered|running|completed|rejected|failed|expired|cancelled|superseded)/);
  assert.doesNotMatch(css, /resource-freshness--(?:current|stale|unknown)/);
});

test("desktop shell is two-pane and rows lead with identity before label metadata", () => {
  const dom = new JSDOM();
  const first = session("session-1", 1n, { name: "core", project: "patchbay", cwd: "/projects/patchbay" });
  first.model = "provider/model-1";
  const second = session("session-2", 1n, { name: "adapter", project: "patchbay", cwd: "/projects/patchbay" });
  second.needsYou = true;
  const model = withSessions(first, second);
  const shell = createCockpitShell(dom.window.document, model, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    isMobile: () => false,
  });
  dom.window.document.body.append(shell.element);

  assert.equal(shell.element.dataset.layout, "two-pane");
  assert.equal(shell.element.querySelector<HTMLElement>(".sidebar")!.hidden, false);
  assert.equal(shell.element.querySelector<HTMLElement>(".main")!.hidden, false);
  assert.equal(shell.detail.header.hidden, true);
  const sessionList = shell.element.querySelector<HTMLElement>(".session-list")!;
  const rows = [...sessionList.querySelectorAll<HTMLButtonElement>(".session-row")];
  assert.equal(sessionList.localName, "ul");
  assert.equal(sessionList.querySelectorAll(":scope > li.session-list__item").length, 2);
  assert.equal(rows.length, 2);
  assert.equal(rows.every((row) => row.localName === "button" && !row.hasAttribute("role")), true);
  assert.equal(rows[0]!.firstElementChild!.className, "session-row__identity");
  assert.match(rows[0]!.querySelector(".session-row__identity")!.textContent!, /pi@laptop · runtime session-2 · gen 1/);
  assert.equal(rows[0]!.classList.contains("session-row--needs-you"), true);
  assert.match(rows[0]!.querySelector(".session-row__context")!.getAttribute("aria-label")!, /\/projects\/patchbay/);
  assert.ok(rows[0]!.querySelector(".connectivity-indicator"));
  assert.ok(rows[0]!.querySelector(".activity-indicator"));
  assert.match(rows[1]!.textContent!, /provider\/model-1/);
  const unavailableControls = [...shell.element.querySelectorAll<HTMLButtonElement>(".sidebar__actions button")];
  assert.deepEqual(unavailableControls.map((button) => button.getAttribute("aria-label")), ["Spawn session unavailable", "Attach session unavailable"]);
  assert.equal(unavailableControls.every((button) => button.disabled && button.querySelector('.icon[aria-hidden="true"]')), true);

  shell.select(sessionKey(first.identity));
  assert.match(shell.detail.header.textContent!, /provider\/model-1/);
  assert.equal(shell.detail.element.dataset.sessionKey, sessionKey(first.identity));
  assert.equal(shell.element.querySelectorAll(".session-detail").length, 1);
});

test("session-list spawn action is inert while lockdown is pending or active", () => {
  for (const lockdown of [
    { active: true, submitting: false },
    { active: false, submitting: true },
  ]) {
    const dom = new JSDOM();
    const model = withAdapterCapabilities(withSessions(session("session-1")));
    model.lockdown = lockdown;
    let spawnCalls = 0;
    const shell = createCockpitShell(dom.window.document, model, {
      markdown: createMarkdownRenderer(dom.window as unknown as Window),
      actions: {
        spawn: () => { spawnCalls += 1; },
      },
    });
    const spawn = shell.element.querySelector<HTMLButtonElement>(".sidebar__actions button")!;
    assert.equal(spawn.disabled, true);
    assert.match(spawn.title, /Disabled during lockdown or while a lockdown decision is pending/);
    spawn.click();
    assert.equal(spawnCalls, 0);
  }
});

test("destination rail uses the signed-off punch-out shell and persists panel collapse", () => {
  const dom = new JSDOM();
  const model = withSessions(session("session-1"));
  let saved = false;
  const shell = createCockpitShell(dom.window.document, model, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    isMobile: () => false,
    preferenceStore: {
      load: () => ({ sessionsPanelCollapsed: false, showToolCalls: true }),
      save: (_domain, value) => { saved = value.sessionsPanelCollapsed; },
    },
  });
  dom.window.document.body.append(shell.element);
  const rail = shell.element.querySelector(".rail")!;
  assert.deepEqual(
    [...rail.querySelectorAll("button")].map((button) => button.getAttribute("aria-label")),
    ["Sessions", "Resources", "Security", "Diagnostics", "Files", "Git", "Settings"],
  );
  rail.querySelector<HTMLButtonElement>('[data-destination="security"]')!.click();
  assert.equal(shell.element.dataset.destination, "security");
  assert.equal(shell.element.querySelector<HTMLElement>(".security-view")!.hidden, false);
  assert.equal([...shell.element.querySelectorAll("button")].some((button) => /exit/i.test(button.textContent ?? "")), false);
  rail.querySelector<HTMLButtonElement>('[data-destination="sessions"]')!.click();
  rail.querySelector<HTMLButtonElement>('[data-destination="sessions"]')!.click();
  assert.equal(saved, true);
  assert.equal(shell.element.classList.contains("cockpit--panel-collapsed"), true);
});

test("settings preference is domain-scoped, modal, keyboard-contained, and presentation-only", async () => {
  const dom = new JSDOM("<!doctype html><body></body>", { url: "https://patchbay.test" });
  const view = session("session-1");
  view.activityDetail = "using bash";
  view.activityDetailProvenance = "tool";
  const model = withSessions(view);
  model.observations.push({
    id: "tool-1",
    session: view.identity,
    role: "tool",
    kind: "tool_requested",
    markdown: "Running **bash**",
    detail: "pwd",
    lsn: 2n,
  });
  const saved = new Map<string, { sessionsPanelCollapsed: boolean; showToolCalls: boolean }>();
  const store = {
    load(domain: string) {
      return saved.get(domain) ?? { sessionsPanelCollapsed: false, showToolCalls: true };
    },
    save(domain: string, value: { sessionsPanelCollapsed: boolean; showToolCalls: boolean }) {
      saved.set(domain, value);
    },
  };
  const shell = createCockpitShell(dom.window.document, model, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    isMobile: () => false,
    preferenceStore: store,
  });
  dom.window.document.body.append(shell.element);
  const opener = shell.element.querySelector<HTMLButtonElement>('.rail [data-destination="settings"]')!;
  opener.focus();
  opener.click();
  await Promise.resolve();

  let dialog = shell.element.querySelector<HTMLElement>(".settings-dialog")!;
  assert.equal(dialog.getAttribute("aria-labelledby"), "cockpit-settings-title");
  assert.equal(dialog.getAttribute("aria-modal"), "true");
  assert.match(dialog.textContent!, /Authority domain: operator-domain/);
  assert.equal(shell.element.querySelector(".cockpit__content")?.hasAttribute("inert"), true);
  const backdrop = shell.element.querySelector<HTMLElement>(".settings-backdrop")!;
  assert.equal(backdrop.localName, "div");
  assert.equal(backdrop.tabIndex, -1);

  let toggle = dialog.querySelector<HTMLButtonElement>(".settings-toggle")!;
  assert.equal(toggle.getAttribute("aria-pressed"), "true");
  assert.equal(dom.window.document.activeElement, toggle);
  toggle.click();
  await Promise.resolve();
  assert.equal(saved.get("operator-domain")?.showToolCalls, false);
  assert.equal(shell.element.querySelector(".msg--tool"), null);
  assert.equal(shell.element.textContent!.includes("using bash"), false);
  assert.ok(shell.element.querySelector(".activity-indicator--working"));
  assert.equal(model.observations.length, 1);

  dialog = shell.element.querySelector<HTMLElement>(".settings-dialog")!;
  toggle = dialog.querySelector<HTMLButtonElement>(".settings-toggle")!;
  const close = dialog.querySelector<HTMLButtonElement>('[aria-label="Close cockpit settings"]')!;
  toggle.focus();
  toggle.dispatchEvent(new dom.window.KeyboardEvent("keydown", { key: "Tab", bubbles: true, cancelable: true }));
  assert.equal(dom.window.document.activeElement, close);
  close.dispatchEvent(new dom.window.KeyboardEvent("keydown", { key: "Tab", shiftKey: true, bubbles: true, cancelable: true }));
  assert.equal(dom.window.document.activeElement, toggle);

  dialog.dispatchEvent(new dom.window.KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }));
  await Promise.resolve();
  assert.equal(shell.element.querySelector(".settings-dialog"), null);
  assert.equal(dom.window.document.activeElement, shell.element.querySelector('.rail [data-destination="settings"]'));

  shell.selectDestination("settings");
  await Promise.resolve();
  shell.element.querySelector<HTMLButtonElement>(".settings-toggle")!.click();
  await Promise.resolve();
  assert.equal(shell.element.querySelector(".msg--tool") !== null, true);

  const otherDom = new JSDOM("<!doctype html><body></body>", { url: "https://patchbay.test" });
  const other = createCockpitShell(otherDom.window.document, { ...withSessions(session("other")), authorityDomainId: "other-domain" }, {
    markdown: createMarkdownRenderer(otherDom.window as unknown as Window),
    isMobile: () => false,
    preferenceStore: store,
  });
  other.selectDestination("settings");
  await Promise.resolve();
  assert.equal(other.element.querySelector<HTMLButtonElement>(".settings-toggle")!.getAttribute("aria-pressed"), "true");
});

test("Settings close restores the reusable mobile Elicitation sheet and a currently visible opener", async () => {
  const dom = new JSDOM("<!doctype html><body></body>", { url: "https://patchbay.test" });
  let mobile = false;
  const view = session("session-1");
  const model = withSessions(view);
  model.elicitations.set("question-1", question(view.identity));
  const mobileSheet = createMobileElicitationSheet(dom.window.document, { isMobile: () => mobile });
  const shell = createCockpitShell(dom.window.document, model, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    isMobile: () => mobile,
    elicitation: {
      mobileSheet,
      operationContext: () => { throw new Error("not submitted"); },
      submit: () => undefined,
    },
  });
  dom.window.document.body.append(shell.element);
  shell.element.querySelector<HTMLButtonElement>('.rail [data-destination="settings"]')!.click();
  await Promise.resolve();
  assert.equal(mobileSheet.element.hasAttribute("inert"), true);

  mobile = true;
  shell.refreshLayout();
  shell.element.querySelector<HTMLButtonElement>('[aria-label="Close cockpit settings"]')!.click();
  await Promise.resolve();

  const visibleOpener = shell.element.querySelector<HTMLButtonElement>("#cockpit-more-destinations")!;
  assert.equal(dom.window.document.activeElement, visibleOpener);
  assert.equal(mobileSheet.element.hasAttribute("inert"), false);
  assert.equal(mobileSheet.backdrop.hasAttribute("inert"), false);

  shell.select(sessionKey(view.identity));
  shell.element.querySelector<HTMLElement>(".elicitation-card--inline-teaser")!.click();
  assert.equal(mobileSheet.isOpen, true);
  assert.equal(mobileSheet.element.hasAttribute("inert"), false);
  assert.equal(dom.window.document.activeElement, mobileSheet.element.querySelector(".sheet__close"));
});

test("actual production mount has no axe-core violations before settings, during modal, or on mobile", async () => {
  const html = await readFile(new URL("../index.html", import.meta.url), "utf8");
  const dom = new JSDOM(html, {
    runScripts: "dangerously",
    url: "https://patchbay.test",
  });
  let mobile = false;
  const view = session("session-1");
  const model = withSessions(view);
  model.elicitations.set("question-1", question(view.identity));
  const mobileSheet = createMobileElicitationSheet(dom.window.document, { isMobile: () => mobile });
  const shell = createCockpitShell(dom.window.document, model, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    isMobile: () => mobile,
    elicitation: {
      mobileSheet,
      operationContext: () => { throw new Error("not submitted"); },
      submit: () => undefined,
    },
  });
  const mount = dom.window.document.querySelector<HTMLElement>("main[data-patchbay-cockpit]")!;
  mount.replaceChildren(shell.element);
  assert.equal(dom.window.document.querySelectorAll("main").length, 1);
  assert.equal(shell.element.querySelector("main"), null);

  dom.window.eval(axe.source);
  await assertAxeClean(dom, "production desktop mount");

  shell.element.querySelector<HTMLButtonElement>('.rail [data-destination="settings"]')!.click();
  await Promise.resolve();
  await assertAxeClean(dom, "production Settings modal");

  shell.element.querySelector<HTMLButtonElement>('[aria-label="Close cockpit settings"]')!.click();
  await Promise.resolve();
  mobile = true;
  shell.select(sessionKey(view.identity));
  shell.refreshLayout();
  shell.element.querySelector<HTMLElement>(".elicitation-card--inline-teaser")!.click();
  assert.equal(mobileSheet.isOpen, true);
  await assertAxeClean(dom, "production mobile Elicitation state");
});

test("mobile reserves bottom-navigation space and More has a complete expanded-state contract", async () => {
  const dom = new JSDOM();
  const shell = createCockpitShell(dom.window.document, withSessions(session("session-1")), {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    isMobile: () => true,
  });
  dom.window.document.body.append(shell.element);
  assert.equal(shell.element.querySelectorAll(".bottom-tabs .tabs__tab").length, 4);
  const tabs = shell.element.querySelector<HTMLElement>(".bottom-tabs")!;
  assert.equal(tabs.getAttribute("aria-label"), "Cockpit destinations");
  assert.equal(tabs.dataset.viewportObstruction, "bottom-tabs");
  assert.equal(shell.element.querySelector(".cockpit__content")?.getAttribute("data-mobile-bottom-nav-reserve"), "bottom-tabs");
  assert.ok(shell.element.querySelector('.session-list[data-mobile-bottom-target="session-list"]'));
  assert.ok(shell.element.querySelector('.session-detail[data-mobile-bottom-target="session-detail"]'));
  assert.ok(shell.element.querySelector('.composer[data-mobile-bottom-target="composer"]'));

  let more = shell.element.querySelector<HTMLButtonElement>("#cockpit-more-destinations")!;
  assert.equal(more.getAttribute("aria-controls"), "cockpit-overflow-menu");
  assert.equal(more.getAttribute("aria-expanded"), "false");
  more.click();
  assert.equal(more.getAttribute("aria-expanded"), "true");
  assert.equal(shell.element.classList.contains("more-open"), true);
  shell.element.querySelector<HTMLButtonElement>('.overflow-menu [data-destination="settings"]')!.click();
  await Promise.resolve();
  assert.equal(shell.element.classList.contains("more-open"), false);
  assert.ok(shell.element.querySelector(".settings-dialog"));
  shell.element.querySelector<HTMLElement>(".settings-dialog")!.dispatchEvent(
    new dom.window.KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }),
  );
  await Promise.resolve();
  more = shell.element.querySelector<HTMLButtonElement>("#cockpit-more-destinations")!;
  assert.equal(shell.element.classList.contains("more-open"), false);
  assert.equal(more.getAttribute("aria-expanded"), "false");
  assert.equal(dom.window.document.activeElement, more);

  shell.element.querySelector<HTMLButtonElement>('.bottom-tabs [aria-label="Security"]')!.click();
  assert.equal(shell.element.dataset.destination, "security");
});

test("pooled session linkage opens the exact resource and mobile resources drill in and back", () => {
  const linkedSession = session("session-linked");
  const resource = pooledResource("shared-pool");
  linkedSession.resourceLinkage = { usageResource: resource.identity };
  const model = withSessions(linkedSession);
  model.resources.set(resourceKey(resource.identity), resource);
  model.resourceCollections.set(resourceCollectionKey("token-commune", "provider_pool"), {
    adapterId: "token-commune",
    resourceKind: "provider_pool",
    completeness: 1,
    sourceAdapterGeneration: 1n,
    revisionLsn: 8n,
    reconciled: true,
  });

  const desktop = createCockpitShell(new JSDOM().window.document, model, {
    markdown: createMarkdownRenderer(new JSDOM().window as unknown as Window),
    isMobile: () => false,
  });
  const link = desktop.element.querySelector<HTMLButtonElement>(".runtime-resource-link__button")!;
  assert.equal(link.disabled, false);
  link.click();
  assert.equal(desktop.element.dataset.destination, "resources");
  assert.equal(desktop.selectedResourceKey, resourceKey(resource.identity));
  assert.match(desktop.element.querySelector(".resource-detail")!.textContent!, /shared-pool/);

  const dom = new JSDOM();
  const mobile = createCockpitShell(dom.window.document, model, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    isMobile: () => true,
  });
  dom.window.document.body.append(mobile.element);
  mobile.element.querySelector<HTMLButtonElement>('.bottom-tabs [aria-label="Resources"]')!.click();
  assert.equal(mobile.element.dataset.destination, "resources");
  assert.equal(mobile.element.querySelector<HTMLElement>(".resource-list")!.hidden, false);
  mobile.element.querySelector<HTMLButtonElement>(".resource-row")!.click();
  assert.equal(mobile.element.querySelector<HTMLElement>(".resource-list")!.hidden, true);
  assert.equal(mobile.element.querySelector<HTMLElement>(".resource-detail")!.hidden, false);
  mobile.element.querySelector<HTMLButtonElement>(".resource-detail__back")!.click();
  assert.equal(mobile.element.querySelector<HTMLElement>(".resource-list")!.hidden, false);
});

test("session rows keep long cwd context accessible without changing identity keys", () => {
  const dom = new JSDOM();
  const view = session("session-long", 4n, {
    name: "Long context",
    project: "patchbay",
    cwd: "/srv/agents/patchbay/worktrees/feature-with-a-very-long-name",
  });
  const row = renderSessionRow(dom.window.document, view, false, () => undefined);
  const context = row.querySelector<HTMLElement>(".session-row__context")!;
  assert.match(context.textContent!, /feature-with-a-very-long-name/);
  assert.equal(context.title, context.textContent);
  assert.match(context.getAttribute("aria-label")!, /Session context:/);
  assert.equal(row.dataset.sessionKey, sessionKey(view.identity));
});

test("session rows render unavailable models honestly", () => {
  const dom = new JSDOM();
  const row = renderSessionRow(dom.window.document, session("session-unknown"), false, () => undefined);
  assert.match(row.textContent!, /Model unknown/);
});

test("hidden tool calls suppress tool-derived detail everywhere but preserve canonical and runtime activity", () => {
  const dom = new JSDOM();
  const toolSession = session("tool-session");
  toolSession.activityDetail = "using bash";
  toolSession.activityDetailProvenance = "tool";
  const model = withSessions(toolSession);
  model.observations.push({
    id: "tool-call",
    session: toolSession.identity,
    role: "tool",
    kind: "tool_finished",
    markdown: "**bash** failed",
    detail: "exit 1",
    lsn: 2n,
  });

  const row = renderSessionRow(dom.window.document, toolSession, false, () => undefined, undefined, false);
  assert.doesNotMatch(row.textContent!, /using bash/);
  assert.match(row.textContent!, /working/);

  const detail = renderSessionDetail(dom.window.document, model, toolSession, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    showToolCalls: false,
  });
  dom.window.document.body.append(detail.element);
  assert.equal(detail.element.querySelector(".msg--tool"), null);
  assert.doesNotMatch(detail.header.textContent!, /using bash/);
  assert.doesNotMatch(detail.element.querySelector(".timeline-activity")!.textContent!, /using bash/);
  assert.match(detail.element.querySelector(".timeline-activity")!.textContent!, /working/);

  const runtimeSession = session("runtime-session");
  runtimeSession.activityDetail = "responding";
  runtimeSession.activityDetailProvenance = "runtime";
  const runtimeRow = renderSessionRow(dom.window.document, runtimeSession, false, () => undefined, undefined, false);
  assert.match(runtimeRow.textContent!, /responding/);
});

test("mobile drill-in swaps containers around the same detail component", () => {
  const dom = new JSDOM();
  let mobile = true;
  const view = session("session-1");
  const shell = createCockpitShell(dom.window.document, withSessions(view), {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    isMobile: () => mobile,
  });
  dom.window.document.body.append(shell.element);

  const sidebar = () => shell.element.querySelector<HTMLElement>(".sidebar")!;
  const main = () => shell.element.querySelector<HTMLElement>(".main")!;
  assert.equal(shell.element.dataset.layout, "drill-in");
  assert.equal(sidebar().hidden, false);
  assert.equal(main().hidden, true);

  shell.select(sessionKey(view.identity));
  const sharedDetail = shell.detail.element;
  assert.equal(sidebar().hidden, true);
  assert.equal(main().hidden, false);
  assert.equal(shell.detail.header.hidden, false);
  shell.back();
  assert.equal(sidebar().hidden, false);
  assert.equal(main().hidden, true);

  mobile = false;
  shell.refreshLayout();
  assert.equal(shell.detail.element, sharedDetail);
  assert.equal(shell.element.dataset.layout, "two-pane");
  assert.equal(sidebar().hidden, false);
  assert.equal(main().hidden, false);
  assert.equal(shell.detail.header.hidden, true);
});

test("identity-before-submission holds across generated incomplete and superseded targets", async () => {
  await fc.assert(
    fc.asyncProperty(
      fc.record({
        adapter: fc.boolean(),
        scope: fc.boolean(),
        runtime: fc.boolean(),
        positiveGeneration: fc.boolean(),
        reconciled: fc.boolean(),
        tombstoned: fc.boolean(),
      }),
      async (shape) => {
        const dom = new JSDOM();
        const view = session("session-1");
        view.identity = {
          adapterId: shape.adapter ? "pi" : "",
          deploymentScope: shape.scope ? "laptop" : "",
          runtimeSessionId: shape.runtime ? "session-1" : "",
          generation: shape.positiveGeneration ? 1n : 0n,
        };
        view.reconciled = shape.reconciled;
        view.tombstoned = shape.tombstoned;
        let sends = 0;
        const detail = renderSessionDetail(dom.window.document, withSessions(view), view, {
          markdown: createMarkdownRenderer(dom.window as unknown as Window),
          actions: { send: () => { sends += 1; } },
        });
        dom.window.document.body.append(detail.element);
        detail.input.value = "continue";
        detail.input.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
        const stable = shape.adapter && shape.scope && shape.runtime && shape.positiveGeneration && shape.reconciled && !shape.tombstoned;
        assert.equal(detail.sendButton.disabled, !stable);
        detail.sendButton.click();
        assert.equal(sends, stable ? 1 : 0);
      },
    ),
    { numRuns: 100 },
  );
});

test("stale-never-live binding holds across generated reconciliation states", async () => {
  await fc.assert(
    fc.asyncProperty(
      fc.constantFrom(
        SessionConnectivityState.LIVE,
        SessionConnectivityState.STALE,
        SessionConnectivityState.OFFLINE,
        SessionConnectivityState.UNKNOWN,
        SessionConnectivityState.FAILED,
      ),
      fc.boolean(),
      fc.boolean(),
      async (connectivity, reconciled, tombstoned) => {
        const dom = new JSDOM();
        const view = session("session-1");
        view.connectivity = connectivity;
        view.reconciled = reconciled;
        view.tombstoned = tombstoned;
        const row = renderSessionRow(dom.window.document, view, false, () => undefined);
        assert.equal(Boolean(row.querySelector(".connectivity-indicator--live")), rendersLive(view));
        if (!rendersLive(view)) assert.equal(Boolean(row.querySelector(".connectivity-indicator--stale")) || connectivity !== SessionConnectivityState.LIVE, true);
      },
    ),
    { numRuns: 100 },
  );
});

test("typed command correlation prevents concurrent or reconnect transcript mismatches", () => {
  const dom = new JSDOM();
  const view = session("session-1");
  const model = withSessions(view);
  const first = runningCommand(view.identity, "command-a", "Payload from command A");
  const second = runningCommand(view.identity, "command-b", "Payload from command B");
  model.commands.set(first.id, first);
  model.commands.set(second.id, second);
  model.observations.push(
    {
      id: "operator-correlated",
      session: view.identity,
      role: "operator",
      kind: "user_confirmed",
      markdown: "Authoritative transcript for command B",
      commandId: second.id,
      lsn: 4n,
    },
    {
      id: "operator-uncorrelated",
      session: view.identity,
      role: "operator",
      kind: "user_confirmed",
      markdown: "Replayed transcript with no command correlation",
      lsn: 5n,
    },
    {
      id: "operator-missing-command",
      session: view.identity,
      role: "operator",
      kind: "user_confirmed",
      markdown: "Transcript with an unavailable command",
      commandId: "command-not-in-snapshot",
      lsn: 6n,
    },
  );

  const detail = renderSessionDetail(dom.window.document, model, view, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
  });
  dom.window.document.body.append(detail.element);

  const merged = detail.element.querySelector<HTMLElement>('[data-command-id="command-b"]')!;
  assert.match(merged.querySelector(".instruction-card__body")!.textContent!, /Authoritative transcript for command B/);
  assert.doesNotMatch(merged.textContent!, /Payload from command B/);
  assert.match(detail.element.querySelector<HTMLElement>('[data-command-id="command-a"]')!.textContent!, /Payload from command A/);
  const plainOperatorMessages = [...detail.element.querySelectorAll<HTMLElement>(".msg--operator")]
    .filter((message) => !message.querySelector(".instruction-card"))
    .map((message) => message.textContent);
  assert.equal(plainOperatorMessages.some((text) => text?.includes("Replayed transcript with no command correlation")), true);
  assert.equal(plainOperatorMessages.some((text) => text?.includes("Transcript with an unavailable command")), true);
  assert.equal(detail.element.querySelectorAll(".instruction-card").length, 2);
});

test("detail integrates markdown, current plus last delivery, failures, contextual actions, and elicitations", () => {
  const dom = new JSDOM();
  const view = session("session-1");
  const model = withAdapterCapabilities(withSessions(view));
  const command = runningCommand(view.identity);
  model.commands.set(command.id, command);
  const failed = failedCommand(view.identity);
  model.commands.set(failed.id, failed);
  model.observations.push(
    {
      id: "operator-1",
      messageId: "operator-1",
      session: view.identity,
      role: "operator",
      kind: "user_confirmed",
      markdown: "Run the checks",
      commandId: command.id,
      lsn: 3n,
    },
    {
      id: "agent-1",
      messageId: "agent-1",
      session: view.identity,
      role: "agent",
      kind: "assistant_committed",
      markdown: "## Result\n\nChecks are **running**.",
      lsn: 4n,
    },
  );
  model.elicitations.set("question-1", question(view.identity));
  let cancelled = 0;
  let interrupted = 0;
  const detail = renderSessionDetail(dom.window.document, model, view, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    actions: {
      send: () => undefined,
      cancel: () => { cancelled += 1; },
      interrupt: () => { interrupted += 1; },
    },
    elicitation: {
      operationContext: () => {
        throw new Error("not submitted in this test");
      },
      submit: () => undefined,
    },
  });
  dom.window.document.body.append(detail.element);

  assert.ok(detail.element.querySelector(".msg--agent .markdown-body h2"));
  const delivery = detail.element.querySelector<HTMLElement>('[data-command-id="command-1"]')!;
  assert.equal(detail.element.querySelectorAll(".instruction-card").length, 2);
  assert.equal(delivery.closest(".instruction-card") !== null, true);
  assert.ok(delivery.querySelector(".delivery-line__actions"));
  assert.match(delivery.textContent!, /running/);
  assert.match(delivery.textContent!, /Last transition: accepted → running/);
  assert.equal(delivery.textContent!.includes("LSN"), false);
  assert.equal(delivery.querySelector("details"), null);
  const failure = detail.element.querySelector<HTMLElement>('[data-command-id="command-failed"] .failure-banner')!;
  assert.match(failure.textContent!, /execution_failed/);
  assert.ok(detail.element.querySelector(".elicitation-card"));
  assert.equal(detail.composer.querySelector("select"), null);

  const contextual = [...detail.element.querySelectorAll<HTMLButtonElement>(".delivery-line .btn")];
  assert.deepEqual(contextual.map((button) => button.getAttribute("aria-label")), ["Cancel running operation", "Interrupt running operation"]);
  assert.equal(contextual.every((button) => button.title === button.getAttribute("aria-label") && button.querySelector('.icon[aria-hidden="true"]')), true);
  const composerControls = [...detail.composer.querySelectorAll<HTMLButtonElement>("button")];
  assert.deepEqual(composerControls.map((button) => button.getAttribute("aria-label")), ["Attach file or image", "Send instruction"]);
  assert.equal(composerControls.every((button) => button.title === button.getAttribute("aria-label") && button.querySelector('.icon[aria-hidden="true"]')), true);
  assert.equal(detail.header.querySelector<HTMLButtonElement>("button")?.getAttribute("aria-label"), "Back to sessions");
  contextual[0]!.click();
  contextual[1]!.click();
  assert.equal(cancelled, 1);
  assert.equal(interrupted, 1);
});

test("continuation cards render only generated adapter-reported context evidence", () => {
  const dom = new JSDOM();
  const view = session("session-1");
  const command = runningCommand(view.identity, "spawn-continuation");
  command.operation.kind = OperationKind.SPAWN;
  command.operation.payload = continuationSpawnPayload(
    create(RuntimeGenerationRefSchema, {
      logicalTargetId: create(LogicalTargetIdSchema, { value: "logical-1" }),
      externalRuntime: create(ExternalRuntimeRefSchema, {
        adapterId: create(AdapterIdSchema, { value: view.identity.adapterId }),
        deploymentScope: view.identity.deploymentScope,
        runtimeSessionId: create(RuntimeSessionIdSchema, {
          value: view.identity.runtimeSessionId,
        }),
        generation: create(GenerationSchema, { value: view.identity.generation }),
      }),
    }),
    { shape: "session" },
  );
  command.continuationContextStatus = ContinuationContextStatus.RESUMED;

  const promoted = renderInstructionCard(dom.window.document, command);
  assert.match(promoted.querySelector(".continuation-context")!.textContent!, /Context: resumed/);
  assert.doesNotMatch(promoted.querySelector(".continuation-context")!.textContent!, /unknown/);

  delete command.continuationContextStatus;
  const pending = renderInstructionCard(dom.window.document, command);
  assert.equal(
    pending.querySelector(".continuation-context")!.textContent,
    "Context: pending adapter report",
  );
});

test("session delivery actions require adapter-declared support and lockdown keeps supported actions inert", () => {
  const view = session("session-1");
  const callbacks = {
    cancel: () => undefined,
    interrupt: () => undefined,
  };

  const unsupportedDom = new JSDOM();
  const unsupportedModel = withAdapterCapabilities(
    withSessions(view),
    [OperationKind.CANCEL, OperationKind.INTERRUPT],
    false,
  );
  unsupportedModel.commands.set("command-1", runningCommand(view.identity));
  const unsupported = renderSessionDetail(unsupportedDom.window.document, unsupportedModel, view, {
    markdown: createMarkdownRenderer(unsupportedDom.window as unknown as Window),
    actions: callbacks,
  });
  assert.equal(unsupported.element.querySelectorAll(".delivery-line__actions button").length, 0);
  assert.equal(unsupported.element.querySelector(".delivery-line__actions")?.getAttribute("aria-hidden"), "true");

  const cancelDom = new JSDOM();
  const cancelModel = withAdapterCapabilities(withSessions(view), [OperationKind.CANCEL]);
  cancelModel.commands.set("command-1", runningCommand(view.identity));
  const cancelOnly = renderSessionDetail(cancelDom.window.document, cancelModel, view, {
    markdown: createMarkdownRenderer(cancelDom.window as unknown as Window),
    actions: callbacks,
  });
  assert.deepEqual(
    [...cancelOnly.element.querySelectorAll(".delivery-line__actions button")].map((button) => button.getAttribute("aria-label")),
    ["Cancel running operation"],
  );

  const lockedDom = new JSDOM();
  const lockedModel = withAdapterCapabilities(withSessions(view));
  lockedModel.commands.set("command-1", runningCommand(view.identity));
  const locked = renderSessionDetail(lockedDom.window.document, lockedModel, view, {
    markdown: createMarkdownRenderer(lockedDom.window as unknown as Window),
    actions: callbacks,
    lockdownActive: true,
  });
  const lockedActions = [...locked.element.querySelectorAll<HTMLButtonElement>(".delivery-line__actions button")];
  assert.equal(lockedActions.length, 2);
  assert.equal(lockedActions.every((button) => button.disabled && button.title.includes("Disabled during lockdown")), true);
});

test("same-correlation questions render as one integrated session-detail card", () => {
  const dom = new JSDOM();
  const view = session("session-1");
  const model = withSessions(view);
  const first = question(view.identity);
  first.id = "question-1";
  first.groupingKey = "pi-agent:batch-command";
  const second = question(view.identity);
  second.id = "question-2";
  second.groupingKey = first.groupingKey;
  second.lsn = 6n;
  model.elicitations.set(first.id, first);
  model.elicitations.set(second.id, second);

  const detail = renderSessionDetail(dom.window.document, model, view, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    elicitation: {
      operationContext: () => { throw new Error("not submitted"); },
      submit: () => undefined,
    },
  });
  dom.window.document.body.append(detail.element);

  const cards = detail.element.querySelectorAll(":scope > .timeline > .elicitation-card");
  assert.equal(cards.length, 1);
  assert.equal(cards[0]!.getAttribute("data-elicitation-group"), "true");
  assert.equal(cards[0]!.querySelectorAll("[data-elicitation-id]").length, 2);
});

test("shell surfaces reconnect and offline state with locked banner primitives", () => {
  const dom = new JSDOM();
  const view = session("session-1");
  const model = withSessions(view);
  model.reconciled = false;
  view.reconciled = false;
  view.connectivity = SessionConnectivityState.STALE;
  const shell = createCockpitShell(dom.window.document, model, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    isMobile: () => false,
  });
  dom.window.document.body.append(shell.element);

  let banner = shell.element.querySelector<HTMLElement>(":scope > .alert")!;
  assert.match(banner.textContent!, /Reconnecting/);
  assert.ok(banner.querySelector(".connectivity-indicator--stale"));

  model.reconciled = true;
  view.reconciled = true;
  view.connectivity = SessionConnectivityState.OFFLINE;
  shell.update(model);
  banner = shell.element.querySelector<HTMLElement>(":scope > .alert")!;
  assert.match(banner.textContent!, /Session offline/);
  assert.ok(banner.querySelector(".connectivity-indicator--offline"));
});

test("submission UNKNOWN consumes each adapter action and undeclared assurance is conservative", () => {
  const result = create(SubmissionResultSchema, {
    outcome: SubmissionOutcome.UNKNOWN,
    operationState: OperationState.UNSPECIFIED,
  });
  for (const [action, qualifier] of [
    [ReconciliationAction.NONE, "unknown"],
    [ReconciliationAction.MANUAL_REQUIRED, "manual-required"],
    [undefined, "manual-required"],
  ] as const) {
    const dom = new JSDOM();
    const view = session("submission-unknown");
    const model = withSessions(view);
    if (action !== undefined) {
      model.adapters.set("pi", {
        adapterId: "pi",
        status: create(AdapterStatusSchema, {
          capability: assuranceCapability(IdempotencyStrength.NONE, action),
        }),
        asOfLsn: 1n,
        recentDiagnostics: [],
      });
    }
    const detail = renderSessionDetail(dom.window.document, model, view, {
      markdown: createMarkdownRenderer(dom.window as unknown as Window),
      submission: {
        state: LocalSubmissionState.UNKNOWN,
        adapterId: "pi",
        result,
      },
    });
    assert.match(detail.composer.textContent ?? "", new RegExp(`Outcome qualifier: ${qualifier}`));
  }

  const dom = new JSDOM();
  const view = session("submission-accepted");
  const model = withAdapterCapabilities(withSessions(view));
  const accepted = renderSessionDetail(dom.window.document, model, view, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    submission: {
      state: LocalSubmissionState.DRAFT,
      adapterId: "pi",
      result: create(SubmissionResultSchema, {
        outcome: SubmissionOutcome.ACCEPTED,
        operationState: OperationState.ACCEPTED,
      }),
    },
  });
  assert.doesNotMatch(accepted.composer.textContent ?? "", /Outcome qualifier:/);
});

test("deduplicated submission is visible as already in flight", () => {
  const dom = new JSDOM();
  const view = session("session-1");
  const result = create(SubmissionResultSchema, {
    outcome: SubmissionOutcome.ACCEPTED,
    operationState: OperationState.RUNNING,
    deduplicated: true,
  });
  const shell = createCockpitShell(dom.window.document, withSessions(view), {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
    isMobile: () => false,
    submission: () => ({ state: LocalSubmissionState.DRAFT, result }),
  });
  dom.window.document.body.append(shell.element);

  const indicator = shell.element.querySelector<HTMLElement>(".retry-safety-indicator")!;
  assert.match(indicator.textContent!, /Already in flight/);
  assert.match(indicator.textContent!, /no duplicate submitted/);
});

async function assertAxeClean(dom: JSDOM, context: string): Promise<void> {
  const result = await (dom.window as unknown as { axe: typeof axe }).axe.run(dom.window.document, {
    rules: { "color-contrast": { enabled: false } },
  });
  assert.deepEqual(
    Array.from(result.violations, (violation) => violation.id),
    [],
    `${context}\n${Array.from(result.violations, (violation) => `${violation.id}: ${violation.help}`).join("\n")}`,
  );
}

function session(
  runtimeSessionId: string,
  generation = 1n,
  label: SessionView["label"] = { name: "core", project: "patchbay", cwd: "/projects/patchbay" },
): SessionView {
  return {
    identity: { adapterId: "pi", deploymentScope: "laptop", runtimeSessionId, generation },
    label,
    connectivity: SessionConnectivityState.LIVE,
    activity: SessionActivityState.WORKING,
    activityDetail: "thinking",
    needsYou: false,
    lastLsn: 1n,
    tombstoned: false,
    reconciled: true,
  };
}

function pooledResource(resourceId: string): ResourceView {
  return {
    identity: { adapterId: "token-commune", resourceKind: "provider_pool", resourceId },
    freshness: ResourceFreshnessState.CURRENT,
    sourceAdapterGeneration: 1n,
    revisionLsn: 8n,
    tombstoned: false,
    hasCachedPayload: true,
    reconciled: true,
    projection: {
      status: "decoded",
      value: {
        kind: "pooled-provider-pool",
        displayName: resourceId,
        providerLabel: "Anthropic",
        health: "serving",
        remainingPercent: 42,
        resetLabel: "resets in 2h",
        controlPosture: "administration-capable",
      },
    },
  };
}

function withSessions(...sessions: SessionView[]): PresentationModel {
  const model = emptyPresentationModel();
  model.authorityDomainId = DOMAIN.value;
  model.reconciled = true;
  for (const view of sessions) model.sessions.set(sessionKey(view.identity), view);
  return model;
}

function withAdapterCapabilities(
  model: PresentationModel,
  supportedOperationKinds = [OperationKind.CANCEL, OperationKind.INTERRUPT],
  cancellationSupport = true,
): PresentationModel {
  model.adapters.set("pi", {
    adapterId: "pi",
    status: create(AdapterStatusSchema, {
      capability: create(AdapterCapabilitySummarySchema, {
        ...assuranceCapability(
          IdempotencyStrength.AT_PATCHBAY_BOUNDARY,
          ReconciliationAction.MANUAL_REQUIRED,
        ),
        cancellationSupport,
        supportedOperationKinds,
      }),
    }),
    asOfLsn: 1n,
    recentDiagnostics: [],
  });
  return model;
}

function assuranceCapability(
  deduplicationStrength: IdempotencyStrength,
  unprovenOutcomeAction: ReconciliationAction,
) {
  return create(AdapterCapabilitySummarySchema, {
    assurance: create(AdapterAssuranceManifestSchema, {
      contract: {
        case: "v1",
        value: create(AdapterAssuranceManifestV1Schema, {
          deduplicationStrength,
          continuationProofSupport: false,
          cursorSupport: false,
          generationFenceSupport: false,
          reconciliationStrength: AdapterReconciliationStrength.NONE,
          unprovenOutcomeAction,
        }),
      },
    }),
  });
}

function runningCommand(
  identity: SessionIdentity,
  id = "command-1",
  text = "Run the checks",
): CommandView {
  const targetScope = target(identity);
  const operation = create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: id }),
    authorityDomainId: DOMAIN,
    kind: OperationKind.INSTRUCT,
    targetScope,
    idempotencyKey: `idem-${id}`,
    payload: create(PayloadEnvelopeSchema, {
      contentType: PayloadContentType.TEXT_UTF8,
      schemaRef: "patchbay.InstructPayload",
      payload: new TextEncoder().encode(text),
    }),
  });
  return {
    id,
    state: OperationState.RUNNING,
    lsn: 2n,
    target: { kind: "runtime-session", identity },
    operation,
    history: [
      { state: OperationState.ACCEPTED, lsn: 1n },
      { state: OperationState.RUNNING, lsn: 2n },
    ],
  };
}

function failedCommand(identity: SessionIdentity): CommandView {
  const command = runningCommand(identity);
  return {
    ...command,
    id: "command-failed",
    state: OperationState.FAILED,
    lsn: 6n,
    failureCode: FailureCode.EXECUTION_FAILED,
    operation: create(OperationSchema, {
      ...command.operation,
      commandId: create(CommandIdSchema, { value: "command-failed" }),
      idempotencyKey: "idem-command-failed",
    }),
    history: [
      { state: OperationState.ACCEPTED, lsn: 1n },
      { state: OperationState.DELIVERED, lsn: 2n },
      { state: OperationState.FAILED, lsn: 6n, failureCode: FailureCode.EXECUTION_FAILED },
    ],
  };
}

function question(identity: SessionIdentity): ElicitationView {
  return {
    id: "question-1",
    kind: "question",
    state: ElicitationState.PENDING,
    target: identity,
    lsn: 5n,
    prompt: "Continue?",
    contract: create(ResponseContractSchema, {
      contractKind: ResponseContractKind.QUESTION,
      contractBody: {
        case: "question",
        value: create(QuestionContractSchema, {
          options: [
            create(ResponseOptionSchema, { optionId: "yes", label: "Yes" }),
            create(ResponseOptionSchema, { optionId: "no", label: "No" }),
          ],
        }),
      },
    }),
  };
}

function target(identity: SessionIdentity) {
  return create(TargetScopeSchema, {
    kind: TargetScopeKind.RUNTIME_SESSION,
    adapterId: create(AdapterIdSchema, { value: identity.adapterId }),
    deploymentScope: identity.deploymentScope,
    runtimeSessionId: create(RuntimeSessionIdSchema, { value: identity.runtimeSessionId }),
    sessionGeneration: create(GenerationSchema, { value: identity.generation }),
  });
}

test("timeline shows an in-chat activity indicator while the session is working", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const view = session("session-1"); // WORKING with detail "thinking"
  const detail = renderSessionDetail(dom.window.document, withSessions(view), view, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
  });
  const indicator = detail.element.querySelector(".timeline-activity");
  assert.ok(indicator, "expected a timeline activity indicator");
  assert.equal(indicator!.getAttribute("role"), "status");
  assert.match(indicator!.textContent ?? "", /thinking/);
});

test("timeline activity indicator is absent when the session is idle", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const view = { ...session("session-1"), activity: SessionActivityState.IDLE, activityDetail: undefined };
  const detail = renderSessionDetail(dom.window.document, withSessions(view), view, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
  });
  assert.equal(detail.element.querySelector(".timeline-activity"), null);
});

test("timeline activity indicator is absent when the session is tombstoned", () => {
  const dom = new JSDOM("<!doctype html><body></body>");
  const view = { ...session("session-1"), tombstoned: true };
  const detail = renderSessionDetail(dom.window.document, withSessions(view), view, {
    markdown: createMarkdownRenderer(dom.window as unknown as Window),
  });
  assert.equal(detail.element.querySelector(".timeline-activity"), null);
});
