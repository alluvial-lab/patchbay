#!/usr/bin/env node

import { pathToFileURL } from "node:url";
import { stderr as processStderr, stdin as processStdin } from "node:process";
import {
  loadConfig,
  makeAdminClient,
  makeControlClient,
} from "./core-client.js";
import { CredentialStore } from "./credentials.js";
import { adapterStatusCommand } from "./commands/adapter-status.js";
import { auditQueryCommand } from "./commands/audit-query.js";
import { cancelCommand } from "./commands/cancel.js";
import { inspectCommandCommand } from "./commands/inspect-command.js";
import { instructCommand } from "./commands/instruct.js";
import { grantRevokeCommand } from "./commands/grant-revoke.js";
import {
  defaultReasonCode,
  revokeAllSessionsCommand,
  revokeEndpointCommand,
  revokePrincipalCommand,
} from "./commands/revocation.js";
import { interruptCommand } from "./commands/interrupt.js";
import { loginCommand } from "./commands/login.js";
import { lockdownEnterCommand, lockdownExitCommand } from "./commands/lockdown.js";
import { logoutCommand } from "./commands/logout.js";
import { sessionHealthCommand } from "./commands/session-health.js";
import { setupCommand } from "./commands/setup.js";
import { resourceInspectCommand, resourceQueryCommand } from "./commands/resources.js";

export interface CliOutput {
  stdout(line: string): void;
  stderr(line: string): void;
}

export interface CliRuntime {
  env?: NodeJS.ProcessEnv;
  readStdin?: () => Promise<string>;
  readSecret?: (prompt: string) => Promise<string>;
}

interface ParsedArguments {
  positionals: string[];
  flags: Set<string>;
  options: Map<string, string>;
}

const BOOLEAN_OPTIONS = new Set(["json"]);
const DUPLICATE_VALUE_OPTIONS = new Set(["kind", "failure-code", "reason-code"]);
const COMMAND_OPTION_GRAMMAR: Record<string, { flags: readonly string[]; values: readonly string[] }> = {
  setup: { flags: [], values: ["operator-id", "endpoint-id", "device-id"] },
  login: { flags: [], values: ["operator-id", "endpoint-id", "device-id"] },
  logout: { flags: [], values: [] },
  "session-health": { flags: ["json"], values: [] },
  "resource-query": { flags: ["json", "replay-events"], values: ["adapter-id", "provider"] },
  "resource-inspect": { flags: ["json", "replay-events"], values: [] },
  instruct: { flags: ["json"], values: ["idempotency-key", "command-id"] },
  cancel: { flags: ["json"], values: ["idempotency-key", "command-id"] },
  interrupt: { flags: ["json"], values: ["idempotency-key", "command-id"] },
  "grant-revoke": { flags: ["json"], values: ["reason", "confirm"] },
  "lockdown-enter": { flags: ["json"], values: ["reason-code", "confirm"] },
  "lockdown-exit": { flags: ["json"], values: ["reason-code"] },
  "revoke-all-sessions": { flags: ["json"], values: ["reason-code"] },
  "revoke-principal": { flags: ["json"], values: ["reason-code"] },
  "revoke-endpoint": { flags: ["json"], values: ["reason-code"] },
  "revoke-device": { flags: ["json"], values: ["reason-code"] },
  "audit-query": { flags: ["json"], values: [
    "kind", "actor-id", "endpoint-id", "command-id", "grant-id", "target", "failure-code",
    "reason-code", "since", "until", "before-event", "limit",
  ] },
  "inspect-command": { flags: ["json"], values: ["audit-before-event", "audit-limit"] },
  "adapter-status": { flags: ["json"], values: ["after-adapter-id", "limit"] },
};
const VALUE_OPTIONS = new Set([
  "operator-id",
  "adapter-id",
  "provider",
  "endpoint-id",
  "device-id",
  "idempotency-key",
  "command-id",
  "grant-id",
  "kind",
  "actor-id",
  "target",
  "failure-code",
  "reason-code",
  "confirm",
  "since",
  "until",
  "before-event",
  "limit",
  "audit-before-event",
  "audit-limit",
  "after-adapter-id",
  "reason",
]);

export const consoleOutput: CliOutput = {
  stdout: (line) => console.log(line),
  stderr: (line) => console.error(line),
};

