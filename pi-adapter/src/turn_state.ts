import type { AgentSessionEvent } from "@earendil-works/pi-coding-agent";

export type TurnPhase = "idle" | "working" | "awaiting_tool" | "streaming" | "error";

export interface TurnSnapshot {
  phase: TurnPhase;
  turnId: string | null;
  activeToolCallId: string | null;
  error: string | null;
}

export function initialTurnSnapshot(): TurnSnapshot {
  return { phase: "idle", turnId: null, activeToolCallId: null, error: null };
}

/** Harvested typed-event reducer; consumers do not re-infer working state. */
export function reduceTurn(
  snapshot: TurnSnapshot,
  event: AgentSessionEvent,
  fallbackTurnId: string,
): TurnSnapshot {
  switch (event.type) {
    case "agent_start":
    case "turn_start":
      return { phase: "working", turnId: snapshot.turnId ?? fallbackTurnId, activeToolCallId: null, error: null };
    case "message_update":
      return { ...snapshot, phase: "streaming" };
    case "tool_execution_start":
      return { ...snapshot, phase: "awaiting_tool", activeToolCallId: event.toolCallId };
    case "tool_execution_end":
      return snapshot.activeToolCallId === event.toolCallId
        ? { ...snapshot, phase: "working", activeToolCallId: null }
        : snapshot;
    case "agent_end":
    case "agent_settled":
    case "turn_end":
      return initialTurnSnapshot();
    case "auto_retry_start":
      return { ...snapshot, phase: "error", error: event.errorMessage };
    case "auto_retry_end":
      return event.success ? { ...snapshot, phase: "working", error: null } : snapshot;
    default:
      return snapshot;
  }
}
