export interface SettingsViewOptions {
  showToolCalls: boolean;
  authorityDomainId: string;
  onShowToolCallsChange(next: boolean): void;
  onClose(): void;
}

export interface SettingsViewComponent {
  backdrop: HTMLElement;
  dialog: HTMLElement;
  toggle: HTMLButtonElement;
}

/**
 * Presentation-only cockpit preferences. The modal deliberately lives in the
 * shell layer rather than the operator domain: hiding tool calls changes only
 * the rendered projection and never the folded transcript.
 */
export function renderSettingsView(
  document: Document,
  options: SettingsViewOptions,
): SettingsViewComponent {
  const backdrop = document.createElement("div");
  backdrop.className = "settings-backdrop";
  backdrop.setAttribute("aria-hidden", "true");
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

  const list = document.createElement("ul");
  list.className = "settings-list";
  const item = document.createElement("li");
  item.className = "settings-list__item";
  const toggle = document.createElement("button");
  toggle.type = "button";
  toggle.className = "settings-toggle";
  toggle.setAttribute("aria-pressed", String(options.showToolCalls));
  toggle.setAttribute("aria-label", `Show tool calls: ${options.showToolCalls ? "on" : "off"}`);

  const copy = document.createElement("span");
  copy.className = "settings-toggle__copy";
  const label = document.createElement("strong");
  label.textContent = "Show tool calls";
  const hint = document.createElement("small");
  hint.textContent = "Keep tool activity visible in the timeline";
  copy.append(label, hint);

  const control = document.createElement("span");
  control.className = "settings-toggle__control";
  control.setAttribute("aria-hidden", "true");
  control.textContent = options.showToolCalls ? "On" : "Off";
  toggle.append(copy, control);
  toggle.addEventListener("click", () => options.onShowToolCallsChange(!options.showToolCalls));
  item.append(toggle);
  list.append(item);

  dialog.append(header, description, scope, list);
  dialog.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      options.onClose();
      return;
    }
    if (event.key !== "Tab") return;
    const controls = focusableControls(dialog);
    if (controls.length === 0) {
      event.preventDefault();
      dialog.focus();
      return;
    }
    const first = controls[0]!;
    const last = controls.at(-1)!;
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  });

  return { backdrop, dialog, toggle };
}

function focusableControls(root: HTMLElement): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(
    'button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
  )].filter((element) => !element.hasAttribute("inert"));
}
