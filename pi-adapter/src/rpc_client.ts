import { randomBytes } from "node:crypto";
import type { Readable, Writable } from "node:stream";

const DEFAULT_MAX_LINE_BYTES = 1_048_576;
const DEFAULT_MAX_STDERR_BYTES = 65_536;
const DEFAULT_REQUEST_TIMEOUT_MS = 10_000;
const REQUEST_PREFIX_PATTERN = /^[A-Za-z0-9_-]{8,64}$/u;

type JsonRecord = Record<string, unknown>;

export interface PiRpcClientStreams {
  readonly stdin: Writable;
  readonly stdout: Readable;
  readonly stderr?: Readable;
}

export interface PiRpcClientOptions {
  readonly streams: PiRpcClientStreams;
  readonly requestPrefix?: string;
  readonly maxLineBytes?: number;
  readonly maxStderrBytes?: number;
  readonly requestTimeoutMs?: number;
}

export interface PiRpcResponse<T = unknown> extends JsonRecord {
  readonly type: "response";
  readonly id: string;
  readonly command: string;
  readonly success: boolean;
  readonly data?: T;
  readonly error?: string;
}

export interface PiRpcEvent extends JsonRecord {
  readonly type: string;
}

export interface PiRpcProcessExit {
  readonly code: number | null;
  readonly signal: NodeJS.Signals | null;
  readonly expected: boolean;
}

export type PiRpcEventListener = (event: PiRpcEvent) => void;
export type PiRpcFailureListener = (error: PiRpcTransportError) => void;

export class PiRpcTransportError extends Error {
  readonly kind:
    | "framing"
    | "protocol"
    | "pipe"
    | "eof"
    | "process_exit"
    | "timeout";
  readonly processExit?: PiRpcProcessExit;

  constructor(
    kind: PiRpcTransportError["kind"],
    message: string,
    processExit?: PiRpcProcessExit,
  ) {
    super(message);
    this.name = "PiRpcTransportError";
    this.kind = kind;
    if (processExit) this.processExit = processExit;
  }
}

export class PiRpcCommandError extends Error {
  readonly command: string;

  constructor(command: string) {
    super(`Pi RPC command failed: ${command}`);
    this.name = "PiRpcCommandError";
    this.command = command;
  }
}

interface PendingRequest {
  readonly command: string;
  readonly resolve: (response: PiRpcResponse) => void;
  readonly reject: (error: Error) => void;
  readonly timer: NodeJS.Timeout;
}

/** Strict bounded LF-JSONL client. Responses never enter the event stream. */
export class PiRpcClient {
  readonly #streams: PiRpcClientStreams;
  readonly #requestPrefix: string;
  readonly #maxLineBytes: number;
  readonly #maxStderrBytes: number;
  readonly #requestTimeoutMs: number;
  readonly #pending = new Map<string, PendingRequest>();
  readonly #eventListeners = new Set<PiRpcEventListener>();
  readonly #failureListeners = new Set<PiRpcFailureListener>();
  readonly #extensionErrors: PiRpcEvent[] = [];
  #stdoutBuffer = Buffer.alloc(0);
  #stderr = Buffer.alloc(0);
  #requestSequence = 0;
  #processExit: PiRpcProcessExit | undefined;
  #failure: PiRpcTransportError | undefined;
  #eofTimer: number | NodeJS.Timeout | undefined;
  #closed = false;

