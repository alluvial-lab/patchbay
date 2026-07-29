import {
  AdapterDiagnosticSeverity,
  FailureCode,
  LocalSubmissionState,
  OperationKind,
  OperationState,
  PayloadContentType,
  SessionActivityState,
  type SubmissionResult,
} from "@patchbay/contracts";

import {
  rendersLive,
  sessionKey,
  stableTarget,
  type CommandView,
  type ElicitationView,
  type ObservationView,
  type AdapterDiagnosticView,
  type PresentationModel,
  type SessionIdentity,
  type SessionView,
} from "../domain/model.js";
import {
  renderElicitation,
  renderElicitationGroup,
  type ElicitationRenderOptions,
} from "./elicitation.js";
import { renderIcon, type IconName } from "./icons.js";
import type { MarkdownRenderer } from "./markdown.js";
import { formatSessionIdentity, renderSessionStatus } from "./session-list.js";
import { diagnosticsForSession, renderAdapterStatus } from "../domain/adapter-diagnostics.js";

export interface SessionDetailActions {
  send?(session: SessionView, text: string): void | Promise<void>;
  attach?(session: SessionView): void | Promise<void>;
  cancel?(command: CommandView): void | Promise<void>;
  interrupt?(command: CommandView): void | Promise<void>;
}

export interface SubmissionFeedback {
  state: LocalSubmissionState;
  result?: SubmissionResult;
  error?: string;
}

export interface SessionDetailOptions {
  markdown: MarkdownRenderer;
  actions?: SessionDetailActions;
  elicitation?: ElicitationRenderOptions;
  submission?: SubmissionFeedback;
  onBack?(): void;
  lockdownActive?: boolean;
}

export interface SessionDetailComponent {
  readonly element: HTMLElement;
  readonly header: HTMLElement;
  readonly composer: HTMLElement;
  readonly sendButton: HTMLButtonElement;
  readonly input: HTMLTextAreaElement;
  setMobile(mobile: boolean): void;
}

export function renderSessionDetail(
  document: Document,
  model: PresentationModel,
  session: SessionView | undefined,
  options: SessionDetailOptions,
): SessionDetailComponent {
  const detail = document.createElement("section");
  detail.className = "session-detail";
  if (session) detail.dataset.sessionKey = sessionKey(session.identity);

  const header = renderHeader(document, model, session, options.onBack);
  const timeline = document.createElement("div");
  timeline.className = "timeline";
  timeline.setAttribute("aria-live", "polite");
  renderTimeline(document, timeline, model, session, options);
  const { composer, input, send } = renderComposer(
    document,
    session,
    options.actions,
    options.submission,
    options.lockdownActive,
  );
  detail.append(header, timeline, composer);

  return {
    element: detail,
    header,
    composer,
    sendButton: send,
    input,
    setMobile(mobile) {
      header.hidden = !mobile;
      detail.dataset.presentation = mobile ? "mobile-drill-in" : "desktop-two-pane";
    },
  };
}

function renderHeader(
  document: Document,
  model: PresentationModel,
  session: SessionView | undefined,
  onBack: (() => void) | undefined,
): HTMLElement {
  const header = document.createElement("header");
  header.className = "session-detail__header nav-bar";
  const back = iconButton(document, "arrow-left", "Back to sessions", "btn btn-ghost btn--sm");
  back.addEventListener("click", () => onBack?.());
  header.append(back);
  if (session) {
    header.append(textElement(document, "span", "session-row__identity", formatSessionIdentity(session.identity)));
    header.append(textElement(document, "span", "session-row__context", session.model ?? "Model unknown"));
    header.append(renderSessionStatus(document, session));
    const adapter = model.adapters.get(session.identity.adapterId);
    if (adapter) {
      header.append(renderAdapterStatus(document, adapter));
      const issues = diagnosticsForSession(adapter, session.identity).filter((diagnostic) => diagnostic.severity === AdapterDiagnosticSeverity.WARNING || diagnostic.severity === AdapterDiagnosticSeverity.ERROR);
      if (adapter.status) {
        header.append(textElement(
          document,
          "span",
          "adapter-issue-summary",
          issues.length > 0 ? "recent reported issue" : "no recent reported issues",
        ));
      } else {
        const unavailable = textElement(document, "span", "adapter-issue-summary alert alert--warning", "adapter diagnostics unavailable");
        unavailable.setAttribute("role", "status");
        header.append(unavailable);
      }
    }
  } else {
    header.append(textElement(document, "span", "nav-bar__brand", "Select a session"));
  }
  return header;
}

