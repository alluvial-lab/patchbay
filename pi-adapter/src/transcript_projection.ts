import type { AgentSessionEvent } from "@earendil-works/pi-coding-agent";
import type { TranscriptEvent } from "./transcript_event.js";

/** Map one typed Pi event to one canonical partial-snapshot fact. */
export function projectAgentEvent(
  event: AgentSessionEvent,
  sessionId: string,
): TranscriptEvent | null {
  const now = Date.now();
  switch (event.type) {
    case "entry_appended": {
      const entry = asRecord(event.entry);
      if (entry["type"] !== "message") return null;
      const message = asRecord(entry["message"]);
      if (message["role"] !== "user") return null;
      const messageId = stringValue(entry["id"]) ?? `user:${messageTimestamp(message, now)}`;
      const ts = messageTimestamp(message, now);
      return {
        kind: "user_confirmed",
        eventId: deterministicTranscriptEventId(sessionId, "user_confirmed", messageId),
        sessionId,
        ts,
        messageId,
        text: stringifyContent(message["content"]),
      };
    }
    case "message_update": {
      const message = asRecord(event.message);
      if (message["role"] !== "assistant") return null;
      const ts = messageTimestamp(message, now);
      const messageId = assistantMessageId(message, ts);
      const update = asRecord(event.assistantMessageEvent);
      const delta = stringValue(update["delta"]);
      if (!delta) return null;
      return {
        kind: "assistant_delta",
        eventId: deterministicTranscriptEventId(
          sessionId,
          "assistant_delta",
          `${messageId}:${stringValue(update["type"]) ?? "delta"}:${stringValue(update["contentIndex"]) ?? stringifyContent(message["content"]).length}`,
        ),
        sessionId,
        ts,
        messageId,
        delta,
      };
    }
    case "message_end": {
      const message = asRecord(event.message);
      if (message["role"] !== "assistant") return null;
      const ts = messageTimestamp(message, now);
      const messageId = assistantMessageId(message, ts);
      const text = stringifyContent(message["content"]);
      const error = stringValue(message["errorMessage"]);
      if (error) {
        return {
          kind: "provider_error",
          eventId: deterministicTranscriptEventId(sessionId, "provider_error", messageId),
          sessionId,
          ts,
          message: error,
        };
      }
      if (!text) return null;
      return {
        kind: "assistant_committed",
        eventId: deterministicTranscriptEventId(sessionId, "assistant_committed", messageId),
        sessionId,
        ts,
        messageId,
        text,
      };
    }
    case "tool_execution_start":
      return {
        kind: "tool_requested",
        eventId: deterministicTranscriptEventId(sessionId, "tool_requested", event.toolCallId),
        sessionId,
        ts: now,
        toolCallId: event.toolCallId,
        tool: event.toolName,
        args: asRecord(event.args),
      };
    case "tool_execution_end":
      return event.isError
        ? {
            kind: "tool_finished",
            eventId: deterministicTranscriptEventId(sessionId, "tool_finished", event.toolCallId),
            sessionId,
            ts: now,
            toolCallId: event.toolCallId,
            tool: event.toolName,
            error: stringifyUnknown(event.result),
          }
        : {
            kind: "tool_finished",
            eventId: deterministicTranscriptEventId(sessionId, "tool_finished", event.toolCallId),
            sessionId,
            ts: now,
            toolCallId: event.toolCallId,
            tool: event.toolName,
            result: event.result,
          };
    case "compaction_end": {
      if (!event.result || event.aborted) return null;
      const result = asRecord(event.result);
      const summary = stringValue(result["summary"]);
      if (!summary) return null;
      return {
        kind: "compaction_recorded",
        eventId: deterministicTranscriptEventId(sessionId, "compaction_recorded", summary),
        sessionId,
        ts: now,
        summary,
        ...(typeof result["tokensBefore"] === "number" ? { tokensBefore: result["tokensBefore"] } : {}),
      };
    }
    default:
      return null;
  }
}

export function deterministicTranscriptEventId(
  sessionId: string,
  kind: TranscriptEvent["kind"],
  stableKey: string,
): string {
  return `pi:${sessionId}:${kind}:${stableKey}`;
}

export function stringifyContent(content: unknown): string {
  if (typeof content === "string") return content;
  if (!Array.isArray(content)) return "";
  return content
    .map((block) => {
      const value = asRecord(block);
      if (value["type"] === "text") return stringValue(value["text"]) ?? "";
      if (value["type"] === "thinking") return stringValue(value["thinking"]) ?? "";
      return "";
    })
    .join("");
}

function assistantMessageId(message: Record<string, unknown>, ts: number): string {
  return stringValue(message["responseId"]) ?? `assistant:${ts}`;
}

function messageTimestamp(message: Record<string, unknown>, fallback: number): number {
  return typeof message["timestamp"] === "number" ? message["timestamp"] : fallback;
}

function asRecord(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}

function stringValue(value: unknown): string | undefined {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

function stringifyUnknown(value: unknown): string {
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}
