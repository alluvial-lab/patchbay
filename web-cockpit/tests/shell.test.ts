import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { create } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  AuthorityDomainIdSchema,
  CommandIdSchema,
  ElicitationState,
  FailureCode,
  GenerationSchema,
  LocalSubmissionState,
  OperationKind,
  OperationSchema,
  OperationState,
  PayloadContentType,
  PayloadEnvelopeSchema,
  QuestionContractSchema,
  ResponseContractKind,
  ResponseContractSchema,
  ResponseOptionSchema,
  RuntimeSessionIdSchema,
  SessionActivityState,
  SessionConnectivityState,
  SubmissionOutcome,
  SubmissionResultSchema,
  TargetScopeKind,
  TargetScopeSchema,
} from "@patchbay/contracts";
import fc from "fast-check";
import { JSDOM } from "jsdom";

import {
  emptyPresentationModel,
  rendersLive,
  sessionKey,
  type CommandView,
  type ElicitationView,
  type PresentationModel,
  type SessionIdentity,
  type SessionView,
} from "../src/domain/model.js";
import { renderIcon, type IconName } from "../src/ui/icons.js";
import { createMarkdownRenderer } from "../src/ui/markdown.js";
import { renderSessionDetail } from "../src/ui/session-detail.js";
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

test("shell stylesheet provides responsive layout without rebinding protocol states", async () => {
  const css = await readFile(new URL("../src/ui/shell.css", import.meta.url), "utf8").catch(
    () => readFile(new URL("../../src/ui/shell.css", import.meta.url), "utf8"),
  );
  assert.match(css, /\.cockpit\s*\{[^}]*display:\s*flex/s);
  assert.match(css, /\.cockpit \.timeline\s*\{[^}]*max-width:\s*860px/s);
  assert.match(css, /\.cockpit \.msg--agent\s*\{[^}]*560px/s);
  assert.match(css, /@media \(max-width:\s*760px\)/);
  assert.doesNotMatch(css, /connectivity-indicator--(?:live|stale|offline|unknown|failed)/);
  assert.doesNotMatch(css, /command-step--(?:accepted|delivered|running|completed|rejected|failed|expired|cancelled|superseded)/);
});

test("desktop shell is two-pane and rows lead with identity before label metadata", () => {
  const dom = new JSDOM();
  const first = session("session-1", 1n, { name: "core", project: "patchbay" });
  const second = session("session-2", 1n, { name: "adapter", project: "patchbay" });
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
  const rows = [...shell.element.querySelectorAll<HTMLElement>(".session-row")];
  assert.equal(rows.length, 2);
  assert.equal(rows[0]!.firstElementChild!.className, "session-row__identity");
  assert.match(rows[0]!.querySelector(".session-row__identity")!.textContent!, /pi@laptop · runtime session-2 · gen 1/);
  assert.equal(rows[0]!.classList.contains("session-row--needs-you"), true);
  assert.ok(rows[0]!.querySelector(".connectivity-indicator"));
  assert.ok(rows[0]!.querySelector(".activity-indicator"));
  const unavailableControls = [...shell.element.querySelectorAll<HTMLButtonElement>(".sidebar__actions button")];
  assert.deepEqual(unavailableControls.map((button) => button.getAttribute("aria-label")), ["Spawn session unavailable", "Attach session unavailable"]);
  assert.equal(unavailableControls.every((button) => button.disabled && button.querySelector('.icon[aria-hidden="true"]')), true);

  shell.select(sessionKey(first.identity));
  assert.equal(shell.detail.element.dataset.sessionKey, sessionKey(first.identity));
  assert.equal(shell.element.querySelectorAll(".session-detail").length, 1);
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

test("detail integrates markdown, current plus last delivery, failures, contextual actions, and elicitations", () => {
  const dom = new JSDOM();
  const view = session("session-1");
  const model = withSessions(view);
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

function withSessions(...sessions: SessionView[]): PresentationModel {
  const model = emptyPresentationModel();
  model.authorityDomainId = DOMAIN.value;
  model.reconciled = true;
  for (const view of sessions) model.sessions.set(sessionKey(view.identity), view);
  return model;
}

function runningCommand(identity: SessionIdentity): CommandView {
  const targetScope = target(identity);
  const operation = create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: "command-1" }),
    authorityDomainId: DOMAIN,
    kind: OperationKind.INSTRUCT,
    targetScope,
    idempotencyKey: "idem-command-1",
    payload: create(PayloadEnvelopeSchema, {
      contentType: PayloadContentType.TEXT_UTF8,
      schemaRef: "patchbay.InstructPayload",
      payload: new TextEncoder().encode("Run the checks"),
    }),
  });
  return {
    id: "command-1",
    state: OperationState.RUNNING,
    lsn: 2n,
    target: identity,
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
