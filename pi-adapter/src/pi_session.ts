import { OperationKind, type Operation } from "@patchbay/contracts";
import {
  createAgentSession,
  createAgentSessionRuntime,
  createAgentSessionServices,
  getAgentDir,
  SessionManager,
  type AgentSession,
  type AgentSessionEvent,
  type AgentSessionRuntime,
  type CreateAgentSessionOptions,
  type CreateAgentSessionRuntimeFactory,
} from "@earendil-works/pi-coding-agent";
import { TranscriptEventLog } from "./transcript_event_log.js";
import {
  deterministicTranscriptEventId,
  projectAgentEvent,
  projectSessionEntries,
} from "./transcript_projection.js";
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
  piSessionId: string;
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

interface SessionBinding {
  readonly session: AgentSession;
  readonly generation: number;
  active: boolean;
  unsubscribe: (() => void) | undefined;
}

interface PendingApproval {
  resolve: (approved: boolean) => void;
}

/** Direct in-process host for one replaceable Pi AgentSession runtime. */
export class PiSession {
  readonly #runtime: AgentSessionRuntime;
  readonly #runtimeSessionId: string;
  readonly #transcriptLog = new TranscriptEventLog();
  readonly #listeners = new Set<(event: TranscriptEvent) => void>();
  #generation: number;
  #turn = initialTurnSnapshot();
  #turnSequence = 0;
  #promptSequence = 0;
  #approvalHandler: ApprovalHandler = () => true;
  #pendingApproval: PendingApproval | undefined;
  #binding: SessionBinding | undefined;
  #pendingGeneration: number | undefined;
  #disposed = false;

  static async create(options: PiSessionOptions): Promise<PiSession> {
    const fixed = options.sessionOptions ?? {};
    const agentDir = fixed.agentDir ?? getAgentDir();
    const createRuntime: CreateAgentSessionRuntimeFactory = async ({
      cwd,
      agentDir: runtimeAgentDir,
      sessionManager,
      sessionStartEvent,
    }) => {
      const services = await createAgentSessionServices({
        cwd,
        agentDir: runtimeAgentDir,
        ...(fixed.authStorage ? { authStorage: fixed.authStorage } : {}),
        ...(fixed.settingsManager ? { settingsManager: fixed.settingsManager } : {}),
        ...(fixed.modelRegistry ? { modelRegistry: fixed.modelRegistry } : {}),
      });
      const model = options.model ? findModel(services.modelRegistry, options.model) : undefined;
      if (options.model && !model) throw new Error(`Pi model is unavailable: ${options.model}`);
      const created = await createAgentSession({
        ...fixed,
        cwd,
        agentDir: runtimeAgentDir,
        authStorage: fixed.authStorage ?? services.authStorage,
        settingsManager: fixed.settingsManager ?? services.settingsManager,
        modelRegistry: fixed.modelRegistry ?? services.modelRegistry,
        resourceLoader: fixed.resourceLoader ?? services.resourceLoader,
        sessionManager,
        ...(sessionStartEvent ? { sessionStartEvent } : {}),
        ...(model ? { model } : {}),
      });
      if (options.name) created.session.setSessionName(options.name);
      return {
        ...created,
        services: {
          ...services,
          settingsManager: created.session.settingsManager,
          modelRegistry: created.session.modelRegistry,
          resourceLoader: created.session.resourceLoader,
        },
        diagnostics: services.diagnostics,
      };
    };
    const sessionManager = fixed.sessionManager ?? SessionManager.create(options.cwd);
    const runtime = await createAgentSessionRuntime(createRuntime, {
      cwd: options.cwd,
      agentDir,
      sessionManager,
    });
    return new PiSession(
      runtime,
      options.runtimeSessionId ?? options.name ?? runtime.session.sessionId,
      options.generation ?? 1,
    );
  }

