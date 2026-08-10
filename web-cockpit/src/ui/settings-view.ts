export interface SettingsViewOptions {
  showToolCalls: boolean;
  authorityDomainId: string;
  onShowToolCallsChange(next: boolean): void;
  onClose(): void;
}

/**
 * Presentation-only cockpit preferences. The dialog deliberately lives in the
 * shell layer rather than the operator domain: hiding tool calls changes only
 * the rendered projection and never the folded transcript.
 */
export function renderSettingsView(
  document: Document,
  options: SettingsViewOptions,
): { backdrop: HTMLElement; dialog: HTMLElement; toggle: HTMLInputElement } {
  const backdrop = document.createElement("button");
  backdrop.type = "button";
  backdrop.className = "settings-backdrop";
  backdrop.setAttribute("aria-label", "Close cockpit settings");
  backdrop.addEventListener("click", () => options.onClose());

  const dialog = document.createElement("section");
  dialog.className = "settings-dialog";
  dialog.setAttribute("role", "dialog");
  dialog.setAttribute("aria-modal", "true");
  dialog.setAttribute("aria-labelledby", "cockpit-settings-title");
  dialog.setAttribute("aria-describedby", "cockpit-settings-description");

  const header = document.createElement("header");
  header.className = "settings-dialog__header";
  const title = document.createElement("h2");
  title.id = "cockpit-settings-title";
  title.textContent = "Cockpit settings";
  const close = document.createElement("button");
  close.type = "button";
  close.className = "btn btn-ghost btn--sm";
  close.textContent = "Close";
  close.setAttribute("aria-label", "Close cockpit settings");
  close.addEventListener("click", () => options.onClose());
  header.append(title, close);

  const description = document.createElement("p");
  description.id = "cockpit-settings-description";
  description.className = "settings-dialog__description";
  description.textContent = "Presentation preferences apply to this cockpit only. The durable transcript remains complete.";

  const scope = document.createElement("p");
  scope.className = "settings-dialog__scope";
  scope.textContent = `Authority domain: ${options.authorityDomainId}`;

  const field = document.createElement("label");
  field.className = "settings-toggle";
  const copy = document.createElement("span");
  copy.className = "settings-toggle__copy";
  const label = document.createElement("strong");
  label.textContent = "Show tool calls";
  const hint = document.createElement("small");
  hint.textContent = "Keep tool activity visible in the timeline";
  copy.append(label, hint);

  const toggle = document.createElement("input");
  toggle.type = "checkbox";
  toggle.checked = options.showToolCalls;
  toggle.setAttribute("aria-label", "Show tool calls");
  toggle.addEventListener("change", () => options.onShowToolCallsChange(toggle.checked));
  field.append(copy, toggle);

  dialog.append(header, description, scope, field);
  return { backdrop, dialog, toggle };
}
