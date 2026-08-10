import {
  FailureCode,
  OperationKind,
  OperationState,
} from "@patchbay/contracts";

import type { CommandView } from "../domain/model.js";
import { renderIcon, type IconName } from "./icons.js";

export interface OperationDeliveryActions {
  cancel?(command: CommandView): void | Promise<void>;
  interrupt?(command: CommandView): void | Promise<void>;
}

export function renderOperationDelivery(
  document: Document,
  command: CommandView,
  actions?: OperationDeliveryActions,
  lockdownActive = false,
): HTMLElement {
  const wrapper = document.createElement("div");
  wrapper.className = "delivery-line";
  wrapper.dataset.commandId = command.id;
  const current = operationStateName(command.state);
  const step = document.createElement("div");
  step.className = `command-step command-step--${current}`;
  step.append(textElement(document, "span", "command-step__marker", ""));
  step.append(textElement(document, "span", "command-step__state", current));
  wrapper.append(step);

  const last = command.history.at(-1);
  const previous = command.history.at(-2);
  if (last) {
    const transition = previous
      ? `Last transition: ${operationStateName(previous.state)} → ${operationStateName(last.state)}`
      : `Last transition: ${operationStateName(last.state)}`;
    wrapper.append(textElement(document, "span", "command-step__race", transition));
  }
  if (command.race) wrapper.append(textElement(document, "span", "command-step__race", command.race));

  if (command.failureCode !== undefined) {
    wrapper.append(renderFailureBanner(document, command.failureCode));
  }

  // Keep an action slot in every state. Controls appear only when the
  // canonical state and adapter-backed actions permit them, but the reserved
  // space keeps instruction cards from jumping as delivery advances.
  const actionSlot = document.createElement("div");
  actionSlot.className = "delivery-line__actions";
  const cancelAvailable = actions?.cancel && (
    command.state === OperationState.ACCEPTED
    || command.state === OperationState.DELIVERED
    || command.state === OperationState.RUNNING
  );
  const interruptAvailable = actions?.interrupt && command.state === OperationState.RUNNING;
  if (cancelAvailable || interruptAvailable) {
    actionSlot.classList.add("delivery-line__actions--available");
    actionSlot.setAttribute("role", "group");
    actionSlot.setAttribute("aria-label", "Operation actions");
    if (cancelAvailable) {
      actionSlot.append(contextButton(
        document,
        "x",
        `Cancel ${current} operation`,
        () => actions.cancel!(command),
        false,
        lockdownActive,
      ));
    }
    if (interruptAvailable) {
      actionSlot.append(contextButton(
        document,
        "square",
        "Interrupt running operation",
        () => actions.interrupt!(command),
        true,
        lockdownActive,
      ));
    }
  } else {
    // Terminal commands retain the same geometry without exposing a phantom
    // accessible control or implying that terminal state can still mutate.
    actionSlot.setAttribute("aria-hidden", "true");
  }
  wrapper.append(actionSlot);
  return wrapper;
}

export function operationStateName(state: OperationState): string {
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

export function operationKindLabel(kind: OperationKind): string {
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

export function renderFailureBanner(
  document: Document,
  failureCode: FailureCode,
  diagnostic?: string,
): HTMLElement {
  const term = failureCodeName(failureCode);
  return renderFailureText(document, term, diagnostic || failureMessage(failureCode));
}

export function renderFailureText(document: Document, term: string, message: string): HTMLElement {
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
  danger: boolean,
  lockdownActive: boolean,
): HTMLButtonElement {
  const button = document.createElement("button");
  button.className = `btn ${danger ? "btn-danger" : "btn-secondary"} btn--sm btn--icon-only`;
  button.type = "button";
  button.setAttribute("aria-label", label);
  button.title = lockdownActive
    ? "Disabled during lockdown: new Operations are rejected."
    : label;
  button.disabled = lockdownActive;
  button.append(renderIcon(document, icon));
  button.addEventListener("click", () => void action());
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
