export type TranscriptEvent =
  | BaseEvent<"turn_started"> & { turnId: string }
  | BaseEvent<"turn_finished"> & { turnId: string }
  | BaseEvent<"user_confirmed"> & { messageId: string; text: string }
  | BaseEvent<"assistant_delta"> & { messageId: string; delta: string }
  | BaseEvent<"assistant_committed"> & { messageId: string; text: string }
  | BaseEvent<"tool_requested"> & {
      toolCallId: string;
      tool: string;
      args: Record<string, unknown>;
    }
  | BaseEvent<"tool_finished"> & {
      toolCallId: string;
      tool: string;
      result?: unknown;
      error?: string;
    }
  | BaseEvent<"compaction_recorded"> & {
      summary: string;
      tokensBefore?: number;
    }
  | BaseEvent<"provider_error"> & { message: string };

type BaseEvent<K extends string> = {
  kind: K;
  eventId: string;
  sessionId: string;
  ts: number;
};
