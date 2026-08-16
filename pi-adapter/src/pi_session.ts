import { createHash } from "node:crypto";
import { OperationKind, type Operation } from "@patchbay/contracts";
import {
  createAgentSession,
  type AgentSession,
  type AgentSessionEvent,
  type ModelRuntime,
  type ResourceLoader,
  type SessionManager,
  type SettingsManager,
} from "@earendil-works/pi-coding-agent";
import {
  deterministicTranscriptEventId,
  projectAgentEvent,
  projectSessionEntries,
} from "./transcript_projection.js";
import type { ManagedPiRuntimePort, PiRpcRuntime, ProcessExit } from "./pi_process.js";
import type { PiRpcEvent, PiRpcTransportError } from "./rpc_client.js";
import {
  RuntimeActionGate,
  type RuntimeActionKind,
  type RuntimeReplacementLease,
} from "./runtime_action_gate.js";
import type { TranscriptEvent } from "./transcript_event.js";
import { initialTurnSnapshot, reduceTurn, type TurnSnapshot } from "./turn_state.js";

type ThinkingLevel = AgentSession["thinkingLevel"];
type SessionEntry = ReturnType<AgentSession["sessionManager"]["getEntries"]>[number];

export interface PiSessionState {
  readonly sessionId: string;
  readonly piSessionId: string;
  readonly generation: number;
  readonly streaming: boolean;
  readonly compacting: boolean;
  readonly pendingMessageCount: number;
  readonly idle: boolean;
  readonly model?: { readonly provider: string; readonly id: string; readonly name: string };
  readonly thinkingLevel: ThinkingLevel;
  readonly turn: TurnSnapshot;
  readonly sessionFile: string;
}

export interface PiModel {
  readonly provider: string;
  readonly id: string;
  readonly name: string;
}

export interface PiRuntimeActivitySnapshot {
  readonly streaming: boolean;
  readonly compacting: boolean;
  readonly pendingMessageCount: number;
  readonly activityEpoch: number;
  readonly settledEpoch: number;
}

export type ApprovalHandler = (request: {
  readonly toolCallId: string;
  readonly tool: string;
  readonly args: unknown;
}) => Promise<boolean> | boolean;

export type SessionLifecycleEvent =
  | { readonly kind: "process_exit"; readonly exit: ProcessExit }
  | { readonly kind: "transport_loss"; readonly error: PiRpcTransportError };

export interface PiSession {
  readonly runtimeSessionId: string;
  readonly generation: number;
  readonly actionGate: RuntimeActionGate;
  readonly processToken?: string;
  getState(): PiSessionState;
  refreshState(lease?: RuntimeReplacementLease): Promise<PiSessionState>;
  activitySnapshot(): PiRuntimeActivitySnapshot;
  waitForSettled(afterActivityEpoch: number, timeoutMs: number): Promise<void>;
  prompt(text: string): Promise<void>;
  cancel(): Promise<void>;
  getEntries(since?: string, lease?: RuntimeReplacementLease): Promise<{ entries: SessionEntry[]; leafId: string | null }>;
  snapshotTranscript(): Promise<readonly TranscriptEvent[]>;
  setModel(provider: string, modelId: string): Promise<void>;
  setThinkingLevel(level: ThinkingLevel): Promise<void>;
  getAvailableModels(): Promise<PiModel[]>;
  compact(instructions?: string): Promise<void>;
  setApprovalHandler(handler: ApprovalHandler): void;
  resolveApproval(operation: Operation, approved: boolean): void;
  onTranscript(listener: (event: TranscriptEvent) => void): () => void;
  onModelChange(listener: (model: string) => void): () => void;
  onLifecycle(listener: (event: SessionLifecycleEvent) => void): () => void;
  publishStagedTranscript(): readonly TranscriptEvent[];
  stagedReadinessDigest(): string;
  dispose(): Promise<void>;
}

export interface RpcPiSessionOptions {
  readonly runtimeSessionId?: string;
  readonly generation: number;
  readonly runtime: PiRpcRuntime;
  readonly runtimePort: ManagedPiRuntimePort;
  readonly actionGate: RuntimeActionGate;
  readonly publication: "current" | "claimed_successor";
}

