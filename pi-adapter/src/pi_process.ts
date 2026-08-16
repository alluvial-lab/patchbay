import { spawn, type ChildProcessWithoutNullStreams } from "node:child_process";
import { randomBytes } from "node:crypto";
import { realpath } from "node:fs/promises";
import { isAbsolute, normalize } from "node:path";
import {
  performPiControlHandshake,
  type PiControlHandshake,
} from "./control_handshake.js";
import {
  PiRpcClient,
  type PiRpcProcessExit,
  type PiRpcTransportError,
} from "./rpc_client.js";

const SAFE_INHERITED_ENV = Object.freeze([
  "HOME",
  "LANG",
  "LC_ALL",
  "PATH",
  "SHELL",
  "TERM",
  "TMPDIR",
  "TZ",
] as const);
const MAX_ARG_BYTES = 8_192;
const MAX_ENV_VALUE_BYTES = 16_384;
const DEFAULT_TERM_TIMEOUT_MS = 2_000;
const DEFAULT_KILL_TIMEOUT_MS = 2_000;

export interface PiLaunchSpec {
  readonly executable: string;
  readonly argv: readonly string[];
  readonly cwd: string;
  readonly launchNonce: string;
  readonly environment?: Readonly<Record<string, string>>;
}

export interface PiRpcLaunchArguments {
  readonly cliPath: string;
  readonly controlExtensionPath: string;
  readonly sessionPath?: string;
  readonly sessionDirectory?: string;
  readonly model?: string;
  readonly name?: string;
  readonly additionalArguments?: readonly string[];
}

export interface TerminationPolicy {
  readonly termTimeoutMs?: number;
  readonly killTimeoutMs?: number;
}

export interface ProcessExit extends PiRpcProcessExit {
  readonly pid: number;
  readonly processToken: string;
  readonly terminatedBySupervisor: boolean;
}

export interface PiRpcRuntime {
  readonly pid: number;
  readonly processToken: string;
  readonly rpc: PiRpcClient;
  readonly exit: Promise<ProcessExit>;
  readonly child: ChildProcessWithoutNullStreams;
  markExpectedTermination(): void;
  onTransportFailure(listener: (error: PiRpcTransportError) => void): () => void;
}

export interface PiHandshakeChallenge {
  readonly expectedProjectCwd: string;
  readonly expectedExtensionPath: string;
  readonly requiredExtensionEpoch?: string;
  readonly previousExtensionEpoch?: string;
  /** Test/startup override for unusually slow real-process handshakes. */
  readonly rpcTimeoutMs?: number;
}

export interface ManagedPiRuntimePort {
  launch(spec: PiLaunchSpec): Promise<PiRpcRuntime>;
  handshake(runtime: PiRpcRuntime, challenge: PiHandshakeChallenge): Promise<PiControlHandshake>;
  terminate(runtime: PiRpcRuntime, policy?: TerminationPolicy): Promise<ProcessExit>;
}

export interface PiProcessLauncher {
  launch(spec: PiLaunchSpec): Promise<PiRpcRuntime>;
  terminate(runtime: PiRpcRuntime, policy?: TerminationPolicy): Promise<ProcessExit>;
}

/** The one production ManagedPiRuntimePort: an isolated Pi JSONL-RPC child. */
export class RpcManagedPiRuntimePort implements ManagedPiRuntimePort {
  constructor(readonly processLauncher: PiProcessLauncher = new NodePiProcessLauncher()) {}

  launch(spec: PiLaunchSpec): Promise<PiRpcRuntime> {
    return this.processLauncher.launch(spec);
  }

  handshake(runtime: PiRpcRuntime, challenge: PiHandshakeChallenge): Promise<PiControlHandshake> {
    return performPiControlHandshake({
      rpc: piControlRpc(runtime.rpc),
      launchNonce: runtimeLaunchNonce(runtime),
      ...challenge,
    });
  }

  terminate(runtime: PiRpcRuntime, policy?: TerminationPolicy): Promise<ProcessExit> {
    return this.processLauncher.terminate(runtime, policy);
  }
}

