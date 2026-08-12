import { Agent, request as httpRequest } from "node:http";
import { Readable } from "node:stream";
import type { GatewayCredential } from "./credential.js";
import { parseGatewayEventKind, type GatewayEventKind } from "./event_observation.js";
import { isLoopbackHttpUrl, requireSafeGatewayBaseUrl } from "./gateway_url.js";

export const GATEWAY_ENDPOINTS = {
  status: "/commune/status", pool: "/commune/pool", me: "/commune/me",
  events: "/commune/events", fingerprints: "/commune/fingerprint", models: "/v1/models",
} as const;
export type GatewayEndpoint = (typeof GATEWAY_ENDPOINTS)[keyof typeof GATEWAY_ENDPOINTS];
export type GatewayErrorKind = "unauthorized" | "forbidden" | "transport" | "timeout" | "http" | "invalid-response";

export interface GatewayBackoffSignal {
  readonly retryAfterMs?: number;
  readonly retryAt?: string;
  readonly invalid?: true;
}

/**
 * One hour is safely below Node's 2^31-1 ms timer ceiling and keeps a malformed
 * upstream delay from silencing an operator-facing observer for days.
 */
export const MAX_RETRY_AFTER_MS = 60 * 60 * 1000;

export class GatewayClientError extends Error {
  readonly name = "GatewayClientError";
  constructor(
    readonly kind: GatewayErrorKind,
    readonly endpoint: GatewayEndpoint,
    readonly status?: number,
    readonly backoff?: GatewayBackoffSignal,
  ) {
    super(status === undefined ? `token-commune gateway ${endpoint} ${kind}` : `token-commune gateway ${endpoint} ${kind} (${status})`);
  }
  toJSON(): { name: string; kind: GatewayErrorKind; endpoint: GatewayEndpoint; status?: number; backoff?: GatewayBackoffSignal } {
    return {
      name: this.name,
      kind: this.kind,
      endpoint: this.endpoint,
      ...(this.status === undefined ? {} : { status: this.status }),
      ...(this.backoff === undefined ? {} : { backoff: this.backoff }),
    };
  }
}

export type GatewayContributionHealth =
  | { readonly state: "fresh" }
  | { readonly state: "exhausted"; readonly exhaustedUntil: string }
  | { readonly state: "auth_broken"; readonly reason: string };
export interface GatewayCapacityReading {
  window: string;
  usedFraction: number | null;
  usedUnits: number | null;
  limitUnits: number | null;
  resetsAt: string | null;
  source: "headers" | "usage_endpoint" | "observed_429" | "declared";
  observedAt: string;
}
export interface GatewayStatusContribution {
  contributionId: string;
  provider: string;
  readings: readonly GatewayCapacityReading[];
}
export interface GatewayStatus {
  ok: boolean;
  anthropicHealth: GatewayContributionHealth;
  contributions: readonly GatewayStatusContribution[];
}
export type GatewayStatusSummary = GatewayStatus;
export interface GatewayPoolFingerprint {
  state: "ok" | "held" | "unknown";
  templateSource: "compiled" | "override";
  since: string | null;
  diffPresent: boolean;
}
export interface GatewayPoolContribution {
  provider: string;
  declaredShare: number;
  health: GatewayContributionHealth;
  capacity: readonly GatewayCapacityReading[];
  fingerprint: GatewayPoolFingerprint;
}
export interface GatewayPool { contributions: readonly GatewayPoolContribution[] }
export interface GatewayDrawReport {
  provider: string;
  limitFraction: number;
  fromDecree: boolean;
  consumedUnits: number;
  drawUnits: number | null;
  exceeded: boolean;
  enforceable: boolean;
  resetsAt: string | null;
}
export interface GatewayMe { displayName: string; reports: readonly GatewayDrawReport[] }
export interface GatewayEvent {
  id: string; occurredAt: string; kind: GatewayEventKind; provider: string;
  contributionId: string | null; message: string;
}
export interface GatewayEventsPage { events: readonly GatewayEvent[]; historyMode: "latest-50-no-cursor" }
export interface GatewayFingerprintState {
  templateSource: string | null; capturedAt: string | null; capturePresent: boolean;
  holdReason: string | null; heldAt: string | null; diffPresent: boolean;
}
export interface GatewayFingerprintSummary { anthropic: GatewayFingerprintState; codex: GatewayFingerprintState }
export type GatewayFingerprints = GatewayFingerprintSummary;
export interface GatewayModel {
  id: string; provider: string; surface: string; upstreamModel: string | null;
  contextWindow: number; maxTokens: number; reasoning: boolean; available: boolean;
}
export interface GatewayModels { models: readonly GatewayModel[] }

