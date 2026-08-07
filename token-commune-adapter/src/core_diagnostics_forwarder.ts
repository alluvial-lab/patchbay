import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import { timestampFromMs } from "@bufbuild/protobuf/wkt";
import {
  AdapterDiagnosticPayloadSchema, AdapterDiagnosticReportSchema,
  AdapterDiagnosticSeverity, AdapterIdSchema, AuthorityDomainIdSchema,
  FailureCode, GenerationSchema, OperationKind, PayloadContentType,
  PayloadEnvelopeSchema, ResourceIdSchema, ResourceKindSchema,
  TargetScopeKind, TargetScopeSchema, TypedCorrelationSchema,
  type AdapterDiagnosticReport, type AdapterDiagnosticReportResult,
} from "@patchbay/contracts";
import type { AdapterDiagnosticEvent, AdapterDiagnosticInput, AdapterDiagnostics } from "./adapter_diagnostics.js";

export const TOKEN_COMMUNE_FORWARDED_DIAGNOSTIC_CODES = {
  "adapter.started": "token_commune_adapter_started",
  "adapter.stopping": "token_commune_adapter_stopping",
  "adapter.attach.failed": "token_commune_adapter_attach_failed",
  "credential.load.failed": "token_commune_credential_load_failed",
  "gateway.auth.failed": "token_commune_gateway_auth_failed",
  "gateway.request.failed": "token_commune_gateway_request_failed",
  "gateway.response.invalid": "token_commune_gateway_response_invalid",
  "delivery.subscription.failed": "token_commune_delivery_subscription_failed",
  "delivery.subscription.retrying": "token_commune_delivery_subscription_retrying",
  "delivery.unsupported": "token_commune_delivery_unsupported",
} as const satisfies Partial<Record<AdapterDiagnosticEvent, string>>;

interface Context { authorityDomainId: string; adapterId: string; adapterGeneration: number }
interface Options {
  maxPending?: number; reportsPerSecond?: number; reportTimeoutMs?: number;
  maxFlushMs?: number; now?: () => Date; delay?: (milliseconds: number) => Promise<void>;
}
type Reporter = (report: AdapterDiagnosticReport, signal?: AbortSignal) => Promise<AdapterDiagnosticReportResult>;
interface Pending { report: AdapterDiagnosticReport; count: number }

export class CoreDiagnosticsForwarder implements AdapterDiagnostics {
  readonly #pending = new Map<string, Pending>();
  readonly #maxPending: number;
  readonly #intervalMs: number;
  readonly #timeoutMs: number;
  readonly #maxFlushMs: number;
  readonly #now: () => Date;
  readonly #delay: (milliseconds: number) => Promise<void>;
  #draining: Promise<void> | undefined;
  #lastSent: number | undefined;
  #active: AbortController | undefined;
  #closed = false;

  constructor(readonly report: Reporter, readonly context: Context, options: Options = {}) {
    if (!context.authorityDomainId || !context.adapterId || !Number.isSafeInteger(context.adapterGeneration) || context.adapterGeneration <= 0) {
      throw new Error("diagnostic forwarder context is incomplete");
    }
    this.#maxPending = positive(options.maxPending, 256);
    this.#intervalMs = Math.max(1, Math.ceil(1_000 / positive(options.reportsPerSecond, 10)));
    this.#timeoutMs = positive(options.reportTimeoutMs, 1_000);
    this.#maxFlushMs = positive(options.maxFlushMs, 1_000);
    this.#now = options.now ?? (() => new Date());
    this.#delay = options.delay ?? ((ms) => new Promise((resolve) => setTimeout(resolve, ms)));
  }