export async function run(
  argv: readonly string[],
  output: CliOutput = consoleOutput,
  runtime: CliRuntime = {},
): Promise<number> {
  try {
    const config = loadConfig(runtime.env ?? process.env);
    const command = argv[0];
    if (!command || command === "help" || command === "--help" || command === "-h") {
      output.stdout(usage());
      return 0;
    }

    const parsed = parseArguments(argv.slice(1));
    validateCommandOptions(command, parsed);
    const store = new CredentialStore(config.credentialPath);
    const json = parsed.flags.has("json");

    switch (command) {
      case "setup":
        requirePositionals(command, parsed.positionals, 0, 0);
        return await setupCommand(
          makeAdminClient(config.adminAddr, config.coreSecret),
          store,
          config.authorityDomainId,
          {
            setupSecret: await secretFromEnvOrPrompt(
              runtime,
              "PATCHBAY_SETUP_SECRET",
              "One-time setup secret",
            ),
            operatorActorId: optionOrEnv(
              parsed,
              "operator-id",
              runtime.env,
              "PATCHBAY_OPERATOR_ID",
            ),
            password: await secretFromEnvOrPrompt(
              runtime,
              "PATCHBAY_OPERATOR_PASSWORD",
              "Operator password",
            ),
            endpointId: parsed.options.get("endpoint-id"),
            deviceId: parsed.options.get("device-id"),
          },
          output,
        );

      case "login":
        requirePositionals(command, parsed.positionals, 0, 0);
        return await loginCommand(
          makeControlClient(config.coreAddr, config.coreSecret),
          store,
          config.authorityDomainId,
          {
            operatorActorId: optionOrEnv(
              parsed,
              "operator-id",
              runtime.env,
              "PATCHBAY_OPERATOR_ID",
            ),
            password: await secretFromEnvOrPrompt(
              runtime,
              "PATCHBAY_OPERATOR_PASSWORD",
              "Operator password",
            ),
            endpointId: parsed.options.get("endpoint-id"),
            deviceId: parsed.options.get("device-id"),
          },
          output,
        );

      case "logout":
        requirePositionals(command, parsed.positionals, 0, 0);
        return await logoutCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          store,
          output,
        );

      case "session-health":
        requirePositionals(command, parsed.positionals, 0, 1);
        return await sessionHealthCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          config.authorityDomainId,
          { sessionId: parsed.positionals[0], json },
          output,
        );

      case "resource-query":
        requirePositionals(command, parsed.positionals, 0, 0);
        return await resourceQueryCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          config.authorityDomainId,
          {
            adapterId: parsed.options.get("adapter-id"),
            provider: parsed.options.get("provider"),
            replayEvents: parsed.flags.has("replay-events"),
            json,
          },
          output,
        );

      case "resource-inspect":
        requirePositionals(command, parsed.positionals, 1, 1);
        return await resourceInspectCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          config.authorityDomainId,
          { identity: parsed.positionals[0]!, replayEvents: parsed.flags.has("replay-events"), json },
          output,
        );

      case "instruct": {
        requirePositionals(command, parsed.positionals, 2, 2);
        const prompt =
          parsed.positionals[1] === "-"
            ? stripOneTrailingNewline(await (runtime.readStdin ?? readStdin)())
            : parsed.positionals[1]!;
        return await instructCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          store,
          config.authorityDomainId,
          {
            target: parsed.positionals[0]!,
            prompt,
            json,
            idempotencyKey: parsed.options.get("idempotency-key"),
            commandId: parsed.options.get("command-id"),
          },
          output,
        );
      }

      case "cancel":
        requirePositionals(command, parsed.positionals, 1, 1);
        return await cancelCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          store,
          config.authorityDomainId,
          {
            targetCommandId: parsed.positionals[0]!,
            json,
            idempotencyKey: parsed.options.get("idempotency-key"),
            commandId: parsed.options.get("command-id"),
          },
          output,
        );

      case "interrupt":
        requirePositionals(command, parsed.positionals, 1, 1);
        return await interruptCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          store,
          config.authorityDomainId,
          {
            targetCommandId: parsed.positionals[0]!,
            json,
            idempotencyKey: parsed.options.get("idempotency-key"),
            commandId: parsed.options.get("command-id"),
          },
          output,
        );

      case "revoke-all-sessions":
        requirePositionals(command, parsed.positionals, 0, 0);
        return await revokeAllSessionsCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          store,
          { reasonCode: parsed.options.get("reason-code") ?? defaultReasonCode(), json },
          output,
        );

      case "revoke-principal":
        requirePositionals(command, parsed.positionals, 1, 1);
        return await revokePrincipalCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          store,
          {
            principalId: parsed.positionals[0]!,
            reasonCode: parsed.options.get("reason-code") ?? defaultReasonCode(),
            json,
          },
          output,
        );

      case "revoke-endpoint":
        requirePositionals(command, parsed.positionals, 1, 1);
        return await revokeEndpointCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          store,
          {
            endpointId: parsed.positionals[0]!,
            reasonCode: parsed.options.get("reason-code") ?? defaultReasonCode(),
            json,
          },
          output,
        );

      case "revoke-device":
        requirePositionals(command, parsed.positionals, 1, 1);
        return await revokeEndpointCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          store,
          {
            deviceId: parsed.positionals[0]!,
            reasonCode: parsed.options.get("reason-code") ?? defaultReasonCode(),
            json,
          },
          output,
        );

      case "lockdown-enter":
        requirePositionals(command, parsed.positionals, 0, 0);
        return await lockdownEnterCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          store,
          config.authorityDomainId,
          {
            reasonCode: parsed.options.get("reason-code") ?? defaultReasonCode(),
            confirm: parsed.options.get("confirm") ?? "",
            json,
          },
          output,
        );

      case "lockdown-exit":
        requirePositionals(command, parsed.positionals, 0, 0);
        return await lockdownExitCommand(
          makeAdminClient(config.adminAddr, config.coreSecret),
          config.authorityDomainId,
          { reasonCode: parsed.options.get("reason-code"), json },
          output,
        );

      case "grant-revoke":
        requirePositionals(command, parsed.positionals, 1, 1);
        return await grantRevokeCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          config.authorityDomainId,
          {
            grantId: parsed.positionals[0]!,
            reason: parsed.options.get("reason"),
            confirm: parsed.options.get("confirm"),
            json,
          },
          output,
        );

      case "audit-query":
        requirePositionals(command, parsed.positionals, 0, 0);
        return await auditQueryCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          store,
          config.authorityDomainId,
          {
            kinds: parsed.options.get("kind"),
            actorId: parsed.options.get("actor-id"),
            endpointId: parsed.options.get("endpoint-id"),
            commandId: parsed.options.get("command-id"),
            grantId: parsed.options.get("grant-id"),
            target: parsed.options.get("target"),
            failureCodes: parsed.options.get("failure-code"),
            reasonCodes: parsed.options.get("reason-code"),
            since: parsed.options.get("since"),
            until: parsed.options.get("until"),
            beforeEvent: parsed.options.get("before-event"),
            limit: parsed.options.get("limit"),
            json,
          },
          output,
        );
      case "inspect-command":
        requirePositionals(command, parsed.positionals, 1, 1);
        return await inspectCommandCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          store,
          config.authorityDomainId,
          {
            commandId: parsed.positionals[0]!,
            auditBeforeEvent: parsed.options.get("audit-before-event"),
            auditLimit: parsed.options.get("audit-limit"),
            json,
          },
          output,
        );
      case "adapter-status":
        requirePositionals(command, parsed.positionals, 0, Number.MAX_SAFE_INTEGER);
        return await adapterStatusCommand(
          makeControlClient(config.coreAddr, config.coreSecret, store),
          store,
          config.authorityDomainId,
          {
            adapterIds: parsed.positionals,
            afterAdapterId: parsed.options.get("after-adapter-id"),
            limit: parsed.options.get("limit"),
            json,
          },
          output,
        );
      default:
        throw new Error(`unknown command: ${command}`);
    }
  } catch (error) {
    output.stderr(error instanceof Error ? error.message : String(error));
    return 1;
  }
}