export interface TokenCommuneGatewayClient {
  getStatus(signal?: AbortSignal): Promise<GatewayStatus>;
  getPool(signal?: AbortSignal): Promise<GatewayPool>;
  getMe(signal?: AbortSignal): Promise<GatewayMe>;
  getEvents(signal?: AbortSignal): Promise<GatewayEventsPage>;
  getFingerprints(signal?: AbortSignal): Promise<GatewayFingerprints>;
  getModels(signal?: AbortSignal): Promise<GatewayModels>;
}

export function createHttpTokenCommuneGatewayClient(options: {
  baseUrl: URL; credential: GatewayCredential; fetch?: typeof globalThis.fetch;
  maxResponseBytes?: number; requestTimeoutMs?: number; now?: () => Date;
  redactionSecrets?: readonly string[];
}): TokenCommuneGatewayClient {
  const maximum = options.maxResponseBytes ?? 1024 * 1024;
  const requestTimeoutMs = options.requestTimeoutMs ?? 10_000;
  if (!Number.isSafeInteger(maximum) || maximum <= 0) throw new Error("maxResponseBytes must be a positive safe integer");
  if (!Number.isSafeInteger(requestTimeoutMs) || requestTimeoutMs <= 0) throw new Error("requestTimeoutMs must be a positive safe integer");
  const base = new URL(options.baseUrl.href);
  requireSafeGatewayBaseUrl(base, "gateway base URL");
  if (!base.pathname.endsWith("/")) base.pathname += "/";
  const fetcher = options.fetch ?? (isLoopbackHttpUrl(base) ? directLoopbackHttpFetch : globalThis.fetch);

  const get = async <T>(endpoint: GatewayEndpoint, decode: (value: unknown) => T, signal?: AbortSignal): Promise<T> => {
    const headers = new Headers({ Accept: "application/json" });
    options.credential.apply(headers);
    const requestSignal = signal
      ? AbortSignal.any([signal, AbortSignal.timeout(requestTimeoutMs)])
      : AbortSignal.timeout(requestTimeoutMs);
    let response: Response;
    try {
      response = await fetcher(new URL(endpoint.slice(1), base), { method: "GET", headers, redirect: "error", signal: requestSignal });
    } catch (error) {
      if (requestSignal.aborted || (error instanceof DOMException && error.name === "AbortError")) throw new GatewayClientError("timeout", endpoint);
      throw new GatewayClientError("transport", endpoint);
    }
    if (response.status === 401) throw new GatewayClientError("unauthorized", endpoint, 401);
    if (response.status === 403) throw new GatewayClientError("forbidden", endpoint, 403);
    if (response.status >= 300 && response.status < 400) throw new GatewayClientError("http", endpoint, response.status);
    if (!response.ok) {
      const backoff = response.status === 429 || response.status >= 500
        ? parseRetryAfter(response.headers.get("retry-after"), options.now?.() ?? new Date())
        : undefined;
      throw new GatewayClientError("http", endpoint, response.status, backoff);
    }
    try {
      const text = await boundedText(response, maximum);
      rejectCredentialReflection(text, [...options.credential.redactionSecrets(), ...(options.redactionSecrets ?? [])]);
      return deepFreeze(decode(JSON.parse(text) as unknown));
    } catch (error) {
      if (error instanceof GatewayClientError) throw error;
      throw new GatewayClientError("invalid-response", endpoint, response.status);
    }
  };

  return Object.freeze({
    getStatus: (signal?: AbortSignal) => get(GATEWAY_ENDPOINTS.status, decodeStatus, signal),
    getPool: (signal?: AbortSignal) => get(GATEWAY_ENDPOINTS.pool, decodePool, signal),
    getMe: (signal?: AbortSignal) => get(GATEWAY_ENDPOINTS.me, decodeMe, signal),
    getEvents: (signal?: AbortSignal) => get(GATEWAY_ENDPOINTS.events, decodeEvents, signal),
    getFingerprints: (signal?: AbortSignal) => get(GATEWAY_ENDPOINTS.fingerprints, decodeFingerprints, signal),
    getModels: (signal?: AbortSignal) => get(GATEWAY_ENDPOINTS.models, decodeModels, signal),
  });
}

const directLoopbackHttpAgent = new Agent();

