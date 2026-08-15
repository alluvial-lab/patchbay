import { create } from "@bufbuild/protobuf";
import { AuthorityDomainIdSchema, SessionConnectivityState, type AuthorityDomainId } from "@patchbay/contracts";

import {
  resourceKey,
  sessionKey,
  stableTarget,
  type PresentationModel,
  type ResourceIdentityView,
  type ResourceView,
  type SessionView,
} from "../domain/model.js";
import { renderIcon, type IconName } from "./icons.js";
import { captureAnchor, restoreAnchor } from "./scroll-anchor.js";
import {
  renderSessionDetail,
  type SessionDetailActions,
  type SessionDetailComponent,
  type SubmissionFeedback,
} from "./session-detail.js";
import { renderSessionList, renderSessionStatus } from "./session-list.js";
import {
  renderResourceDestination,
  type ResourceDestinationComponent,
} from "./resource-view.js";
import { renderSecurityView, type SecurityViewActions } from "./security-view.js";
import type { ElicitationRenderOptions } from "./elicitation.js";
import type { MarkdownRenderer } from "./markdown.js";
import { renderSettingsView } from "./settings-view.js";

export type CockpitDestination = "sessions" | "resources" | "security" | "diagnostics" | "files" | "git" | "settings";
type NavigationSource = "rail" | "bottom-tabs" | "overflow";

export interface CockpitShellPreferences {
  sessionsPanelCollapsed: boolean;
  showToolCalls: boolean;
}

export interface CockpitShellPreferenceStore {
  load(authorityDomainId: string): CockpitShellPreferences;
  save(authorityDomainId: string, value: CockpitShellPreferences): void;
}

export interface CockpitShellOptions {
  markdown: MarkdownRenderer;
  onSelectionChange?(session: SessionView | undefined, reason: "initial" | "selection" | "connectivity"): void;
  actions?: SessionDetailActions;
  elicitation?: ElicitationRenderOptions;
  submission?: () => SubmissionFeedback | undefined;
  isMobile?: () => boolean;
  authorityDomainId?: AuthorityDomainId;
  securityActions?: SecurityViewActions;
  preferenceStore?: CockpitShellPreferenceStore;
}

export interface CockpitShell {
  readonly element: HTMLElement;
  readonly selectedSessionKey?: string;
  readonly selectedResourceKey?: string;
  readonly detail: SessionDetailComponent;
  select(sessionKey: string): void;
  openResource(identity: ResourceIdentityView): void;
  back(): void;
  update(model: PresentationModel): void;
  refreshLayout(): void;
  selectDestination(destination: CockpitDestination): void;
  destroy(): void;
}

/**
 * Composes the locked session row/detail primitives. Desktop and mobile move
 * through one container policy; they never fork the session-detail component.
 */
