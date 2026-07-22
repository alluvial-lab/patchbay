import { homedir } from "node:os";
import { join } from "node:path";
import { createClient, type Client, type Interceptor } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import { AdminService, ControlService } from "@patchbay/contracts";
import { authInterceptor } from "./auth.js";
import type { CredentialReader } from "./credentials.js";

const DEFAULT_CORE_ADDR = "http://127.0.0.1:50051";
const DEFAULT_ADMIN_ADDR = "http://127.0.0.1:50052";
const DEFAULT_AUTHORITY_DOMAIN_ID = "default";

export type ControlClient = Client<typeof ControlService>;
export type AdminClient = Client<typeof AdminService>;

export interface CliConfig {
  coreAddr: string;
  adminAddr: string;
  coreSecret: string;
  authorityDomainId: string;
  credentialPath: string;
}

export function loadConfig(env: NodeJS.ProcessEnv = process.env): CliConfig {
  const coreSecret = env["PATCHBAY_CORE_SECRET"] ?? "";
  assertCoreSecret(coreSecret);
  const authorityDomainId = env["PATCHBAY_AUTHORITY_DOMAIN_ID"] ?? DEFAULT_AUTHORITY_DOMAIN_ID;
  if (!authorityDomainId) throw new Error("PATCHBAY_AUTHORITY_DOMAIN_ID must not be empty");

  return {
    coreAddr: normalizeAddress(env["PATCHBAY_CORE_ADDR"] ?? DEFAULT_CORE_ADDR),
    adminAddr: normalizeAddress(env["PATCHBAY_CORE_ADMIN_ADDR"] ?? DEFAULT_ADMIN_ADDR),
    coreSecret,
    authorityDomainId,
    credentialPath:
      env["PATCHBAY_CREDENTIALS_PATH"] ?? join(homedir(), ".patchbay", "cli-credentials.json"),
  };
}

export function makeControlClient(
  coreAddr: string,
  coreSecret: string,
  credentials?: CredentialReader,
): ControlClient {
  const interceptors = [coreSecretInterceptor(coreSecret)];
  if (credentials) interceptors.push(authInterceptor(credentials));
  return createClient(
    ControlService,
    createGrpcTransport({ baseUrl: normalizeAddress(coreAddr), interceptors }),
  );
}

export function makeAdminClient(adminAddr: string, coreSecret: string): AdminClient {
  const baseUrl = normalizeAddress(adminAddr);
  assertLoopbackAdminAddress(baseUrl);
  return createClient(
    AdminService,
    createGrpcTransport({
      baseUrl,
      interceptors: [coreSecretInterceptor(coreSecret)],
    }),
  );
}

export function coreSecretInterceptor(coreSecret: string): Interceptor {
  assertCoreSecret(coreSecret);
  return (next) => async (request) => {
    request.header.set("x-patchbay-core-secret", coreSecret);
    return next(request);
  };
}

export function assertLoopbackAdminAddress(address: string): void {
  const host = new URL(address).hostname.toLowerCase();
  const loopback =
    host === "localhost" || host === "::1" || host === "[::1]" || /^127(?:\.\d{1,3}){3}$/.test(host);
  if (!loopback) {
    throw new Error("PATCHBAY_CORE_ADMIN_ADDR must use a loopback host");
  }
}

function assertCoreSecret(secret: string): void {
  if (!secret) throw new Error("PATCHBAY_CORE_SECRET must be configured and non-empty");
  if (!/^[\x20-\x7e]+$/.test(secret)) {
    throw new Error("PATCHBAY_CORE_SECRET must contain ASCII metadata characters");
  }
}

function normalizeAddress(address: string): string {
  const normalized = /^[a-z][a-z\d+.-]*:\/\//i.test(address) ? address : `http://${address}`;
  const url = new URL(normalized);
  if (url.protocol !== "http:" && url.protocol !== "https:") {
    throw new Error(`unsupported core address protocol: ${url.protocol}`);
  }
  return url.toString().replace(/\/$/, "");
}