const directLoopbackHttpFetch: typeof globalThis.fetch = async (input, init) => {
  const url = input instanceof URL
    ? input
    : typeof input === "string"
      ? new URL(input)
      : new URL(input.url);
  if (!isLoopbackHttpUrl(url)) throw new Error("direct loopback transport requires a loopback HTTP URL");
  if (init?.body) throw new Error("direct loopback transport does not accept request bodies");

  return new Promise<Response>((resolve, reject) => {
    const headers = new Headers(init?.headers);
    const request = httpRequest(url, {
      method: init?.method ?? "GET",
      headers: Object.fromEntries(headers.entries()),
      agent: directLoopbackHttpAgent,
      signal: init?.signal ?? undefined,
    }, (incoming) => {
      const status = incoming.statusCode;
      if (status === undefined) {
        incoming.destroy();
        reject(new Error("loopback gateway returned no HTTP status"));
        return;
      }
      const responseHeaders = new Headers();
      for (let index = 0; index < incoming.rawHeaders.length; index += 2) {
        const name = incoming.rawHeaders[index];
        const value = incoming.rawHeaders[index + 1];
        if (name !== undefined && value !== undefined) responseHeaders.append(name, value);
      }
      const hasBody = init?.method !== "HEAD" && ![204, 205, 304].includes(status);
      if (!hasBody) incoming.resume();
      try {
        resolve(new Response(
          hasBody ? Readable.toWeb(incoming) as unknown as BodyInit : null,
          { status, statusText: incoming.statusMessage ?? "", headers: responseHeaders },
        ));
      } catch (error) {
        incoming.destroy();
        reject(error);
      }
    });
    request.once("error", reject);
    request.end();
  });
};

async function boundedText(response: Response, maximum: number): Promise<string> {
  const declared = response.headers.get("content-length");
  if (declared !== null && (!/^\d+$/.test(declared) || Number(declared) > maximum)) throw new Error("response too large");
  if (!response.body) {
    const bytes = new Uint8Array(await response.arrayBuffer());
    if (bytes.byteLength > maximum) throw new Error("response too large");
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  }
  const reader = response.body.getReader();
  const chunks: Uint8Array[] = [];
  let length = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      length += value.byteLength;
      if (length > maximum) { await reader.cancel(); throw new Error("response too large"); }
      chunks.push(value);
    }
  } finally { reader.releaseLock(); }
  const bytes = new Uint8Array(length);
  let offset = 0;
  for (const chunk of chunks) { bytes.set(chunk, offset); offset += chunk.byteLength; }
  return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
}

function rejectCredentialReflection(text: string, secrets: readonly string[]): void {
  for (const secret of secrets.filter(Boolean)) {
    const forms = [
      secret,
      encodeURIComponent(secret),
      Buffer.from(secret).toString("base64"),
      JSON.stringify(secret),
    ];
    if (forms.some((form) => text.includes(form))) {
      throw new Error("gateway response reflected credential material");
    }
  }
}

