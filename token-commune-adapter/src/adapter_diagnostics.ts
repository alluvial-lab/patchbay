import { mkdir, open, rename, stat, type FileHandle } from "node:fs/promises";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { homedir } from "node:os";
import { FailureCode, OperationKind } from "@patchbay/contracts";

export const ADAPTER_DIAGNOSTIC_EVENTS = [
  "adapter.starting", "adapter.started", "adapter.stopping", "adapter.stopped",
  "adapter.attach.started", "adapter.attach.succeeded", "adapter.attach.failed",
  "credential.load.failed", "gateway.auth.failed", "gateway.request.failed",
  "gateway.response.invalid", "delivery.subscription.failed",
  "delivery.subscription.retrying", "delivery.received", "delivery.acknowledged",
  "delivery.unsupported", "log.records_dropped",
] as const;

export type AdapterDiagnosticEvent = (typeof ADAPTER_DIAGNOSTIC_EVENTS)[number];
export type AdapterDiagnosticLevel = "info" | "warn" | "error";

export interface AdapterDiagnosticResourceRef {
  resourceKind: string;
  resourceId: string;
}

export interface AdapterDiagnosticError {
  name: string;
  code?: string;
}

export interface AdapterDiagnosticInput {
  event: AdapterDiagnosticEvent;
  level: AdapterDiagnosticLevel;
  resource?: AdapterDiagnosticResourceRef;
  commandId?: string;
  operationKind?: OperationKind;
  failureCode?: FailureCode;
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
  const xdg = env["XDG_STATE_HOME"];
  return join(xdg && isAbsolute(xdg) ? xdg : join(homeDirectory, ".local", "state"), "patchbay", "token-commune-adapter.log");
}

export function diagnosticError(error: unknown): AdapterDiagnosticError {
  try {
    if (error instanceof Error) {
      const result: AdapterDiagnosticError = { name: error.name || "Error" };
      const code = (error as Error & { code?: unknown }).code;
      if (typeof code === "string" || typeof code === "number") result.code = String(code);
      return result;
    }
    if (typeof error === "object" && error !== null) {
      const value = error as { name?: unknown; code?: unknown; constructor?: { name?: unknown } };
      const result: AdapterDiagnosticError = {
        name: typeof value.name === "string"
          ? value.name
          : typeof value.constructor?.name === "string"
            ? value.constructor.name
            : "ThrownValue",
      };
      if (typeof value.code === "string" || typeof value.code === "number") result.code = String(value.code);
      return result;
    }
    return { name: typeof error === "string" ? "String" : typeof error };
  } catch {
    return { name: "Error", code: "DIAGNOSTIC_ERROR" };
  }
}

export async function openAdapterDiagnostics(options: OpenAdapterDiagnosticsOptions): Promise<AdapterDiagnostics> {
  const sink = new FileAdapterDiagnostics(options);
  await sink.open();
  return sink;
}

class FileAdapterDiagnostics implements AdapterDiagnostics {
  readonly #secrets: readonly string[];
  readonly #queue: string[] = [];
  readonly #maxPending: number;
  readonly #rotateAt: number;
  readonly #now: () => Date;
  #handle: FileHandle | undefined;
  #draining: Promise<void> | undefined;
  #dropped = 0;
  #failureReported = false;
  #closed = false;
  #closePromise: Promise<void> | undefined;

  constructor(readonly options: OpenAdapterDiagnosticsOptions) {
    this.#secrets = (options.secrets ?? []).filter(Boolean);
    this.#maxPending = positive(options.maxPendingRecords, 1_024);
    this.#rotateAt = positive(options.rotateAtBytes, 10 * 1024 * 1024);
    this.#now = options.now ?? (() => new Date());
  }

