import type { AuthorityDomainId } from "@patchbay/contracts";

import type { PresentationModel } from "../domain/model.js";

export interface SecurityViewActions {
  enterLockdown(reasonCode: string): Promise<void>;
  revokeAllSessions(): Promise<void>;
  revokePrincipal(principalId: string): Promise<void>;
  revokeEndpoint(endpointId: string): Promise<void>;
  revokeDevice(deviceId: string): Promise<void>;
  revokeGrant(grantId: string): Promise<void>;
}

export function renderSecurityView(
  document: Document,
  model: PresentationModel,
  authorityDomainId: AuthorityDomainId,
  actions?: SecurityViewActions,
): HTMLElement {
  const view = document.createElement("section");
  view.className = "view security-view";
  view.setAttribute("aria-labelledby", "security-title");

  const header = document.createElement("header");
  header.className = "view-header";
  const heading = document.createElement("div");
  heading.append(textElement(document, "h1", "", "Security"));
  heading.firstElementChild!.id = "security-title";
  heading.append(textElement(document, "p", "identity", `operator · authority domain ${authorityDomainId.value}`));
  header.append(heading, textElement(document, "span", "connectivity-indicator connectivity-indicator--live", "core live"));
  view.append(header);

  if (model.lockdown.active) {
    const readOnly = alert(
      document,
      "Read-only during lockdown.",
      "Revocations and new Operations are unavailable until trusted bootstrap exit.",
      "warning",
    );
    readOnly.id = "security-lockdown-reason";
    view.append(readOnly);
  }

  const flow = document.createElement("div");
  flow.className = "security-flow";
  flow.append(lockdownHero(document, model, actions));
  flow.append(operatorSessions(document, model, actions));
  flow.append(simpleCard(document, "Endpoints & devices", "Control-surface inventory is redacted to safe endpoint and device identifiers."));
  flow.append(simpleCard(document, "Active grants", "Grant ids, target scope, allowed OperationKinds, expiry, and revocation status are shown without provenance prose."));
  view.append(flow);
  return view;
}

function lockdownHero(
  document: Document,
  model: PresentationModel,
  actions?: SecurityViewActions,
): HTMLElement {
  const card = document.createElement("section");
  card.className = "card card--raised security-hero";
  card.setAttribute("aria-labelledby", "lockdown-title");
  const heading = document.createElement("div");
  heading.className = "section-heading";
  heading.append(textElement(document, "h2", "", "Security lockdown"));
  heading.firstElementChild!.id = "lockdown-title";
  heading.append(textElement(document, "span", "retry-safety-indicator retry-safety-indicator--unsafe", "high impact"));
  card.append(heading);
  card.append(textElement(
    document,
    "p",
    "",
    "Reject new Operations, mark runtime sessions stale, and require fresh login. Routine web authentication cannot exit this posture.",
  ));

  const controls = document.createElement("div");
  controls.className = "hero-actions";
  const arm = button(document, model.lockdown.active ? "Lockdown active" : "Arm lockdown", "btn btn-danger btn--lg");
  arm.disabled = model.lockdown.active || !actions;
  arm.title = model.lockdown.active
    ? "Disabled during lockdown: the posture is already active."
    : "Opens a deliberate two-step lockdown ritual.";
  controls.append(arm);
  card.append(controls);

  const armDialog = dialog(document, "Arm security lockdown", "Step 1 of 2: arm this action. No change is made yet.");
  const reason = input(document, "Reason code", "suspected_endpoint_compromise");
  reason.field.pattern = "[a-z0-9_]{1,64}";
  reason.field.id = "lockdown-reason-code";
  reason.label.htmlFor = reason.field.id;
  armDialog.body.append(reason.label, reason.field);
  const continueButton = button(document, "Continue to confirmation", "btn btn-danger");
  const cancelArm = button(document, "Cancel", "btn btn-secondary");
  armDialog.actions.append(cancelArm, continueButton);

  const confirmDialog = dialog(
    document,
    "Confirm lockdown",
    "Step 2 of 2: this durably rejects new Operations and ejects routine sessions. Exit requires patchbay-cli lockdown-exit from the local console.",
  );
  const confirmation = input(document, "Type LOCKDOWN", "");
  confirmation.field.id = "lockdown-confirmation";
  confirmation.field.autocomplete = "off";
  confirmation.label.htmlFor = confirmation.field.id;
  confirmation.field.setAttribute("aria-describedby", "lockdown-confirmation-help");
  confirmDialog.body.append(confirmation.label, confirmation.field);
  confirmDialog.body.append(textElement(document, "p", "field__helper", "This is not an Elicitation; it is a core-imposed security posture."));
  confirmDialog.body.lastElementChild!.id = "lockdown-confirmation-help";
  const cancelConfirm = button(document, "Cancel", "btn btn-secondary");
  const enter = button(document, "Enter lockdown", "btn btn-danger");
  enter.disabled = true;
  confirmDialog.actions.append(cancelConfirm, enter);
  card.append(armDialog.element, confirmDialog.element);

  const close = (element: HTMLElement) => { element.hidden = true; };
  arm.addEventListener("click", () => {
    armDialog.element.hidden = false;
    reason.field.focus();
  });
  cancelArm.addEventListener("click", () => close(armDialog.element));
  continueButton.addEventListener("click", () => {
    if (!/^[a-z0-9_]{1,64}$/.test(reason.field.value)) {
      reason.field.setCustomValidity("Use a safe lower-snake-case reason code.");
      reason.field.reportValidity?.();
      return;
    }
    reason.field.setCustomValidity("");
    close(armDialog.element);
    confirmDialog.element.hidden = false;
    confirmation.field.focus();
  });
  cancelConfirm.addEventListener("click", () => close(confirmDialog.element));
  confirmation.field.addEventListener("input", () => {
    enter.disabled = confirmation.field.value !== "LOCKDOWN";
  });
  enter.addEventListener("click", () => {
    if (enter.disabled || confirmation.field.value !== "LOCKDOWN") return;
    close(confirmDialog.element);
    void actions?.enterLockdown(reason.field.value);
  });
  return card;
}

