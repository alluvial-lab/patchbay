import type { Timestamp } from "@bufbuild/protobuf/wkt";
import { timestampFromDate } from "@bufbuild/protobuf/wkt";
import { Code, ConnectError } from "@connectrpc/connect";
import type { EventId, Observation, ResourceReport } from "@patchbay/contracts";
import type { AdapterDiagnostics } from "./adapter_diagnostics.js";
import { diagnosticError, NOOP_ADAPTER_DIAGNOSTICS } from "./adapter_diagnostics.js";
import { mapEventGap, mapPoolEvent } from "./event_observation.js";
import { LatestEventWindowTracker } from "./event_window.js";
import {
  GatewayClientError,
  type GatewayBackoffSignal,
  type GatewayEventsPage,
  type TokenCommuneGatewayClient,
} from "./gateway_client.js";
import type { ResourceIdentitySynthesizer, SynthesizedResourceIdentity } from "./identity.js";
import { TOKEN_COMMUNE_RESOURCE_KINDS } from "./resource_contract.js";
import { projectTokenCommuneSnapshot, type EndpointSnapshot } from "./snapshot_projection.js";

export interface PollClock {
  now(): Date;
}

export interface PollWaiter {
  wait(milliseconds: number, signal: AbortSignal): Promise<void>;
}

export interface PollerCoreSink {
  ingestResourceReport(report: ResourceReport): Promise<EventId | undefined>;
  ingestEvent(observation: Observation): Promise<EventId | undefined>;
}

export interface TokenCommunePollerOptions {
  adapterId: string;
  adapterGeneration: number;
  authorityDomainId: string;
  pollIntervalMs: number;
  gateway: TokenCommuneGatewayClient;
  core: PollerCoreSink;
  identities: ResourceIdentitySynthesizer;
  diagnostics?: AdapterDiagnostics;
  clock?: PollClock;
  waiter?: PollWaiter;
}

interface SettledCycle {
  readonly status: PromiseSettledResult<Awaited<ReturnType<TokenCommuneGatewayClient["getStatus"]>>>;
  readonly pool: PromiseSettledResult<Awaited<ReturnType<TokenCommuneGatewayClient["getPool"]>>>;
  readonly me: PromiseSettledResult<Awaited<ReturnType<TokenCommuneGatewayClient["getMe"]>>>;
  readonly fingerprints: PromiseSettledResult<Awaited<ReturnType<TokenCommuneGatewayClient["getFingerprints"]>>>;
  readonly models: PromiseSettledResult<Awaited<ReturnType<TokenCommuneGatewayClient["getModels"]>>>;
  readonly events: PromiseSettledResult<GatewayEventsPage>;
}

export class TokenCommunePoller {
  readonly #clock: PollClock;
  readonly #waiter: PollWaiter;
  readonly #diagnostics: AdapterDiagnostics;
  readonly #tracker = new LatestEventWindowTracker();
  readonly #acknowledgedGapTargets = new Map<string, Set<string>>();

  constructor(readonly options: TokenCommunePollerOptions) {
    if (
      !options.adapterId.trim()
      || !options.authorityDomainId.trim()
      || !Number.isSafeInteger(options.adapterGeneration)
      || options.adapterGeneration <= 0
      || !Number.isSafeInteger(options.pollIntervalMs)
      || options.pollIntervalMs <= 0
    ) throw new Error("poller identity, generation, and positive interval are required");
    this.#clock = options.clock ?? { now: () => new Date() };
    this.#waiter = options.waiter ?? SYSTEM_POLLER_WAITER;
    this.#diagnostics = options.diagnostics ?? NOOP_ADAPTER_DIAGNOSTICS;
  }

  async run(signal: AbortSignal): Promise<void> {
    while (!signal.aborted) {
      const { nextDelayMs } = await this.pollOnce(signal);
      if (signal.aborted) return;
      await this.#waiter.wait(nextDelayMs, signal);
    }
  }