function decodeStatus(value: unknown): GatewayStatus {
  const root = object(value);
  return {
    ok: boolean(root, "ok"),
    anthropicHealth: health(root, "anthropicHealth"),
    contributions: array(root, "contributions").map((item) => {
      const row = object(item);
      return {
        contributionId: string(row, "contributionId"),
        provider: canonicalProvider(row, "provider"),
        readings: capacityArray(row, "readings"),
      };
    }),
  };
}
function decodePool(value: unknown): GatewayPool {
  const root = object(value);
  return { contributions: array(root, "providers").map((item) => {
    const row = object(item);
    return {
      provider: canonicalProvider(row, "provider"), declaredShare: fraction(row, "declaredShare"),
      health: health(row, "health"), capacity: capacityArray(row, "capacity"),
      fingerprint: poolFingerprint(row["fingerprint"]),
    };
  }) };
}
function decodeMe(value: unknown): GatewayMe {
  const root = object(value);
  return { displayName: string(root, "member"), reports: array(root, "draw").map((item) => {
    const row = object(item);
    return {
      provider: canonicalProvider(row, "provider"), limitFraction: fraction(row, "limitFraction"), fromDecree: boolean(row, "fromDecree"),
      consumedUnits: nonnegative(row, "consumedUnits"), drawUnits: nullableNumber(row, "drawUnits"), exceeded: boolean(row, "exceeded"),
      enforceable: boolean(row, "enforceable"), resetsAt: nullableEpochTimestamp(row, "resetsAt"),
    };
  }) };
}
function decodeEvents(value: unknown): GatewayEventsPage {
  const root = object(value);
  const events = array(root, "events");
  if (events.length > 50) throw new Error("too many events");
  return { historyMode: "latest-50-no-cursor", events: events.map((item) => {
    const row = object(item);
    return { id: string(row, "id"), occurredAt: epochTimestamp(row, "at"), kind: parseGatewayEventKind(row["kind"]), provider: canonicalProvider(row, "provider"), contributionId: nullableString(row, "contributionId"), message: boundedString(row, "message", 1024) };
  }) };
}
function decodeFingerprints(value: unknown): GatewayFingerprints {
  const root = object(value);
  return { anthropic: fingerprint(root["anthropic"]), codex: fingerprint(root["openai-codex"]) };
}
function decodeModels(value: unknown): GatewayModels {
  const root = object(value);
  const rows = root["models"] ?? root["data"];
  if (!Array.isArray(rows)) throw new Error("models must be an array");
  return { models: rows.map((item) => {
    const row = object(item);
    return {
      id: string(row, "id"), provider: canonicalProvider(row, "provider"), surface: string(row, "surface"),
      upstreamModel: row["upstream_model"] === undefined ? null : string(row, "upstream_model"),
      contextWindow: positiveNumber(row, "context_window"), maxTokens: positiveNumber(row, "max_tokens"),
      reasoning: boolean(row, "reasoning"), available: boolean(row, "available"),
    };
  }) };
}
function capacityArray(row: Record<string, unknown>, key: string): GatewayCapacityReading[] {
  return array(row, key).map((item) => {
    const reading = object(item);
    return {
      window: string(reading, "window"), usedFraction: nullableFraction(reading, "usedFraction"),
      usedUnits: nullableNumber(reading, "usedUnits"), limitUnits: nullableNumber(reading, "limitUnits"),
      resetsAt: nullableEpochTimestamp(reading, "resetsAt"), source: capacitySource(reading, "source"),
      observedAt: epochTimestamp(reading, "observedAt"),
    };
  });
}
function poolFingerprint(value: unknown): GatewayPoolFingerprint {
  const row = object(value);
  const state = enumString(row, "state", ["ok", "held", "unknown"] as const);
  const templateSource = enumString(row, "templateSource", ["compiled", "override"] as const);
  const diff = nullableObject(row, "diff");
  return { state, templateSource, since: nullableEpochTimestamp(row, "since"), diffPresent: diff !== null };
}
function fingerprint(value: unknown): GatewayFingerprintState {
  const row = object(value);
  const templateSource = enumString(row, "templateSource", ["compiled", "override"] as const);
  const capture = nullableObject(row, "lastCapture");
  const lastDiff = nullableObject(row, "lastDiff");
  const hold = nullableObject(row, "hold");
  const holdDiff = hold ? nullableObject(hold, "diff") : null;
  return {
    templateSource,
    capturedAt: nullableEpochTimestamp(row, "lastCaptureAt"),
    capturePresent: capture !== null,
    holdReason: hold ? boundedString(hold, "reason", 512) : null,
    heldAt: hold ? epochTimestamp(hold, "since") : null,
    diffPresent: lastDiff !== null || holdDiff !== null,
  };
}
function object(value: unknown): Record<string, unknown> { if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("expected object"); return value as Record<string, unknown>; }
function nullableObject(row: Record<string, unknown>, key: string): Record<string, unknown> | null {
  return row[key] === null ? null : object(row[key]);
}
function array(row: Record<string, unknown>, key: string): unknown[] { const value = row[key]; if (!Array.isArray(value)) throw new Error(`${key} must be an array`); return value; }
function string(row: Record<string, unknown>, key: string): string { return boundedString(row, key, 512); }
function boundedString(row: Record<string, unknown>, key: string, max: number): string { const value = row[key]; if (typeof value !== "string" || !value.trim() || value.length > max) throw new Error(`${key} must be a bounded string`); return value; }
function canonicalProvider(row: Record<string, unknown>, key: string): string { return boundedString(row, key, 512).trim(); }
function nullableString(row: Record<string, unknown>, key: string): string | null { return row[key] === null ? null : string(row, key); }
function boolean(row: Record<string, unknown>, key: string): boolean { if (typeof row[key] !== "boolean") throw new Error(`${key} must be boolean`); return row[key]; }
function number(row: Record<string, unknown>, key: string): number { const value = row[key]; if (typeof value !== "number" || !Number.isFinite(value)) throw new Error(`${key} must be finite`); return value; }
function nonnegative(row: Record<string, unknown>, key: string): number { const value = number(row, key); if (value < 0) throw new Error(`${key} must be nonnegative`); return value; }
function positiveNumber(row: Record<string, unknown>, key: string): number { const value = number(row, key); if (value <= 0) throw new Error(`${key} must be positive`); return value; }
function fraction(row: Record<string, unknown>, key: string): number { const value = number(row, key); if (value < 0 || value > 1) throw new Error(`${key} must be a fraction`); return value; }
function nullableNumber(row: Record<string, unknown>, key: string): number | null { return row[key] === null ? null : nonnegative(row, key); }
function nullableFraction(row: Record<string, unknown>, key: string): number | null { return row[key] === null ? null : fraction(row, key); }
function epochTimestamp(row: Record<string, unknown>, key: string): string {
  const value = nonnegative(row, key);
  if (!Number.isSafeInteger(value)) throw new Error(`${key} must be an epoch-millisecond timestamp`);
  const timestamp = new Date(value);
  if (!Number.isFinite(timestamp.getTime())) throw new Error(`${key} must be an epoch-millisecond timestamp`);
  return timestamp.toISOString();
}
function nullableEpochTimestamp(row: Record<string, unknown>, key: string): string | null {
  return row[key] === null ? null : epochTimestamp(row, key);
}
function health(row: Record<string, unknown>, key: string): GatewayContributionHealth {
  const value = object(row[key]);
  const state = value["state"];
  if (state === "fresh") return { state };
  if (state === "exhausted") {
    const exhaustedUntil = string(value, "exhaustedUntil");
    if (!Number.isFinite(Date.parse(exhaustedUntil))) throw new Error(`${key}.exhaustedUntil must be a timestamp`);
    return { state, exhaustedUntil };
  }
  if (state === "auth_broken") {
    return { state, reason: boundedString(value, "reason", 512) };
  }
  throw new Error(`${key} is unknown`);
}
function capacitySource(row: Record<string, unknown>, key: string): GatewayCapacityReading["source"] {
  return enumString(row, key, ["headers", "usage_endpoint", "observed_429", "declared"] as const);
}
const HTTP_DATE_PATTERNS = [
  /^(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun), [0-9]{2} (?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) [0-9]{4} [0-9]{2}:[0-9]{2}:[0-9]{2} GMT$/,
  /^(?:Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday), [0-9]{2}-(?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)-[0-9]{2} [0-9]{2}:[0-9]{2}:[0-9]{2} GMT$/,
  /^(?:Mon|Tue|Wed|Thu|Fri|Sat|Sun) (?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) (?: [1-9]|[12][0-9]|3[01]) [0-9]{2}:[0-9]{2}:[0-9]{2} [0-9]{4}$/,
] as const;