function operatorSessions(document: Document, model: PresentationModel, actions?: SecurityViewActions): HTMLElement {
  const card = document.createElement("section");
  card.className = "card";
  const heading = document.createElement("div");
  heading.className = "section-heading";
  heading.append(textElement(document, "h2", "", "Operator sessions"));
  const revoke = button(document, "Revoke all sessions", "btn btn-danger btn--sm");
  revoke.disabled = model.lockdown.active || !actions;
  revoke.title = model.lockdown.active ? "Disabled during lockdown: revocations are unavailable." : "Revoke all operator sessions";
  revoke.addEventListener("click", () => void actions?.revokeAllSessions());
  heading.append(revoke);
  card.append(heading);
  const count = [...model.sessions.values()].length;
  card.append(textElement(document, "p", "identity", `${count} runtime session${count === 1 ? "" : "s"} currently visible; opaque operator-session ids are never shown.`));
  return card;
}

function simpleCard(document: Document, title: string, body: string): HTMLElement {
  const card = document.createElement("section");
  card.className = "card";
  card.append(textElement(document, "h2", "", title), textElement(document, "p", "identity", body));
  return card;
}

function alert(document: Document, title: string, body: string, tone: "warning" | "danger"): HTMLElement {
  const value = document.createElement("p");
  value.className = `alert alert--${tone}`;
  value.setAttribute("role", "status");
  value.append(textElement(document, "strong", "", title), document.createTextNode(` ${body}`));
  return value;
}

function button(document: Document, label: string, className: string): HTMLButtonElement {
  const value = document.createElement("button");
  value.type = "button";
  value.className = className;
  value.textContent = label;
  return value;
}

function input(document: Document, label: string, value: string): { label: HTMLLabelElement; field: HTMLInputElement } {
  const field = document.createElement("input");
  field.className = "input";
  field.value = value;
  const labelElement = document.createElement("label");
  labelElement.className = "field__label";
  labelElement.htmlFor = field.id;
  labelElement.textContent = label;
  return { label: labelElement, field };
}

function dialog(document: Document, title: string, bodyText: string): {
  element: HTMLElement;
  body: HTMLElement;
  actions: HTMLElement;
} {
  const element = document.createElement("div");
  element.className = "card security-dialog";
  element.hidden = true;
  element.setAttribute("role", "dialog");
  element.setAttribute("aria-modal", "true");
  const titleElement = textElement(document, "h2", "", title);
  const titleId = `dialog-${Math.random().toString(36).slice(2)}`;
  titleElement.id = titleId;
  element.setAttribute("aria-labelledby", titleId);
  const body = document.createElement("div");
  body.className = "dialog-form";
  body.append(textElement(document, "p", "", bodyText));
  const actions = document.createElement("div");
  actions.className = "dialog-actions";
  element.append(titleElement, body, actions);
  return { element, body, actions };
}

function textElement<K extends keyof HTMLElementTagNameMap>(
  document: Document,
  tag: K,
  className: string,
  text: string,
): HTMLElementTagNameMap[K] {
  const value = document.createElement(tag);
  value.className = className;
  value.textContent = text;
  return value;
}
