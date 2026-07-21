import {
  SessionActivityState,
  SessionConnectivityState,
} from "@patchbay/contracts";

import {
  rendersLive,
  sessionKey,
  stableTarget,
  type SessionIdentity,
  type SessionView,
} from "../domain/model.js";

export interface SessionListOptions {
  selectedKey?: string;
  onSelect(session: SessionView): void;
  filter?: string;
}

export function renderSessionList(
  document: Document,
  sessions: Iterable<SessionView>,
  options: SessionListOptions,
): HTMLElement {
  const list = document.createElement("div");
  list.className = "session-list";
  list.setAttribute("role", "list");
  const filter = options.filter?.trim().toLocaleLowerCase();
  const visible = [...sessions].filter((session) => !filter || searchableSession(session).includes(filter));
  visible.sort(compareSessions);

  if (visible.length === 0) {
    const empty = document.createElement("div");
    empty.className = "empty-state";
    empty.append(textElement(document, "p", "empty-state__title", "No sessions"));
    empty.append(textElement(document, "p", "empty-state__body", "Attach or spawn a runtime session to begin."));
    list.append(empty);
    return list;
  }

  for (const session of visible) {
    list.append(renderSessionRow(document, session, options.selectedKey === sessionKey(session.identity), options.onSelect));
  }
  return list;
}

export function renderSessionRow(
  document: Document,
  session: SessionView,
  selected: boolean,
  onSelect: (session: SessionView) => void,
): HTMLButtonElement {
  const row = document.createElement("button");
  row.type = "button";
  row.className = "session-row";
  row.setAttribute("role", "listitem");
  row.setAttribute("aria-pressed", String(selected));
  row.dataset.sessionKey = sessionKey(session.identity);
  if (selected) row.classList.add("session-row--active");
  if (session.needsYou && stableTarget(session)) row.classList.add("session-row--needs-you");

  row.append(textElement(document, "span", "session-row__identity", formatSessionIdentity(session.identity)));
  row.append(textElement(document, "span", "session-row__label", sessionLabel(session)));
  row.append(textElement(document, "span", "session-row__context", sessionContext(session)));

  const badges = document.createElement("span");
  badges.className = "session-row__badges";
  badges.append(renderSessionStatus(document, session));
  if (session.needsYou && stableTarget(session)) {
    const attention = document.createElement("span");
    attention.className = "attention-badge";
    attention.setAttribute("aria-label", "Needs your attention");
    attention.append(textElement(document, "span", "attention-badge__dot", ""));
    badges.append(attention);
  }
  row.append(badges);
  row.addEventListener("click", () => onSelect(session));
  return row;
}

export function renderSessionStatus(document: Document, session: SessionView): HTMLElement {
  const connectivity = effectiveConnectivity(session);
  const connectivityName = connectivityStateName(connectivity);
  const activityName = activityStateName(session.activity);
  const status = document.createElement("span");
  status.className = `session-status${connectivity === SessionConnectivityState.LIVE ? "" : ` session-status--${connectivityName}`}`;

  const connectivityIndicator = document.createElement("span");
  connectivityIndicator.className = `connectivity-indicator connectivity-indicator--${connectivityName}`;
  connectivityIndicator.append(textElement(document, "span", "connectivity-indicator__dot", ""));
  connectivityIndicator.append(document.createTextNode(connectivityName));

  const activityIndicator = document.createElement("span");
  activityIndicator.className = `activity-indicator activity-indicator--${activityName}`;
  activityIndicator.append(textElement(document, "span", "activity-indicator__icon", ""));
  activityIndicator.append(document.createTextNode(activityName));
  if (session.activityDetail) {
    activityIndicator.append(textElement(document, "span", "activity-indicator__detail", session.activityDetail));
  }
  status.append(connectivityIndicator, activityIndicator);
  return status;
}

export function formatSessionIdentity(identity: SessionIdentity): string {
  return `${identity.adapterId}@${identity.deploymentScope} · runtime ${identity.runtimeSessionId} · gen ${identity.generation}`;
}

function effectiveConnectivity(session: SessionView): SessionConnectivityState {
  if (rendersLive(session)) return SessionConnectivityState.LIVE;
  return session.connectivity === SessionConnectivityState.LIVE
    ? SessionConnectivityState.STALE
    : session.connectivity;
}

function connectivityStateName(state: SessionConnectivityState): string {
  switch (state) {
    case SessionConnectivityState.LIVE: return "live";
    case SessionConnectivityState.STALE: return "stale";
    case SessionConnectivityState.OFFLINE: return "offline";
    case SessionConnectivityState.UNKNOWN:
    case SessionConnectivityState.UNSPECIFIED: return "unknown";
    case SessionConnectivityState.FAILED: return "failed";
    default: throw new Error(`unsupported connectivity state ${state}`);
  }
}

function activityStateName(state: SessionActivityState): string {
  switch (state) {
    case SessionActivityState.IDLE: return "idle";
    case SessionActivityState.WORKING: return "working";
    case SessionActivityState.UNKNOWN:
    case SessionActivityState.UNSPECIFIED: return "unknown";
    default: throw new Error(`unsupported activity state ${state}`);
  }
}

function sessionLabel(session: SessionView): string {
  return session.label.name ?? session.label.project ?? session.identity.runtimeSessionId;
}

function sessionContext(session: SessionView): string {
  const metadata = [session.label.project, session.label.cwd].filter(Boolean).join(" · ");
  const updated = session.lastUpdate ? ` · updated ${session.lastUpdate.toLocaleString()}` : "";
  return `${metadata || "No label metadata"}${updated}`;
}

function searchableSession(session: SessionView): string {
  return `${formatSessionIdentity(session.identity)} ${session.label.name ?? ""} ${session.label.project ?? ""} ${session.label.cwd ?? ""}`.toLocaleLowerCase();
}

function compareSessions(left: SessionView, right: SessionView): number {
  if (left.needsYou !== right.needsYou) return left.needsYou ? -1 : 1;
  if (left.tombstoned !== right.tombstoned) return left.tombstoned ? 1 : -1;
  return sessionLabel(left).localeCompare(sessionLabel(right));
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
