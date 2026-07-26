import { mkdir, open, rename, stat, type FileHandle } from "node:fs/promises";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { homedir } from "node:os";
import {
  FailureCode,
  OperationKind,
  SessionActivityState,
  SessionConnectivityState,
} from "@patchbay/contracts";

export const ADAPTER_DIAGNOSTIC_EVENTS = [
  "adapter.starting",
  "adapter.started",
  "adapter.stopping",
  "adapter.stopped",
  "adapter.attach.started",
  "adapter.attach.succeeded",
  "adapter.attach.failed",
  "session.register.started",
  "session.register.succeeded",
  "session.register.failed",
  "session.activity.reported",
  "session.model.changed",
  "session.generation.changed",
  "session.dispose.started",
  "session.dispose.succeeded",
  "session.dispose.failed",
  "delivery.subscription.failed",
  "delivery.subscription.retrying",
  "delivery.received",
  "delivery.acknowledged",
  "delivery.running",
  "delivery.completed",
  "delivery.rejected",
  "delivery.failed",
  "observation.failed",
  "observation.flush_failed",
  "log.records_dropped",
] as const;

export type AdapterDiagnosticEvent = (typeof ADAPTER_DIAGNOSTIC_EVENTS)[number];
export type AdapterDiagnosticLevel = "info" | "warn" | "error";

export interface AdapterDiagnosticSessionRef {
  runtimeSessionId: string;
  deploymentScope: string;
  generation: number;
}

export interface AdapterDiagnosticError {
  name: string;
  code?: string;
}

export interface AdapterDiagnosticInput {
  event: AdapterDiagnosticEvent;
  level: AdapterDiagnosticLevel;
  session?: AdapterDiagnosticSessionRef;
  commandId?: string;
  operationKind?: OperationKind;
  failureCode?: FailureCode;
  outcome?: string;
  observationKind?: "transcript" | "session-report";
  sessionActivity?: SessionActivityState;
  sessionConnectivity?: SessionConnectivityState;
  fromGeneration?: number;
  toGeneration?: number;
  reason?: string;
  error?: AdapterDiagnosticError;
  count?: number;
}

export interface AdapterDiagnostics {
  record(input: AdapterDiagnosticInput): void;
  flush(): Promise<void>;
  close(): Promise<void>;
}

export interface OpenAdapterDiagnosticsOptions {
  path: string;
  adapterId: string;
  adapterGeneration: number;
  secrets?: readonly string[];
  now?: () => Date;
  rotateAtBytes?: number;
  maxPendingRecords?: number;
  reportFailure?: (code: string) => void;
}

export function resolveAdapterLogPath(
  env: NodeJS.ProcessEnv = process.env,
  homeDirectory: string = homedir(),
): string {
  const override = env["PATCHBAY_ADAPTER_LOG"];
  if (override) return resolve(override);
  const xdgStateHome = env["XDG_STATE_HOME"];
  const stateHome = xdgStateHome && isAbsolute(xdgStateHome)
    ? xdgStateHome
    : join(homeDirectory, ".local", "state");
  return join(stateHome, "patchbay", "adapter.log");
}

export function diagnosticError(error: unknown): AdapterDiagnosticError {
  if (error instanceof Error) {
    const result: AdapterDiagnosticError = { name: error.name || "Error" };
    const code = (error as Error & { code?: unknown }).code;
    if (typeof code === "string" || typeof code === "number") result.code = String(code);
    return result;
  }
  if (typeof error === "object" && error !== null) {
    const value = error as { name?: unknown; code?: unknown; constructor?: { name?: unknown } };
    const name = typeof value.name === "string"
      ? value.name
      : typeof value.constructor?.name === "string"
        ? value.constructor.name
        : "ThrownValue";
    const result: AdapterDiagnosticError = { name };
    if (typeof value.code === "string" || typeof value.code === "number") {
      result.code = String(value.code);
    }
    return result;
  }
  return { name: typeof error === "string" ? "String" : typeof error };
}

