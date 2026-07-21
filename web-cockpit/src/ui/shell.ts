import {
  sessionKey,
  stableTarget,
  type PresentationModel,
  type SessionView,
} from "../domain/model.js";
import {
  renderSessionDetail,
  type SessionDetailActions,
  type SessionDetailComponent,
} from "./session-detail.js";
import { renderSessionList } from "./session-list.js";
import type { ElicitationRenderOptions } from "./elicitation.js";
import type { MarkdownRenderer } from "./markdown.js";

export interface CockpitShellOptions {
  markdown: MarkdownRenderer;
  actions?: SessionDetailActions;
  elicitation?: ElicitationRenderOptions;
  isMobile?: () => boolean;
}

export interface CockpitShell {
  readonly element: HTMLElement;
  readonly selectedSessionKey?: string;
  readonly detail: SessionDetailComponent;
  select(sessionKey: string): void;
  back(): void;
  update(model: PresentationModel): void;
  refreshLayout(): void;
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
  let detail!: SessionDetailComponent;
  const isMobile = options.isMobile ?? (() => document.defaultView?.matchMedia?.("(max-width: 760px)").matches ?? false);

  const resize = () => applyLayout();
  document.defaultView?.addEventListener("resize", resize);

  function selectedSession(): SessionView | undefined {
    return selectedKey ? model.sessions.get(selectedKey) : undefined;
  }

  function render(): void {
    root.replaceChildren();
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
      onBack() {
        mobileDetailOpen = false;
        applyLayout();
      },
    });
    main.append(detail.element);
    root.append(sidebar, main);
    if (options.elicitation?.mobileSheet) {
      root.append(options.elicitation.mobileSheet.backdrop, options.elicitation.mobileSheet.element);
    }
    applyLayout();
  }

  function applyLayout(): void {
    const mobile = isMobile();
    const sidebar = root.querySelector<HTMLElement>(".sidebar");
    const main = root.querySelector<HTMLElement>(".main");
    if (!sidebar || !main || !detail) return;
    root.classList.toggle("cockpit--mobile", mobile);
    root.classList.toggle("cockpit--desktop", !mobile);
    root.dataset.layout = mobile ? "drill-in" : "two-pane";
    sidebar.hidden = mobile && mobileDetailOpen;
    main.hidden = mobile && !mobileDetailOpen;
    detail.setMobile(mobile);
  }

  const shell: CockpitShell = {
    element: root,
    get selectedSessionKey() {
      return selectedKey;
    },
    get detail() {
      return detail;
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
      onSelect: actions.select,
    }),
  );
  return sidebar;
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
