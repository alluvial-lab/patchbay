import { create } from "@bufbuild/protobuf";
import { AuthorityDomainIdSchema, SessionConnectivityState, type AuthorityDomainId } from "@patchbay/contracts";

import {
  sessionKey,
  stableTarget,
  type PresentationModel,
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
import { renderSecurityView, type SecurityViewActions } from "./security-view.js";
import type { ElicitationRenderOptions } from "./elicitation.js";
import type { MarkdownRenderer } from "./markdown.js";

export type CockpitDestination = "sessions" | "security" | "diagnostics" | "files" | "git" | "settings";

export interface CockpitShellPreferences {
  sessionsPanelCollapsed: boolean;
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
  readonly detail: SessionDetailComponent;
  select(sessionKey: string): void;
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
  let mobileDetailOpen = false;
  let filter = "";
  let destination: CockpitDestination = "sessions";
  const authorityDomainId = options.authorityDomainId
    ?? create(AuthorityDomainIdSchema, { value: model.authorityDomainId ?? "default" });
  const preferenceStore = options.preferenceStore ?? browserPreferenceStore(document);
  let panelCollapsed = preferenceStore.load(authorityDomainId.value).sessionsPanelCollapsed;
  let detail!: SessionDetailComponent;
  let observedSelectedKey: string | undefined;
  let observedConnectivity: SessionConnectivityState | undefined;
  const isMobile = options.isMobile ?? (() => document.defaultView?.matchMedia?.("(max-width: 760px)").matches ?? false);

  const resize = () => applyLayout();
  document.defaultView?.addEventListener("resize", resize);

  function selectedSession(): SessionView | undefined {
    return selectedKey ? model.sessions.get(selectedKey) : undefined;
  }

  function render(): void {
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
    const rail = renderRail(document, destination, (next) => selectDestination(next));
    const sidebar = renderSidebar(document, model, selectedKey, filter, {
      select(session) {
        selectedKey = sessionKey(session.identity);
        mobileDetailOpen = true;
        render();
      },
      filter(value) {
        filter = value;
        render();
      },
    });
    const main = document.createElement("main");
    main.className = "main";
    detail = renderSessionDetail(document, model, selectedSession(), {
      markdown: options.markdown,
      actions: options.actions,
      elicitation: options.elicitation,
      submission: options.submission?.(),
      lockdownActive: model.lockdown.active || Boolean(model.lockdown.submitting),
      onBack() {
        mobileDetailOpen = false;
        applyLayout();
      },
    });
    const security = renderSecurityView(document, model, authorityDomainId, options.securityActions);
    security.hidden = destination !== "security";
    const planned = renderPlannedView(document, destination);
    planned.hidden = !isPlannedDestination(destination);
    detail.element.hidden = destination !== "sessions";
    main.append(detail.element, security, planned);
    content.append(rail, sidebar, main);
    const degraded = destination === "sessions" ? renderDegradedBanner(document, model, selectedSession()) : undefined;
    if (degraded) root.append(degraded);
    if (model.lockdown.active) root.append(renderLockdownBanner(document, model));
    root.append(content, renderBottomTabs(document, destination, (next) => selectDestination(next)));
    root.append(renderOverflowMenu(document, destination, (next) => selectDestination(next)));
    if (options.elicitation?.mobileSheet) {
      root.append(options.elicitation.mobileSheet.backdrop, options.elicitation.mobileSheet.element);
    }
    applyLayout();
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
    if (!sidebar || !main || !detail) return;
    root.classList.toggle("cockpit--mobile", mobile);
    root.classList.toggle("cockpit--desktop", !mobile);
    root.classList.toggle("cockpit--panel-collapsed", panelCollapsed && !mobile);
    root.dataset.layout = mobile ? "drill-in" : "two-pane";
    root.dataset.destination = destination;
    sidebar.hidden = destination !== "sessions" || (mobile && mobileDetailOpen) || (!mobile && panelCollapsed);
    main.hidden = mobile && destination === "sessions" && !mobileDetailOpen;
    detail.setMobile(mobile);
  }

  function selectDestination(next: CockpitDestination): void {
    const mobile = isMobile();
    if (next === "sessions" && destination === "sessions" && !mobile) {
      panelCollapsed = !panelCollapsed;
      preferenceStore.save(authorityDomainId.value, { sessionsPanelCollapsed: panelCollapsed });
    } else {
      destination = next;
      if (next === "sessions" && !mobile) panelCollapsed = false;
    }
    if (next !== "sessions") mobileDetailOpen = false;
    render();
  }

  const shell: CockpitShell = {
    element: root,
    get selectedSessionKey() {
      return selectedKey;
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
    back() {
      mobileDetailOpen = false;
      applyLayout();
    },
    update(nextModel) {
      model = nextModel;
      if (!selectedKey || !model.sessions.has(selectedKey)) selectedKey = preferredSessionKey(model);
      render();
    },
    refreshLayout: applyLayout,
    destroy() {
      document.defaultView?.removeEventListener("resize", resize);
      options.elicitation?.mobileSheet?.close();
      root.replaceChildren();
    },
  };
  render();
  return shell;
}

const DESTINATION_ICONS: Record<CockpitDestination, IconName> = {
  sessions: "chevron-right",
  security: "square",
  diagnostics: "chevron-down",
  files: "folder",
  git: "link",
  settings: "plus",
};

function renderRail(
  document: Document,
  selected: CockpitDestination,
  onSelect: (destination: CockpitDestination) => void,
): HTMLElement {
  const rail = document.createElement("aside");
  rail.className = "rail";
  rail.setAttribute("aria-label", "Cockpit navigation");
  const nav = document.createElement("nav");
  nav.className = "destination-list";
  for (const destination of ["sessions", "security", "diagnostics", "files", "git", "settings"] as CockpitDestination[]) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "btn btn-ghost destination";
    button.dataset.destination = destination;
    button.setAttribute("aria-label", capitalize(destination));
    button.dataset.tip = capitalize(destination);
    if (selected === destination) button.setAttribute("aria-current", "page");
    button.append(renderIcon(document, DESTINATION_ICONS[destination]), textElement(document, "span", "destination__label", capitalize(destination)));
    button.addEventListener("click", () => onSelect(destination));
    nav.append(button);
  }
  rail.append(nav);
  return rail;
}

function renderBottomTabs(
  document: Document,
  selected: CockpitDestination,
  onSelect: (destination: CockpitDestination) => void,
): HTMLElement {
  const nav = document.createElement("nav");
  nav.className = "bottom-tabs";
  nav.setAttribute("aria-label", "Cockpit destinations");
  for (const destination of ["sessions", "security"] as CockpitDestination[]) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "tabs__tab";
    button.setAttribute("aria-label", capitalize(destination));
    if (selected === destination) button.setAttribute("aria-current", "page");
    button.append(renderIcon(document, DESTINATION_ICONS[destination]), textElement(document, "span", "", capitalize(destination)));
    button.addEventListener("click", () => onSelect(destination));
    nav.append(button);
  }
  const more = document.createElement("button");
  more.type = "button";
  more.className = "tabs__tab";
  more.dataset.more = "true";
  more.setAttribute("aria-label", "More destinations");
  more.append(renderIcon(document, "chevron-down"), textElement(document, "span", "", "More"));
  more.addEventListener("click", () => nav.parentElement?.classList.toggle("more-open"));
  nav.append(more);
  return nav;
}

