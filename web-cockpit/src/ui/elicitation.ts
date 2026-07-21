import { create, toBinary } from "@bufbuild/protobuf";
import {
  ApprovalDecision,
  ApprovalResponsePayloadSchema,
  CommandIdSchema,
  ElicitationIdSchema,
  ElicitationResponsePayloadSchema,
  ElicitationState,
  OperationKind,
  OperationSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  ResponseContractKind,
  TypedCorrelationSchema,
  type AuthorityDomainId,
  type Operation,
  type QuestionContract,
  type TargetScope,
} from "@patchbay/contracts";

import {
  isTerminalElicitation,
  type ElicitationView,
} from "../domain/model.js";

const APPROVAL_SCHEMA = "patchbay.ApprovalResponsePayload";
const QUESTION_SCHEMA = "patchbay.ElicitationResponsePayload";

export interface ResponseOperationContext {
  authorityDomainId: AuthorityDomainId;
  targetScope: TargetScope;
  commandId: string;
  idempotencyKey: string;
}

export interface QuestionAnswer {
  selectedOptionId?: string;
  freeText?: string;
  clarification?: string;
}

export interface ElicitationRenderOptions {
  operationContext(elicitation: ElicitationView): ResponseOperationContext;
  submit(operation: Operation): void | Promise<void>;
  reportError?(error: unknown): void;
  mobileSheet?: MobileElicitationSheet;
}

export interface MobileElicitationSheet {
  readonly element: HTMLElement;
  readonly backdrop: HTMLElement;
  readonly isOpen: boolean;
  open(source: HTMLElement): void;
  close(): void;
}

export interface MobileSheetOptions {
  isMobile?: () => boolean;
}

export function buildApprovalResponse(
  elicitation: ElicitationView,
  decision: ApprovalDecision.APPROVED | ApprovalDecision.DENIED,
  context: ResponseOperationContext,
): Operation {
  assertAnswerable(elicitation, "approval");
  const payload = create(ApprovalResponsePayloadSchema, { decision });
  return responseOperation(
    elicitation,
    OperationKind.APPROVAL_RESPONSE,
    APPROVAL_SCHEMA,
    toBinary(ApprovalResponsePayloadSchema, payload),
    context,
  );
}

export function buildQuestionResponse(
  elicitation: ElicitationView,
  answer: QuestionAnswer,
  context: ResponseOperationContext,
): Operation {
  assertAnswerable(elicitation, "question");
  const contract = questionContract(elicitation);
  const selectedOptionId = answer.selectedOptionId ?? "";
  const freeText = answer.freeText ?? "";
  if (Boolean(selectedOptionId) === Boolean(freeText)) {
    throw new Error("question response requires exactly one selected option or free-text answer");
  }
  if (selectedOptionId && !contract.options.some((option) => option.optionId === selectedOptionId)) {
    throw new Error(`option ${selectedOptionId} is not declared by the question contract`);
  }
  if (freeText && (!contract.allowFreeText || !freeText.trim())) {
    throw new Error("free-text answer is empty or not allowed by the question contract");
  }

  const payload = create(ElicitationResponsePayloadSchema, {
    selectedOptionId,
    freeText,
    clarification: answer.clarification ?? "",
  });
  return responseOperation(
    elicitation,
    OperationKind.ELICITATION_RESPONSE,
    QUESTION_SCHEMA,
    toBinary(ElicitationResponsePayloadSchema, payload),
    context,
  );
}

/** Renders one typed approval/question response card using the locked primitives. */
export function renderElicitation(
  document: Document,
  elicitation: ElicitationView,
  options: ElicitationRenderOptions,
): HTMLElement {
  const card = document.createElement("section");
  card.className = `elicitation-card${elicitationStateModifier(elicitation.state)}`;
  card.dataset.elicitationId = elicitation.id;
  card.dataset.elicitationState = elicitationStateName(elicitation.state);
  card.append(cardHeader(document, elicitation));

  const prompt = document.createElement("p");
  prompt.className = "elicitation-card__prompt";
  prompt.textContent = elicitation.prompt;
  card.append(prompt);

  if (elicitation.kind === "approval") renderApprovalControls(document, card, elicitation, options);
  else renderQuestionControls(document, card, elicitation, options);

  if (options.mobileSheet) bindMobileTeaser(card, options.mobileSheet);
  return card;
}