function renderTimeline(
  document: Document,
  timeline: HTMLElement,
  model: PresentationModel,
  session: SessionView | undefined,
  options: SessionDetailOptions,
): void {
  if (!session) {
    timeline.append(emptyState(document, "No session selected", "Choose a stable target from the session list."));
    return;
  }

  const observations = model.observations.filter((item) => sameIdentity(item.session, session.identity));
  const commands = [...model.commands.values()].filter((item) => sameIdentity(item.target, session.identity));
  const elicitations = [...model.elicitations.values()].filter((item) => sameIdentity(item.target, session.identity));
  const diagnostics = diagnosticsForSession(model.adapters.get(session.identity.adapterId), session.identity);
  const associatedCommands = new Set<string>();
  const entries: TimelineEntry[] = [];

  for (const diagnostic of diagnostics) {
    entries.push({ lsn: diagnostic.lsn, type: "diagnostic", diagnostic });
  }
  for (const observation of observations) {
    const command = observation.role === "operator"
      ? nearestCommand(observation, commands, associatedCommands)
      : undefined;
    if (command) associatedCommands.add(command.id);
    entries.push({ lsn: observation.lsn, type: "observation", observation, command });
  }
  for (const command of commands) {
    // Place a command by its accepted LSN, not its latest transition — the
    // advancing lsn would otherwise put it after the agent turn.
    if (!associatedCommands.has(command.id)) entries.push({ lsn: acceptedLsn(command), type: "command", command });
  }
  const groupedIds = new Set<string>();
  const batches = new Map<string, ElicitationView[]>();
  for (const elicitation of elicitations) {
    if (!elicitation.groupingKey || elicitation.kind !== "question") continue;
    const batch = batches.get(elicitation.groupingKey) ?? [];
    batch.push(elicitation);
    batches.set(elicitation.groupingKey, batch);
  }
  for (const batch of batches.values()) {
    if (batch.length < 2) continue;
    batch.sort((left, right) => left.lsn < right.lsn ? -1 : left.lsn > right.lsn ? 1 : 0);
    for (const elicitation of batch) groupedIds.add(elicitation.id);
    entries.push({ lsn: batch[0]!.lsn, type: "elicitation-group", elicitations: batch });
  }
  for (const elicitation of elicitations) {
    if (!groupedIds.has(elicitation.id)) {
      entries.push({ lsn: elicitation.lsn, type: "elicitation", elicitation });
    }
  }
  entries.sort((left, right) => left.lsn < right.lsn ? -1 : left.lsn > right.lsn ? 1 : 0);

  if (entries.length === 0) {
    timeline.append(emptyState(document, "No messages yet", "Send an instruction to start this session timeline."));
    if (rendersLive(session) && session.activity === SessionActivityState.WORKING) {
      timeline.append(renderTimelineActivity(document, session));
    }
    return;
  }

  for (const entry of entries) {
    if (entry.type === "diagnostic") {
      timeline.append(renderDiagnostic(document, entry.diagnostic));
    } else if (entry.type === "observation") {
      timeline.append(renderObservation(document, entry.observation, entry.command, options));
    } else if (entry.type === "command") {
      timeline.append(renderCommandMessage(document, entry.command, options.actions, options.lockdownActive));
    } else if (entry.type === "elicitation") {
      if (!options.elicitation) continue;
      const card = renderElicitation(document, entry.elicitation, {
        ...options.elicitation,
        lockdownActive: options.lockdownActive,
      });
      if (!stableTarget(session) || options.lockdownActive) disableSubmission(card, document, options.lockdownActive);
      timeline.append(card);
    } else {
      if (!options.elicitation) continue;
      const card = renderElicitationGroup(document, entry.elicitations, {
        ...options.elicitation,
        lockdownActive: options.lockdownActive,
      });
      if (!stableTarget(session) || options.lockdownActive) disableSubmission(card, document, options.lockdownActive);
      timeline.append(card);
    }
  }

  // In-chat activity affordance: the session list and header show state, but
  // the operator's eyes are at the end of the timeline during a turn (found
  // in live dogfooding: "nothing in the chatbox indicates the agent is
  // working"). Only for a live, working session.
  if (rendersLive(session) && session.activity === SessionActivityState.WORKING) {
    timeline.append(renderTimelineActivity(document, session));
  }
}

