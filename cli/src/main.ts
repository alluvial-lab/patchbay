#!/usr/bin/env node

import { pathToFileURL } from "node:url";
import { stdin as processStdin } from "node:process";
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
import { interruptCommand } from "./commands/interrupt.js";
import { loginCommand } from "./commands/login.js";
import { logoutCommand } from "./commands/logout.js";
import { sessionHealthCommand } from "./commands/session-health.js";
import { setupCommand } from "./commands/setup.js";

export interface CliOutput {
  stdout(line: string): void;
  stderr(line: string): void;
}

export interface CliRuntime {
  env?: NodeJS.ProcessEnv;
  readStdin?: () => Promise<string>;
}

interface ParsedArguments {
  positionals: string[];
  flags: Set<string>;
  options: Map<string, string>;
}

const BOOLEAN_OPTIONS = new Set(["json"]);
const VALUE_OPTIONS = new Set([
  "setup-secret",
  "operator-id",
  "password",
  "endpoint-id",
  "device-id",
  "idempotency-key",
  "command-id",
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
            setupSecret: optionOrEnv(parsed, "setup-secret", runtime.env, "PATCHBAY_SETUP_SECRET"),
            operatorActorId: optionOrEnv(
              parsed,
              "operator-id",
              runtime.env,
              "PATCHBAY_OPERATOR_ID",
            ),
            password: optionOrEnv(
              parsed,
              "password",
              runtime.env,
              "PATCHBAY_OPERATOR_PASSWORD",
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
            password: optionOrEnv(
              parsed,
              "password",
              runtime.env,
              "PATCHBAY_OPERATOR_PASSWORD",
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

      case "audit-query":
        return auditQueryCommand(output);
      case "inspect-command":
        return inspectCommandCommand(output);
      case "adapter-status":
        return adapterStatusCommand(output);
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
      parsed.flags.add(rawName);
      continue;
    }
    if (!VALUE_OPTIONS.has(rawName)) throw new Error(`unknown option: --${rawName}`);
    const value = inlineValue ?? args[++index];
    if (value === undefined || value.startsWith("--")) {
      throw new Error(`--${rawName} requires a value`);
    }
    parsed.options.set(rawName, value);
  }
  return parsed;
}

export function usage(): string {
  return [
    "Usage: patchbay-cli <command> [options]",
    "",
    "Environment: PATCHBAY_CORE_ADDR, PATCHBAY_CORE_ADMIN_ADDR, PATCHBAY_CORE_SECRET,",
    "             PATCHBAY_AUTHORITY_DOMAIN_ID, PATCHBAY_CREDENTIALS_PATH",
    "",
    "Commands:",
    "  setup --setup-secret S --operator-id ID --password P",
    "      Bootstrap the first operator through the loopback-only admin listener.",
    "  login --operator-id ID --password P",
    "      Authenticate through the throttled core RPC and enroll a fresh CLI endpoint.",
    "  logout",
    "      Revoke the current core-issued operator session and remove local credentials.",
    "  session-health [session-id] [--json]",
    "      Show authoritative connectivity × activity state.",
    "  instruct <target> <prompt|-> [--idempotency-key K] [--command-id ID] [--json]",
    "      Submit an instruction; '-' reads the prompt from stdin.",
    "  cancel <command-id> [--idempotency-key K] [--command-id ID] [--json]",
    "  interrupt <command-id> [--idempotency-key K] [--command-id ID] [--json]",
    "  audit-query        Requires core-diagnostics (stub)",
    "  inspect-command    Requires core-diagnostics (stub)",
    "  adapter-status     Requires core-diagnostics (stub)",
    "",
    "Target may be a unique runtime session id/name or the stable identity printed by",
    "session-health. Secrets may also be supplied via PATCHBAY_SETUP_SECRET,",
    "PATCHBAY_OPERATOR_ID, and PATCHBAY_OPERATOR_PASSWORD.",
  ].join("\n");
}

function optionOrEnv(
  parsed: ParsedArguments,
  option: string,
  env: NodeJS.ProcessEnv | undefined,
  environmentName: string,
): string {
  return parsed.options.get(option) ?? env?.[environmentName] ?? process.env[environmentName] ?? "";
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
