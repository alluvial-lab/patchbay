import assert from "node:assert/strict";
import test from "node:test";

import { create, fromBinary } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  ApprovalDecision,
  ApprovalResponsePayloadSchema,
  AuthorityDomainIdSchema,
  ElicitationResponsePayloadSchema,
  ElicitationState,
  GenerationSchema,
  OperationKind,
  QuestionContractSchema,
  ResponseContractKind,
  ResponseContractSchema,
  ResponseOptionSchema,
  RuntimeSessionIdSchema,
  TargetScopeKind,
  TargetScopeSchema,
  type Operation,
} from "@patchbay/contracts";
import { JSDOM } from "jsdom";

import type { ElicitationView } from "../src/domain/model.js";
import {
  buildApprovalResponse,
  buildQuestionResponse,
  createMobileElicitationSheet,
  renderElicitation,
  renderElicitationGroup,
  type ElicitationRenderOptions,
  type ResponseOperationContext,
} from "../src/ui/elicitation.js";

const DOMAIN = create(AuthorityDomainIdSchema, { value: "operator-domain" });
const TARGET = create(TargetScopeSchema, {
  kind: TargetScopeKind.RUNTIME_SESSION,
  adapterId: create(AdapterIdSchema, { value: "pi" }),
  deploymentScope: "laptop",
  runtimeSessionId: create(RuntimeSessionIdSchema, { value: "session-1" }),
  sessionGeneration: create(GenerationSchema, { value: 2n }),
});

function context(id = "response-1"): ResponseOperationContext {
  return {
    authorityDomainId: DOMAIN,
    targetScope: TARGET,
    commandId: id,
    idempotencyKey: `idem-${id}`,
  };
}

test("approval buttons build distinct typed response Operations", () => {
  const view = approval("approval-1");
  const approved = buildApprovalResponse(view, ApprovalDecision.APPROVED, context("approved"));
  const denied = buildApprovalResponse(view, ApprovalDecision.DENIED, context("denied"));

  assert.equal(approved.kind, OperationKind.APPROVAL_RESPONSE);
  assert.equal(denied.kind, OperationKind.APPROVAL_RESPONSE);
  assert.equal(approved.correlations[0]!.ref.case, "elicitationId");
  assert.equal(approved.payload!.schemaRef, "patchbay.ApprovalResponsePayload");
  assert.equal(
    fromBinary(ApprovalResponsePayloadSchema, approved.payload!.payload).decision,
    ApprovalDecision.APPROVED,
  );
  assert.equal(
    fromBinary(ApprovalResponsePayloadSchema, denied.payload!.payload).decision,
    ApprovalDecision.DENIED,
  );
});

test("approval renders direct Deny and Approve actions without a selection step", () => {
  const dom = new JSDOM();
  const submitted: Operation[] = [];
  const card = renderElicitation(dom.window.document, approval("approval-ui"), renderOptions(submitted));
  dom.window.document.body.append(card);

  assert.equal(card.querySelectorAll("input, select").length, 0);
  const buttons = [...card.querySelectorAll<HTMLButtonElement>("button")];
  assert.deepEqual(buttons.map((button) => button.textContent), ["Deny", "Approve"]);
  buttons[0]!.click();
  buttons[1]!.click();
  assert.deepEqual(
    submitted.map((operation) =>
      fromBinary(ApprovalResponsePayloadSchema, operation.payload!.payload).decision,
    ),
    [ApprovalDecision.DENIED, ApprovalDecision.APPROVED],
  );
});

test("free-text and answer-and produce the committed singular question payloads", () => {
  const view = question("question-1", true);
  const freeText = buildQuestionResponse(view, { freeText: "release/next" }, context("free"));
  const answerAnd = buildQuestionResponse(
    view,
    { selectedOptionId: "main", clarification: "run the full suite" },
    context("answer-and"),
  );

  assert.equal(freeText.kind, OperationKind.ELICITATION_RESPONSE);
  assert.deepEqual(fromBinary(ElicitationResponsePayloadSchema, freeText.payload!.payload), {
    $typeName: "patchbay.ElicitationResponsePayload",
    selectedOptionId: "",
    freeText: "release/next",
    clarification: "",
  });
  assert.deepEqual(fromBinary(ElicitationResponsePayloadSchema, answerAnd.payload!.payload), {
    $typeName: "patchbay.ElicitationResponsePayload",
    selectedOptionId: "main",
    freeText: "",
    clarification: "run the full suite",
  });
  assert.throws(
    () => buildQuestionResponse(view, { selectedOptionId: "main", freeText: "also" }, context()),
    /exactly one/,
  );
});

test("every question contract renders select-one radios even with a select-many hint", () => {
  const dom = new JSDOM();
  const view = question("hinted", true, ElicitationState.PENDING, ["select-many"]);
  const card = renderElicitation(dom.window.document, view, renderOptions([]));

  assert.equal(card.querySelector("form")!.getAttribute("data-control-shape"), "select-one");
  assert.equal(card.querySelectorAll('input[type="radio"]').length, 3);
  assert.equal(card.querySelectorAll('input[type="checkbox"]').length, 0);
  assert.equal(card.querySelectorAll('input[type="text"]').length, 1);
});

