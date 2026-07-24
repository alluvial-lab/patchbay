import cookie from "@fastify/cookie";
import { Code, ConnectError } from "@connectrpc/connect";
import Fastify, { type FastifyInstance } from "fastify";
import { readFileSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  CorePrincipalStore,
  makeCoreClient,
  type CoreClient,
} from "./core-client.js";
import { LoginLimiter } from "./login-limiter.js";
import { registerCsrfTokenRoute } from "./routes/csrf-token.js";
import {
  type OperatorAuthenticator,
  type OperatorSessionRevoker,
  type PasswordVerifier,
  registerSessionRoutes,
} from "./routes/login.js";
import { registerRpcRoutes } from "./routes/rpc.js";
import { assertPasswordHash, SessionStore } from "./sessions.js";

const DEFAULT_CORE_ADDR = "http://127.0.0.1:50051";
const DEFAULT_WEB_BIND_ADDR = "127.0.0.1:3000";
const DEFAULT_AUTHORITY_DOMAIN_ID = "default";

export interface WebServerConfig {
  coreAddr: string;
  coreSecret: string;
  bindHost: string;
  bindPort: number;
  authorityDomainId: string;
  operatorId: string;
  operatorPasswordHash?: string;
  principalEndpointId?: string;
  principalDeviceId?: string;
  principalGeneration?: bigint;
  trustedLoopbackProxy?: boolean;
  tls?: { cert: Buffer; key: Buffer };
}

export interface AppOptions {
  config: WebServerConfig;
  coreClient?: CoreClient;
  sessions?: SessionStore;
  loginLimiter?: LoginLimiter;
  passwordVerifier?: PasswordVerifier;
  operatorAuthenticator?: OperatorAuthenticator;
  corePrincipals?: CorePrincipalStore;
  cockpitAssetsDir?: string;
  logger?: boolean;
}

export function loadConfig(env: NodeJS.ProcessEnv = process.env): WebServerConfig {
  const coreSecret = requireNonEmpty(env.PATCHBAY_CORE_SECRET, "PATCHBAY_CORE_SECRET");
  const operatorId = requireNonEmpty(env.PATCHBAY_OPERATOR_ID, "PATCHBAY_OPERATOR_ID");
  const authorityDomainId = requireNonEmpty(
    env.PATCHBAY_AUTHORITY_DOMAIN_ID ?? DEFAULT_AUTHORITY_DOMAIN_ID,
    "PATCHBAY_AUTHORITY_DOMAIN_ID",
  );
  const operatorPasswordHash = optionalNonEmpty(
    env.PATCHBAY_OPERATOR_PASSWORD_HASH,
    "PATCHBAY_OPERATOR_PASSWORD_HASH",
  );
  if (operatorPasswordHash) assertPasswordHash(operatorPasswordHash);
  const principalGeneration = parsePositiveBigInt(
    env.PATCHBAY_WEB_ENDPOINT_GENERATION ?? "1",
    "PATCHBAY_WEB_ENDPOINT_GENERATION",
  );
  const { host: bindHost, port: bindPort } = parseBindAddress(
    env.PATCHBAY_WEB_BIND_ADDR ?? DEFAULT_WEB_BIND_ADDR,
  );

  const certPath = env.PATCHBAY_TLS_CERT;
  const keyPath = env.PATCHBAY_TLS_KEY;
  if ((certPath === undefined) !== (keyPath === undefined)) {
    throw new Error("PATCHBAY_TLS_CERT and PATCHBAY_TLS_KEY must be configured together");
  }

  return {
    coreAddr: env.PATCHBAY_CORE_ADDR ?? DEFAULT_CORE_ADDR,
    coreSecret,
    bindHost,
    bindPort,
    authorityDomainId,
    operatorId,
    operatorPasswordHash,
    principalEndpointId: env.PATCHBAY_WEB_ENDPOINT_ID ?? "patchbay-web-server",
    principalDeviceId: env.PATCHBAY_WEB_DEVICE_ID ?? "patchbay-web-host",
    principalGeneration,
    trustedLoopbackProxy: parseBoolean(
      env.PATCHBAY_TRUST_LOOPBACK_PROXY,
      "PATCHBAY_TRUST_LOOPBACK_PROXY",
    ),
    tls:
      certPath !== undefined && keyPath !== undefined
        ? { cert: readFileSync(certPath), key: readFileSync(keyPath) }
        : undefined,
  };
}