  async open(): Promise<void> {
    try { await mkdir(dirname(this.options.path), { recursive: true }); }
    catch { this.#failure("open"); return; }
    try {
      if ((await stat(this.options.path)).size >= this.#rotateAt) {
        try { await rename(this.options.path, `${this.options.path}.1`); }
        catch { this.#failure("rotate"); }
      }
    } catch (error) {
      if (!isMissing(error)) this.#failure("rotate");
    }
    try {
      this.#handle = await open(this.options.path, "a", 0o600);
      try { await this.#handle.chmod(0o600); } catch { this.#failure("chmod"); }
    } catch { this.#failure("open"); }
  }

  record(input: AdapterDiagnosticInput): void {
    if (this.#closed) return;
    try { this.#enqueue(`${JSON.stringify(this.#wire(input))}\n`); }
    catch { this.#failure("serialize"); }
  }

  async flush(): Promise<void> {
    if (this.#closed && !this.#draining) return;
    try {
      await this.#ensureDrain();
      if (this.#dropped > 0) {
        const dropped = this.#dropped;
        this.#dropped = 0;
        this.#enqueue(`${JSON.stringify(this.#wire({ event: "log.records_dropped", level: "warn", count: dropped }))}\n`);
        await this.#ensureDrain();
      }
    } catch { this.#failure("flush"); }
  }

  close(): Promise<void> {
    this.#closePromise ??= this.#close();
    return this.#closePromise;
  }

  #wire(input: AdapterDiagnosticInput): Record<string, unknown> {
    const output: Record<string, unknown> = {
      ts: this.#sanitize(this.#now().toISOString()), level: input.level, event: input.event,
      adapter_id: this.#sanitize(this.options.adapterId), adapter_generation: this.options.adapterGeneration,
    };
    if (input.resource) output["resource"] = {
      resource_kind: this.#sanitize(input.resource.resourceKind), resource_id: this.#sanitize(input.resource.resourceId),
    };
    if (input.commandId !== undefined) output["command_id"] = this.#sanitize(input.commandId);
    if (input.operationKind !== undefined) output["operation_kind"] = OperationKind[input.operationKind] ?? "UNRECOGNIZED";
    if (input.failureCode !== undefined) output["failure_code"] = FailureCode[input.failureCode] ?? "UNRECOGNIZED";
    if (input.error) output["error"] = {
      name: this.#sanitize(input.error.name), ...(input.error.code ? { code: this.#sanitize(input.error.code) } : {}),
    };
    if (input.count !== undefined) output["count"] = input.count;
    return output;
  }

  #sanitize(value: string): string {
    let result = value;
    for (const secret of this.#secrets) result = result.split(secret).join("[REDACTED]");
    result = result
      .replace(/\bbearer(?:\s*[:=]\s*|\s+)[^,\s;]+/gi, "bearer=[REDACTED]")
      .replace(/\b(token|password|secret|api[_-]?key|authorization|cookie|csrf)\s*[:=]\s*([^,\s;]+)/gi, "$1=[REDACTED]");
    return result.length > 256 ? `${result.slice(0, 253)}...` : result;
  }

  #enqueue(line: string): void {
    if (this.#queue.length >= this.#maxPending) { this.#dropped += 1; return; }
    this.#queue.push(line);
    queueMicrotask(() => { if (!this.#closed) void this.#ensureDrain(); });
  }

  #ensureDrain(): Promise<void> {
    this.#draining ??= this.#drain().finally(() => {
      this.#draining = undefined;
      if (this.#queue.length && !this.#closed) void this.#ensureDrain();
    });
    return this.#draining;
  }

  async #drain(): Promise<void> {
    while (this.#queue.length) {
      const line = this.#queue.shift();
      if (!line || !this.#handle) continue;
      try { await this.#handle.write(line, undefined, "utf8"); }
      catch {
        this.#failure("write");
        try { await this.#handle.close(); } catch { /* best effort */ }
        this.#handle = undefined;
      }
    }
  }

  async #close(): Promise<void> {
    await this.flush();
    this.#closed = true;
    const handle = this.#handle;
    this.#handle = undefined;
    if (handle) try { await handle.close(); } catch { this.#failure("close"); }
  }

  #failure(code: string): void {
    if (this.#failureReported) return;
    this.#failureReported = true;
    try { this.options.reportFailure?.(code); } catch { /* non-interference */ }
    try { process.stderr.write(`[patchbay-token-commune-adapter] diagnostics failure: ${code}\n`); } catch { /* teardown */ }
  }
}

export const NOOP_ADAPTER_DIAGNOSTICS: AdapterDiagnostics = Object.freeze({
  record: () => undefined, flush: async () => undefined, close: async () => undefined,
});

function positive(value: number | undefined, fallback: number): number {
  return value !== undefined && Number.isSafeInteger(value) && value > 0 ? value : fallback;
}
function isMissing(error: unknown): boolean {
  return typeof error === "object" && error !== null && "code" in error && (error as { code?: unknown }).code === "ENOENT";
}
