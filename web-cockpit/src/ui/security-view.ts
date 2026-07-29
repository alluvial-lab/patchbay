import { OperationKind, TargetScopeKind, type AuthorityDomainId } from "@patchbay/contracts";

import type { PresentationModel } from "../domain/model.js";

export interface SecurityViewActions {
  enterLockdown(reasonCode: string): Promise<void>;
  revokeCurrentSession?(): Promise<void>;
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
  flow.append(controlSurfaceInventory(document, model, actions));
  flow.append(grantInventory(document, model, actions));
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
  const card = inventoryCard(document, "Operator sessions");
  const revoke = button(document, "Revoke all sessions", "btn btn-danger btn--sm");
  revoke.disabled = model.lockdown.active || !actions;
  revoke.title = mutationTitle(model.lockdown.active, "Revoke all operator sessions");
  revoke.addEventListener("click", () => void actions?.revokeAllSessions());
  const heading = card.querySelector<HTMLElement>(".section-heading")!;
  heading.append(revoke);
  if (actions?.revokeCurrentSession) {
    const current = button(document, "Revoke current session", "btn btn-secondary btn--sm");
    current.disabled = model.lockdown.active;
    current.title = mutationTitle(model.lockdown.active, "Revoke current browser session");
    current.addEventListener("click", () => void actions.revokeCurrentSession?.());
    heading.append(current);
  }

  if (model.security.operatorSessions.length === 0) {
    card.append(textElement(document, "p", "identity", "No operator sessions are currently visible."));
    return card;
  }
  const list = document.createElement("div");
  list.className = "security-list";
  model.security.operatorSessions.forEach((summary, index) => {
    const row = document.createElement("div");
    row.className = "security-row";
    row.append(
      textElement(document, "strong", "", "Operator session"),
      textElement(document, "span", "identity", `actor ${summary.actorId} · endpoint ${summary.endpointId} · device ${summary.deviceId} · generation ${summary.generation}`),
      textElement(document, "span", "identity", sessionStatus(summary)),
    );
    list.append(row);
    if (index < model.security.operatorSessions.length - 1) list.append(document.createElement("hr"));
  });
  card.append(list);
  return card;
}

function controlSurfaceInventory(document: Document, model: PresentationModel, actions?: SecurityViewActions): HTMLElement {
  const card = inventoryCard(document, "Endpoints & devices");
  if (model.security.controlSurfaces.length === 0) {
    card.append(textElement(document, "p", "identity", "No enrolled endpoints or devices are currently visible."));
    return card;
  }
  const list = document.createElement("div");
  list.className = "security-list";
  model.security.controlSurfaces.forEach((summary, index) => {
    const row = document.createElement("div");
    row.className = "security-row";
    row.append(
      textElement(document, "strong", "", summary.deviceId),
      textElement(document, "span", "identity", `principal ${summary.principalId} · endpoint ${summary.endpointId} · generation ${summary.generation}`),
    );
    const actionsRow = document.createElement("div");
    actionsRow.className = "row-actions";
    const endpoint = button(document, "Revoke endpoint", "btn btn-secondary btn--sm");
    endpoint.disabled = model.lockdown.active || !actions;
    endpoint.title = mutationTitle(model.lockdown.active, `Revoke endpoint ${summary.endpointId}`);
    endpoint.addEventListener("click", () => void actions?.revokeEndpoint(summary.endpointId));
    const device = button(document, "Revoke device", "btn btn-secondary btn--sm");
    device.disabled = model.lockdown.active || !actions;
    device.title = mutationTitle(model.lockdown.active, `Revoke device ${summary.deviceId}`);
    device.addEventListener("click", () => void actions?.revokeDevice(summary.deviceId));
    actionsRow.append(endpoint, device);
    row.append(actionsRow);
    list.append(row);
    if (index < model.security.controlSurfaces.length - 1) list.append(document.createElement("hr"));
  });
  card.append(list);
  return card;
}

function grantInventory(document: Document, model: PresentationModel, actions?: SecurityViewActions): HTMLElement {
  const card = inventoryCard(document, "Active grants");
  if (model.security.grants.length === 0) {
    card.append(textElement(document, "p", "identity", "No grants are currently visible."));
    return card;
  }
  const list = document.createElement("div");
  list.className = "security-list";
  model.security.grants.forEach((summary, index) => {
    const row = document.createElement("div");
    row.className = "security-row";
    row.append(
      textElement(document, "strong", "", summary.grantId),
      textElement(document, "span", "identity", `${summary.subjectActorId} · ${scopeLabel(summary.targetScope)} · ${summary.allowedOperationKinds.map(operationKindLabel).join(", ") || "no Operations"}`),
      textElement(document, "span", "identity", summary.revoked ? "revoked" : summary.expiresAt ? `expires ${summary.expiresAt.toISOString()}` : "active"),
    );
    const revoke = button(document, "Revoke grant", "btn btn-secondary btn--sm");
    revoke.disabled = model.lockdown.active || !actions;
    revoke.title = mutationTitle(model.lockdown.active, `Revoke grant ${summary.grantId}`);
    revoke.addEventListener("click", () => void actions?.revokeGrant(summary.grantId));
    row.append(revoke);
    list.append(row);
    if (index < model.security.grants.length - 1) list.append(document.createElement("hr"));
  });
  card.append(list);
  return card;
}

function inventoryCard(document: Document, title: string): HTMLElement {
  const card = document.createElement("section");
  card.className = "card";
  const heading = document.createElement("div");
  heading.className = "section-heading";
  heading.append(textElement(document, "h2", "", title));
  card.append(heading);
  return card;
}

function sessionStatus(summary: { active: boolean; revoked: boolean; expired: boolean }): string {
  if (summary.revoked) return "revoked";
  if (summary.expired) return "expired";
  return summary.active ? "authenticated" : "inactive";
}

function operationKindLabel(value: number): string {
  const label = OperationKind[value];
  return label ? label.replace(/^RESERVED_/, "").toLowerCase().replaceAll("_", "-") : `kind-${value}`;
}

function scopeLabel(scope: { kind: number; adapterId?: { value: string }; runtimeSessionId?: { value: string }; deploymentScope: string } | undefined): string {
  if (!scope) return "unknown scope";
  if (scope.kind === TargetScopeKind.AUTHORITY_DOMAIN) return "authority domain";
  if (scope.kind === TargetScopeKind.RUNTIME_SESSION) {
    return `${scope.adapterId?.value ?? "adapter"}/${scope.deploymentScope}/${scope.runtimeSessionId?.value ?? "session"}`;
  }
  return `scope kind ${scope.kind}`;
}

function mutationTitle(active: boolean, action: string): string {
  return active ? "Disabled during lockdown: revocations are unavailable." : action;
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
