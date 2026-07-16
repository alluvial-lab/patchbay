import {
  createAgentSession,
  type AgentSession,
  type AgentSessionEvent,
  type CreateAgentSessionOptions,
} from "@earendil-works/pi-coding-agent";
import { TranscriptEventLog } from "./transcript_event_log.js";
import { deterministicTranscriptEventId, projectAgentEvent } from "./transcript_projection.js";
import type { TranscriptEvent } from "./transcript_event.js";
import { initialTurnSnapshot, reduceTurn, type TurnSnapshot } from "./turn_state.js";

type ThinkingLevel = AgentSession["thinkingLevel"];
type SessionEntry = ReturnType<AgentSession["sessionManager"]["getEntries"]>[number];

export interface PiSessionOptions {
  cwd: string;
  runtimeSessionId?: string;
  name?: string;
  model?: string;
  generation?: number;
  sessionOptions?: Omit<CreateAgentSessionOptions, "cwd" | "model">;
}

export interface ApprovalRequest {
  toolCallId: string;
  tool: string;
  args: unknown;
}

export type ApprovalHandler = (request: ApprovalRequest) => Promise<boolean> | boolean;

export interface PiSessionState {
  sessionId: string;
  generation: number;
  streaming: boolean;
  idle: boolean;
  model?: { provider: string; id: string; name: string };
  thinkingLevel: ThinkingLevel;
  turn: TurnSnapshot;
}

export interface PiModel {
  provider: string;
  id: string;
  name: string;
}

/** Direct in-process host for one Pi AgentSession. */
export class PiSession {
  readonly #session: AgentSession;
  readonly #runtimeSessionId: string;
  readonly #transcriptLog = new TranscriptEventLog();
  readonly #listeners = new Set<(event: TranscriptEvent) => void>();
  #generation: number;
  #turn = initialTurnSnapshot();
  #turnSequence = 0;
  #promptSequence = 0;
  #unsubscribe: (() => void) | undefined;
  #approvalHandler: ApprovalHandler = () => true;

  static async create(options: PiSessionOptions): Promise<PiSession> {
    const createOptions: CreateAgentSessionOptions = {
      ...options.sessionOptions,
      cwd: options.cwd,
    };
    if (options.model) {
      const registry = options.sessionOptions?.modelRegistry;
      const model = registry ? findModel(registry, options.model) : undefined;
      if (!model) throw new Error(`Pi model is unavailable: ${options.model}`);
      createOptions.model = model;
    }
    const { session } = await createAgentSession(createOptions);
    if (options.name) session.setSessionName(options.name);
    return new PiSession(
      session,
      options.runtimeSessionId ?? options.name ?? session.sessionId,
      options.generation ?? 1,
    );
  }