export function parseArguments(args: readonly string[]): ParsedArguments {
  const parsed: ParsedArguments = { positionals: [], flags: new Set(), options: new Map() };
  let optionsEnabled = true;
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index]!;
    if (optionsEnabled && argument === "--") {
      optionsEnabled = false;
      continue;
    }
    if (!optionsEnabled || !argument.startsWith("--")) {
      parsed.positionals.push(argument);
      continue;
    }

    const option = argument.slice(2);
    const equals = option.indexOf("=");
    const rawName = equals === -1 ? option : option.slice(0, equals);
    const inlineValue = equals === -1 ? undefined : option.slice(equals + 1);
    if (BOOLEAN_OPTIONS.has(rawName)) {
      if (inlineValue !== undefined) throw new Error(`--${rawName} does not accept a value`);
      if (parsed.flags.has(rawName)) throw new Error(`duplicate option: --${rawName}`);
      parsed.flags.add(rawName);
      continue;
    }
    if (!VALUE_OPTIONS.has(rawName)) throw new Error(`unknown option: --${rawName}`);
    const value = inlineValue ?? args[++index];
    if (value === undefined || value.startsWith("--")) {
      throw new Error(`--${rawName} requires a value`);
    }
    if (DUPLICATE_VALUE_OPTIONS.has(rawName) && parsed.options.has(rawName)) {
      throw new Error(`duplicate option: --${rawName}`);
    }
    parsed.options.set(rawName, value);
  }
  return parsed;
}

