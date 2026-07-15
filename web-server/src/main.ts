import Fastify, { type FastifyInstance } from "fastify";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { makeCoreClient, type CoreClient } from "./core-client.js";

const DEFAULT_CORE_ADDR = "http://127.0.0.1:50051";
const DEFAULT_WEB_BIND_ADDR = "127.0.0.1:3000";

export interface WebServerConfig {
  coreAddr: string;
  coreSecret: string;
  bindHost: string;
  bindPort: number;
  operatorId: string;
  tls?: { cert: Buffer; key: Buffer };
}

export interface AppOptions {
  config: WebServerConfig;
  coreClient?: CoreClient;
}

export function loadConfig(env: NodeJS.ProcessEnv = process.env): WebServerConfig {
  const coreSecret = requireNonEmpty(env.PATCHBAY_CORE_SECRET, "PATCHBAY_CORE_SECRET");
  const operatorId = requireNonEmpty(env.PATCHBAY_OPERATOR_ID, "PATCHBAY_OPERATOR_ID");
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
    tls:
      certPath !== undefined && keyPath !== undefined
        ? { cert: readFileSync(certPath), key: readFileSync(keyPath) }
        : undefined,
  };
}

export function buildApp(options: AppOptions): FastifyInstance {
  const app = (options.config.tls
    ? Fastify({ https: options.config.tls })
    : Fastify()) as FastifyInstance;

  // Constructing the client at composition time validates the trust root and
  // keeps the web process ready to translate protocol calls.
  const coreClient =
    options.coreClient ?? makeCoreClient(options.config.coreAddr, options.config.coreSecret);
  app.decorate("coreClient", coreClient);

  app.get("/healthz", async () => ({ status: "ok" }));
  return app;
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
  }
}