function parseRetryAfter(value: string | null, now: Date): GatewayBackoffSignal | undefined {
  if (value === null) return undefined;
  const trimmed = value.trim();
  if (/^[0-9]+$/.test(trimmed)) {
    const seconds = BigInt(trimmed);
    const maximumSeconds = BigInt(Math.floor(MAX_RETRY_AFTER_MS / 1000));
    return seconds > maximumSeconds
      ? { retryAfterMs: MAX_RETRY_AFTER_MS, invalid: true }
      : { retryAfterMs: Number(seconds) * 1000 };
  }
  if (!HTTP_DATE_PATTERNS.some((pattern) => pattern.test(trimmed))) return { invalid: true };
  const milliseconds = Date.parse(trimmed);
  if (!Number.isFinite(milliseconds) || !Number.isFinite(now.getTime())) return { invalid: true };
  if (milliseconds - now.getTime() > MAX_RETRY_AFTER_MS) {
    return { retryAfterMs: MAX_RETRY_AFTER_MS, invalid: true };
  }
  return { retryAt: new Date(milliseconds).toISOString() };
}
function enumString<const T extends readonly string[]>(row: Record<string, unknown>, key: string, allowed: T): T[number] {
  const value = row[key];
  if (typeof value !== "string" || !allowed.includes(value)) throw new Error(`${key} is unknown`);
  return value as T[number];
}
function deepFreeze<T>(value: T): T { if (value && typeof value === "object") { Object.freeze(value); for (const nested of Object.values(value as Record<string, unknown>)) deepFreeze(nested); } return value; }