  constructor(options: PiRpcClientOptions) {
    this.#streams = options.streams;
    this.#requestPrefix = options.requestPrefix ?? randomBytes(12).toString("base64url");
    this.#maxLineBytes = boundedInteger(
      options.maxLineBytes ?? DEFAULT_MAX_LINE_BYTES,
      256,
      16 * 1_048_576,
      "maxLineBytes",
    );
    this.#maxStderrBytes = boundedInteger(
      options.maxStderrBytes ?? DEFAULT_MAX_STDERR_BYTES,
      1_024,
      1_048_576,
      "maxStderrBytes",
    );
    this.#requestTimeoutMs = boundedInteger(
      options.requestTimeoutMs ?? DEFAULT_REQUEST_TIMEOUT_MS,
      1,
      120_000,
      "requestTimeoutMs",
    );
    if (!REQUEST_PREFIX_PATTERN.test(this.#requestPrefix)) {
      throw new Error("RPC request prefix is invalid");
    }

    options.streams.stdout.on("data", this.#onStdoutData);
    options.streams.stdout.once("end", this.#onStdoutEnd);
    options.streams.stdout.once("error", this.#onStdoutError);
    options.streams.stdin.once("error", this.#onStdinError);
    options.streams.stderr?.on("data", this.#onStderrData);
  }

  get extensionErrors(): readonly PiRpcEvent[] {
    return this.#extensionErrors;
  }

  /** Adapter-local only. Never place this value in core diagnostics/evidence. */
  stderrSnapshot(): string {
    return this.#stderr.toString("utf8");
  }

  onEvent(listener: PiRpcEventListener): () => void {
    this.#eventListeners.add(listener);
    return () => this.#eventListeners.delete(listener);
  }

  onFailure(listener: PiRpcFailureListener): () => void {
    this.#failureListeners.add(listener);
    if (this.#failure) queueMicrotask(() => listener(this.#failure!));
    return () => this.#failureListeners.delete(listener);
  }

  markProcessExit(exit: PiRpcProcessExit): void {
    if (this.#processExit) return;
    this.#processExit = Object.freeze({ ...exit });
    if (this.#eofTimer) {
      clearTimeout(this.#eofTimer);
      this.#eofTimer = undefined;
    }
    if (!isConfirmedCleanExit(exit)) {
      this.#fail(
        new PiRpcTransportError(
          "process_exit",
          "Pi RPC process exited before a confirmed clean shutdown",
          exit,
        ),
      );
    } else if (!this.#closed) {
      // Expected clean process exit is authoritative lifecycle evidence, not a
      // transport-loss event. Outstanding commands still cannot complete.
      this.#rejectPending(new PiRpcTransportError("eof", "Pi RPC process closed its transport", exit));
    }
  }

  async request<T = unknown>(command: JsonRecord & { readonly type: string }): Promise<T> {
    if (this.#closed || this.#failure) {
      throw this.#failure ?? new PiRpcTransportError("pipe", "Pi RPC client is closed");
    }
    if (!isBoundedToken(command.type, 128) || Object.hasOwn(command, "id")) {
      throw new Error("Pi RPC command type is invalid or already contains an id");
    }
    this.#requestSequence += 1;
    if (!Number.isSafeInteger(this.#requestSequence)) {
      throw new PiRpcTransportError("protocol", "Pi RPC request id space is exhausted");
    }
    const id = `${this.#requestPrefix}-${this.#requestSequence.toString(36)}`;
    const bytes = Buffer.from(`${JSON.stringify({ ...command, id })}\n`, "utf8");
    if (bytes.byteLength > this.#maxLineBytes) {
      throw new PiRpcTransportError("framing", "Pi RPC command exceeds the line bound");
    }

    const response = await new Promise<PiRpcResponse>((resolve, reject) => {
      const timer = setTimeout(() => {
        this.#pending.delete(id);
        reject(new PiRpcTransportError("timeout", `Pi RPC request timed out: ${command.type}`));
      }, this.#requestTimeoutMs);
      this.#pending.set(id, { command: command.type, resolve, reject, timer });
      this.#streams.stdin.write(bytes, (error) => {
        if (!error) return;
        const pending = this.#pending.get(id);
        if (!pending) return;
        this.#pending.delete(id);
        clearTimeout(pending.timer);
        pending.reject(new PiRpcTransportError("pipe", "Pi RPC stdin write failed"));
      });
    });
    if (!response.success) throw new PiRpcCommandError(response.command);
    return response.data as T;
  }

  close(): void {
    if (this.#closed) return;
    this.#closed = true;
    if (this.#eofTimer) clearTimeout(this.#eofTimer);
    this.#eofTimer = undefined;
    this.#streams.stdout.off("data", this.#onStdoutData);
    this.#streams.stderr?.off("data", this.#onStderrData);
    this.#rejectPending(new PiRpcTransportError("pipe", "Pi RPC client closed"));
    this.#eventListeners.clear();
    this.#failureListeners.clear();
  }

  readonly #onStdoutData = (chunk: Buffer | string): void => {
    if (this.#closed || this.#failure) return;
    const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    this.#stdoutBuffer = Buffer.concat([this.#stdoutBuffer, bytes]);
    if (this.#stdoutBuffer.byteLength > this.#maxLineBytes * 2) {
      this.#fail(new PiRpcTransportError("framing", "Pi RPC stdout buffer exceeded its bound"));
      return;
    }
    while (true) {
      const lf = this.#stdoutBuffer.indexOf(0x0a);
      if (lf < 0) {
        if (this.#stdoutBuffer.byteLength > this.#maxLineBytes) {
          this.#fail(new PiRpcTransportError("framing", "Pi RPC stdout line exceeded its bound"));
        }
        return;
      }
      let line = this.#stdoutBuffer.subarray(0, lf);
      this.#stdoutBuffer = this.#stdoutBuffer.subarray(lf + 1);
      if (line.byteLength > 0 && line[line.byteLength - 1] === 0x0d) {
        line = line.subarray(0, line.byteLength - 1);
      }
      if (line.byteLength === 0 || line.byteLength > this.#maxLineBytes) {
        this.#fail(new PiRpcTransportError("framing", "Pi RPC emitted an invalid JSONL record"));
        return;
      }
      this.#handleLine(line);
      if (this.#failure) return;
    }
  };

  readonly #onStdoutEnd = (): void => {
    if (this.#closed || this.#failure) return;
    if (this.#stdoutBuffer.byteLength !== 0) {
      this.#fail(new PiRpcTransportError("framing", "Pi RPC stdout ended without LF framing"));
      return;
    }
    if (this.#processExit) {
      if (!isConfirmedCleanExit(this.#processExit)) {
        this.#fail(new PiRpcTransportError(
          "eof",
          "Pi RPC stdout ended after an unclean process exit",
          this.#processExit,
        ));
      }
      return;
    }
    // ChildProcess exit usually follows stdout EOF. Give that authoritative
    // evidence one event-loop grace interval before classifying bare EOF as a
    // stale transport loss.
    this.#eofTimer = setTimeout(() => {
      this.#eofTimer = undefined;
      if (!this.#processExit) {
        this.#fail(new PiRpcTransportError(
          "eof",
          "Pi RPC stdout ended without correlated process-exit evidence",
        ));
      }
    }, 100);
  };

  readonly #onStdoutError = (): void => {
    this.#fail(new PiRpcTransportError("pipe", "Pi RPC stdout failed"));
  };

  readonly #onStdinError = (): void => {
    this.#fail(new PiRpcTransportError("pipe", "Pi RPC stdin failed"));
  };

  readonly #onStderrData = (chunk: Buffer | string): void => {
    const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    this.#stderr = Buffer.concat([this.#stderr, bytes]);
    if (this.#stderr.byteLength > this.#maxStderrBytes) {
      this.#stderr = this.#stderr.subarray(this.#stderr.byteLength - this.#maxStderrBytes);
    }
  };

  #handleLine(bytes: Buffer): void {
    let text: string;
    try {
      text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    } catch {
      this.#fail(new PiRpcTransportError("framing", "Pi RPC emitted invalid UTF-8"));
      return;
    }
    let value: unknown;
    try {
      value = JSON.parse(text);
    } catch {
      this.#fail(new PiRpcTransportError("framing", "Pi RPC emitted invalid JSON"));
      return;
    }
    if (!isRecord(value) || !isBoundedToken(value["type"], 128)) {
      this.#fail(new PiRpcTransportError("protocol", "Pi RPC emitted an invalid record envelope"));
      return;
    }
    if (value["type"] === "response") {
      this.#handleResponse(value);
      return;
    }
    if (value["type"] === "extension_error") {
      if (this.#extensionErrors.length >= 64) this.#extensionErrors.shift();
      this.#extensionErrors.push(Object.freeze({ ...value }) as PiRpcEvent);
    }
    const event = Object.freeze({ ...value }) as PiRpcEvent;
    for (const listener of this.#eventListeners) listener(event);
  }

  #handleResponse(value: JsonRecord): void {
    const id = value["id"];
    const command = value["command"];
    const success = value["success"];
    if (!isBoundedToken(id, 128) || !isBoundedToken(command, 128) || typeof success !== "boolean") {
      this.#fail(new PiRpcTransportError("protocol", "Pi RPC emitted an invalid response"));
      return;
    }
    const pending = this.#pending.get(id);
    if (!pending || pending.command !== command) {
      this.#fail(new PiRpcTransportError("protocol", "Pi RPC response correlation failed"));
      return;
    }
    if (!success && !isBoundedText(value["error"], 4_096)) {
      this.#fail(new PiRpcTransportError("protocol", "Pi RPC error response is malformed"));
      return;
    }
    this.#pending.delete(id);
    clearTimeout(pending.timer);
    pending.resolve(value as PiRpcResponse);
  }

  #fail(error: PiRpcTransportError): void {
    if (this.#closed || this.#failure) return;
    this.#failure = error;
    this.#rejectPending(error);
    for (const listener of this.#failureListeners) listener(error);
  }

  #rejectPending(error: Error): void {
    for (const pending of this.#pending.values()) {
      clearTimeout(pending.timer);
      pending.reject(error);
    }
    this.#pending.clear();
  }
}

function isConfirmedCleanExit(exit: PiRpcProcessExit): boolean {
  return exit.expected && (
    (exit.code === 0 && exit.signal === null) ||
    (exit.code === 143 && exit.signal === null) ||
    (exit.code === null && exit.signal === "SIGTERM")
  );
}

function boundedInteger(value: number, minimum: number, maximum: number, name: string): number {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new Error(`${name} is outside its supported bound`);
  }
  return value;
}

function isRecord(value: unknown): value is JsonRecord {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isBoundedToken(value: unknown, maximum: number): value is string {
  return typeof value === "string" && value.length > 0 && value.length <= maximum && !value.includes("\0");
}

function isBoundedText(value: unknown, maximumBytes: number): value is string {
  return isBoundedToken(value, maximumBytes) && Buffer.byteLength(value) <= maximumBytes;
}
