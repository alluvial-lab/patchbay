import cookie from "@fastify/cookie";
import Fastify, { type FastifyInstance } from "fastify";
import { readFileSync } from "node:fs";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { makeCoreClient, type CoreClient } from "./core-client.js";
import { LoginLimiter } from "./login-limiter.js";
import { registerCsrfTokenRoute } from "./routes/csrf-token.js";
import {
  type PasswordVerifier,
  registerSessionRoutes,
} from "./routes/login.js";
import { registerRpcRoutes } from "./routes/rpc.js";
import { assertPasswordHash, SessionStore } from "./sessions.js";

const DEFAULT_CORE_ADDR = "http://127.0.0.1:50051";
const DEFAULT_WEB_BIND_ADDR = "127.0.0.1:3000";

export interface WebServerConfig {
  coreAddr: string;
  coreSecret: string;
  bindHost: string;
  bindPort: number;
  operatorId: string;
  operatorPasswordHash: string;
  tls?: { cert: Buffer; key: Buffer };
}

export interface AppOptions {
  config: WebServerConfig;
  coreClient?: CoreClient;
  sessions?: SessionStore;
  loginLimiter?: LoginLimiter;
  passwordVerifier?: PasswordVerifier;
  cockpitAssetsDir?: string;
  logger?: boolean;
}

export function loadConfig(env: NodeJS.ProcessEnv = process.env): WebServerConfig {
  const coreSecret = requireNonEmpty(env.PATCHBAY_CORE_SECRET, "PATCHBAY_CORE_SECRET");
  const operatorId = requireNonEmpty(env.PATCHBAY_OPERATOR_ID, "PATCHBAY_OPERATOR_ID");
  const operatorPasswordHash = requireNonEmpty(
    env.PATCHBAY_OPERATOR_PASSWORD_HASH,
    "PATCHBAY_OPERATOR_PASSWORD_HASH",
  );
  assertPasswordHash(operatorPasswordHash);
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
    operatorId,
    operatorPasswordHash,
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

  assertPasswordHash(options.config.operatorPasswordHash);
  const sessions = options.sessions ?? new SessionStore();
  app.decorate("sessions", sessions);
  app.register(cookie);
  registerSessionRoutes(
    app,
    sessions,
    {
      actorId: options.config.operatorId,
      passwordHash: options.config.operatorPasswordHash,
    },
    {
      loginLimiter: options.loginLimiter,
      passwordVerifier: options.passwordVerifier,
    },
  );
  registerCsrfTokenRoute(app);
  registerRpcRoutes(app, options.config.coreSecret);
  registerCockpitAssets(
    app,
    options.cockpitAssetsDir ?? fileURLToPath(new URL("../../../web-cockpit/dist/", import.meta.url)),
  );

  app.get("/healthz", async () => ({ status: "ok" }));
  return app;
}

function registerCockpitAssets(app: FastifyInstance, root: string): void {
  const assets = new Map<string, { file: string; contentType: string }>([
    ["/", { file: "index.html", contentType: "text/html; charset=utf-8" }],
    ["/assets/cockpit.js", { file: "assets/cockpit.js", contentType: "text/javascript; charset=utf-8" }],
    ["/assets/tokens.css", { file: "assets/tokens.css", contentType: "text/css; charset=utf-8" }],
    ["/assets/components.css", { file: "assets/components.css", contentType: "text/css; charset=utf-8" }],
    ["/assets/markdown.css", { file: "assets/markdown.css", contentType: "text/css; charset=utf-8" }],
    ["/assets/shell.css", { file: "assets/shell.css", contentType: "text/css; charset=utf-8" }],
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

export async function run(env: NodeJS.ProcessEnv = process.env): Promise<void> {
  const config = loadConfig(env);
  const app = buildApp({ config });
  await app.listen({ host: config.bindHost, port: config.bindPort });
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
    sessions: SessionStore;
  }
}