function renderTimelineActivity(document: Document, session: SessionView): HTMLElement {
  const row = document.createElement("div");
  row.className = "timeline-activity";
  row.setAttribute("role", "status");
  const indicator = document.createElement("span");
  indicator.className = "activity-indicator activity-indicator--working";
  indicator.append(textElement(document, "span", "activity-indicator__icon", ""));
  indicator.append(document.createTextNode(session.activityDetail ?? "working"));
  row.append(indicator);
  return row;
}

type TimelineEntry =
  | { lsn: bigint; type: "diagnostic"; diagnostic: AdapterDiagnosticView }
  | { lsn: bigint; type: "observation"; observation: ObservationView; command?: CommandView }
  | { lsn: bigint; type: "command"; command: CommandView }
  | { lsn: bigint; type: "elicitation"; elicitation: ElicitationView }
  | { lsn: bigint; type: "elicitation-group"; elicitations: ElicitationView[] };

function renderDiagnostic(document: Document, diagnostic: AdapterDiagnosticView): HTMLElement {
  const wrapper = document.createElement("article");
  wrapper.className = diagnostic.severity === AdapterDiagnosticSeverity.INFO ? "alert alert--info" : "failure-banner";
  wrapper.dataset.diagnosticId = diagnostic.sourceEventId;
  wrapper.setAttribute("role", diagnostic.severity === AdapterDiagnosticSeverity.INFO ? "status" : "alert");
  const term = diagnostic.failureCode !== undefined ? failureCodeName(diagnostic.failureCode) : "adapter_diagnostic";
  const detail = `Adapter ${diagnostic.adapterId} · generation ${diagnostic.adapterGeneration} · ${diagnostic.code} · count ${diagnostic.count}`;
  wrapper.append(textElement(document, "span", "failure-banner__term", term));
  wrapper.append(textElement(document, "p", "failure-banner__message", detail));
  if (diagnostic.observedAt) wrapper.append(textElement(document, "time", "msg__footer", diagnostic.observedAt.toISOString()));
  return wrapper;
}

function renderObservation(
  document: Document,
  observation: ObservationView,
  command: CommandView | undefined,
  options: SessionDetailOptions,
): HTMLElement {
  const message = document.createElement("article");
  message.className = `msg msg--${observation.role === "operator" ? "operator" : "agent"}`;
  message.dataset.observationId = observation.id;
  const body = document.createElement("div");
  body.className = "msg__body";
  if (observation.role === "agent" || observation.role === "tool") {
    body.innerHTML = options.markdown.render(observation.markdown);
  } else {
    body.textContent = observation.markdown;
  }
  message.append(body);
  if (observation.detail) {
    // Args/result preview lives INSIDE the message body — one card with a
    // divider, not a second floating box. Plain text, never markdown — tool
    // args are untrusted content.
    const detail = document.createElement("pre");
    detail.className = "msg__detail";
    detail.textContent = observation.detail;
    body.append(detail);
  }
  message.append(textElement(document, "div", "msg__footer", `${observation.role} · ${observation.kind}`));
  if (command) message.append(renderDelivery(document, command, options.actions, options.lockdownActive));
  return message;
}

function renderCommandMessage(
  document: Document,
  command: CommandView,
  actions: SessionDetailActions | undefined,
  lockdownActive = false,
): HTMLElement {
  const message = document.createElement("article");
  message.className = "msg msg--operator msg--action";
  message.dataset.commandId = command.id;
  const bodyText = operationText(command) || operationKindName(command.operation.kind);
  if (bodyText) message.append(textElement(document, "div", "msg__body", bodyText));
  message.append(renderDelivery(document, command, actions, lockdownActive));
  return message;
}