export function createCockpitShell(
  document: Document,
  initialModel: PresentationModel,
  options: CockpitShellOptions,
): CockpitShell {
  const root = document.createElement("div");
  root.className = "cockpit";
  let model = initialModel;
  let selectedKey = preferredSessionKey(model);
  let selectedResourceKey = preferredResourceKey(model);
  let mobileDetailOpen = false;
  let mobileResourceDetailOpen = false;
  let filter = "";
  let destination: CockpitDestination = "sessions";
  let settingsOpen = false;
  let settingsOpenerSource: NavigationSource | undefined;
  let pendingSettingsFocusRestore: NavigationSource | undefined;
  const authorityDomainId = options.authorityDomainId
    ?? create(AuthorityDomainIdSchema, { value: model.authorityDomainId ?? "default" });
  const preferenceStore = options.preferenceStore ?? browserPreferenceStore(document);
  let preferences = normalizePreferences(preferenceStore.load(authorityDomainId.value));
  let panelCollapsed = preferences.sessionsPanelCollapsed;
  let showToolCalls = preferences.showToolCalls;
  let detail!: SessionDetailComponent;
  let resourceDestination!: ResourceDestinationComponent;
  let observedSelectedKey: string | undefined;
  let observedConnectivity: SessionConnectivityState | undefined;
  const isMobile = options.isMobile ?? (() => document.defaultView?.matchMedia?.("(max-width: 760px)").matches ?? false);
  const settingsBackgroundInert = new Map<HTMLElement, boolean>();

  const resize = () => applyLayout();
  document.defaultView?.addEventListener("resize", resize);

  function selectedSession(): SessionView | undefined {
    return selectedKey ? model.sessions.get(selectedKey) : undefined;
  }

  function render(): void {
    restoreSettingsBackgroundInert();
    // The render is a full DOM rebuild, which would lose the timeline scroll
    // position on every streamed delta. Capture whether the user was already
    // near the bottom before the rebuild; only stick to the bottom if so
    // (never yank a user reading history back down).
    const previousTimeline = root.querySelector<HTMLElement>(".timeline");
    const stickToBottom = !previousTimeline || isNearBottom(previousTimeline);
    // Reading history (not near bottom): capture the visible anchor so the
    // rebuild restores the same viewport position instead of resetting to 0.
    const anchor = !stickToBottom && previousTimeline ? captureAnchor(previousTimeline) : undefined;
    root.replaceChildren();
    const content = document.createElement("div");
    content.className = "cockpit__content";
    content.dataset.mobileBottomNavReserve = "bottom-tabs";
    const rail = renderRail(document, destination, settingsOpen, (next, source) => selectDestination(next, source));
    const sidebar = renderSidebar(document, model, selectedKey, filter, showToolCalls, {
      select(session) {
        selectedKey = sessionKey(session.identity);
        mobileDetailOpen = true;
        render();
      },
      filter(value) {
        filter = value;
        render();
      },
      spawn: options.actions?.spawn
        ? (adapterId) => options.actions!.spawn!(adapterId)
        : undefined,
    });
    const main = document.createElement("section");
    main.className = "main";
    detail = renderSessionDetail(document, model, selectedSession(), {
      markdown: options.markdown,
      actions: options.actions,
      elicitation: options.elicitation,
      submission: options.submission?.(),
      showToolCalls,
      lockdownActive: model.lockdown.active || Boolean(model.lockdown.submitting),
      onBack() {
        mobileDetailOpen = false;
        applyLayout();
      },
      onOpenResource: openResource,
    });
    resourceDestination = renderResourceDestination(document, model, {
      selectedKey: selectedResourceKey,
      mobileDetailOpen: mobileResourceDetailOpen,
      lockdownActive: model.lockdown.active || Boolean(model.lockdown.submitting),
      onSelect(resource) {
        selectedResourceKey = resourceKey(resource.identity);
        mobileResourceDetailOpen = true;
        render();
      },
      onBack() {
        mobileResourceDetailOpen = false;
        render();
      },
    });
    resourceDestination.element.hidden = destination !== "resources";
    const security = renderSecurityView(document, model, authorityDomainId, options.securityActions);
    security.hidden = destination !== "security";
    const planned = renderPlannedView(document, destination);
    planned.hidden = !isPlannedDestination(destination);
    detail.element.hidden = destination !== "sessions";
    main.append(detail.element, resourceDestination.element, security, planned);
    content.append(rail, sidebar, main);
    const degraded = destination === "sessions"
      ? renderDegradedBanner(document, model, selectedSession(), showToolCalls)
      : undefined;
    if (degraded) root.append(degraded);
    if (model.lockdown.active) root.append(renderLockdownBanner(document, model));
    root.append(
      content,
      renderBottomTabs(document, destination, settingsOpen, (next, source) => selectDestination(next, source)),
    );
    root.append(
      renderOverflowMenu(document, destination, settingsOpen, (next, source) => selectDestination(next, source)),
    );
    if (options.elicitation?.mobileSheet) {
      root.append(options.elicitation.mobileSheet.backdrop, options.elicitation.mobileSheet.element);
    }
    if (settingsOpen) {
      const settings = renderSettingsView(document, {
        showToolCalls,
        authorityDomainId: authorityDomainId.value,
        onShowToolCallsChange(next) {
          showToolCalls = next;
          preferences = { ...preferences, showToolCalls: next };
          preferenceStore.save(authorityDomainId.value, preferences);
          render();
        },
        onClose: closeSettings,
      });
      for (const background of [...root.children] as HTMLElement[]) {
        settingsBackgroundInert.set(background, background.hasAttribute("inert"));
        background.setAttribute("inert", "");
      }
      root.append(settings.backdrop, settings.dialog);
      queueMicrotask(() => settings.toggle.focus());
    }
    applyLayout();
    if (pendingSettingsFocusRestore) {
      const source = pendingSettingsFocusRestore;
      pendingSettingsFocusRestore = undefined;
      queueMicrotask(() => settingsOpener(root, source, isMobile())?.focus());
    }
    const timeline = root.querySelector<HTMLElement>(".timeline");
    if (timeline) {
      if (stickToBottom) timeline.scrollTop = timeline.scrollHeight;
      else if (anchor) restoreAnchor(timeline, anchor);
    }
    const selected = selectedSession();
    const selectedIdentity = selected ? sessionKey(selected.identity) : undefined;
    // Reconciliation completion is delivered by Reconciler directly to the
    // diagnostics controller. Shell renders are intentionally not used as a
    // lifecycle edge because intermediate unreconciled models may not render.
    const reason = observedSelectedKey === undefined
      ? "initial"
      : observedSelectedKey !== selectedIdentity
        ? "selection"
        : observedConnectivity !== selected?.connectivity
          ? "connectivity"
          : undefined;
    observedSelectedKey = selectedIdentity;
    observedConnectivity = selected?.connectivity;
    if (reason) queueMicrotask(() => options.onSelectionChange?.(selected, reason));
  }

  function isNearBottom(el: HTMLElement): boolean {
    return el.scrollHeight - el.scrollTop - el.clientHeight < 80;
  }

  function applyLayout(): void {
    const mobile = isMobile();
    const sidebar = root.querySelector<HTMLElement>(".sidebar");
    const main = root.querySelector<HTMLElement>(".main");
    if (!sidebar || !main || !detail || !resourceDestination) return;
    root.classList.toggle("cockpit--mobile", mobile);
    root.classList.toggle("cockpit--desktop", !mobile);
    root.classList.toggle("cockpit--panel-collapsed", panelCollapsed && !mobile);
    root.dataset.layout = mobile ? "drill-in" : "two-pane";
    root.dataset.destination = destination;
    sidebar.hidden = destination !== "sessions" || (mobile && mobileDetailOpen) || (!mobile && panelCollapsed);
    main.hidden = mobile && destination === "sessions" && !mobileDetailOpen;
    detail.setMobile(mobile);
    resourceDestination.setMobile(mobile);
  }

  function closeSettings(): void {
    pendingSettingsFocusRestore = settingsOpenerSource;
    settingsOpen = false;
    render();
  }

  function restoreSettingsBackgroundInert(): void {
    for (const [background, wasInert] of settingsBackgroundInert) {
      background.toggleAttribute("inert", wasInert);
    }
    settingsBackgroundInert.clear();
  }

  function selectDestination(next: CockpitDestination, source?: NavigationSource): void {
    root.classList.remove("more-open");
    if (next === "settings") {
      settingsOpenerSource = source ?? (isMobile() ? "overflow" : "rail");
      settingsOpen = true;
      render();
      return;
    }
    settingsOpen = false;
    const mobile = isMobile();
    if (next === "sessions" && destination === "sessions" && !mobile) {
      panelCollapsed = !panelCollapsed;
      preferences = { ...preferences, sessionsPanelCollapsed: panelCollapsed };
      preferenceStore.save(authorityDomainId.value, preferences);
    } else {
      destination = next;
      if (next === "sessions" && !mobile) panelCollapsed = false;
    }
    if (next !== "sessions") mobileDetailOpen = false;
    if (next !== "resources") mobileResourceDetailOpen = false;
    render();
  }

  function openResource(identity: ResourceIdentityView): void {
    selectedResourceKey = resourceKey(identity);
    destination = "resources";
    mobileDetailOpen = false;
    mobileResourceDetailOpen = true;
    render();
  }

  const shell: CockpitShell = {
    element: root,
    get selectedSessionKey() {
      return selectedKey;
    },
    get selectedResourceKey() {
      return selectedResourceKey;
    },
    get detail() {
      return detail;
    },
    selectDestination(next) {
      selectDestination(next);
    },
    select(nextKey) {
      if (!model.sessions.has(nextKey)) throw new Error(`unknown session ${nextKey}`);
      selectedKey = nextKey;
      mobileDetailOpen = true;
      render();
    },
    openResource,
    back() {
      if (destination === "resources") {
        mobileResourceDetailOpen = false;
        render();
      } else {
        mobileDetailOpen = false;
        applyLayout();
      }
    },
    update(nextModel) {
      model = nextModel;
      if (!selectedKey || !model.sessions.has(selectedKey)) selectedKey = preferredSessionKey(model);
      if (!selectedResourceKey || !model.resources.has(selectedResourceKey)) {
        selectedResourceKey = preferredResourceKey(model);
      }
      render();
    },
    refreshLayout: applyLayout,
    destroy() {
      document.defaultView?.removeEventListener("resize", resize);
      restoreSettingsBackgroundInert();
      options.elicitation?.mobileSheet?.close();
      root.replaceChildren();
    },
  };
  render();
  return shell;
}