export function buildApp(options: AppOptions): FastifyInstance {
  const logger = options.logger ?? true;
  const app = (options.config.tls
    ? Fastify({ https: options.config.tls, logger })
    : Fastify({ logger })) as FastifyInstance;

  // Constructing the client at composition time validates the trust root and
  // keeps the web process ready to translate protocol calls.
  const coreClient =
    options.coreClient ?? makeCoreClient(options.config.coreAddr, options.config.coreSecret);
  app.decorate("coreClient", coreClient);

  if (options.config.operatorPasswordHash) {
    assertPasswordHash(options.config.operatorPasswordHash);
  }
  const corePrincipals = options.corePrincipals ?? new CorePrincipalStore();
  app.decorate("corePrincipals", corePrincipals);
  const sessions = options.sessions ?? new SessionStore();
  app.decorate("sessions", sessions);
  app.register(cookie);
  registerSessionRoutes(
    app,
    sessions,
    {
      actorId: options.config.operatorId,
      passwordHash: options.config.operatorPasswordHash ?? "",
    },
    {
      loginLimiter: options.loginLimiter,
      passwordVerifier: options.passwordVerifier,
      operatorAuthenticator:
        options.operatorAuthenticator ??
        (options.passwordVerifier
          ? undefined
          : coreOperatorAuthenticator(options.config, coreClient, corePrincipals)),
      operatorSessionRevoker: coreOperatorSessionRevoker(
        options.config,
        coreClient,
        corePrincipals,
      ),
      trustedLoopbackProxy: options.config.trustedLoopbackProxy,
    },
  );
  const secureTransport = { trustedLoopbackProxy: options.config.trustedLoopbackProxy };
  registerCsrfTokenRoute(app, secureTransport);
  registerRpcRoutes(app, options.config.coreSecret, secureTransport);
  registerCockpitAssets(
    app,
    options.cockpitAssetsDir ?? fileURLToPath(new URL("../../../web-cockpit/dist/", import.meta.url)),
    options.config.authorityDomainId,
  );

  app.get("/healthz", async () => ({ status: "ok" }));
  return app;
}

function registerCockpitAssets(
  app: FastifyInstance,
  root: string,
  authorityDomainId: string,
): void {
  app.get("/", async (_request, reply) => {
    try {
      const source = await readFile(join(root, "index.html"), "utf8");
      const body = source.replaceAll(
        "__PATCHBAY_AUTHORITY_DOMAIN_ID__",
        escapeHtmlAttribute(authorityDomainId),
      );
      return reply
        .header("content-type", "text/html; charset=utf-8")
        .header("x-content-type-options", "nosniff")
        .send(body);
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code === "ENOENT") {
        return reply.code(503).send({ error: "cockpit_assets_unavailable" });
      }
      throw error;
    }
  });

  const assets = new Map<string, { file: string; contentType: string }>([
    ["/assets/cockpit.js", { file: "assets/cockpit.js", contentType: "text/javascript; charset=utf-8" }],
    ["/assets/tokens.css", { file: "assets/tokens.css", contentType: "text/css; charset=utf-8" }],
    ["/assets/components.css", { file: "assets/components.css", contentType: "text/css; charset=utf-8" }],
    ["/assets/markdown.css", { file: "assets/markdown.css", contentType: "text/css; charset=utf-8" }],
    ["/assets/shell.css", { file: "assets/shell.css", contentType: "text/css; charset=utf-8" }],
    ["/assets/login.css", { file: "assets/login.css", contentType: "text/css; charset=utf-8" }],
  ]);
  for (const [route, asset] of assets) {
    app.get(route, async (_request, reply) => {
      try {
        const body = await readFile(join(root, asset.file));
        return reply
          .header("content-type", asset.contentType)
          .header("x-content-type-options", "nosniff")
          .send(body);
      } catch (error) {
        if ((error as NodeJS.ErrnoException).code === "ENOENT") {
          return reply.code(503).send({ error: "cockpit_assets_unavailable" });
        }
        throw error;
      }
    });
  }
}

