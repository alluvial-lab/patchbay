import { resolveAdapterLogPath } from "./adapter_diagnostics.js";
import { requireSafeGatewayBaseUrl } from "./gateway_url.js";

export interface TokenCommuneAdapterConfig {
  coreAddress: string;
  adapterId: string;
  adapterGeneration: number;
  authorityDomainId: string;
  attachmentEvidence: string;
  gatewayBaseUrl: URL;
  gatewayCredentialFile: string;
  pollIntervalMs: number;
  diagnosticPath: string;
}

export function loadTokenCommuneAdapterConfig(
  env: NodeJS.ProcessEnv = process.env,
): TokenCommuneAdapterConfig {
  const coreAddress = httpBaseUrl(required(env, "PATCHBAY_CORE_ADDR"), "PATCHBAY_CORE_ADDR").href.replace(/\/$/, "");
  const attachmentEvidence = required(env, "PATCHBAY_ADAPTER_ATTACHMENT_SECRET");
  const gatewayText = required(env, "PATCHBAY_TOKEN_COMMUNE_GATEWAY_URL");
  const gatewayCredentialFile = required(env, "PATCHBAY_TOKEN_COMMUNE_MEMBER_KEY_FILE");
  const adapterId = nonEmpty(env["PATCHBAY_ADAPTER_ID"] ?? "token-commune", "PATCHBAY_ADAPTER_ID");
  const authorityDomainId = nonEmpty(
    env["PATCHBAY_AUTHORITY_DOMAIN_ID"] ?? "default",
    "PATCHBAY_AUTHORITY_DOMAIN_ID",
  );
  const adapterGeneration = positiveInteger(
    env["PATCHBAY_ADAPTER_GENERATION"] ?? "1",
    "PATCHBAY_ADAPTER_GENERATION",
  );
  const pollIntervalMs = positiveInteger(
    env["PATCHBAY_TOKEN_COMMUNE_POLL_INTERVAL_MS"] ?? "30000",
    "PATCHBAY_TOKEN_COMMUNE_POLL_INTERVAL_MS",
  );
  let gatewayBaseUrl: URL;
  try {
    gatewayBaseUrl = new URL(gatewayText);
  } catch {
    throw new Error("PATCHBAY_TOKEN_COMMUNE_GATEWAY_URL must be an absolute URL");
  }
  requireSafeGatewayBaseUrl(gatewayBaseUrl, "PATCHBAY_TOKEN_COMMUNE_GATEWAY_URL");
  return {
    coreAddress,
    adapterId,
    adapterGeneration,
    authorityDomainId,
    attachmentEvidence,
    gatewayBaseUrl,
    gatewayCredentialFile,
    pollIntervalMs,
    diagnosticPath: resolveAdapterLogPath(env),
  };
}

function required(env: NodeJS.ProcessEnv, name: string): string {
  return nonEmpty(env[name], name);
}

function httpBaseUrl(value: string, name: string): URL {
  let parsed: URL;
  try { parsed = new URL(value); }
  catch { throw new Error(`${name} must be a credential-free http(s) URL`); }
  if (!["http:", "https:"].includes(parsed.protocol) || parsed.username || parsed.password || parsed.search || parsed.hash) {
    throw new Error(`${name} must be a credential-free http(s) URL`);
  }
  return parsed;
}

function nonEmpty(value: string | undefined, name: string): string {
  if (!value?.trim()) throw new Error(`${name} is required`);
  return value.trim();
}

function positiveInteger(value: string, name: string): number {
  if (!/^[1-9][0-9]*$/.test(value)) throw new Error(`${name} must be a positive safe integer`);
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed)) throw new Error(`${name} must be a positive safe integer`);
  return parsed;
}