function validateCommandOptions(command: string, parsed: ParsedArguments): void {
  const grammar = COMMAND_OPTION_GRAMMAR[command];
  if (!grammar) return;
  for (const flag of parsed.flags) {
    if (!grammar.flags.includes(flag)) throw new Error(`unknown option: --${flag} for ${command}`);
  }
  for (const option of parsed.options.keys()) {
    if (!grammar.values.includes(option)) throw new Error(`unknown option: --${option} for ${command}`);
  }
}

export function usage(): string {
  return [
    "Usage: patchbay-cli <command> [options]",
    "",
    "Environment: PATCHBAY_CORE_ADDR, PATCHBAY_CORE_ADMIN_ADDR, PATCHBAY_CORE_SECRET,",
    "             PATCHBAY_AUTHORITY_DOMAIN_ID, PATCHBAY_CREDENTIALS_PATH",
    "",
    "Commands:",
    "  setup --operator-id ID",
    "      Bootstrap through the loopback-only admin listener; read secrets from env or a TTY prompt.",
    "  login --operator-id ID",
    "      Authenticate through the throttled core RPC; read the password from env or a TTY prompt.",
    "  logout",
    "      Revoke the current core-issued operator session and remove local credentials.",
    "  session-health [session-id] [--json]",
    "      Show authoritative connectivity × activity state.",
    "  resource-query [--adapter-id ID] [--provider PROVIDER] [--replay-events] [--json]",
    "      Show core-authorized token-commune pool summaries from canonical snapshots.",
    "      --replay-events additionally requires authority-domain query authority.",
    "  resource-inspect <adapter=...;resource-kind=...;resource=...> [--replay-events] [--json]",
    "      Inspect one canonical resource wrapper and its shared safe summary.",
    "      --replay-events additionally requires authority-domain query authority.",
    "  instruct <target> <prompt|-> [--idempotency-key K] [--command-id ID] [--json]",
    "      Submit an instruction; '-' reads the prompt from stdin.",
    "  cancel <command-id> [--idempotency-key K] [--command-id ID] [--json]",
    "  interrupt <command-id> [--idempotency-key K] [--command-id ID] [--json]",
    "  lockdown-enter --confirm LOCKDOWN [--reason-code CODE] [--json]",
    "      Enter lockdown through the authenticated ControlService; confirmed entry clears local credentials.",
    "  lockdown-exit [--reason-code CODE] [--json]",
    "      Exit lockdown only through the loopback AdminService bootstrap channel; never uses stored credentials.",
    "  grant-revoke <grant-id> [--reason TEXT] [--confirm REVOKE_AUTHORITY] [--json]",
    "      Revoke a grant; broad authority-domain grants require explicit high-impact confirmation.",
    "  revoke-all-sessions [--reason-code CODE] [--json]",
    "      Revoke every core operator session for the authenticated actor; local credentials are cleared.",
    "  revoke-principal <principal-id> [--reason-code CODE] [--json]",
    "  revoke-endpoint <endpoint-id> [--reason-code CODE] [--json]",
    "  revoke-device <device-id> [--reason-code CODE] [--json]",
    "      Revoke a control-surface identity; self-targeted credentials are cleared only after confirmation.",
    "  audit-query [--kind K[,K...]] [--actor-id ID] [--endpoint-id ID] [--command-id ID]",
    "      [--grant-id ID] [--target TARGET] [--failure-code C[,C...]] [--reason-code C[,C...]]",
    "      [--since RFC3339] [--until RFC3339] [--before-event LSN] [--limit 1..500] [--json]",
    "      Query redacted audit records. --since is inclusive; --until and --before-event are exclusive.",
    "      TARGET is authority-domain, fleet, actor=ID, adapter=ID, group=VALUE, resource=ID (audit-only),",
    "      adapter=...;scope=...;runtime=...;generation=..., or adapter=...;resource-kind=...;resource=...;",
    "      canonical runtime identity and canonical resource identity components are percent-encoded.",
    "      Enum lists are comma-separated.",
    "  inspect-command <command-id> [--audit-before-event LSN] [--audit-limit 1..200] [--json]",
    "      Inspect command lifecycle and its redacted audit projection; the audit cursor is exclusive.",
    "  adapter-status [adapter-id ...] [--after-adapter-id ID] [--limit 1..500] [--json]",
    "      Show adapter registry status; the opaque adapter cursor is exclusive.",
    "      Empty results are successful (exit 0); exit codes are 0 success, 1 local/transport/protocol error,",
    "      2 rejected before acceptance, 3 failed execution, and 4 unknown submission outcome.",
    "",
    "Target may be a unique runtime session id/name or the stable identity printed by",
    "session-health. Supply secrets with PATCHBAY_SETUP_SECRET and",
    "PATCHBAY_OPERATOR_PASSWORD, or enter them at a non-echoing TTY prompt. Never pass secrets as arguments.",
    "PATCHBAY_OPERATOR_ID may also supply the operator id.",
  ].join("\n");
}

