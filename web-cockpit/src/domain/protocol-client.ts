import { createClient, type Client, type Interceptor, type Transport } from "@connectrpc/connect";
import { createGrpcWebTransport } from "@connectrpc/connect-web";
import { ControlService } from "@patchbay/contracts";

const CSRF_HEADER = "x-patchbay-csrf";

export type ControlServiceClient = Client<typeof ControlService>;

export interface ProtocolClientOptions {
  baseUrl?: string;
  csrfToken?: () => string | undefined;
  fetch?: typeof globalThis.fetch;
}

export interface ProtocolClient {
  client: ControlServiceClient;
  transport: Transport;
}

/**
 * Creates the browser client for the web-server's binary gRPC-Web bridge.
 * Sender fields in submitted Operations are never treated as authority; the
 * web server replaces them from its verified operator-session record.
 */
export function createProtocolClient(options: ProtocolClientOptions = {}): ProtocolClient {
  const interceptors: Interceptor[] = [];
  if (options.csrfToken) interceptors.push(csrfInterceptor(options.csrfToken));

  const transport = createGrpcWebTransport({
    baseUrl: options.baseUrl ?? "/",
    fetch: options.fetch,
    interceptors,
    useBinaryFormat: true,
  });
  return { client: createClient(ControlService, transport), transport };
}

/** Fetches the proof required by the web server before a state-changing Submit. */
export async function fetchCsrfToken(
  fetcher: typeof globalThis.fetch = globalThis.fetch,
  url = "/csrf-token",
): Promise<string> {
  const response = await fetcher(url, {
    credentials: "same-origin",
    headers: { accept: "application/json" },
  });
  if (!response.ok) throw new Error(`CSRF token request failed (${response.status})`);
  const body: unknown = await response.json();
  if (!isCsrfResponse(body)) throw new Error("CSRF token response is malformed");
  return body.csrfToken;
}

function csrfInterceptor(readToken: () => string | undefined): Interceptor {
  return (next) => async (request) => {
    if (request.method.name === "submit") {
      const token = readToken();
      if (!token) throw new Error("Submit requires a session-bound CSRF token");
      request.header.set(CSRF_HEADER, token);
    }
    return next(request);
  };
}

function isCsrfResponse(value: unknown): value is { csrfToken: string } {
  return (
    typeof value === "object" &&
    value !== null &&
    "csrfToken" in value &&
    typeof value.csrfToken === "string" &&
    value.csrfToken.length > 0
  );
}