function renderDelivery(
  document: Document,
  command: CommandView,
  actions: SessionDetailActions | undefined,
  lockdownActive = false,
): HTMLElement {
  const wrapper = document.createElement("div");
  wrapper.className = "delivery-line";
  wrapper.dataset.commandId = command.id;
  const current = commandStateName(command.state);
  const step = document.createElement("div");
  step.className = `command-step command-step--${current}`;
  step.append(textElement(document, "span", "command-step__marker", ""));
  step.append(textElement(document, "span", "command-step__state", current));
  wrapper.append(step);

  const last = command.history.at(-1);
  const previous = command.history.at(-2);
  if (last) {
    const transition = previous
      ? `Last transition: ${commandStateName(previous.state)} → ${commandStateName(last.state)}`
      : `Last transition: ${commandStateName(last.state)}`;
    wrapper.append(textElement(document, "span", "command-step__race", transition));
  }
  if (command.race) wrapper.append(textElement(document, "span", "command-step__race", command.race));

  // Full history + LSN disclosure is a reserved post-v0.1.0 seam.
  if (command.failureCode !== undefined) {
    wrapper.append(renderFailureBanner(document, command.failureCode));
  }

  if (command.state === OperationState.RUNNING && (actions?.cancel || actions?.interrupt)) {
    const contextual = document.createElement("div");
    contextual.className = "btn-group";
    if (actions.cancel) contextual.append(contextButton(document, "x", "Cancel running operation", () => actions.cancel!(command), false, lockdownActive));
    if (actions.interrupt) contextual.append(contextButton(document, "square", "Interrupt running operation", () => actions.interrupt!(command), true, lockdownActive));
    wrapper.append(contextual);
  }
  return wrapper;
}

function renderComposer(
  document: Document,
  session: SessionView | undefined,
  actions: SessionDetailActions | undefined,
  submission: SubmissionFeedback | undefined,
  lockdownActive = false,
): { composer: HTMLElement; input: HTMLTextAreaElement; send: HTMLButtonElement } {
  const composer = document.createElement("form");
  composer.className = "composer";
  const targetStable = stableTarget(session) && !lockdownActive;

  const attach = iconButton(document, "paperclip", "Attach file or image", "btn btn-secondary");
  attach.disabled = !targetStable || !actions?.attach;
  attach.addEventListener("click", () => {
    if (session && stableTarget(session)) void actions?.attach?.(session);
  });

  const input = document.createElement("textarea");
  input.className = "composer__input";
  input.rows = 1;
  input.placeholder = targetStable ? "Instruct or enter a slash-command…" : "Select a stable target before sending";
  input.disabled = !targetStable;
  input.setAttribute("aria-label", "Instruction");

  const send = iconButton(document, "arrow-up", "Send instruction", "btn btn-primary btn--sm", "submit");
  send.disabled = true;
  function updateSend(): void {
    send.disabled = !targetStable || !input.value.trim() || !actions?.send;
  }
  input.addEventListener("input", updateSend);
  // Enter sends; Shift+Enter inserts a newline (standard chat-composer behavior).
  input.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      composer.requestSubmit();
    }
  });
  composer.addEventListener("submit", (event) => {
    event.preventDefault();
    if (!session || !stableTarget(session) || send.disabled) return;
    const text = input.value;
    void actions?.send?.(session, text);
  });
  composer.append(attach, input, send);
  if (session) {
    composer.append(
      textElement(
        document,
        "span",
        "composer__idempotency session-row__identity",
        `Target: ${formatSessionIdentity(session.identity)}`,
      ),
    );
  }
  if (submission) renderSubmissionFeedback(document, composer, submission);
  return { composer, input, send };
}

function renderSubmissionFeedback(
  document: Document,
  composer: HTMLElement,
  submission: SubmissionFeedback,
): void {
  if (submission.state === LocalSubmissionState.SUBMITTING) {
    composer.append(
      textElement(document, "span", "composer__idempotency", "Submitting durable Operation…"),
    );
  } else if (submission.state === LocalSubmissionState.SUBMIT_FAILED) {
    composer.append(renderFailureText(document, "submit_failed", submission.error ?? "Submission failed before an outcome was confirmed."));
  } else if (submission.state === LocalSubmissionState.UNKNOWN) {
    composer.append(renderFailureText(document, "unknown", submission.error ?? "Acceptance is unknown; reconcile before claiming success or retrying."));
  }

  const result = submission.result;
  if (!result) return;
  if (result.failureCode !== FailureCode.UNSPECIFIED) {
    composer.append(renderFailureBanner(document, result.failureCode, result.diagnosticMessage));
  }
  if (result.deduplicated) {
    const indicator = textElement(
      document,
      "span",
      "retry-safety-indicator retry-safety-indicator--safe",
      "Already in flight — existing command returned; no duplicate submitted",
    );
    indicator.setAttribute("role", "status");
    composer.append(indicator);
  }
}