  async pollOnce(signal: AbortSignal): Promise<{ nextDelayMs: number }> {
    if (signal.aborted) throw abortReason(signal);
    const cycle = await this.#collect(signal);
    if (signal.aborted) throw abortReason(signal);
    const completedAt = this.#clock.now();
    const observedAt = checkedTimestamp(completedAt);
    const nextDelayMs = this.#nextDelay(cycle, completedAt);
    const report = projectTokenCommuneSnapshot({
      adapterId: this.options.adapterId,
      adapterGeneration: this.options.adapterGeneration,
      observedAt,
      identities: this.options.identities,
      gateway: {
        status: endpointSnapshot(cycle.status),
        pool: endpointSnapshot(cycle.pool),
        me: endpointSnapshot(cycle.me),
        fingerprints: endpointSnapshot(cycle.fingerprints),
        models: endpointSnapshot(cycle.models),
      },
    });

    try {
      requireAcknowledgement(await this.options.core.ingestResourceReport(report), "resource report");
    } catch (error) {
      if (isRetryableCoreFailure(error)) return { nextDelayMs };
      throw error;
    }

    if (cycle.events.status === "fulfilled") {
      try {
        await this.#emitEventPlan(cycle.events.value, report, observedAt);
      } catch (error) {
        if (isRetryableCoreFailure(error)) return { nextDelayMs };
        throw error;
      }
    }
    return { nextDelayMs };
  }

  async #collect(signal: AbortSignal): Promise<SettledCycle> {
    const [status, pool, me, fingerprints, models, events] = await Promise.allSettled([
      this.options.gateway.getStatus(signal),
      this.options.gateway.getPool(signal),
      this.options.gateway.getMe(signal),
      this.options.gateway.getFingerprints(signal),
      this.options.gateway.getModels(signal),
      this.options.gateway.getEvents(signal),
    ]);
    const cycle = { status, pool, me, fingerprints, models, events };
    for (const result of Object.values(cycle)) {
      if (result.status === "rejected") {
        if (!(result.reason instanceof GatewayClientError)) throw result.reason;
        this.#record({
          event: result.reason.kind === "unauthorized" || result.reason.kind === "forbidden"
            ? "gateway.auth.failed"
            : result.reason.kind === "invalid-response"
              ? "gateway.response.invalid"
              : "gateway.request.failed",
          level: result.reason.kind === "unauthorized" || result.reason.kind === "forbidden" ? "error" : "warn",
          error: diagnosticError(result.reason),
        });
        if (result.reason.backoff?.invalid) this.#record({ event: "poll.retry_after.invalid", level: "warn" });
      }
    }
    return cycle;
  }

  #nextDelay(cycle: SettledCycle, completedAt: Date): number {
    let advised = 0;
    for (const result of Object.values(cycle)) {
      if (result.status !== "rejected" || !(result.reason instanceof GatewayClientError)) continue;
      advised = Math.max(advised, backoffMilliseconds(result.reason.backoff, completedAt));
    }
    return Math.max(this.options.pollIntervalMs, advised);
  }

  async #emitEventPlan(page: GatewayEventsPage, report: ResourceReport, detectedAt: Timestamp): Promise<void> {
    const plan = this.#tracker.plan(page);
    if (plan.gap) {
      const targets = currentProviderTargets(report, page, this.options.identities);
      const observations = mapEventGap({
        authorityDomainId: this.options.authorityDomainId,
        adapterId: this.options.adapterId,
        targets,
        detectedAt,
        gap: plan.gap,
      });
      if (observations.length === 0) {
        this.#record({ event: "event.gap.detected", level: "warn" });
      } else {
        const acknowledged = this.#rememberGapTargets(plan.gap.key);
        for (const observation of observations) {
          const targetKey = observation.targetScope?.resource?.resourceId?.value;
          if (!targetKey) throw new Error("gap observation is missing its resource target");
          if (acknowledged.has(targetKey)) continue;
          requireAcknowledgement(await this.options.core.ingestEvent(observation), "event gap");
          acknowledged.add(targetKey);
        }
      }
      this.#tracker.acknowledgeGap(plan.gap.key);
      this.#acknowledgedGapTargets.delete(plan.gap.key);
    }

    for (const event of plan.events) {
      const mapped = mapPoolEvent({
        authorityDomainId: this.options.authorityDomainId,
        adapterId: this.options.adapterId,
        identities: this.options.identities,
        event,
      });
      if (mapped.status === "declared-but-unemitted") {
        this.#record({
          event: "event.declared_only",
          level: "info",
          resource: {
            resourceKind: TOKEN_COMMUNE_RESOURCE_KINDS.providerPool,
            resourceId: this.options.identities.providerPool(event.provider).resourceId,
          },
        });
        this.#tracker.consumeDeclaredOnly(event.id);
        continue;
      }
      requireAcknowledgement(await this.options.core.ingestEvent(mapped.observation), "pool event");
      this.#tracker.acknowledgeEvent(event.id);
    }
    this.#tracker.commitWindow(page);
  }

  #rememberGapTargets(key: string): Set<string> {
    const existing = this.#acknowledgedGapTargets.get(key);
    if (existing) return existing;
    const created = new Set<string>();
    this.#acknowledgedGapTargets.set(key, created);
    while (this.#acknowledgedGapTargets.size > 2) {
      const oldest = this.#acknowledgedGapTargets.keys().next().value as string | undefined;
      if (oldest === undefined) break;
      this.#acknowledgedGapTargets.delete(oldest);
    }
    return created;
  }

  #record(input: Parameters<AdapterDiagnostics["record"]>[0]): void {
    try { this.#diagnostics.record(input); } catch { /* diagnostics cannot veto polling */ }
  }
}