  private constructor(runtime: AgentSessionRuntime, runtimeSessionId: string, generation: number) {
    if (!runtimeSessionId) throw new Error("runtimeSessionId must not be empty");
    if (!Number.isSafeInteger(generation) || generation < 1) {
      throw new Error("session generation must be a positive safe integer");
    }
    this.#runtime = runtime;
    this.#runtimeSessionId = runtimeSessionId;
    this.#generation = generation;
    this.#bind(runtime.session, generation);
    runtime.setBeforeSessionInvalidate(() => this.#invalidateBinding());
    runtime.setRebindSession(async (session) => {
      const replacementGeneration = this.#pendingGeneration;
      if (replacementGeneration === undefined) {
        throw new Error("Pi runtime replaced a session without a pending generation bump");
      }
      this.#generation = replacementGeneration;
      this.#turn = initialTurnSnapshot();
      this.#turnSequence = 0;
      this.#promptSequence = 0;
      this.#bind(session, replacementGeneration);
    });
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

  resolveApproval(operation: Operation, approved: boolean): void {
    if (operation.kind !== OperationKind.APPROVAL_RESPONSE) {
      throw new Error(`cannot resolve approval from OperationKind ${operation.kind}`);
    }
    const pending = this.#pendingApproval;
    if (!pending) throw new Error("Pi session has no pending approval gate");
    pending.resolve(approved);
  }

  onTranscript(listener: (event: TranscriptEvent) => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  transcriptEvents(): readonly TranscriptEvent[] {
    return this.#transcriptLog.forSession(this.#transcriptSessionId(this.#generation));
  }

  /** Rebuild the partial transcript snapshot from Pi's current persisted entries. */
  snapshotTranscript(): readonly TranscriptEvent[] {
    const sessionId = this.#transcriptSessionId(this.#generation);
    this.#transcriptLog.appendAll(
      projectSessionEntries(this.#runtime.session.sessionManager.getEntries(), sessionId),
    );
    return this.#transcriptLog.forSession(sessionId);
  }

  prompt(text: string): Promise<void> {
    const session = this.#runtime.session;
    const generation = this.#generation;
    this.#promptSequence += 1;
    const messageId = `prompt-${this.#promptSequence}`;
    this.#append({
      kind: "user_confirmed",
      eventId: deterministicTranscriptEventId(
        this.#transcriptSessionId(generation),
        "user_confirmed",
        messageId,
      ),
      sessionId: this.#transcriptSessionId(generation),
      ts: Date.now(),
      messageId,
      text,
    });
    return session.sendUserMessage(text);
  }

  cancel(): Promise<void> {
    return this.#runtime.session.abort();
  }

  getState(): PiSessionState {
    const session = this.#runtime.session;
    const model = session.model;
    return {
      sessionId: this.#runtimeSessionId,
      piSessionId: session.sessionId,
      generation: this.#generation,
      streaming: session.isStreaming,
      idle: session.isIdle,
      ...(model
        ? { model: { provider: model.provider, id: model.id, name: model.name } }
        : {}),
      thinkingLevel: session.thinkingLevel,
      turn: this.#turn,
    };
  }

  getEntries(since?: string): { entries: SessionEntry[]; leafId: string | null } {
    const manager = this.#runtime.session.sessionManager;
    const entries = manager.getEntries();
    if (!since) return { entries, leafId: manager.getLeafId() };
    const cursor = entries.findIndex((entry) => entry.id === since);
    return {
      entries: cursor < 0 ? entries : entries.slice(cursor + 1),
      leafId: manager.getLeafId(),
    };
  }

  async setModel(provider: string, modelId: string): Promise<void> {
    const session = this.#runtime.session;
    const model = session.modelRegistry.find(provider, modelId);
    if (!model) throw new Error(`Pi model is unavailable: ${provider}/${modelId}`);
    await session.setModel(model);
  }

  async setThinkingLevel(level: ThinkingLevel): Promise<void> {
    this.#runtime.session.setThinkingLevel(level);
  }

  getAvailableModels(): PiModel[] {
    return this.#runtime.session.modelRegistry.getAll().map((model) => ({
      provider: model.provider,
      id: model.id,
      name: model.name,
    }));
  }

  async newSession(): Promise<number> {
    if (this.#pendingGeneration !== undefined) {
      throw new Error("Pi session replacement is already in progress");
    }
    const nextGeneration = this.#generation + 1;
    this.#pendingGeneration = nextGeneration;
    try {
      const result = await this.#runtime.newSession();
      if (result.cancelled) throw new Error("Pi session replacement was cancelled");
      if (this.#generation !== nextGeneration) {
        throw new Error("Pi session replacement did not bind the new generation");
      }
      return this.#generation;
    } finally {
      this.#pendingGeneration = undefined;
    }
  }

  async compact(instructions?: string): Promise<void> {
    await this.#runtime.session.compact(instructions);
  }

  async dispose(): Promise<void> {
    if (this.#disposed) return;
    this.#disposed = true;
    this.#invalidateBinding();
    this.#listeners.clear();
    await this.#runtime.dispose();
  }

  #bind(session: AgentSession, generation: number): void {
    const binding: SessionBinding = {
      session,
      generation,
      active: true,
      unsubscribe: undefined,
    };
    this.#binding = binding;
    session.agent.beforeToolCall = async ({ toolCall, args }) => {
      if (!this.#isLive(binding)) {
        return { block: true, reason: "Stale Pi session context" };
      }
      const request: ApprovalRequest = {
        toolCallId: toolCall.id,
        tool: toolCall.name,
        args,
      };
      this.#append({
        kind: "tool_requested",
        eventId: deterministicTranscriptEventId(
          this.#transcriptSessionId(generation),
          "tool_requested",
          toolCall.id,
        ),
        sessionId: this.#transcriptSessionId(generation),
        ts: Date.now(),
        toolCallId: toolCall.id,
        tool: toolCall.name,
        args: asRecord(args),
      });
      const approved = await this.#awaitApproval(request);
      return approved ? undefined : { block: true, reason: "Blocked by Patchbay approval policy" };
    };
    binding.unsubscribe = session.subscribe((event) => {
      if (this.#isLive(binding)) this.#handleEvent(event, generation);
    });
  }

  async #awaitApproval(request: ApprovalRequest): Promise<boolean> {
    if (this.#pendingApproval) {
      throw new Error("Pi session already has a pending approval gate");
    }

    let resolveDelivered!: (approved: boolean) => void;
    const delivered = new Promise<boolean>((resolve) => {
      resolveDelivered = resolve;
    });
    const pending: PendingApproval = { resolve: resolveDelivered };
    this.#pendingApproval = pending;
    try {
      return await Promise.race([Promise.resolve(this.#approvalHandler(request)), delivered]);
    } finally {
      if (this.#pendingApproval === pending) this.#pendingApproval = undefined;
    }
  }

  #invalidateBinding(): void {
    const binding = this.#binding;
    if (!binding) return;
    binding.active = false;
    binding.unsubscribe?.();
    binding.unsubscribe = undefined;
    this.#pendingApproval?.resolve(false);
    this.#pendingApproval = undefined;
    if (this.#binding === binding) this.#binding = undefined;
  }

  #isLive(binding: SessionBinding): boolean {
    return (
      !this.#disposed &&
      binding.active &&
      this.#binding === binding &&
      binding.generation === this.#generation &&
      this.#runtime.session === binding.session
    );
  }

  #handleEvent(event: AgentSessionEvent, generation: number): void {
    if (event.type === "turn_start") this.#turnSequence += 1;
    this.#turn = reduceTurn(this.#turn, event, `turn-${this.#turnSequence}`);
    const transcriptEvent = projectAgentEvent(event, this.#transcriptSessionId(generation));
    if (transcriptEvent) this.#append(transcriptEvent);
  }

  #append(event: TranscriptEvent): void {
    if (!this.#transcriptLog.append(event)) return;
    for (const listener of this.#listeners) listener(event);
  }

  #transcriptSessionId(generation: number): string {
    return `${this.#runtimeSessionId}:${generation}`;
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