function renderFailureBanner(
  document: Document,
  failureCode: FailureCode,
  diagnostic?: string,
): HTMLElement {
  const term = failureCodeName(failureCode);
  return renderFailureText(document, term, diagnostic || failureMessage(failureCode));
}

function renderFailureText(document: Document, term: string, message: string): HTMLElement {
  const banner = document.createElement("div");
  banner.className = "failure-banner";
  banner.setAttribute("role", "alert");
  banner.append(textElement(document, "span", "failure-banner__term", term));
  banner.append(textElement(document, "p", "failure-banner__message", message));
  return banner;
}

export function failureCodeName(code: FailureCode): string {
  switch (code) {
    case FailureCode.VALIDATION_FAILED: return "validation_failed";
    case FailureCode.AUTHORIZATION_DENIED: return "authorization_denied";
    case FailureCode.TARGET_NOT_FOUND: return "target_not_found";
    case FailureCode.UNSUPPORTED_COMMAND: return "unsupported_command";
    case FailureCode.TARGET_OFFLINE: return "target_offline";
    case FailureCode.ADAPTER_UNAVAILABLE: return "adapter_unavailable";
    case FailureCode.TRANSPORT_TIMEOUT: return "transport_timeout";
    case FailureCode.DELIVERY_REJECTED: return "delivery_rejected";
    case FailureCode.EXECUTION_FAILED: return "execution_failed";
    case FailureCode.EXPIRED: return "expired";
    case FailureCode.CANCELLED: return "cancelled";
    case FailureCode.SUPERSEDED: return "superseded";
    case FailureCode.STALE_EVENT: return "stale_event";
    case FailureCode.EXECUTION_OUTCOME_UNKNOWN: return "execution_outcome_unknown";
    case FailureCode.UNSPECIFIED:
    default: throw new Error(`unsupported failure code ${code}`);
  }
}

function failureMessage(code: FailureCode): string {
  switch (code) {
    case FailureCode.VALIDATION_FAILED: return "The Operation failed boundary validation before acceptance.";
    case FailureCode.AUTHORIZATION_DENIED: return "The verified operator endpoint lacks authority for this target.";
    case FailureCode.TARGET_NOT_FOUND: return "The addressed target identity could not be resolved.";
    case FailureCode.UNSUPPORTED_COMMAND: return "The target adapter does not support this Operation kind.";
    case FailureCode.TARGET_OFFLINE: return "The target is authoritatively offline.";
    case FailureCode.ADAPTER_UNAVAILABLE: return "The adapter required for delivery is unavailable.";
    case FailureCode.TRANSPORT_TIMEOUT: return "The transport timed out; acceptance or execution must not be inferred.";
    case FailureCode.DELIVERY_REJECTED: return "The adapter received the Operation but refused delivery responsibility.";
    case FailureCode.EXECUTION_FAILED: return "Execution began or was accepted, then the target reported failure.";
    case FailureCode.EXPIRED: return "The Operation validity window expired.";
    case FailureCode.CANCELLED: return "Cancellation became the authoritative terminal outcome.";
    case FailureCode.SUPERSEDED: return "A newer Operation or policy superseded this one.";
    case FailureCode.STALE_EVENT: return "A late event was retained for audit and did not rewrite live state.";
    case FailureCode.EXECUTION_OUTCOME_UNKNOWN: return "Execution may have occurred; evaluate adapter idempotency before retrying.";
    case FailureCode.UNSPECIFIED:
    default: throw new Error(`unsupported failure code ${code}`);
  }
}

function contextButton(
  document: Document,
  icon: IconName,
  label: string,
  action: () => void | Promise<void>,
  danger = false,
  lockdownActive = false,
): HTMLButtonElement {
  const button = iconButton(document, icon, label, `btn ${danger ? "btn-danger" : "btn-secondary"} btn--sm`);
  button.disabled = lockdownActive;
  if (lockdownActive) button.title = "Disabled during lockdown: new Operations are rejected.";
  button.addEventListener("click", () => void action());
  return button;
}