function renderOverflowMenu(
  document: Document,
  selected: CockpitDestination,
  onSelect: (destination: CockpitDestination) => void,
): HTMLElement {
  const menu = document.createElement("nav");
  menu.className = "overflow-menu";
  menu.setAttribute("aria-label", "More cockpit destinations");
  for (const destination of ["diagnostics", "files", "git", "settings"] as CockpitDestination[]) {
    const button = document.createElement("button");
    button.type = "button";
    button.className = "btn btn-ghost destination";
    button.dataset.destination = destination;
    button.setAttribute("aria-current", selected === destination ? "page" : "false");
    button.textContent = capitalize(destination);
    button.addEventListener("click", () => onSelect(destination));
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
  return destination !== "sessions" && destination !== "security";
}

function browserPreferenceStore(document: Document): CockpitShellPreferenceStore {
  const key = (domain: string) => `patchbay.cockpit.${domain}.shell`;
  return {
    load(domain) {
      try {
        const value = document.defaultView?.localStorage.getItem(key(domain));
        return value ? JSON.parse(value) as CockpitShellPreferences : { sessionsPanelCollapsed: false };
      } catch {
        return { sessionsPanelCollapsed: false };
      }
    },
    save(domain, value) {
      try { document.defaultView?.localStorage.setItem(key(domain), JSON.stringify(value)); } catch { /* storage is optional */ }
    },
  };
}

function capitalize(value: string): string {
  return value.charAt(0).toUpperCase() + value.slice(1);
}

interface SidebarActions {
  select(session: SessionView): void;
  filter(value: string): void;
}

function renderSidebar(
  document: Document,
  model: PresentationModel,
  selectedKey: string | undefined,
  filter: string,
  actions: SidebarActions,
): HTMLElement {
  const sidebar = document.createElement("aside");
  sidebar.className = "sidebar";
  const header = document.createElement("header");
  header.className = "nav-bar";
  header.append(textElement(document, "span", "nav-bar__brand", "Patchbay · Sessions"));
  const headerActions = document.createElement("div");
  headerActions.className = "sidebar__actions";
  headerActions.append(
    unavailableIconButton(document, "plus", "Spawn session unavailable"),
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
      onSelect: actions.select,
    }),
  );
  return sidebar;
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
): HTMLElement | undefined {
  if (!model.reconciled) {
    const banner = alertBanner(
      document,
      "Reconnecting",
      "The projection is unreconciled. Cached session state remains stale until snapshot replay and the live stream catch up.",
      "warning",
    );
    if (session) banner.append(renderSessionStatus(document, session));
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
  banner.append(renderSessionStatus(document, session));
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