test("terminal elicitations use locked state modifiers, show state, and disable controls", () => {
  const dom = new JSDOM();
  const card = renderElicitation(
    dom.window.document,
    question("terminal", false, ElicitationState.STALE),
    renderOptions([]),
  );

  assert.equal(card.classList.contains("elicitation-card--stale"), true);
  assert.match(card.textContent!, /Terminal: stale/);
  for (const control of card.querySelectorAll<HTMLInputElement | HTMLButtonElement | HTMLTextAreaElement>(
    "input, button, textarea",
  )) {
    assert.equal(control.disabled, true);
  }
});

test("a grouped card submits N independent single-answer Operations", () => {
  const dom = new JSDOM();
  const submitted: Operation[] = [];
  let sequence = 0;
  const options: ElicitationRenderOptions = {
    operationContext: () => context(`group-${++sequence}`),
    submit(operation) {
      submitted.push(operation);
    },
  };
  const group = renderElicitationGroup(
    dom.window.document,
    [question("q-1", false), question("q-2", false)],
    options,
  );

  dom.window.document.body.append(group);
  const first = group.querySelector<HTMLElement>('[data-elicitation-id="q-1"]')!;
  const second = group.querySelector<HTMLElement>('[data-elicitation-id="q-2"]')!;
  answer(first);
  assert.equal(submitted.length, 1);
  assert.equal(submitted[0]!.correlations[0]!.ref.case, "elicitationId");
  if (submitted[0]!.correlations[0]!.ref.case === "elicitationId") {
    assert.equal(submitted[0]!.correlations[0]!.ref.value.value, "q-1");
  }
  assert.equal(second.querySelector<HTMLButtonElement>('button[type="submit"]')!.disabled, true);
  answer(second);
  assert.equal(submitted.length, 2);
  if (submitted[1]!.correlations[0]!.ref.case === "elicitationId") {
    assert.equal(submitted[1]!.correlations[0]!.ref.value.value, "q-2");
  }
});

test("mobile sheet clones the tapped card and force-shows teaser content", () => {
  const dom = new JSDOM();
  const submitted: Operation[] = [];
  const sheet = createMobileElicitationSheet(dom.window.document, { isMobile: () => true });
  const card = renderElicitation(
    dom.window.document,
    question("mobile", true),
    { ...renderOptions(submitted), mobileSheet: sheet },
  );
  dom.window.document.body.append(card, sheet.backdrop, sheet.element);

  card.dispatchEvent(new dom.window.MouseEvent("click", { bubbles: true }));
  assert.equal(sheet.isOpen, true);
  assert.notEqual(sheet.element.querySelector(".elicitation-card"), card);
  const options = sheet.element.querySelector<HTMLElement>(".elicitation-card__options")!;
  assert.match(options.getAttribute("style") ?? "", /display: revert !important/);
  assert.equal(options.querySelectorAll('input[type="radio"]').length, 3);
  const freeText = options.querySelector<HTMLInputElement>('input[type="text"]')!;
  freeText.value = "mobile answer";
  freeText.dispatchEvent(new dom.window.Event("input", { bubbles: true }));
  const submit = options.querySelector<HTMLButtonElement>('button[type="submit"]')!;
  assert.equal(submit.disabled, false);
  submit.click();
  assert.equal(submitted.length, 1);
  assert.equal(
    fromBinary(ElicitationResponsePayloadSchema, submitted[0]!.payload!.payload).freeText,
    "mobile answer",
  );
  sheet.close();
  assert.equal(sheet.isOpen, false);
});

function answer(container: HTMLElement): void {
  const radio = container.querySelector<HTMLInputElement>('input[type="radio"]')!;
  radio.checked = true;
  radio.dispatchEvent(new radio.ownerDocument.defaultView!.Event("change", { bubbles: true }));
  const submit = container.querySelector<HTMLButtonElement>('button[type="submit"]')!;
  assert.equal(submit.disabled, false);
  submit.click();
}

function renderOptions(submitted: Operation[]): ElicitationRenderOptions {
  return {
    operationContext: () => context(),
    submit(operation) {
      submitted.push(operation);
    },
  };
}

function approval(id: string, state = ElicitationState.PENDING): ElicitationView {
  return {
    id,
    kind: "approval",
    state,
    contract: create(ResponseContractSchema, {
      contractKind: ResponseContractKind.APPROVAL,
      contractBody: { case: undefined },
    }),
    prompt: "Approve this tool call?",
    lsn: 1n,
  };
}

function question(
  id: string,
  allowFreeText: boolean,
  state = ElicitationState.PENDING,
  uiHints: string[] = ["select-one"],
): ElicitationView {
  return {
    id,
    kind: "question",
    state,
    contract: create(ResponseContractSchema, {
      contractKind: ResponseContractKind.QUESTION,
      uiHints,
      contractBody: {
        case: "question",
        value: create(QuestionContractSchema, {
          options: [
            create(ResponseOptionSchema, { optionId: "main", label: "main" }),
            create(ResponseOptionSchema, { optionId: "feature", label: "feature" }),
          ],
          allowFreeText,
        }),
      },
    }),
    prompt: "Which branch?",
    lsn: 1n,
  };
}