/**
 * Groups independently terminal single-answer Elicitations in one visual card.
 * Every child keeps its own form, correlation, operation, and terminal binding.
 */
export function renderElicitationGroup(
  document: Document,
  elicitations: readonly ElicitationView[],
  options: ElicitationRenderOptions,
): HTMLElement {
  if (elicitations.length === 0) throw new Error("elicitation group must contain at least one item");
  const group = document.createElement("section");
  group.className = "elicitation-card";
  group.dataset.elicitationGroup = "true";

  const header = document.createElement("div");
  header.className = "elicitation-card__header";
  header.append(textElement(document, "span", "elicitation-card__kind", "questions"));
  header.append(
    textElement(
      document,
      "span",
      "elicitation-card__contract",
      `${elicitations.length} independent single-answer questions`,
    ),
  );
  group.append(header);

  for (const [index, elicitation] of elicitations.entries()) {
    if (elicitation.kind !== "question") throw new Error("grouped elicitation must use question contracts");
    const item = document.createElement("fieldset");
    item.className = `field${elicitationStateModifier(elicitation.state)}`;
    item.dataset.elicitationId = elicitation.id;
    item.dataset.elicitationState = elicitationStateName(elicitation.state);
    const legend = document.createElement("legend");
    legend.className = "field__label";
    legend.textContent = `${index + 1}. ${elicitation.prompt}`;
    item.append(legend);
    renderQuestionControls(document, item, elicitation, options);
    group.append(item);
  }

  if (options.mobileSheet) bindMobileTeaser(group, options.mobileSheet);
  return group;
}

/** Creates the mobile dialog that clones a tapped card while forwarding controls to its live source. */
export function createMobileElicitationSheet(
  document: Document,
  options: MobileSheetOptions = {},
): MobileElicitationSheet {
  const isMobile = options.isMobile ?? (() => document.defaultView?.matchMedia?.("(max-width: 760px)").matches ?? false);
  const backdrop = document.createElement("div");
  backdrop.className = "sheet-backdrop";
  backdrop.hidden = true;

  const sheet = document.createElement("section");
  sheet.className = "sheet";
  sheet.hidden = true;
  sheet.setAttribute("role", "dialog");
  sheet.setAttribute("aria-modal", "true");
  sheet.setAttribute("aria-label", "Elicitation response");
  sheet.append(textElement(document, "div", "sheet__handle", ""));

  const header = document.createElement("div");
  header.className = "sheet__header";
  const kind = textElement(document, "span", "sheet__kind", "Response required");
  const close = textElement(document, "button", "sheet__close", "Close") as HTMLButtonElement;
  close.type = "button";
  header.append(kind, close);
  const body = document.createElement("div");
  body.className = "sheet__body";
  sheet.append(header, body);

  let source: HTMLElement | undefined;
  const controller: MobileElicitationSheet = {
    element: sheet,
    backdrop,
    get isOpen() {
      return !sheet.hidden;
    },
    open(nextSource) {
      if (!isMobile()) return;
      source = nextSource;
      body.replaceChildren();
      const clone = nextSource.cloneNode(true) as HTMLElement;
      clone.classList.remove("elicitation-card--inline-teaser");
      forceShowClone(clone);
      body.append(clone);
      kind.textContent = nextSource.dataset.elicitationGroup === "true" ? "Questions" : "Response required";
      sheet.hidden = false;
      backdrop.hidden = false;
      sheet.classList.add("is-open");
      backdrop.classList.add("is-open");
      close.focus();
    },
    close() {
      source = undefined;
      sheet.hidden = true;
      backdrop.hidden = true;
      sheet.classList.remove("is-open");
      backdrop.classList.remove("is-open");
      body.replaceChildren();
    },
  };

  close.addEventListener("click", () => controller.close());
  backdrop.addEventListener("click", () => controller.close());
  body.addEventListener("focusin", (event) => {
    forwardSheetFocus(event, source);
    syncSheetControls(source, body);
  });
  body.addEventListener("input", (event) => {
    forwardSheetInput(event, source);
    syncSheetControls(source, body);
  });
  body.addEventListener("change", (event) => {
    forwardSheetInput(event, source);
    syncSheetControls(source, body);
  });
  body.addEventListener("click", (event) => forwardSheetClick(event, source));
  return controller;
}