  record(input: AdapterDiagnosticInput): void {
    if (this.#closed) return;
    try {
      const report = this.#map(input);
      const key = reportKey(report);
      const existing = this.#pending.get(key);
      const count = Math.min(1_000, positive(input.count, 1));
      if (existing) {
        existing.count = Math.min(1_000, existing.count + count);
        existing.report = withCount(existing.report, existing.count);
      } else if (this.#pending.size < this.#maxPending) {
        this.#pending.set(key, { report: withCount(report, count), count });
      }
      queueMicrotask(() => { if (!this.#closed) void this.#ensureDrain(); });
    } catch { /* observer-only mapping */ }
  }

  async flush(): Promise<void> {
    try { await withTimeout(this.#ensureDrain(), this.#maxFlushMs); } catch { /* bounded */ }
  }

  async close(): Promise<void> {
    if (this.#closed) return;
    this.#closed = true;
    this.#pending.clear();
    this.#active?.abort(new Error("diagnostic forwarder closed"));
    if (this.#draining) try { await withTimeout(this.#draining, this.#maxFlushMs); } catch { /* bounded */ }
  }

  #map(input: AdapterDiagnosticInput): AdapterDiagnosticReport {
    const code = (TOKEN_COMMUNE_FORWARDED_DIAGNOSTIC_CODES as Partial<Record<AdapterDiagnosticEvent, string>>)[input.event];
    if (!code) throw new Error("diagnostic event is local-only");
    const severity = severityFor(input.level);
    const failureCode = failureFor(input, severity);
    const operationKind = validOperationKind(input.operationKind);
    if (input.commandId && operationKind === OperationKind.UNSPECIFIED) throw new Error("correlated diagnostic lacks operation kind");
    const targetScope = input.resource
      ? create(TargetScopeSchema, {
          kind: TargetScopeKind.RESOURCE,
          resource: {
            adapterId: create(AdapterIdSchema, { value: this.context.adapterId }),
            resourceKind: create(ResourceKindSchema, { value: input.resource.resourceKind }),
            resourceId: create(ResourceIdSchema, { value: input.resource.resourceId }),
          },
        })
      : create(TargetScopeSchema, {
          kind: TargetScopeKind.ADAPTER,
          adapterId: create(AdapterIdSchema, { value: this.context.adapterId }),
        });
    return create(AdapterDiagnosticReportSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: this.context.authorityDomainId }),
      targetScope,
      correlations: input.commandId ? [create(TypedCorrelationSchema, {
        ref: { case: "commandId", value: { value: input.commandId } },
      })] : [],
      observedAt: timestampFromMs(this.#now().getTime()),
      failureCode,
      payload: create(PayloadEnvelopeSchema, {
        payload: toBinary(AdapterDiagnosticPayloadSchema, create(AdapterDiagnosticPayloadSchema, {
          code, severity, adapterGeneration: create(GenerationSchema, { value: BigInt(this.context.adapterGeneration) }),
          operationKind, count: 1,
        })),
        contentType: PayloadContentType.PROTOBUF,
        schemaRef: "patchbay.AdapterDiagnosticPayload",
      }),
    });
  }

  #ensureDrain(): Promise<void> {
    this.#draining ??= this.#drain().catch(() => undefined).finally(() => {
      this.#draining = undefined;
      if (this.#pending.size && !this.#closed) void this.#ensureDrain();
    });
    return this.#draining;
  }

  async #drain(): Promise<void> {
    while (!this.#closed && this.#pending.size) {
      const first = this.#pending.entries().next().value as [string, Pending] | undefined;
      if (!first) return;
      this.#pending.delete(first[0]);
      const now = this.#now().getTime();
      const wait = this.#lastSent === undefined ? 0 : Math.max(0, this.#intervalMs - (now - this.#lastSent));
      if (wait) await this.#delay(wait);
      this.#lastSent = this.#now().getTime();
      const controller = new AbortController();
      this.#active = controller;
      try { await reportWithTimeout(this.report(first[1].report, controller.signal), controller, this.#timeoutMs); }
      catch { /* no retry and no recursive report */ }
      finally { if (this.#active === controller) this.#active = undefined; }
    }
  }
}

export function composeAdapterDiagnostics(sinks: readonly AdapterDiagnostics[]): AdapterDiagnostics {
  return {
    record(input) { for (const sink of sinks) try { sink.record(input); } catch { /* isolate */ } },
    async flush() { await Promise.all(sinks.map((sink) => sink.flush().catch(() => undefined))); },
    async close() { await Promise.all(sinks.map((sink) => sink.close().catch(() => undefined))); },
  };
}

function severityFor(level: AdapterDiagnosticInput["level"]): AdapterDiagnosticSeverity {
  if (level === "info") return AdapterDiagnosticSeverity.INFO;
  if (level === "warn") return AdapterDiagnosticSeverity.WARNING;
  return AdapterDiagnosticSeverity.ERROR;
}
function failureFor(input: AdapterDiagnosticInput, severity: AdapterDiagnosticSeverity): FailureCode {
  if (input.failureCode !== undefined) return input.failureCode;
  switch (input.event) {
    case "adapter.attach.failed": case "delivery.subscription.failed": return FailureCode.ADAPTER_UNAVAILABLE;
    case "delivery.subscription.retrying": return FailureCode.TRANSPORT_TIMEOUT;
    case "delivery.unsupported": return FailureCode.UNSUPPORTED_COMMAND;
    case "gateway.auth.failed": return FailureCode.AUTHORIZATION_DENIED;
    case "credential.load.failed": case "gateway.request.failed": case "gateway.response.invalid": return FailureCode.EXECUTION_FAILED;
    default: return severity === AdapterDiagnosticSeverity.INFO ? FailureCode.UNSPECIFIED : FailureCode.EXECUTION_FAILED;
  }
}
function validOperationKind(value: OperationKind | undefined): OperationKind {
  return value !== undefined && Number.isInteger(value) && value >= OperationKind.UNSPECIFIED && value <= OperationKind.SESSION_MANAGEMENT
    ? value : OperationKind.UNSPECIFIED;
}
function withCount(report: AdapterDiagnosticReport, count: number): AdapterDiagnosticReport {
  if (!report.payload) return report;
  const decoded = fromBinary(AdapterDiagnosticPayloadSchema, report.payload.payload);
  return create(AdapterDiagnosticReportSchema, { ...report, payload: create(PayloadEnvelopeSchema, {
    ...report.payload,
    payload: toBinary(AdapterDiagnosticPayloadSchema, create(AdapterDiagnosticPayloadSchema, { ...decoded, count })),
  }) });
}
function reportKey(report: AdapterDiagnosticReport): string {
  const payload = report.payload ? fromBinary(AdapterDiagnosticPayloadSchema, report.payload.payload) : undefined;
  return [payload?.code, payload?.severity, payload?.operationKind, report.failureCode,
    report.targetScope?.resource?.resourceKind?.value, report.targetScope?.resource?.resourceId?.value,
    report.correlations[0]?.ref.case === "commandId" ? report.correlations[0].ref.value.value : ""].join("|");
}
function positive(value: number | undefined, fallback: number): number {
  return value !== undefined && Number.isSafeInteger(value) && value > 0 ? value : fallback;
}
async function reportWithTimeout(request: Promise<unknown>, controller: AbortController, timeoutMs: number): Promise<void> {
  let timer: NodeJS.Timeout | undefined;
  let timedOut = false;
  try {
    await Promise.race([request, new Promise<void>((resolve) => {
      timer = setTimeout(() => { timedOut = true; controller.abort(new Error("diagnostic report timed out")); resolve(); }, timeoutMs);
    })]);
    if (timedOut) await request;
  } finally { if (timer) clearTimeout(timer); }
}
function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
  let timer: NodeJS.Timeout | undefined;
  return Promise.race([promise, new Promise<T>((_, reject) => {
    timer = setTimeout(() => reject(new Error("operation timed out")), timeoutMs);
  })]).finally(() => { if (timer) clearTimeout(timer); });
}