const DESTINATION_ICONS: Record<CockpitDestination, IconName> = {
  sessions: "chevron-right",
  resources: "sliders-horizontal",
  security: "square",
  diagnostics: "chevron-down",
  files: "folder",
  git: "link",
  settings: "settings",
};

function renderRail(
  document: Document,
  selected: CockpitDestination,
  settingsOpen: boolean,
  onSelect: (destination: CockpitDestination, source: NavigationSource) => void,
): HTMLElement {
  const rail = document.createElement("aside");
  rail.className = "rail";
  rail.setAttribute("aria-label", "Cockpit navigation");
  const nav = document.createElement("nav");
  nav.className = "destination-list";
  for (const destination of ["sessions", "resources", "security", "diagnostics", "files", "git", "settings"] as CockpitDestination[]) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "btn btn-ghost destination";
    button.dataset.destination = destination;
    button.setAttribute("aria-label", capitalize(destination));
    button.dataset.tip = capitalize(destination);
    if (selected === destination && !settingsOpen) button.setAttribute("aria-current", "page");
    if (destination === "settings") button.setAttribute("aria-expanded", String(settingsOpen));
    button.append(renderIcon(document, DESTINATION_ICONS[destination]), textElement(document, "span", "destination__label", capitalize(destination)));
    button.addEventListener("click", () => onSelect(destination, "rail"));
    nav.append(button);
  }
  rail.append(nav);
  return rail;
}