/** Production Pi session: a process-token-fenced view over one RPC child. */
export class RpcPiSession implements PiSession {
  readonly runtimeSessionId: string;
  readonly generation: number;
  readonly actionGate: RuntimeActionGate;
  readonly processToken: string;
  readonly #runtime: PiRpcRuntime;
  readonly #runtimePort: ManagedPiRuntimePort;
  readonly #listeners = new Set<(event: TranscriptEvent) => void>();
  readonly #modelListeners = new Set<(model: string) => void>();
  readonly #lifecycleListeners = new Set<(event: SessionLifecycleEvent) => void>();
  readonly #seenTranscriptEventIds = new Set<string>();
  readonly #stagedTranscript: TranscriptEvent[] = [];
  readonly #deltaOrdinals = new Map<string, number>();
  readonly #settledWaiters = new Set<{
    readonly after: number;
    readonly resolve: () => void;
    readonly reject: (error: Error) => void;
    readonly timer: NodeJS.Timeout;
  }>();
  #publication: "current" | "claimed_successor";
  #turn = initialTurnSnapshot();
  #turnSequence = 0;
  #activityEpoch = 0;
  #settledEpoch = 0;
  #disposed = false;
  #terminalLifecycle: SessionLifecycleEvent | undefined;
  #state: PiSessionState;
  #unsubscribeEvent: () => void;
  #unsubscribeFailure: () => void;

  private constructor(
    options: RpcPiSessionOptions & { readonly runtimeSessionId: string },
    initialState: PiSessionState,
  ) {
    this.runtimeSessionId = options.runtimeSessionId;
    this.generation = options.generation;
    this.actionGate = options.actionGate;
    this.processToken = options.runtime.processToken;
    this.#runtime = options.runtime;
    this.#runtimePort = options.runtimePort;
    this.#publication = options.publication;
    this.#state = initialState;
    const processToken = options.runtime.processToken;
    this.#unsubscribeEvent = options.runtime.rpc.onEvent((event) => {
      if (this.#isCurrentProcess(processToken)) this.#handleRpcEvent(event);
    });
    this.#unsubscribeFailure = options.runtime.onTransportFailure((error) => {
      if (!this.#isCurrentProcess(processToken)) return;
      const event: SessionLifecycleEvent = { kind: "transport_loss", error };
      this.#terminalLifecycle = event;
      for (const listener of this.#lifecycleListeners) listener(event);
    });
    void options.runtime.exit.then((exit) => {
      if (!this.#isCurrentProcess(processToken)) return;
      const event: SessionLifecycleEvent = { kind: "process_exit", exit };
      this.#terminalLifecycle = event;
      for (const listener of this.#lifecycleListeners) listener(event);
    });
  }

  static async bind(options: RpcPiSessionOptions, lease?: RuntimeReplacementLease): Promise<RpcPiSession> {
    if (options.runtimeSessionId !== undefined) {
      requireRuntimeIdentity(options.runtimeSessionId, options.generation);
    } else if (!Number.isSafeInteger(options.generation) || options.generation < 1) {
      throw new Error("runtime generation is invalid");
    }
    const raw = await rpcRequest<Record<string, unknown>>(
      options.runtime,
      { type: "get_state" },
      lease,
      options.actionGate,
      "query",
    );
    const piSessionId = boundedString(raw["sessionId"], "sessionId");
    const runtimeSessionId = options.runtimeSessionId ?? piSessionId;
    return new RpcPiSession(
      { ...options, runtimeSessionId } as RpcPiSessionOptions & { readonly runtimeSessionId: string },
      parseRpcState(raw, runtimeSessionId, options.generation, initialTurnSnapshot()),
    );
  }

  getState(): PiSessionState {
    return this.#state;
  }

  async refreshState(lease?: RuntimeReplacementLease): Promise<PiSessionState> {
    const raw = await this.#request<Record<string, unknown>>({ type: "get_state" }, "query", lease);
    this.#state = parseRpcState(raw, this.runtimeSessionId, this.generation, this.#turn);
    return this.#state;
  }

  activitySnapshot(): PiRuntimeActivitySnapshot {
    return Object.freeze({
      streaming: this.#state.streaming,
      compacting: this.#state.compacting,
      pendingMessageCount: this.#state.pendingMessageCount,
      activityEpoch: this.#activityEpoch,
      settledEpoch: this.#settledEpoch,
    });
  }

  waitForSettled(afterActivityEpoch: number, timeoutMs: number): Promise<void> {
    if (!Number.isSafeInteger(afterActivityEpoch) || afterActivityEpoch < 0) {
      return Promise.reject(new Error("settlement activity epoch is invalid"));
    }
    if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > 120_000) {
      return Promise.reject(new Error("settlement timeout is invalid"));
    }
    if (this.#settledEpoch >= afterActivityEpoch && !this.#state.streaming && !this.#state.compacting) {
      return Promise.resolve();
    }
    return new Promise<void>((resolve, reject) => {
      const waiter = {
        after: afterActivityEpoch,
        resolve: () => {
          clearTimeout(waiter.timer);
          this.#settledWaiters.delete(waiter);
          resolve();
        },
        reject: (error: Error) => {
          clearTimeout(waiter.timer);
          this.#settledWaiters.delete(waiter);
          reject(error);
        },
        timer: setTimeout(() => undefined, timeoutMs),
      };
      clearTimeout(waiter.timer);
      waiter.timer = setTimeout(
        () => waiter.reject(new Error("Pi runtime did not settle within the configured bound")),
        timeoutMs,
      );
      this.#settledWaiters.add(waiter);
    });
  }

  async prompt(text: string): Promise<void> {
    if (!text) throw new Error("Pi prompt is empty");
    const before = this.#activityEpoch;
    await this.#request({ type: "prompt", message: text }, "delivery");
    const state = await this.refreshState();
    if (state.streaming || this.#activityEpoch > before) {
      await this.waitForSettled(Math.max(before + 1, this.#activityEpoch), 120_000);
      await this.refreshState();
    }
  }

  async cancel(): Promise<void> {
    await this.#request({ type: "abort" }, "delivery");
  }

  async getEntries(
    since?: string,
    lease?: RuntimeReplacementLease,
  ): Promise<{ entries: SessionEntry[]; leafId: string | null }> {
    const data = await this.#request<Record<string, unknown>>(
      { type: "get_entries", ...(since ? { since } : {}) },
      "query",
      lease,
    );
    if (!Array.isArray(data["entries"]) || !(data["leafId"] === null || typeof data["leafId"] === "string")) {
      throw new Error("Pi get_entries response is malformed");
    }
    return { entries: data["entries"] as SessionEntry[], leafId: data["leafId"] };
  }

  async snapshotTranscript(): Promise<readonly TranscriptEvent[]> {
    const entries = await this.getEntries();
    const projected = projectSessionEntries(entries.entries, this.#transcriptSessionId());
    for (const event of projected) this.#seenTranscriptEventIds.add(event.eventId);
    return projected;
  }

  async setModel(provider: string, modelId: string): Promise<void> {
    const data = await this.#request<Record<string, unknown>>(
      { type: "set_model", provider, modelId },
      "reconfigure",
    );
    const model = parseModel(data);
    this.#state = Object.freeze({ ...this.#state, model });
    this.#emitModel(`${model.provider}/${model.id}`);
  }

  async setThinkingLevel(level: ThinkingLevel): Promise<void> {
    await this.#request({ type: "set_thinking_level", level }, "reconfigure");
    this.#state = Object.freeze({ ...this.#state, thinkingLevel: level });
  }

  async getAvailableModels(): Promise<PiModel[]> {
    const data = await this.#request<Record<string, unknown>>({ type: "get_available_models" }, "query");
    if (!Array.isArray(data["models"])) throw new Error("Pi model response is malformed");
    return data["models"].map((model) => parseModel(asRecord(model)));
  }

  async compact(instructions?: string): Promise<void> {
    await this.#request(
      { type: "compact", ...(instructions ? { customInstructions: instructions } : {}) },
      "compaction",
    );
  }

  setApprovalHandler(_handler: ApprovalHandler): void {
    throw new Error("RPC extension UI approval handling is not implemented by this checkpoint");
  }

  resolveApproval(operation: Operation, _approved: boolean): void {
    if (operation.kind !== OperationKind.APPROVAL_RESPONSE) {
      throw new Error("approval response OperationKind is invalid");
    }
    throw new Error("RPC extension UI approval handling is not implemented by this checkpoint");
  }

  onTranscript(listener: (event: TranscriptEvent) => void): () => void {
    this.#listeners.add(listener);
    return () => this.#listeners.delete(listener);
  }

  onModelChange(listener: (model: string) => void): () => void {
    this.#modelListeners.add(listener);
    return () => this.#modelListeners.delete(listener);
  }

  onLifecycle(listener: (event: SessionLifecycleEvent) => void): () => void {
    this.#lifecycleListeners.add(listener);
    const terminal = this.#terminalLifecycle;
    if (terminal) {
      queueMicrotask(() => {
        if (this.#lifecycleListeners.has(listener)) listener(terminal);
      });
    }
    return () => this.#lifecycleListeners.delete(listener);
  }

  publishStagedTranscript(): readonly TranscriptEvent[] {
    if (this.#publication === "current") return [];
    this.#publication = "current";
    const staged = this.#stagedTranscript.splice(0);
    for (const event of staged) for (const listener of this.#listeners) listener(event);
    return staged;
  }

  stagedReadinessDigest(): string {
    const hash = createHash("sha256");
    for (const event of this.#stagedTranscript) hash.update(JSON.stringify(event));
    return hash.digest("hex");
  }

  /** Supervisor-only request path while its exact replacement lease owns stdin. */
  requestUnderLease<T>(
    command: Record<string, unknown> & { readonly type: string },
    lease: RuntimeReplacementLease,
  ): Promise<T> {
    return this.#request<T>(command, "query", lease);
  }

  async dispose(): Promise<void> {
    if (this.#disposed) return;
    this.#disposed = true;
    this.#unsubscribeEvent();
    this.#unsubscribeFailure();
    for (const waiter of this.#settledWaiters) waiter.reject(new Error("Pi session was disposed"));
    this.#settledWaiters.clear();
    this.#listeners.clear();
    this.#modelListeners.clear();
    this.#lifecycleListeners.clear();
    await this.#runtimePort.terminate(this.#runtime).catch(() => this.#runtime.exit);
    this.#runtime.rpc.close();
  }

  runtimeForSupervisor(lease: RuntimeReplacementLease): PiRpcRuntime {
    lease.assertCurrent();
    return this.#runtime;
  }

  async #request<T = unknown>(
    command: Record<string, unknown> & { readonly type: string },
    kind: RuntimeActionKind,
    lease?: RuntimeReplacementLease,
  ): Promise<T> {
    if (this.#disposed) throw new Error("Pi session is disposed");
    return rpcRequest(this.#runtime, command, lease, this.actionGate, kind);
  }

  #handleRpcEvent(event: PiRpcEvent): void {
    if (event.type === "agent_start" || event.type === "compaction_start" || event.type === "auto_retry_start") {
      this.#activityEpoch += 1;
      this.#state = Object.freeze({
        ...this.#state,
        streaming: event.type === "agent_start" ? true : this.#state.streaming,
        compacting: event.type === "compaction_start" ? true : this.#state.compacting,
        idle: false,
      });
    }
    if (event.type === "agent_settled") {
      this.#settledEpoch = this.#activityEpoch;
      this.#state = Object.freeze({
        ...this.#state,
        streaming: false,
        compacting: false,
        pendingMessageCount: 0,
        idle: true,
      });
      for (const waiter of [...this.#settledWaiters]) {
        if (this.#settledEpoch >= waiter.after) waiter.resolve();
      }
    }
    if (event.type === "turn_start") this.#turnSequence += 1;
    const agentEvent = event as unknown as AgentSessionEvent;
    try {
      this.#turn = reduceTurn(this.#turn, agentEvent, `turn-${this.#turnSequence}`);
      this.#state = Object.freeze({ ...this.#state, turn: this.#turn });
      const transcript = projectAgentEvent(agentEvent, this.#transcriptSessionId(), this.#deltaOrdinals);
      if (transcript) this.#append(transcript);
    } catch {
      // Unknown Pi event variants remain event-stream caveats; they are not
      // allowed to corrupt state or bypass strict persisted reconciliation.
    }
    if (event.type === "entry_appended" && isRecord(event["entry"])) {
      const entry = event["entry"];
      if (entry["type"] === "model_change" && typeof entry["provider"] === "string" && typeof entry["modelId"] === "string") {
        this.#emitModel(`${entry["provider"]}/${entry["modelId"]}`);
      }
    }
  }

  #append(event: TranscriptEvent): void {
    if (this.#seenTranscriptEventIds.has(event.eventId)) return;
    this.#seenTranscriptEventIds.add(event.eventId);
    if (this.#publication === "claimed_successor") {
      this.#stagedTranscript.push(event);
      return;
    }
    for (const listener of this.#listeners) listener(event);
  }

  #emitModel(model: string): void {
    for (const listener of this.#modelListeners) listener(model);
  }

  #isCurrentProcess(processToken: string): boolean {
    return !this.#disposed && this.#runtime.processToken === processToken;
  }

  #transcriptSessionId(): string {
    return `${this.runtimeSessionId}:${this.generation}`;
  }
}

async function rpcRequest<T>(
  runtime: PiRpcRuntime,
  command: Record<string, unknown> & { readonly type: string },
  lease: RuntimeReplacementLease | undefined,
  gate: RuntimeActionGate,
  kind: RuntimeActionKind,
): Promise<T> {
  if (lease) {
    lease.assertCurrent();
    if (lease.gate !== gate) throw new Error("replacement lease belongs to another runtime gate");
    return runtime.rpc.request<T>(command);
  }
  return gate.runAction(kind, () => runtime.rpc.request<T>(command));
}

function parseRpcState(
  value: Record<string, unknown>,
  runtimeSessionId: string,
  generation: number,
  turn: TurnSnapshot,
): PiSessionState {
  const sessionId = boundedString(value["sessionId"], "sessionId");
  const sessionFile = boundedString(value["sessionFile"], "sessionFile");
  const thinking = value["thinkingLevel"];
  if (!isThinkingLevel(thinking)) throw new Error("Pi get_state thinking level is malformed");
  const streaming = booleanField(value, "isStreaming");
  const compacting = booleanField(value, "isCompacting");
  const pendingMessageCount = nonnegativeInteger(value["pendingMessageCount"]);
  const modelValue = value["model"];
  const model = modelValue === null || modelValue === undefined ? undefined : parseModel(asRecord(modelValue));
  return Object.freeze({
    sessionId: runtimeSessionId,
    piSessionId: sessionId,
    generation,
    streaming,
    compacting,
    pendingMessageCount,
    idle: !streaming && !compacting && pendingMessageCount === 0,
    ...(model ? { model } : {}),
    thinkingLevel: thinking,
    turn,
    sessionFile,
  });
}

function parseModel(value: Record<string, unknown>): PiModel {
  return Object.freeze({
    provider: boundedString(value["provider"], "model provider"),
    id: boundedString(value["id"], "model id"),
    name: typeof value["name"] === "string" && value["name"].length > 0
      ? value["name"]
      : boundedString(value["id"], "model id"),
  });
}

function requireRuntimeIdentity(runtimeSessionId: string, generation: number): void {
  if (!runtimeSessionId || runtimeSessionId.length > 1_024) throw new Error("runtime session id is invalid");
  if (!Number.isSafeInteger(generation) || generation < 1) throw new Error("runtime generation is invalid");
}

function boundedString(value: unknown, field: string): string {
  if (typeof value !== "string" || value.length === 0 || value.length > 4_096 || value.includes("\0")) {
    throw new Error(`Pi RPC ${field} is malformed`);
  }
  return value;
}

function booleanField(value: Record<string, unknown>, field: string): boolean {
  if (typeof value[field] !== "boolean") throw new Error(`Pi RPC ${field} is malformed`);
  return value[field];
}

function nonnegativeInteger(value: unknown): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) throw new Error("Pi pendingMessageCount is malformed");
  return value as number;
}

function isThinkingLevel(value: unknown): value is ThinkingLevel {
  return typeof value === "string" && ["off", "minimal", "low", "medium", "high", "xhigh", "max"].includes(value);
}

function asRecord(value: unknown): Record<string, unknown> {
  if (!isRecord(value)) throw new Error("Pi RPC record is malformed");
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

export interface AgentSessionRuntimeFixtureServices {
  readonly modelRuntime: ModelRuntime;
  readonly resourceLoader: ResourceLoader;
  readonly sessionManager: SessionManager;
  readonly settingsManager: SettingsManager;
  /** Explicit marker proving tests supplied catalog/auth stubs rather than ambient discovery. */
  readonly modelCatalogAuthStub: { readonly kind: "offline-injected" };
}

export interface AgentSessionRuntimeFixtureOptions {
  readonly cwd: string;
  readonly runtimeSessionId: string;
  readonly generation: number;
  readonly model: string;
  readonly services: AgentSessionRuntimeFixtureServices;
  readonly name?: string;
  readonly tools?: readonly string[];
  readonly noTools?: "all" | "builtin";
}

/** Test-only SDK substitute for the same PiSession runtime port. */
export class AgentSessionRuntimeFixture implements PiSession {
  readonly runtimeSessionId: string;
  readonly generation: number;
  readonly actionGate = new RuntimeActionGate();
  readonly #session: AgentSession;
  readonly #listeners = new Set<(event: TranscriptEvent) => void>();
  readonly #modelListeners = new Set<(model: string) => void>();
  readonly #seen = new Set<string>();
  readonly #deltaOrdinals = new Map<string, number>();
  #turn = initialTurnSnapshot();
  #turnSequence = 0;
  #approvalHandler: ApprovalHandler = () => true;
  #pendingApproval: ((approved: boolean) => void) | undefined;
  #unsubscribe: () => void;

  private constructor(options: AgentSessionRuntimeFixtureOptions, session: AgentSession) {
    this.runtimeSessionId = options.runtimeSessionId;
    this.generation = options.generation;
    this.#session = session;
    this.#unsubscribe = session.subscribe((event) => this.#handleEvent(event));
    session.agent.beforeToolCall = async ({ toolCall, args }) => {
      const approved = await new Promise<boolean>((resolve) => {
        this.#pendingApproval = resolve;
        Promise.resolve(this.#approvalHandler({ toolCallId: toolCall.id, tool: toolCall.name, args }))
          .then(resolve, () => resolve(false));
      });
      this.#pendingApproval = undefined;
      return approved ? undefined : { block: true, reason: "Blocked by Patchbay approval policy" };
    };
  }

  static async create(options: AgentSessionRuntimeFixtureOptions): Promise<AgentSessionRuntimeFixture> {
    requireRuntimeIdentity(options.runtimeSessionId, options.generation);
    if (options.services.modelCatalogAuthStub.kind !== "offline-injected") {
      throw new Error("AgentSessionRuntimeFixture requires offline catalog/auth stubs");
    }
    const slash = options.model.indexOf("/");
    if (slash < 1) throw new Error("fixture model must be provider/model");
    const model = options.services.modelRuntime.getModel(
      options.model.slice(0, slash),
      options.model.slice(slash + 1),
    );
    if (!model) throw new Error("fixture model is absent from the injected catalog");
    const result = await createAgentSession({
      cwd: options.cwd,
      model,
      modelRuntime: options.services.modelRuntime,
      resourceLoader: options.services.resourceLoader,
      sessionManager: options.services.sessionManager,
      settingsManager: options.services.settingsManager,
      ...(options.tools ? { tools: [...options.tools] } : {}),
      ...(options.noTools ? { noTools: options.noTools } : {}),
    });
    if (options.name) result.session.setSessionName(options.name);
    return new AgentSessionRuntimeFixture(options, result.session);
  }

  getState(): PiSessionState {
    const model = this.#session.model;
    return Object.freeze({
      sessionId: this.runtimeSessionId,
      piSessionId: this.#session.sessionId,
      generation: this.generation,
      streaming: this.#session.isStreaming,
      compacting: this.#session.isCompacting,
      pendingMessageCount: this.#session.pendingMessageCount,
      idle: this.#session.isIdle,
      ...(model ? { model: { provider: model.provider, id: model.id, name: model.name } } : {}),
      thinkingLevel: this.#session.thinkingLevel,
      turn: this.#turn,
      sessionFile: this.#session.sessionFile ?? "memory-only",
    });
  }
  async refreshState(): Promise<PiSessionState> { return this.getState(); }
  activitySnapshot(): PiRuntimeActivitySnapshot {
    const state = this.getState();
    return { streaming: state.streaming, compacting: state.compacting, pendingMessageCount: state.pendingMessageCount, activityEpoch: 0, settledEpoch: state.idle ? 0 : -1 };
  }
  async waitForSettled(_after: number, timeoutMs: number): Promise<void> {
    await Promise.race([
      this.#session.agent.waitForIdle(),
      new Promise<never>((_resolve, reject) => setTimeout(() => reject(new Error("fixture settle timeout")), timeoutMs)),
    ]);
  }
  async prompt(text: string): Promise<void> {
    await this.actionGate.runAction("delivery", () => this.#session.prompt(text));
    this.#publishSnapshotDelta();
  }
  cancel(): Promise<void> { return this.actionGate.runAction("delivery", () => this.#session.abort()); }
  async getEntries(since?: string): Promise<{ entries: SessionEntry[]; leafId: string | null }> {
    const entries = this.#session.sessionManager.getEntries();
    const index = since ? entries.findIndex((entry) => entry.id === since) : -1;
    return { entries: since && index >= 0 ? entries.slice(index + 1) : entries, leafId: this.#session.sessionManager.getLeafId() };
  }
  async snapshotTranscript(): Promise<readonly TranscriptEvent[]> {
    const projected = projectSessionEntries(this.#session.sessionManager.getEntries(), `${this.runtimeSessionId}:${this.generation}`);
    for (const event of projected) this.#seen.add(event.eventId);
    return projected;
  }
  async setModel(provider: string, modelId: string): Promise<void> {
    const model = this.#session.modelRuntime.getModel(provider, modelId);
    if (!model) throw new Error("fixture model is unavailable");
    await this.actionGate.runAction("reconfigure", () => this.#session.setModel(model));
    for (const listener of this.#modelListeners) listener(`${provider}/${modelId}`);
  }
  async setThinkingLevel(level: ThinkingLevel): Promise<void> {
    await this.actionGate.runAction("reconfigure", async () => {
      this.#session.setThinkingLevel(level);
    });
  }
  async getAvailableModels(): Promise<PiModel[]> { return this.#session.modelRuntime.getModels().map(({ provider, id, name }) => ({ provider, id, name })); }
  async compact(instructions?: string): Promise<void> { await this.actionGate.runAction("compaction", () => this.#session.compact(instructions)); }
  setApprovalHandler(handler: ApprovalHandler): void { this.#approvalHandler = handler; }
  resolveApproval(operation: Operation, approved: boolean): void {
    if (operation.kind !== OperationKind.APPROVAL_RESPONSE || !this.#pendingApproval) throw new Error("fixture has no matching approval");
    this.#pendingApproval(approved);
  }
  onTranscript(listener: (event: TranscriptEvent) => void): () => void { this.#listeners.add(listener); return () => this.#listeners.delete(listener); }
  onModelChange(listener: (model: string) => void): () => void { this.#modelListeners.add(listener); return () => this.#modelListeners.delete(listener); }
  onLifecycle(_listener: (event: SessionLifecycleEvent) => void): () => void { return () => undefined; }
  publishStagedTranscript(): readonly TranscriptEvent[] { return []; }
  stagedReadinessDigest(): string { return createHash("sha256").digest("hex"); }
  async dispose(): Promise<void> { this.#unsubscribe(); this.#pendingApproval?.(false); this.#session.dispose(); }
  #publishSnapshotDelta(): void {
    const projected = projectSessionEntries(
      this.#session.sessionManager.getEntries(),
      `${this.runtimeSessionId}:${this.generation}`,
    );
    for (const event of projected) {
      if (this.#seen.has(event.eventId)) continue;
      this.#seen.add(event.eventId);
      for (const listener of this.#listeners) listener(event);
    }
  }
  #handleEvent(event: AgentSessionEvent): void {
    if (event.type === "turn_start") this.#turnSequence += 1;
    this.#turn = reduceTurn(this.#turn, event, `turn-${this.#turnSequence}`);
    const projected = projectAgentEvent(event, `${this.runtimeSessionId}:${this.generation}`, this.#deltaOrdinals);
    if (projected && !this.#seen.has(projected.eventId)) {
      this.#seen.add(projected.eventId);
      for (const listener of this.#listeners) listener(projected);
    }
    if (event.type === "entry_appended" && event.entry.type === "model_change") {
      for (const listener of this.#modelListeners) listener(`${event.entry.provider}/${event.entry.modelId}`);
    }
  }
}