function responseOperation(
  elicitation: ElicitationView,
  kind: OperationKind,
  schemaRef: string,
  payload: Uint8Array,
  context: ResponseOperationContext,
): Operation {
  if (!context.authorityDomainId.value) throw new Error("response authority domain is missing");
  if (!context.commandId) throw new Error("response command id is missing");
  if (!context.idempotencyKey) throw new Error("response idempotency key is missing");
  return create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: context.commandId }),
    authorityDomainId: context.authorityDomainId,
    kind,
    targetScope: context.targetScope,
    idempotencyKey: context.idempotencyKey,
    payload: create(PayloadEnvelopeSchema, {
      payload,
      contentType: PayloadContentType.PROTOBUF,
      schemaRef,
    }),
    correlations: [
      create(TypedCorrelationSchema, {
        ref: {
          case: "elicitationId",
          value: create(ElicitationIdSchema, { value: elicitation.id }),
        },
      }),
    ],
  });
}

function renderApprovalControls(
  document: Document,
  parent: HTMLElement,
  elicitation: ElicitationView,
  options: ElicitationRenderOptions,
): void {
  if (elicitation.contract.contractKind !== ResponseContractKind.APPROVAL) {
    throw new Error("approval view does not carry an approval contract");
  }
  const actions = document.createElement("div");
  actions.className = "elicitation-card__actions";
  const deny = actionButton(document, "Deny", "btn btn-danger btn--sm", `${elicitation.id}:deny`);
  const approve = actionButton(document, "Approve", "btn btn-primary btn--sm", `${elicitation.id}:approve`);
  const disabled = isTerminalElicitation(elicitation.state);
  deny.disabled = disabled;
  approve.disabled = disabled;
  deny.addEventListener("click", () => submitApproval(elicitation, ApprovalDecision.DENIED, options));
  approve.addEventListener("click", () => submitApproval(elicitation, ApprovalDecision.APPROVED, options));
  actions.append(deny, approve);
  parent.append(actions);
}

function renderQuestionControls(
  document: Document,
  parent: HTMLElement,
  elicitation: ElicitationView,
  options: ElicitationRenderOptions,
): void {
  const contract = questionContract(elicitation);
  const form = document.createElement("form");
  form.className = "elicitation-card__options";
  form.dataset.controlShape = "select-one";
  form.dataset.uiHints = elicitation.contract.uiHints.join(",");
  const terminal = isTerminalElicitation(elicitation.state);
  const radioName = `elicitation-${elicitation.id}`;

  for (const option of contract.options) {
    const label = document.createElement("label");
    label.className = "field__label";
    const radio = document.createElement("input");
    radio.type = "radio";
    radio.name = radioName;
    radio.value = option.optionId;
    radio.disabled = terminal;
    radio.dataset.elicitationControl = `${elicitation.id}:option:${option.optionId}`;
    label.append(radio, document.createTextNode(` ${option.label}`));
    form.append(label);
  }

  let freeText: HTMLInputElement | undefined;
  if (contract.allowFreeText) {
    const field = document.createElement("label");
    field.className = "field";
    const label = document.createElement("span");
    label.className = "field__label";
    const radio = document.createElement("input");
    radio.type = "radio";
    radio.name = radioName;
    radio.value = "__free_text__";
    radio.disabled = terminal;
    radio.dataset.elicitationControl = `${elicitation.id}:free-choice`;
    label.append(radio, document.createTextNode(" Or type your own answer"));
    freeText = document.createElement("input");
    freeText.type = "text";
    freeText.className = "input";
    freeText.placeholder = "Your answer";
    freeText.disabled = terminal;
    freeText.dataset.elicitationControl = `${elicitation.id}:free-text`;
    freeText.addEventListener("focus", () => {
      radio.checked = true;
      updateSubmit();
    });
    field.append(label, freeText);
    form.append(field);
  }

  const clarification = document.createElement("textarea");
  clarification.className = "textarea";
  clarification.placeholder = "And… optional context";
  clarification.disabled = terminal;
  clarification.dataset.elicitationControl = `${elicitation.id}:clarification`;
  form.append(clarification);

  const actions = document.createElement("div");
  actions.className = "elicitation-card__actions";
  const submit = actionButton(document, "Answer", "btn btn-primary btn--sm", `${elicitation.id}:submit`);
  submit.type = "submit";
  submit.disabled = true;
  actions.append(submit);
  form.append(actions);

  const status = textElement(
    document,
    "span",
    "elicitation-card__contract",
    terminal ? `Terminal: ${elicitationStateName(elicitation.state)}` : "Select one answer",
  );
  status.setAttribute("role", "status");
  form.append(status);

  function updateSubmit(): void {
    const selected = form.querySelector<HTMLInputElement>(`input[type="radio"][name="${cssEscape(radioName)}"]:checked`);
    submit.disabled = terminal || !selected || (selected.value === "__free_text__" && !freeText?.value.trim());
  }

  form.addEventListener("input", updateSubmit);
  form.addEventListener("change", updateSubmit);
  form.addEventListener("submit", (event) => {
    event.preventDefault();
    if (terminal) return;
    const selected = form.querySelector<HTMLInputElement>(`input[type="radio"][name="${cssEscape(radioName)}"]:checked`);
    if (!selected) return;
    const answer: QuestionAnswer = {
      clarification: clarification.value,
      ...(selected.value === "__free_text__"
        ? { freeText: freeText?.value ?? "" }
        : { selectedOptionId: selected.value }),
    };
    submitOperation(() => buildQuestionResponse(elicitation, answer, options.operationContext(elicitation)), options);
  });
  parent.append(form);
}