function escapeHtmlAttribute(value: string): string {
  return value.replace(/[&<>"']/g, (character) => ({
    "&": "&amp;",
    "<": "&lt;",
    ">": "&gt;",
    '"': "&quot;",
    "'": "&#39;",
  })[character]!);
}

export async function run(env: NodeJS.ProcessEnv = process.env): Promise<void> {
  const config = loadConfig(env);
  const app = buildApp({ config });
  await app.listen({ host: config.bindHost, port: config.bindPort });
}

function coreOperatorAuthenticator(
  config: WebServerConfig,
  coreClient: CoreClient,
  principals: CorePrincipalStore,
): OperatorAuthenticator {
  return async (password) => {
    try {
      const result = await coreClient.verifyOperatorPassword({
        operatorActorId: { value: config.operatorId },
        password,
        principal: {
          endpointId: { value: config.principalEndpointId ?? "patchbay-web-server" },
          deviceId: { value: config.principalDeviceId ?? "patchbay-web-host" },
          endpointGeneration: { value: config.principalGeneration ?? 1n },
        },
      });
      if (!result.principal || !result.operatorSessionId?.value) {
        throw new Error("core password verification returned incomplete principal/session evidence");
      }
      principals.set(result.principal);
      const actorId = result.principal.operatorActorId?.value;
      return actorId
        ? { actorId, coreSessionId: result.operatorSessionId.value }
        : null;
    } catch (error) {
      const rpcError = ConnectError.from(error, Code.Internal);
      if (rpcError.code === Code.Unauthenticated) return null;
      throw error;
    }
  };
}

function coreOperatorSessionRevoker(
  config: WebServerConfig,
  coreClient: CoreClient,
  principals: CorePrincipalStore,
): OperatorSessionRevoker {
  return async (coreSessionId) => {
    const principal = principals.get();
    if (!principal) throw new Error("control-surface principal enrollment is required");
    const headers = new Headers();
    headers.set("x-patchbay-core-secret", config.coreSecret);
    headers.set("x-patchbay-principal-id", principal.principalId);
    headers.set("x-patchbay-principal-secret", principal.secret);
    headers.set("x-patchbay-operator-id", principal.operatorActorId?.value ?? config.operatorId);
    headers.set("x-patchbay-operator-session-id", coreSessionId);
    await coreClient.revokeOperatorSession({}, { headers });
  };
}

function optionalNonEmpty(value: string | undefined, name: string): string | undefined {
  if (value === undefined) return undefined;
  if (value.length === 0) throw new Error(`${name} must not be empty when configured`);
  return value;
}

function parseBoolean(value: string | undefined, name: string): boolean {
  if (value === undefined) return false;
  if (value === "true") return true;
  if (value === "false") return false;
  throw new Error(`${name} must be true or false when configured`);
}

function parsePositiveBigInt(value: string, name: string): bigint {
  if (!/^[0-9]+$/.test(value)) throw new Error(`${name} must be a positive integer`);
  const parsed = BigInt(value);
  if (parsed <= 0n) throw new Error(`${name} must be a positive integer`);
  return parsed;
}

function requireNonEmpty(value: string | undefined, name: string): string {
  if (value === undefined || value.length === 0) {
    throw new Error(`${name} is required; refusing to start without it`);
  }
  return value;
}

function parseBindAddress(value: string): { host: string; port: number } {
  const parsed = new URL(`http://${value}`);
  if (parsed.pathname !== "/" || parsed.search || parsed.hash || parsed.username || parsed.password) {
    throw new Error("PATCHBAY_WEB_BIND_ADDR must have host:port form");
  }
  const port = Number(parsed.port);
  if (!parsed.hostname || !Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error("PATCHBAY_WEB_BIND_ADDR must contain a valid host and port");
  }
  return { host: parsed.hostname.replace(/^\[|\]$/g, ""), port };
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  run().catch((error: unknown) => {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  });
}

declare module "fastify" {
  interface FastifyInstance {
    coreClient: CoreClient;
    corePrincipals: CorePrincipalStore;
    sessions: SessionStore;
  }
}