function iconButton(
  document: Document,
  icon: IconName,
  label: string,
  className: string,
  type: "button" | "submit" = "button",
): HTMLButtonElement {
  const button = document.createElement("button");
  button.className = `${className} btn--icon-only`;
  button.type = type;
  button.setAttribute("aria-label", label);
  button.title = label;
  button.append(renderIcon(document, icon));
  return button;
}

function disableSubmission(card: HTMLElement, document: Document, lockdownActive = false): void {
  for (const control of card.querySelectorAll<HTMLInputElement | HTMLTextAreaElement | HTMLButtonElement>(
    "input, textarea, button",
  )) control.disabled = true;
  card.append(
    textElement(
      document,
      "p",
      "field__error",
      lockdownActive
        ? "Read-only during lockdown. Responses are rejected until trusted bootstrap exit."
        : "Target identity is stale or superseded; reconcile before responding.",
    ),
  );
}

function nearestCommand(
  observation: ObservationView,
  commands: readonly CommandView[],
  used: ReadonlySet<string>,
): CommandView | undefined {
  // Match on the command's accepted LSN: command.lsn advances with every
  // transition (delivered/running/completed), so at final fold it would sit
  // after the observation and never associate — duplicating the instruct as
  // a separate card after the agent turn.
  return commands
    .filter((command) => !used.has(command.id) && acceptedLsn(command) <= observation.lsn)
    .sort((left, right) => acceptedLsn(left) > acceptedLsn(right) ? -1 : acceptedLsn(left) < acceptedLsn(right) ? 1 : 0)[0];
}

function acceptedLsn(command: CommandView): bigint {
  return command.history.at(0)?.lsn ?? command.lsn;
}

function sameIdentity(left: SessionIdentity | undefined, right: SessionIdentity): boolean {
  return Boolean(left && sessionKey(left) === sessionKey(right));
}

function operationText(command: CommandView): string {
  const envelope = command.operation.payload;
  if (!envelope || envelope.contentType !== PayloadContentType.TEXT_UTF8) return "";
  return new TextDecoder().decode(envelope.payload);
}

function operationKindName(kind: OperationKind): string {
  switch (kind) {
    case OperationKind.SPAWN: return "Spawn";
    case OperationKind.ATTACH: return "Attach";
    case OperationKind.INSTRUCT: return "Instruction";
    case OperationKind.CANCEL: return "Cancel";
    case OperationKind.INTERRUPT: return "Interrupt";
    case OperationKind.QUERY: return "Query";
    case OperationKind.APPROVAL_RESPONSE: return "Approval response";
    case OperationKind.ELICITATION_RESPONSE: return "Elicitation response";
    case OperationKind.RECONFIGURE: return "Reconfigure";
    case OperationKind.SESSION_MANAGEMENT: return "Session management";
    case OperationKind.UNSPECIFIED:
    case OperationKind.RESERVED_AGENT_SEND:
    case OperationKind.RESERVED_ADAPTER_UTILITY_EXEC:
    default: throw new Error(`unsupported visible operation kind ${kind}`);
  }
}

export function commandStateName(state: OperationState): string {
  switch (state) {
    case OperationState.ACCEPTED: return "accepted";
    case OperationState.DELIVERED: return "delivered";
    case OperationState.RUNNING: return "running";
    case OperationState.COMPLETED: return "completed";
    case OperationState.REJECTED: return "rejected";
    case OperationState.FAILED: return "failed";
    case OperationState.EXPIRED: return "expired";
    case OperationState.CANCELLED: return "cancelled";
    case OperationState.SUPERSEDED: return "superseded";
    case OperationState.UNSPECIFIED:
    default: throw new Error(`unsupported operation state ${state}`);
  }
}

function emptyState(document: Document, title: string, body: string): HTMLElement {
  const empty = document.createElement("div");
  empty.className = "empty-state";
  empty.append(textElement(document, "p", "empty-state__title", title));
  empty.append(textElement(document, "p", "empty-state__body", body));
  return empty;
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