function submitApproval(
  elicitation: ElicitationView,
  decision: ApprovalDecision.APPROVED | ApprovalDecision.DENIED,
  options: ElicitationRenderOptions,
): void {
  submitOperation(() => buildApprovalResponse(elicitation, decision, options.operationContext(elicitation)), options);
}

function submitOperation(build: () => Operation, options: ElicitationRenderOptions): void {
  try {
    const submitted = Promise.resolve(options.submit(build()));
    if (options.reportError) submitted.catch(options.reportError);
  } catch (error) {
    if (options.reportError) options.reportError(error);
    else throw error;
  }
}

function questionContract(elicitation: ElicitationView): QuestionContract {
  if (elicitation.contract.contractKind !== ResponseContractKind.QUESTION) {
    throw new Error("question view does not carry a question contract");
  }
  const body = elicitation.contract.contractBody;
  if (body.case !== "question") throw new Error("question contract is missing its typed body");
  return body.value;
}

function assertAnswerable(elicitation: ElicitationView, kind: ElicitationView["kind"]): void {
  if (elicitation.kind !== kind) throw new Error(`${elicitation.id} is not a ${kind} elicitation`);
  if (isTerminalElicitation(elicitation.state)) {
    throw new Error(`elicitation ${elicitation.id} is already terminal`);
  }
}

function cardHeader(document: Document, elicitation: ElicitationView): HTMLElement {
  const header = document.createElement("div");
  header.className = "elicitation-card__header";
  header.append(textElement(document, "span", "elicitation-card__kind", elicitation.kind));
  header.append(
    textElement(
      document,
      "span",
      "elicitation-card__contract",
      `contract: ${elicitation.kind} · ${elicitationStateName(elicitation.state)}`,
    ),
  );
  return header;
}