function optionOrEnv(
  parsed: ParsedArguments,
  option: string,
  env: NodeJS.ProcessEnv | undefined,
  environmentName: string,
): string {
  return parsed.options.get(option) ?? environmentValue(env, environmentName) ?? "";
}

async function secretFromEnvOrPrompt(
  runtime: CliRuntime,
  environmentName: string,
  prompt: string,
): Promise<string> {
  const value = environmentValue(runtime.env, environmentName);
  if (value !== undefined) return value;
  return await (runtime.readSecret ?? readSecretFromTty)(prompt);
}

function environmentValue(
  env: NodeJS.ProcessEnv | undefined,
  environmentName: string,
): string | undefined {
  return env?.[environmentName] ?? process.env[environmentName];
}

async function readSecretFromTty(prompt: string): Promise<string> {
  if (!processStdin.isTTY || typeof processStdin.setRawMode !== "function") {
    throw new Error(`${prompt} requires ${promptEnvironmentName(prompt)} or an interactive TTY`);
  }

  processStderr.write(`${prompt}: `);
  return await new Promise<string>((resolve, reject) => {
    let value = "";
    const finish = (result?: string, error?: Error) => {
      processStdin.off("data", onData);
      processStdin.setRawMode(false);
      processStdin.pause();
      processStderr.write("\n");
      if (error) reject(error);
      else resolve(result ?? value);
    };
    const onData = (chunk: Buffer) => {
      for (const character of chunk.toString("utf8")) {
        if (character === "\r" || character === "\n") return finish(value);
        if (character === "\u0003") return finish(undefined, new Error("secret entry cancelled"));
        if (character === "\u007f" || character === "\b") {
          value = value.slice(0, -1);
        } else {
          value += character;
        }
      }
    };
    processStdin.setRawMode(true);
    processStdin.resume();
    processStdin.on("data", onData);
  });
}

function promptEnvironmentName(prompt: string): string {
  return prompt === "One-time setup secret"
    ? "PATCHBAY_SETUP_SECRET"
    : "PATCHBAY_OPERATOR_PASSWORD";
}

function requirePositionals(
  command: string,
  positionals: readonly string[],
  minimum: number,
  maximum: number,
): void {
  if (positionals.length < minimum || positionals.length > maximum) {
    throw new Error(`${command} received ${positionals.length} positional arguments; expected ${minimum === maximum ? minimum : `${minimum}-${maximum}`}`);
  }
}

async function readStdin(): Promise<string> {
  processStdin.setEncoding("utf8");
  let input = "";
  for await (const chunk of processStdin) input += chunk;
  return input;
}

function stripOneTrailingNewline(value: string): string {
  return value.replace(/\r?\n$/, "");
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) {
  process.exitCode = await run(process.argv.slice(2));
}
