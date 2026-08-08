import { createServer, type Server } from "node:http";
import { once } from "node:events";
import type { AddressInfo } from "node:net";
import { GATEWAY_ENDPOINTS, type GatewayEndpoint } from "../../src/gateway_client.js";

export interface ScriptedGatewayResponse {
  status: number;
  body: unknown;
  headers?: Readonly<Record<string, string>>;
}

export interface ScriptedGatewayStep {
  responses: Readonly<Record<GatewayEndpoint, ScriptedGatewayResponse>>;
  expectedAuthorization: string;
}

export class ScriptedTokenCommuneGateway {
  readonly requests: Array<{ step: number; endpoint: GatewayEndpoint; authorization: string | null }> = [];
  #steps: readonly ScriptedGatewayStep[] = [];
  #step = 0;
  #server: Server | undefined;

  async start(steps: readonly ScriptedGatewayStep[]): Promise<URL> {
    if (this.#server) throw new Error("scripted gateway is already running");
    if (steps.length === 0) throw new Error("scripted gateway requires at least one step");
    this.#steps = steps;
    this.#step = 0;
    const endpoints = new Set<string>(Object.values(GATEWAY_ENDPOINTS));
    this.#server = createServer((request, response) => {
      const endpoint = request.url?.split("?", 1)[0] as GatewayEndpoint | undefined;
      const step = this.#steps[this.#step];
      if (request.method !== "GET" || !endpoint || !endpoints.has(endpoint) || !step) {
        response.writeHead(404).end();
        return;
      }
      const authorization = request.headers.authorization ?? null;
      this.requests.push({ step: this.#step, endpoint, authorization });
      if (authorization !== step.expectedAuthorization) {
        response.writeHead(401, { "content-type": "application/json" }).end(JSON.stringify({ error: "unauthorized" }));
        return;
      }
      const scripted = step.responses[endpoint];
      response.writeHead(scripted.status, { "content-type": "application/json", ...(scripted.headers ?? {}) });
      response.end(JSON.stringify(scripted.body));
    });
    this.#server.listen(0, "127.0.0.1");
    await once(this.#server, "listening");
    const address = this.#server.address() as AddressInfo;
    return new URL(`http://127.0.0.1:${address.port}/`);
  }

  advance(): void {
    if (this.#step + 1 >= this.#steps.length) throw new Error("scripted gateway is already at its final step");
    this.#step += 1;
  }

  async close(): Promise<void> {
    const server = this.#server;
    this.#server = undefined;
    if (!server) return;
    server.close();
    await once(server, "close");
  }
}