function renderBottomTabs(
  document: Document,
  selected: CockpitDestination,
  settingsOpen: boolean,
  onSelect: (destination: CockpitDestination, source: NavigationSource) => void,
): HTMLElement {
  const nav = document.createElement("nav");
  nav.id = "cockpit-mobile-tabs";
  nav.className = "bottom-tabs";
  nav.dataset.viewportObstruction = "bottom-tabs";
  nav.setAttribute("aria-label", "Cockpit destinations");
  for (const destination of ["sessions", "resources", "security"] as CockpitDestination[]) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "tabs__tab";
    button.setAttribute("aria-label", capitalize(destination));
    if (selected === destination) button.setAttribute("aria-current", "page");
    button.append(renderIcon(document, DESTINATION_ICONS[destination]), textElement(document, "span", "", capitalize(destination)));
    button.addEventListener("click", () => onSelect(destination, "bottom-tabs"));
    nav.append(button);
  }
  const more = document.createElement("button");
  more.type = "button";
  more.id = "cockpit-more-destinations";
  more.className = "tabs__tab";
  more.dataset.more = "true";
  more.setAttribute("aria-label", "More destinations");
  more.setAttribute("aria-controls", "cockpit-overflow-menu");
  more.setAttribute("aria-expanded", "false");
  more.setAttribute("aria-haspopup", "true");
  if (settingsOpen || isOverflowDestination(selected)) more.setAttribute("aria-current", "page");
  more.append(renderIcon(document, "chevron-down"), textElement(document, "span", "", "More"));
  more.addEventListener("click", () => {
    const root = nav.parentElement;
    if (!root) return;
    const expanded = !root.classList.contains("more-open");
    root.classList.toggle("more-open", expanded);
    more.setAttribute("aria-expanded", String(expanded));
  });
  nav.append(more);
  return nav;
}

function renderOverflowMenu(
  document: Document,
  selected: CockpitDestination,
  settingsOpen: boolean,
  onSelect: (destination: CockpitDestination, source: NavigationSource) => void,
): HTMLElement {
  const menu = document.createElement("nav");
  menu.id = "cockpit-overflow-menu";
  menu.className = "overflow-menu";
  menu.setAttribute("aria-label", "More cockpit destinations");
  for (const destination of ["diagnostics", "files", "git", "settings"] as CockpitDestination[]) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "btn btn-ghost destination";
    button.dataset.destination = destination;
    button.setAttribute("aria-current", selected === destination && !settingsOpen ? "page" : "false");
    if (destination === "settings") button.setAttribute("aria-expanded", String(settingsOpen));
    button.textContent = capitalize(destination);
    button.addEventListener("click", () => onSelect(destination, "overflow"));
    menu.append(button);
  }
  return menu;
}