  private constructor(session: AgentSession, runtimeSessionId: string, generation: number) {
    if (!runtimeSessionId) throw new Error("runtimeSessionId must not be empty");
    if (!Number.isSafeInteger(generation) || generation < 1) {
      throw new Error("session generation must be a positive safe integer");
    }
    this.#session = session;
    this.#runtimeSessionId = runtimeSessionId;
    this.#generation = generation;

    // AgentSession 0.80 exposes the typed hook on its public Agent. This is the
    // same hook AgentSession installs for extensions, but Patchbay owns it
    // directly because the adapter is the Pi host rather than an extension.
    this.#session.agent.beforeToolCall = async ({ toolCall, args }) => {
      const request: ApprovalRequest = {
        toolCallId: toolCall.id,
        tool: toolCall.name,
        args,
      };
      this.#append({
        kind: "tool_requested",
        eventId: deterministicTranscriptEventId(
          this.#transcriptSessionId(),
          "tool_requested",
          toolCall.id,
        ),
        sessionId: this.#transcriptSessionId(),
        ts: Date.now(),
        toolCallId: toolCall.id,
        tool: toolCall.name,
        args: asRecord(args),
      });
      const approved = await this.#approvalHandler(request);
      return approved ? undefined : { block: true, reason: "Blocked by Patchbay approval policy" };
    };

    this.#unsubscribe = this.#session.subscribe((event) => this.#handleEvent(event));
  }

  get runtimeSessionId(): string {
    return this.#runtimeSessionId;
  }

  get generation(): number {
    return this.#generation;
  }

  setApprovalHandler(handler: ApprovalHandler): void {
    this.#approvalHandler = handler;
  }

  onTranscript(listener: (event: TranscriptEvent) => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  transcriptEvents(): readonly TranscriptEvent[] {
    return this.#transcriptLog.forSession(this.#transcriptSessionId());
  }

  prompt(text: string): Promise<void> {
    this.#promptSequence += 1;
    const messageId = `prompt-${this.#promptSequence}`;
    this.#append({
      kind: "user_confirmed",
      eventId: deterministicTranscriptEventId(
        this.#transcriptSessionId(),
        "user_confirmed",
        messageId,
      ),
      sessionId: this.#transcriptSessionId(),
      ts: Date.now(),
      messageId,
      text,
    });
    return this.#session.sendUserMessage(text);
  }

  cancel(): Promise<void> {
    return this.#session.abort();
  }

  getState(): PiSessionState {
    const model = this.#session.model;
    return {
      sessionId: this.#runtimeSessionId,
      generation: this.#generation,
      streaming: this.#session.isStreaming,
      idle: this.#session.isIdle,
      ...(model
        ? { model: { provider: model.provider, id: model.id, name: model.name } }
        : {}),
      thinkingLevel: this.#session.thinkingLevel,
      turn: this.#turn,
    };
  }

  getEntries(since?: string): { entries: SessionEntry[]; leafId: string | null } {
    const entries = this.#session.sessionManager.getEntries();
    if (!since) return { entries, leafId: this.#session.sessionManager.getLeafId() };
    const cursor = entries.findIndex((entry) => entry.id === since);
    return {
      entries: cursor < 0 ? entries : entries.slice(cursor + 1),
      leafId: this.#session.sessionManager.getLeafId(),
    };
  }

  async setModel(provider: string, modelId: string): Promise<void> {
    const model = this.#session.modelRegistry.find(provider, modelId);
    if (!model) throw new Error(`Pi model is unavailable: ${provider}/${modelId}`);
    await this.#session.setModel(model);
  }

  async setThinkingLevel(level: ThinkingLevel): Promise<void> {
    this.#session.setThinkingLevel(level);
  }

  getAvailableModels(): PiModel[] {
    return this.#session.modelRegistry.getAll().map((model) => ({
      provider: model.provider,
      id: model.id,
      name: model.name,
    }));
  }

  async newSession(): Promise<number> {
    await this.#session.abort();
    this.#session.sessionManager.newSession();
    this.#session.agent.reset();
    this.#generation += 1;
    this.#turn = initialTurnSnapshot();
    return this.#generation;
  }

  async compact(instructions?: string): Promise<void> {
    await this.#session.compact(instructions);
  }

  dispose(): void {
    this.#unsubscribe?.();
    this.#unsubscribe = undefined;
    this.#session.dispose();
  }

  #handleEvent(event: AgentSessionEvent): void {
    if (event.type === "turn_start") this.#turnSequence += 1;
    this.#turn = reduceTurn(this.#turn, event, `turn-${this.#turnSequence}`);
    const transcriptEvent = projectAgentEvent(event, this.#transcriptSessionId());
    if (transcriptEvent) this.#append(transcriptEvent);
  }

  #append(event: TranscriptEvent): void {
    if (!this.#transcriptLog.append(event)) return;
    for (const listener of this.#listeners) listener(event);
  }

  #transcriptSessionId(): string {
    return `${this.#runtimeSessionId}:${this.#generation}`;
  }
}

function findModel(
  registry: AgentSession["modelRegistry"],
  requested: string,
): ReturnType<AgentSession["modelRegistry"]["find"]> {
  const slash = requested.indexOf("/");
  if (slash >= 1) return registry.find(requested.slice(0, slash), requested.slice(slash + 1));
  return registry.getAll().find((model) => model.id === requested);
}

function asRecord(value: unknown): Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value)
    ? (value as Record<string, unknown>)
    : {};
}