/** POSIX process-group launcher. Windows remains an explicitly reserved seam. */
export class NodePiProcessLauncher implements PiProcessLauncher {
  async launch(input: PiLaunchSpec): Promise<PiRpcRuntime> {
    const spec = await validateLaunchSpec(input);
    const child = spawn(spec.executable, [...spec.argv], {
      cwd: spec.cwd,
      env: sanitizedEnvironment(spec.launchNonce, spec.environment),
      detached: true,
      stdio: ["pipe", "pipe", "pipe"],
    });
    const processToken = randomBytes(18).toString("base64url");
    let expectedTermination = false;
    let terminatedBySupervisor = false;
    let resolveExit!: (exit: ProcessExit) => void;
    const exit = new Promise<ProcessExit>((resolve) => {
      resolveExit = resolve;
    });
    const rpc = new PiRpcClient({
      streams: { stdin: child.stdin, stdout: child.stdout, stderr: child.stderr },
      requestPrefix: processToken,
    });
    const runtime: PiRpcRuntime = {
      pid: child.pid ?? 0,
      processToken,
      rpc,
      child,
      exit,
      markExpectedTermination() {
        expectedTermination = true;
        terminatedBySupervisor = true;
      },
      onTransportFailure(listener) {
        return rpc.onFailure(listener);
      },
    };

    const spawned = new Promise<void>((resolve, reject) => {
      child.once("spawn", resolve);
      child.once("error", reject);
    });
    child.once("exit", (code, signal) => {
      const evidence: ProcessExit = Object.freeze({
        pid: runtime.pid,
        processToken,
        code,
        signal,
        expected: expectedTermination,
        terminatedBySupervisor,
      });
      rpc.markProcessExit(evidence);
      resolveExit(evidence);
    });
    try {
      await spawned;
    } catch {
      rpc.close();
      throw new Error("Pi RPC process failed before external identity was established");
    }
    if (!Number.isSafeInteger(runtime.pid) || runtime.pid <= 0) {
      rpc.close();
      throw new Error("Pi RPC process did not expose a positive pid");
    }
    // Correlate the launch nonce without putting it in the public runtime type.
    Object.defineProperty(runtime, launchNonceSymbol, { value: spec.launchNonce });
    return runtime;
  }

  async terminate(runtime: PiRpcRuntime, policy: TerminationPolicy = {}): Promise<ProcessExit> {
    const termTimeoutMs = boundedTimeout(policy.termTimeoutMs ?? DEFAULT_TERM_TIMEOUT_MS);
    const killTimeoutMs = boundedTimeout(policy.killTimeoutMs ?? DEFAULT_KILL_TIMEOUT_MS);
    runtime.markExpectedTermination();
    signalProcessGroup(runtime.pid, "SIGTERM");
    const termExit = await raceExit(runtime.exit, termTimeoutMs);
    if (termExit) {
      runtime.rpc.close();
      return termExit;
    }
    signalProcessGroup(runtime.pid, "SIGKILL");
    const killExit = await raceExit(runtime.exit, killTimeoutMs);
    if (!killExit) throw new Error("Pi RPC process group did not exit after SIGKILL");
    runtime.rpc.close();
    return killExit;
  }
}

const launchNonceSymbol = Symbol("patchbay-launch-nonce");

type RuntimeWithNonce = PiRpcRuntime & { readonly [launchNonceSymbol]: string };

function runtimeLaunchNonce(runtime: PiRpcRuntime): string {
  const nonce = (runtime as RuntimeWithNonce)[launchNonceSymbol];
  if (!nonce) throw new Error("Pi RPC runtime is missing its launch nonce correlation");
  return nonce;
}

export function buildPiRpcArgv(arguments_: PiRpcLaunchArguments): readonly string[] {
  requireAbsolutePath(arguments_.cliPath, "Pi CLI path");
  requireAbsolutePath(arguments_.controlExtensionPath, "control extension path");
  if (arguments_.sessionPath) requireAbsolutePath(arguments_.sessionPath, "session path");
  if (arguments_.sessionDirectory) requireAbsolutePath(arguments_.sessionDirectory, "session directory");
  const argv = [
    arguments_.cliPath,
    "--mode",
    "rpc",
    "--no-extensions",
    "--extension",
    arguments_.controlExtensionPath,
  ];
  if (arguments_.sessionPath) argv.push("--session", arguments_.sessionPath);
  if (arguments_.sessionDirectory) argv.push("--session-dir", arguments_.sessionDirectory);
  if (arguments_.model) argv.push("--model", boundedArgument(arguments_.model));
  if (arguments_.name) argv.push("--name", boundedArgument(arguments_.name));
  for (const value of arguments_.additionalArguments ?? []) argv.push(boundedArgument(value));
  return Object.freeze(argv);
}

export function sanitizedEnvironment(
  launchNonce: string,
  explicit: Readonly<Record<string, string>> = {},
): NodeJS.ProcessEnv {
  const result: NodeJS.ProcessEnv = {};
  for (const name of SAFE_INHERITED_ENV) {
    const value = process.env[name];
    if (value !== undefined) result[name] = boundedEnvironmentValue(value);
  }
  for (const [name, value] of Object.entries(explicit)) {
    if (!/^[A-Z][A-Z0-9_]{0,127}$/u.test(name) || forbiddenCredentialName(name)) {
      throw new Error("Pi launch environment contains a forbidden variable");
    }
    result[name] = boundedEnvironmentValue(value);
  }
  result["PATCHBAY_LAUNCH_NONCE"] = boundedEnvironmentValue(launchNonce);
  return result;
}