export async function openAdapterDiagnostics(
  options: OpenAdapterDiagnosticsOptions,
): Promise<AdapterDiagnostics> {
  const sink = new FileAdapterDiagnostics(options);
  await sink.open();
  return sink;
}

class FileAdapterDiagnostics implements AdapterDiagnostics {
  readonly #options: OpenAdapterDiagnosticsOptions;
  readonly #secrets: readonly string[];
  readonly #maxPendingRecords: number;
  readonly #rotateAtBytes: number;
  readonly #now: () => Date;
  readonly #queue: string[] = [];
  #handle: FileHandle | undefined;
  #draining: Promise<void> | undefined;
  #dropped = 0;
  #failureReported = false;
  #closed = false;
  #closePromise: Promise<void> | undefined;

  constructor(options: OpenAdapterDiagnosticsOptions) {
    this.#options = options;
    this.#secrets = (options.secrets ?? []).filter((secret) => secret.length > 0);
    this.#maxPendingRecords = positiveInteger(options.maxPendingRecords, 1_024);
    this.#rotateAtBytes = positiveInteger(options.rotateAtBytes, 10 * 1024 * 1024);
    this.#now = options.now ?? (() => new Date());
  }

  async open(): Promise<void> {
    try {
      await mkdir(dirname(this.#options.path), { recursive: true });
    } catch {
      this.#reportFailure("open");
      return;
    }

    try {
      const existing = await stat(this.#options.path);
      if (existing.size >= this.#rotateAtBytes) {
        try {
          await rename(this.#options.path, `${this.#options.path}.1`);
        } catch {
          this.#reportFailure("rotate");
        }
      }
    } catch (error) {
      if (!isMissingFile(error)) this.#reportFailure("rotate");
    }

    try {
      this.#handle = await open(this.#options.path, "a", 0o600);
      try {
        await this.#handle.chmod(0o600);
      } catch {
        this.#reportFailure("chmod");
      }
    } catch {
      this.#reportFailure("open");
    }
  }

  record(input: AdapterDiagnosticInput): void {
    if (this.#closed) return;
    try {
      const serialized = JSON.stringify(this.#wireRecord(input)) + "\n";
      this.#enqueue(serialized);
    } catch {
      this.#reportFailure("serialize");
    }
  }

  async flush(): Promise<void> {
    if (this.#closed && !this.#draining) return;
    try {
      await this.#ensureDrain();
      if (this.#dropped > 0) {
        const count = this.#dropped;
        this.#dropped = 0;
        this.#enqueue(this.#serializeDropped(count));
        await this.#ensureDrain();
      }
    } catch {
      this.#reportFailure("flush");
    }
  }

  async close(): Promise<void> {
    if (this.#closePromise) return this.#closePromise;
    this.#closePromise = this.#close();
    return this.#closePromise;
  }

  #wireRecord(input: AdapterDiagnosticInput): Record<string, unknown> {
    const record: Record<string, unknown> = {
      ts: this.#sanitize(this.#now().toISOString()),
      level: input.level,
      event: input.event,
      adapter_id: this.#sanitize(this.#options.adapterId),
      adapter_generation: this.#options.adapterGeneration,
    };
    if (input.session) {
      record["session"] = {
        runtime_session_id: this.#sanitize(input.session.runtimeSessionId),
        deployment_scope: this.#sanitize(input.session.deploymentScope),
        generation: input.session.generation,
      };
    }
    if (input.commandId !== undefined) record["command_id"] = this.#sanitize(input.commandId);
    if (input.operationKind !== undefined) record["operation_kind"] = enumName(OperationKind, input.operationKind);
    if (input.failureCode !== undefined) record["failure_code"] = enumName(FailureCode, input.failureCode);
    if (input.outcome !== undefined) record["outcome"] = this.#sanitize(input.outcome);
    if (input.observationKind !== undefined) record["observation_kind"] = input.observationKind;
    if (input.sessionActivity !== undefined) record["session_activity"] = enumName(SessionActivityState, input.sessionActivity);
    if (input.sessionConnectivity !== undefined) record["session_connectivity"] = enumName(SessionConnectivityState, input.sessionConnectivity);
    if (input.fromGeneration !== undefined) record["from_generation"] = input.fromGeneration;
    if (input.toGeneration !== undefined) record["to_generation"] = input.toGeneration;
    if (input.reason !== undefined) record["reason"] = this.#sanitize(input.reason);
    if (input.error !== undefined) {
      const error: Record<string, string> = { name: this.#sanitize(input.error.name) };
      if (input.error.code !== undefined) error["code"] = this.#sanitize(input.error.code);
      record["error"] = error;
    }
    if (input.count !== undefined) record["count"] = input.count;
    return record;
  }

  #serializeDropped(count: number): string {
    return JSON.stringify(this.#wireRecord({
      event: "log.records_dropped",
      level: "warn",
      count,
    })) + "\n";
  }

  #enqueue(line: string): void {
    if (this.#dropped > 0 && this.#queue.length < this.#maxPendingRecords) {
      const count = this.#dropped;
      this.#dropped = 0;
      this.#queue.push(this.#serializeDropped(count));
    }
    if (this.#queue.length >= this.#maxPendingRecords) {
      this.#dropped += 1;
      return;
    }
    this.#queue.push(line);
    queueMicrotask(() => {
      if (!this.#closed) void this.#ensureDrain();
    });
  }

  #ensureDrain(): Promise<void> {
    if (!this.#draining) {
      this.#draining = this.#drain().finally(() => {
        this.#draining = undefined;
        if (this.#queue.length > 0 && !this.#closed) void this.#ensureDrain();
      });
    }
    return this.#draining;
  }

  async #drain(): Promise<void> {
    while (this.#queue.length > 0) {
      const line = this.#queue.shift();
      if (line === undefined) continue;
      if (!this.#handle) continue;
      try {
        await this.#handle.write(line, undefined, "utf8");
      } catch {
        this.#reportFailure("write");
        try {
          await this.#handle.close();
        } catch {
          // The write failure is already reported and close remains best effort.
        }
        this.#handle = undefined;
      }
    }
  }

  async #close(): Promise<void> {
    await this.flush();
    this.#closed = true;
    const handle = this.#handle;
    this.#handle = undefined;
    if (handle) {
      try {
        await handle.close();
      } catch {
        this.#reportFailure("close");
      }
    }
  }

  #sanitize(value: string): string {
    let sanitized = value;
    for (const secret of this.#secrets) sanitized = sanitized.split(secret).join("[REDACTED]");
    sanitized = sanitized
      .replace(/\b(bearer)\s+[^,\s;]+/gi, "$1=[REDACTED]")
      .replace(
        /\b(token|password|passwd|secret|api[_-]?key|authorization|cookie|csrf|access[_-]?token|refresh[_-]?token)\s*[:=]\s*([^,\s;]+)/gi,
        "$1=[REDACTED]",
      );
    return sanitized.length > 256 ? `${sanitized.slice(0, 253)}...` : sanitized;
  }

  #reportFailure(code: string): void {
    if (this.#failureReported) return;
    this.#failureReported = true;
    try {
      this.#options.reportFailure?.(code);
    } catch {
      // Failure reporting is itself outside the adapter's control path.
    }
    try {
      process.stderr.write(`[patchbay-adapter] diagnostics failure: ${code}\n`);
    } catch {
      // stderr may be unavailable during process teardown.
    }
  }
}

export const NOOP_ADAPTER_DIAGNOSTICS: AdapterDiagnostics = Object.freeze({
  record: () => undefined,
  flush: async () => undefined,
  close: async () => undefined,
});

function enumName<T extends Record<number, string>>(registry: T, value: number): string {
  return registry[value] ?? "UNRECOGNIZED";
}

function positiveInteger(value: number | undefined, fallback: number): number {
  return value !== undefined && Number.isSafeInteger(value) && value > 0 ? value : fallback;
}

function isMissingFile(error: unknown): boolean {
  return typeof error === "object" && error !== null && "code" in error && (error as { code?: unknown }).code === "ENOENT";
}