const SYSTEM_POLLER_WAITER: PollWaiter = {
  wait(milliseconds, signal) {
    if (signal.aborted) return Promise.resolve();
    return new Promise((resolve) => {
      let timer: NodeJS.Timeout;
      const finish = () => {
        clearTimeout(timer);
        signal.removeEventListener("abort", finish);
        resolve();
      };
      timer = setTimeout(finish, milliseconds);
      signal.addEventListener("abort", finish, { once: true });
      if (signal.aborted) finish();
    });
  },
};

function endpointSnapshot<T>(result: PromiseSettledResult<T>): EndpointSnapshot<T> {
  return result.status === "fulfilled"
    ? { status: "reported", value: result.value }
    : { status: "unavailable" };
}

function checkedTimestamp(date: Date): Timestamp {
  if (!Number.isFinite(date.getTime())) throw new Error("poll clock returned an invalid date");
  return timestampFromDate(date);
}

function backoffMilliseconds(signal: GatewayBackoffSignal | undefined, completedAt: Date): number {
  if (!signal || signal.invalid) return 0;
  const delta = signal.retryAfterMs ?? 0;
  const absolute = signal.retryAt === undefined ? 0 : Math.max(0, Date.parse(signal.retryAt) - completedAt.getTime());
  const result = Math.max(delta, absolute);
  return Number.isSafeInteger(result) && result >= 0 ? result : 0;
}

function requireAcknowledgement(eventId: EventId | undefined, name: string): void {
  if (!eventId?.authorityDomainId?.value || eventId.lsn?.value === undefined) {
    throw new Error(`core ${name} acknowledgement is missing its event id`);
  }
}

function isRetryableCoreFailure(error: unknown): boolean {
  return error instanceof ConnectError && [
    Code.Canceled,
    Code.Aborted,
    Code.DeadlineExceeded,
    Code.ResourceExhausted,
    Code.Unavailable,
  ].includes(error.code);
}

function currentProviderTargets(
  report: ResourceReport,
  page: GatewayEventsPage,
  identities: ResourceIdentitySynthesizer,
): SynthesizedResourceIdentity[] {
  const targets = new Map<string, SynthesizedResourceIdentity>();
  if (report.report.case === "snapshot") {
    for (const view of report.report.value.views) {
      if (view.resourceKind?.value !== TOKEN_COMMUNE_RESOURCE_KINDS.providerPool) continue;
      for (const mutation of view.mutations) {
        const identity = mutation.identity;
        if (!identity?.adapterId?.value || !identity.resourceKind?.value || !identity.resourceId?.value) continue;
        const target: SynthesizedResourceIdentity = {
          adapterId: identity.adapterId.value,
          resourceKind: TOKEN_COMMUNE_RESOURCE_KINDS.providerPool,
          resourceId: identity.resourceId.value,
        };
        targets.set(target.resourceId, target);
      }
    }
  }
  for (const event of page.events) {
    const target = identities.providerPool(event.provider);
    targets.set(target.resourceId, target);
  }
  return [...targets.values()];
}

function abortReason(signal: AbortSignal): Error {
  return signal.reason instanceof Error ? signal.reason : new DOMException("polling aborted", "AbortError");
}