function forbiddenCredentialName(name: string): boolean {
  return /(?:API_KEY|ACCESS_TOKEN|AUTH_TOKEN|BEARER|CREDENTIAL|PASSWORD|SECRET)$/u.test(name);
}

async function validateLaunchSpec(input: PiLaunchSpec): Promise<PiLaunchSpec> {
  requireAbsolutePath(input.executable, "Pi executable");
  requireAbsolutePath(input.cwd, "Pi cwd");
  const [executable, cwd] = await Promise.all([realpath(input.executable), realpath(input.cwd)]);
  if (executable !== normalize(input.executable) || cwd !== normalize(input.cwd)) {
    throw new Error("Pi launch executable and cwd must be canonical paths");
  }
  if (input.argv.length === 0 || input.argv.length > 256) {
    throw new Error("Pi launch argv is outside its supported bound");
  }
  const argv = input.argv.map(boundedArgument);
  if (!isAbsolute(argv[0] ?? "")) {
    throw new Error("Pi launch argv[0] must be an absolute CLI path");
  }
  if (!/^[A-Za-z0-9_-]{43}$/u.test(input.launchNonce)) {
    throw new Error("Pi launch nonce is invalid");
  }
  return Object.freeze({
    executable,
    argv: Object.freeze(argv),
    cwd,
    launchNonce: input.launchNonce,
    ...(input.environment ? { environment: Object.freeze({ ...input.environment }) } : {}),
  });
}

function piControlRpc(rpc: PiRpcClient) {
  return {
    async getCommands() {
      const data = await rpc.request<{ commands?: unknown }>({ type: "get_commands" });
      if (!Array.isArray(data.commands)) throw new Error("Pi get_commands response is malformed");
      return data.commands.map((value) => {
        if (typeof value !== "object" || value === null || Array.isArray(value)) {
          throw new Error("Pi get_commands entry is malformed");
        }
        const command = value as Record<string, unknown>;
        const sourceInfo = typeof command["sourceInfo"] === "object" &&
          command["sourceInfo"] !== null && !Array.isArray(command["sourceInfo"])
          ? command["sourceInfo"] as Record<string, unknown>
          : undefined;
        return {
          name: boundedArgument(String(command["name"] ?? "")),
          source: boundedArgument(String(command["source"] ?? "")),
          ...(typeof sourceInfo?.["path"] === "string" ? { path: sourceInfo["path"] } : {}),
        };
      });
    },
    async prompt(message: string) {
      await rpc.request({ type: "prompt", message });
      return { success: true } as const;
    },
    async getEntries() {
      return rpc.request<{ entries: unknown[]; leafId: string | null }>({ type: "get_entries" });
    },
    async getState() {
      return rpc.request<{ sessionId: string; sessionFile: string }>({ type: "get_state" });
    },
    async getSessionStats() {
      return rpc.request<{ sessionId: string; sessionFile: string }>({ type: "get_session_stats" });
    },
  };
}

function boundedArgument(value: string): string {
  if (!value || value.includes("\0") || Buffer.byteLength(value) > MAX_ARG_BYTES) {
    throw new Error("Pi launch argument is invalid");
  }
  return value;
}

function boundedEnvironmentValue(value: string): string {
  if (value.includes("\0") || Buffer.byteLength(value) > MAX_ENV_VALUE_BYTES) {
    throw new Error("Pi launch environment value is invalid");
  }
  return value;
}

function requireAbsolutePath(value: string, label: string): void {
  if (!isAbsolute(value) || normalize(value) !== value || value.includes("\0")) {
    throw new Error(`${label} must be a canonical absolute path`);
  }
}

function boundedTimeout(value: number): number {
  if (!Number.isSafeInteger(value) || value < 1 || value > 60_000) {
    throw new Error("termination timeout is outside its supported bound");
  }
  return value;
}

function signalProcessGroup(pid: number, signal: NodeJS.Signals): void {
  try {
    process.kill(-pid, signal);
  } catch (error) {
    const code = typeof error === "object" && error !== null && "code" in error ? error.code : undefined;
    if (code !== "ESRCH") throw error;
  }
}

async function raceExit(exit: Promise<ProcessExit>, timeoutMs: number): Promise<ProcessExit | undefined> {
  let timer: number | NodeJS.Timeout | undefined;
  try {
    return await Promise.race([
      exit,
      new Promise<undefined>((resolve) => {
        timer = setTimeout(resolve, timeoutMs);
      }),
    ]);
  } finally {
    if (timer) clearTimeout(timer);
  }
}