function renderPlannedView(document: Document, destination: CockpitDestination): HTMLElement {
  const view = document.createElement("section");
  view.className = "view planned-view";
  const card = document.createElement("div");
  card.className = "empty-state";
  card.append(textElement(document, "p", "identity", "PLANNED DESTINATION"));
  card.append(textElement(document, "h1", "empty-state__title", capitalize(destination)));
  card.append(textElement(document, "p", "empty-state__body", `${capitalize(destination)} is planned for a future Patchbay release. Sessions and Security remain available now.`));
  view.append(card);
  return view;
}

function renderLockdownBanner(document: Document, model: PresentationModel): HTMLElement {
  const banner = document.createElement("section");
  banner.className = "alert alert--danger lockdown-banner";
  banner.setAttribute("role", "alert");
  banner.setAttribute("aria-live", "assertive");
  const reason = model.lockdown.reasonCode?.replaceAll("_", " ") ?? "security incident";
  const entered = model.lockdown.enteredAt?.toISOString() ?? "unknown time";
  banner.append(
    textElement(document, "strong", "alert__title", "Security lockdown active"),
    textElement(document, "span", "alert__body", `Reason: ${reason} · entered ${entered} · authority domain ${model.authorityDomainId ?? "default"}`),
    textElement(document, "code", "lockdown-exit-instruction", "patchbay-cli lockdown-exit"),
  );
  return banner;
}

function isPlannedDestination(destination: CockpitDestination): boolean {
  return destination !== "sessions" && destination !== "resources" && destination !== "security";
}

function normalizePreferences(value: Partial<CockpitShellPreferences> | null | undefined): CockpitShellPreferences {
  return {
    sessionsPanelCollapsed: value?.sessionsPanelCollapsed === true,
    showToolCalls: value?.showToolCalls !== false,
  };
}

function browserPreferenceStore(document: Document): CockpitShellPreferenceStore {
  const key = (domain: string) => `patchbay.cockpit.${domain}.shell`;
  return {
    load(domain) {
      try {
        const value = document.defaultView?.localStorage.getItem(key(domain));
        return value ? normalizePreferences(JSON.parse(value) as Partial<CockpitShellPreferences>) : normalizePreferences(undefined);
      } catch {
        return normalizePreferences(undefined);
      }
    },
    save(domain, value) {
      try { document.defaultView?.localStorage.setItem(key(domain), JSON.stringify(value)); } catch { /* storage is optional */ }
    },
  };
}

function isOverflowDestination(value: CockpitDestination): boolean {
  return value === "diagnostics" || value === "files" || value === "git" || value === "settings";
}

function settingsOpener(
  root: HTMLElement,
  source: NavigationSource,
  mobile: boolean,
): HTMLButtonElement | null {
  const visibleSelector = mobile
    ? '#cockpit-more-destinations'
    : '.rail [data-destination="settings"]';
  const sourceSelector = source === "rail"
    ? '.rail [data-destination="settings"]'
    : source === "overflow"
      ? '#cockpit-more-destinations'
      : '[data-destination="settings"]';
  return root.querySelector<HTMLButtonElement>(visibleSelector)
    ?? root.querySelector<HTMLButtonElement>(sourceSelector);
}