function actionButton(document: Document, label: string, className: string, control: string): HTMLButtonElement {
  const button = textElement(document, "button", className, label) as HTMLButtonElement;
  button.type = "button";
  button.dataset.elicitationControl = control;
  return button;
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

function bindMobileTeaser(card: HTMLElement, sheet: MobileElicitationSheet): void {
  card.classList.add("elicitation-card--inline-teaser");
  card.addEventListener("click", (event) => {
    if ((event.target as Element).closest("button, input, textarea, label")) return;
    sheet.open(card);
  });
}

function forceShowClone(clone: HTMLElement): void {
  for (const element of clone.querySelectorAll<HTMLElement>(
    ".elicitation-card__options, .elicitation-card__actions, .field, .elicitation-card__prompt",
  )) {
    element.style.setProperty("display", "revert", "important");
    element.style.setProperty("overflow", "visible", "important");
    element.style.setProperty("-webkit-line-clamp", "unset", "important");
  }
}

function forwardSheetInput(event: Event, source: HTMLElement | undefined): void {
  if (!source || !isFormValueControl(event.target)) return;
  const control = event.target.dataset.elicitationControl;
  if (!control) return;
  const original = findControl(source, control);
  if (!isFormValueControl(original)) return;
  original.value = event.target.value;
  if (event.target.tagName === "INPUT" && original.tagName === "INPUT") {
    (original as HTMLInputElement).checked = (event.target as HTMLInputElement).checked;
  }
  if (control.endsWith(":free-text")) {
    const freeChoice = findControl(source, control.replace(/:free-text$/, ":free-choice"));
    if (freeChoice?.tagName === "INPUT") (freeChoice as HTMLInputElement).checked = true;
  }
  original.dispatchEvent(new original.ownerDocument.defaultView!.Event(event.type, { bubbles: true }));
}

function forwardSheetFocus(event: FocusEvent, source: HTMLElement | undefined): void {
  if (!source || !isElement(event.target)) return;
  const control = (event.target as HTMLElement).dataset.elicitationControl;
  if (control) findControl(source, control)?.focus();
}

function forwardSheetClick(event: MouseEvent, source: HTMLElement | undefined): void {
  if (!source || !isElement(event.target)) return;
  const target = event.target.closest<HTMLElement>("[data-elicitation-control]");
  if (!target || target.tagName !== "BUTTON") return;
  event.preventDefault();
  const original = findControl(source, target.dataset.elicitationControl!);
  if (original?.tagName === "BUTTON") (original as HTMLButtonElement).click();
}

function syncSheetControls(source: HTMLElement | undefined, body: HTMLElement): void {
  if (!source) return;
  for (const clone of body.querySelectorAll<HTMLElement>("[data-elicitation-control]")) {
    const control = clone.dataset.elicitationControl;
    const original = control ? findControl(source, control) : null;
    if (!original || original.tagName !== clone.tagName) continue;
    if (isFormValueControl(clone) && isFormValueControl(original)) {
      clone.value = original.value;
      if (clone.tagName === "INPUT") {
        (clone as HTMLInputElement).checked = (original as HTMLInputElement).checked;
      }
      clone.toggleAttribute("disabled", original.hasAttribute("disabled"));
    } else if (clone.tagName === "BUTTON") {
      (clone as HTMLButtonElement).disabled = (original as HTMLButtonElement).disabled;
    }
  }
}

function isElement(value: EventTarget | null): value is Element {
  return Boolean(value && "nodeType" in value && (value as Node).nodeType === 1);
}

function isFormValueControl(value: EventTarget | null): value is HTMLInputElement | HTMLTextAreaElement {
  return isElement(value) && (value.tagName === "INPUT" || value.tagName === "TEXTAREA");
}

function findControl(source: HTMLElement, control: string): HTMLElement | null {
  return [...source.querySelectorAll<HTMLElement>("[data-elicitation-control]")].find(
    (candidate) => candidate.dataset.elicitationControl === control,
  ) ?? null;
}

function elicitationStateModifier(state: ElicitationState): string {
  switch (state) {
    case ElicitationState.OPENED:
    case ElicitationState.PENDING:
      return "";
    case ElicitationState.ANSWERED:
      return " elicitation-card--answered";
    case ElicitationState.DECLINED:
      return " elicitation-card--declined";
    case ElicitationState.EXPIRED:
      return " elicitation-card--expired";
    case ElicitationState.CANCELLED:
      return " elicitation-card--cancelled";
    case ElicitationState.WITHDRAWN:
      return " elicitation-card--withdrawn";
    case ElicitationState.SUPERSEDED:
      return " elicitation-card--superseded";
    case ElicitationState.STALE:
      return " elicitation-card--stale";
    case ElicitationState.UNSPECIFIED:
    default:
      throw new Error(`unsupported elicitation state ${state}`);
  }
}

export function elicitationStateName(state: ElicitationState): string {
  switch (state) {
    case ElicitationState.OPENED: return "opened";
    case ElicitationState.PENDING: return "pending";
    case ElicitationState.ANSWERED: return "answered";
    case ElicitationState.DECLINED: return "declined";
    case ElicitationState.EXPIRED: return "expired";
    case ElicitationState.CANCELLED: return "cancelled";
    case ElicitationState.WITHDRAWN: return "withdrawn";
    case ElicitationState.SUPERSEDED: return "superseded";
    case ElicitationState.STALE: return "stale";
    case ElicitationState.UNSPECIFIED:
    default:
      throw new Error(`unsupported elicitation state ${state}`);
  }
}

function cssEscape(value: string): string {
  return globalThis.CSS?.escape ? globalThis.CSS.escape(value) : value.replaceAll('"', '\\"');
}