function capitalize(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

interface SidebarActions {
  select(session: SessionView): void;
  filter(value: string): void;
  spawn?(adapterId: string): void | Promise<void>;
}

function renderSidebar(
  document: Document,
  model: PresentationModel,
  selectedKey: string | undefined,
  filter: string,
  showToolCalls: boolean,
  actions: SidebarActions,
): HTMLElement {
  const sidebar = document.createElement("aside");
  sidebar.className = "sidebar";
  const header = document.createElement("header");
  header.className = "nav-bar";
  header.append(textElement(document, "span", "nav-bar__brand", "Patchbay · Sessions"));
  const headerActions = document.createElement("div");
  headerActions.className = "sidebar__actions";
  const knownAdapters = [...new Set([
    ...model.adapters.keys(),
    ...[...model.sessions.values()].map((session) => session.identity.adapterId),
  ])].sort();
  const spawn = iconActionButton(
    document,
    "plus",
    knownAdapters.length === 1 && actions.spawn
      ? `Spawn session on ${knownAdapters[0]}`
      : knownAdapters.length === 1
        ? "Spawn session unavailable"
        : "Spawn requires exactly one selected adapter",
    knownAdapters.length === 1 && actions.spawn
      ? () => actions.spawn!(knownAdapters[0]!)
      : undefined,
  );
  headerActions.append(
    spawn,
    unavailableIconButton(document, "link", "Attach session unavailable"),
  );
  header.append(headerActions);
  const needsYou = [...model.sessions.values()].filter((session) => session.needsYou && stableTarget(session)).length;
  if (needsYou > 0) {
    const attention = document.createElement("span");
    attention.className = "attention-badge";
    attention.setAttribute("aria-label", `${needsYou} sessions need attention`);
    attention.append(textElement(document, "span", "attention-badge__dot", ""));
    attention.append(textElement(document, "span", "attention-badge__count", String(needsYou)));
    header.append(attention);
  }

  const searchField = document.createElement("label");
  searchField.className = "field";
  searchField.append(textElement(document, "span", "field__label", "Filter sessions"));
  const search = document.createElement("input");
  search.type = "search";
  search.className = "input";
  search.value = filter;
  search.placeholder = "Identity, project, or cwd";
  search.addEventListener("change", () => actions.filter(search.value));
  searchField.append(search);

  sidebar.append(header, searchField);
  sidebar.append(
    renderSessionList(document, model.sessions.values(), {
      selectedKey,
      filter,
      adapters: model.adapters,
      showToolCalls,
      onSelect: actions.select,
    }),
  );
  return sidebar;
}

function iconActionButton(
  document: Document,
  icon: IconName,
  label: string,
  action: (() => void | Promise<void>) | undefined,
): HTMLButtonElement {
  const button = document.createElement("button");
  button.className = "btn btn-ghost btn--sm btn--icon-only";
  button.type = "button";
  button.disabled = !action;
  button.setAttribute("aria-label", label);
  button.title = label;
  button.append(renderIcon(document, icon));
  if (action) button.addEventListener("click", () => void action());
  return button;
}

function unavailableIconButton(document: Document, icon: IconName, label: string): HTMLButtonElement {
  const button = document.createElement("button");
  button.className = "btn btn-ghost btn--sm btn--icon-only";
  button.type = "button";
  button.disabled = true;
  button.setAttribute("aria-label", label);
  button.title = label;
  button.append(renderIcon(document, icon));
  return button;
}

function renderDegradedBanner(
  document: Document,
  model: PresentationModel,
  session: SessionView | undefined,
  showToolCalls: boolean,
): HTMLElement | undefined {
  if (!model.reconciled) {
    const banner = alertBanner(
      document,
      "Reconnecting",
      "The projection is unreconciled. Cached session state remains stale until snapshot replay and the live stream catch up.",
      "warning",
    );
    if (session) banner.append(renderSessionStatus(document, session, showToolCalls));
    return banner;
  }
  if (!session || session.connectivity === SessionConnectivityState.LIVE) return undefined;

  const names: Record<number, string> = {
    [SessionConnectivityState.STALE]: "Stale session",
    [SessionConnectivityState.OFFLINE]: "Session offline",
    [SessionConnectivityState.UNKNOWN]: "Session connectivity unknown",
    [SessionConnectivityState.FAILED]: "Session connection failed",
  };
  const title = names[session.connectivity];
  if (!title) return undefined;
  const banner = alertBanner(
    document,
    title,
    "The current connectivity state is authoritative; delivery may be unavailable or delayed.",
    session.connectivity === SessionConnectivityState.FAILED ? "danger" : "warning",
  );
  banner.append(renderSessionStatus(document, session, showToolCalls));
  return banner;
}

function alertBanner(
  document: Document,
  title: string,
  body: string,
  tone: "warning" | "danger",
): HTMLElement {
  const banner = document.createElement("div");
  banner.className = `alert alert--${tone}`;
  banner.setAttribute("role", "status");
  const copy = document.createElement("div");
  copy.append(textElement(document, "p", "alert__title", title));
  copy.append(textElement(document, "p", "alert__body", body));
  banner.append(copy);
  return banner;
}

function preferredSessionKey(model: PresentationModel): string | undefined {
  const sessions = [...model.sessions.values()];
  const preferred = sessions.find(stableTarget) ?? sessions.find((session) => !session.tombstoned) ?? sessions[0];
  return preferred ? sessionKey(preferred.identity) : undefined;
}

function preferredResourceKey(model: PresentationModel): string | undefined {
  const resources = [...model.resources.values()].filter((resource: ResourceView) => !resource.tombstoned);
  const preferred = resources.find((resource) => resource.reconciled) ?? resources[0];
  return preferred ? resourceKey(preferred.identity) : undefined;
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
