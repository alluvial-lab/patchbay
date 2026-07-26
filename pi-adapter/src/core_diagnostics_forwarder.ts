import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import { timestampFromMs } from "@bufbuild/protobuf/wkt";
import type { AdapterDiagnosticReport, AdapterDiagnosticReportResult } from "@patchbay/contracts";
import {
  AdapterDiagnosticPayloadSchema,
  AdapterDiagnosticReportSchema,
  AdapterDiagnosticSeverity,
  AdapterIdSchema,
  AuthorityDomainIdSchema,
  FailureCode,
  GenerationSchema,
  OperationKind,
  PayloadContentType,
  PayloadEnvelopeSchema,
  TargetScopeKind,
  TargetScopeSchema,
  TypedCorrelationSchema,
} from "@patchbay/contracts";
import type {
  AdapterDiagnosticEvent,
  AdapterDiagnosticInput,
  AdapterDiagnostics,
} from "./adapter_diagnostics.js";

/** The one adapter-owned registry shared by the capability manifest and sink. */
export const PI_FORWARDED_DIAGNOSTIC_CODES = {
  "adapter.started": "pi_adapter_started",
  "adapter.stopping": "pi_adapter_stopping",
  "adapter.attach.failed": "pi_adapter_attach_failed",
  "session.register.failed": "pi_session_register_failed",
  "session.dispose.failed": "pi_session_dispose_failed",
  "delivery.subscription.failed": "pi_delivery_subscription_failed",
  "delivery.subscription.retrying": "pi_delivery_subscription_retrying",
  "delivery.rejected": "pi_delivery_rejected",
  "delivery.failed": "pi_delivery_failed",
  "observation.failed": "pi_observation_failed",
  "observation.flush_failed": "pi_observation_flush_failed",
} as const satisfies Partial<Record<AdapterDiagnosticEvent, string>>;

const DEFAULT_MAX_PENDING = 256;
const DEFAULT_REPORTS_PER_SECOND = 10;
const DEFAULT_REPORT_TIMEOUT_MS = 1_000;
const DEFAULT_MAX_FLUSH_MS = 1_000;
const MAX_COALESCED_COUNT = 1_000;

export interface CoreDiagnosticsForwarderOptions {
  maxPending?: number;
  reportsPerSecond?: number;
  reportTimeoutMs?: number;
  maxFlushMs?: number;
  now?: () => Date;
  delay?: (milliseconds: number) => Promise<void>;
}

interface ForwarderContext {
  authorityDomainId: string;
  adapterId: string;
  adapterGeneration: number;
}

interface PendingReport {
  report: AdapterDiagnosticReport;
  count: number;
}

type ReportDiagnostic = (
  value: AdapterDiagnosticReport,
  signal?: AbortSignal,
) => Promise<AdapterDiagnosticReportResult>;

export class CoreDiagnosticsForwarder implements AdapterDiagnostics {
  readonly #report: ReportDiagnostic;
  readonly #context: ForwarderContext;
  readonly #maxPending: number;
  readonly #intervalMs: number;
  readonly #reportTimeoutMs: number;
  readonly #maxFlushMs: number;
  readonly #now: () => Date;
  readonly #delay: (milliseconds: number) => Promise<void>;
  readonly #pending = new Map<string, PendingReport>();
  #draining: Promise<void> | undefined;
  #lastSentAt: number | undefined;
  #activeAbortController: AbortController | undefined;
  #closed = false;

  constructor(
    report: ReportDiagnostic,
    context: ForwarderContext,
    options: CoreDiagnosticsForwarderOptions = {},
  ) {
    if (!context.authorityDomainId || !context.adapterId || !Number.isSafeInteger(context.adapterGeneration) || context.adapterGeneration <= 0) {
      throw new Error("diagnostic forwarder context is incomplete");
    }
    this.#report = report;
    this.#context = context;
    this.#maxPending = positiveInteger(options.maxPending, DEFAULT_MAX_PENDING);
    const reportsPerSecond = positiveInteger(options.reportsPerSecond, DEFAULT_REPORTS_PER_SECOND);
    this.#intervalMs = Math.max(1, Math.ceil(1_000 / reportsPerSecond));
    this.#reportTimeoutMs = positiveInteger(options.reportTimeoutMs, DEFAULT_REPORT_TIMEOUT_MS);
    this.#maxFlushMs = positiveInteger(options.maxFlushMs, DEFAULT_MAX_FLUSH_MS);
    this.#now = options.now ?? (() => new Date());
    this.#delay = options.delay ?? delay;
  }

  record(input: AdapterDiagnosticInput): void {
    if (this.#closed) return;
    try {
      const report = this.#reportFor(input);
      const key = reportKey(report);
      const existing = this.#pending.get(key);
      if (existing) {
        existing.count = Math.min(MAX_COALESCED_COUNT, existing.count + diagnosticCount(input));
        existing.report = withCount(existing.report, existing.count);
      } else if (this.#pending.size < this.#maxPending) {
        const count = Math.min(MAX_COALESCED_COUNT, diagnosticCount(input));
        this.#pending.set(key, { report: withCount(report, count), count });
      }
      queueMicrotask(() => {
        if (!this.#closed) void this.#ensureDrain();
      });
    } catch {
      // Forwarding is a diagnostic observer. Mapping failures are local-only
      // and must not enter the adapter's control-loop error path.
    }
  }

  async flush(): Promise<void> {
    if (this.#closed && !this.#draining) return;
    const drain = this.#ensureDrain();
    try {
      await withTimeout(drain, this.#maxFlushMs);
    } catch {
      // Flush is deliberately bounded and non-throwing.
    }
  }

  async close(): Promise<void> {
    if (this.#closed) return;
    // Abort before waiting so close cannot leave a Connect call alive behind a
    // bounded flush. The in-flight drain remains sequential and observes the
    // abort through the transport's signal.
    this.#closed = true;
    this.#pending.clear();
    this.#activeAbortController?.abort(new Error("diagnostic forwarder closed"));
    const drain = this.#draining;
    if (drain) {
      try {
        await withTimeout(drain, this.#maxFlushMs);
      } catch {
        // Close is deliberately bounded and non-throwing.
      }
    }
  }

  #reportFor(input: AdapterDiagnosticInput): AdapterDiagnosticReport {
    const code = (PI_FORWARDED_DIAGNOSTIC_CODES as Partial<Record<AdapterDiagnosticEvent, string>>)[input.event];
    if (!code) throw new Error("event is not in the forwarded registry");
    if (input.commandId !== undefined && !input.commandId) throw new Error("command id is empty");
    if (input.session && (!input.session.runtimeSessionId || !input.session.deploymentScope || input.session.generation <= 0)) {
      throw new Error("session diagnostic context is incomplete");
    }
    const severity = severityFor(input.level);
    const failureCode = canonicalFailure(input, severity);
    if (severity !== AdapterDiagnosticSeverity.INFO && failureCode === FailureCode.UNSPECIFIED) {
      throw new Error("warning/error diagnostic has no canonical failure");
    }
    const operationKind = canonicalOperationKind(input.operationKind);
    if (input.commandId && operationKind === OperationKind.UNSPECIFIED) {
      throw new Error("command-correlated diagnostic has no operation kind");
    }
    const correlations = input.commandId
      ? [create(TypedCorrelationSchema, {
          ref: { case: "commandId", value: { value: input.commandId } },
        })]
      : [];
    const targetScope = input.session
      ? create(TargetScopeSchema, {
          kind: TargetScopeKind.RUNTIME_SESSION,
          adapterId: create(AdapterIdSchema, { value: this.#context.adapterId }),
          deploymentScope: input.session.deploymentScope,
          runtimeSessionId: { value: input.session.runtimeSessionId },
          sessionGeneration: { value: BigInt(input.session.generation) },
        })
      : create(TargetScopeSchema, {
          kind: TargetScopeKind.ADAPTER,
          adapterId: create(AdapterIdSchema, { value: this.#context.adapterId }),
        });
    return create(AdapterDiagnosticReportSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: this.#context.authorityDomainId }),
      targetScope,
      correlations,
      observedAt: timestampFromMs(this.#now().getTime()),
      failureCode,
      payload: createPayload(code, operationKind, severity, this.#context.adapterGeneration),
    });
  }

  #ensureDrain(): Promise<void> {
    if (!this.#draining) {
      this.#draining = this.#drain()
        .catch(() => undefined)
        .finally(() => {
          this.#draining = undefined;
          if (this.#pending.size > 0 && !this.#closed) void this.#ensureDrain();
        });
    }
    return this.#draining;
  }

  async #drain(): Promise<void> {
    while (!this.#closed && this.#pending.size > 0) {
      const first = this.#pending.entries().next().value as [string, PendingReport] | undefined;
      if (!first) return;
      this.#pending.delete(first[0]);
      const now = this.#now().getTime();
      const wait = this.#lastSentAt === undefined
        ? 0
        : Math.max(0, this.#intervalMs - (now - this.#lastSentAt));
      if (wait > 0) {
        try {
          await this.#delay(wait);
        } catch {
          return;
        }
      }
      this.#lastSentAt = this.#now().getTime();
      const controller = new AbortController();
      this.#activeAbortController = controller;
      try {
        await reportWithCancellation(
          this.#report,
          first[1].report,
          controller,
          this.#reportTimeoutMs,
        );
      } catch {
        // No retry, no recursive failure report, and no control-loop impact.
      } finally {
        if (this.#activeAbortController === controller) {
          this.#activeAbortController = undefined;
        }
      }
    }
  }
}

export function composeAdapterDiagnostics(
  sinks: readonly AdapterDiagnostics[],
): AdapterDiagnostics {
  return {
    record(input) {
      for (const sink of sinks) {
        try {
          sink.record(input);
        } catch {
          // One sink cannot veto another sink or adapter work.
        }
      }
    },
    async flush() {
      await Promise.all(sinks.map(async (sink) => {
        try {
          await sink.flush();
        } catch {
          // Best effort for every sink.
        }
      }));
    },
    async close() {
      await Promise.all(sinks.map(async (sink) => {
        try {
          await sink.close();
        } catch {
          // Best effort for every sink.
        }
      }));
    },
  };
}

function createPayload(
  code: string,
  operationKind: OperationKind,
  severity: AdapterDiagnosticSeverity,
  generation: number,
) {
  return create(PayloadEnvelopeSchema, {
    payload: toBinary(AdapterDiagnosticPayloadSchema, create(AdapterDiagnosticPayloadSchema, {
      code,
      severity,
      adapterGeneration: create(GenerationSchema, { value: BigInt(generation) }),
      operationKind,
      count: 1,
    })),
    contentType: PayloadContentType.PROTOBUF,
    schemaRef: "patchbay.AdapterDiagnosticPayload",
  });
}

function withCount(report: AdapterDiagnosticReport, count: number): AdapterDiagnosticReport {
  const payload = report.payload;
  if (!payload) return report;
  const decoded = fromBinary(AdapterDiagnosticPayloadSchema, payload.payload);
  const nextPayload = create(PayloadEnvelopeSchema, {
    ...payload,
    payload: toBinary(AdapterDiagnosticPayloadSchema, create(AdapterDiagnosticPayloadSchema, {
      ...decoded,
      count,
    })),
  });
  return create(AdapterDiagnosticReportSchema, { ...report, payload: nextPayload });
}

function reportKey(report: AdapterDiagnosticReport): string {
  const target = report.targetScope;
  const correlation = report.correlations[0]?.ref.case === "commandId"
    ? report.correlations[0].ref.value.value
    : "";
  const payload = report.payload ? fromBinary(AdapterDiagnosticPayloadSchema, report.payload.payload) : undefined;
  return [
    payload?.code,
    payload?.severity,
    payload?.operationKind,
    payload?.adapterGeneration?.value,
    target?.kind,
    target?.deploymentScope,
    target?.runtimeSessionId?.value,
    target?.sessionGeneration?.value,
    correlation,
    report.failureCode,
  ].join("|");
}

function diagnosticCount(input: AdapterDiagnosticInput): number {
  return Number.isSafeInteger(input.count) && input.count !== undefined && input.count > 0
    ? Math.min(MAX_COALESCED_COUNT, input.count)
    : 1;
}

function severityFor(level: AdapterDiagnosticInput["level"]): AdapterDiagnosticSeverity {
  switch (level) {
    case "info": return AdapterDiagnosticSeverity.INFO;
    case "warn": return AdapterDiagnosticSeverity.WARNING;
    case "error": return AdapterDiagnosticSeverity.ERROR;
    default: throw new Error("unknown diagnostic level");
  }
}

function canonicalFailure(
  input: AdapterDiagnosticInput,
  severity: AdapterDiagnosticSeverity,
): FailureCode {
  if (input.failureCode !== undefined && isFailureCode(input.failureCode)) return input.failureCode;
  switch (input.event) {
    case "adapter.attach.failed":
    case "delivery.subscription.failed": return FailureCode.ADAPTER_UNAVAILABLE;
    case "delivery.subscription.retrying": return FailureCode.TRANSPORT_TIMEOUT;
    case "delivery.rejected": return FailureCode.DELIVERY_REJECTED;
    case "delivery.failed": return FailureCode.EXECUTION_FAILED;
    case "session.register.failed":
    case "session.dispose.failed":
    case "observation.failed":
    case "observation.flush_failed": return FailureCode.EXECUTION_FAILED;
    default: return severity === AdapterDiagnosticSeverity.INFO ? FailureCode.UNSPECIFIED : FailureCode.EXECUTION_FAILED;
  }
}

function canonicalOperationKind(value: OperationKind | undefined): OperationKind {
  if (value === undefined) return OperationKind.UNSPECIFIED;
  if (!Number.isInteger(value) || value < OperationKind.UNSPECIFIED || value > OperationKind.SESSION_MANAGEMENT) {
    throw new Error("operation kind is unknown or reserved");
  }
  return value;
}

function isFailureCode(value: FailureCode): boolean {
  return Number.isInteger(value) && value >= FailureCode.UNSPECIFIED && value <= FailureCode.EXECUTION_OUTCOME_UNKNOWN;
}

function positiveInteger(value: number | undefined, fallback: number): number {
  return value !== undefined && Number.isSafeInteger(value) && value > 0 ? value : fallback;
}

async function reportWithCancellation(
  report: ReportDiagnostic,
  value: AdapterDiagnosticReport,
  controller: AbortController,
  timeoutMs: number,
): Promise<void> {
  let timer: NodeJS.Timeout | undefined;
  let timedOut = false;
  const request = report(value, controller.signal);
  try {
    await Promise.race([
      request,
      new Promise<void>((resolve) => {
        timer = setTimeout(() => {
          timedOut = true;
          controller.abort(new Error("diagnostic report timed out"));
          resolve();
        }, timeoutMs);
      }),
    ]);
    if (timedOut) {
      // Connect rejects promptly when its signal is aborted. Awaiting the
      // request here is important: the next sequential report must not begin
      // while the timed-out network call is still active.
      await request;
    }
  } finally {
    if (timer) clearTimeout(timer);
    if (timedOut) controller.abort();
  }
}

function withTimeout<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
  let timer: NodeJS.Timeout | undefined;
  return Promise.race([
    promise,
    new Promise<T>((_, reject) => {
      timer = setTimeout(() => reject(new Error("operation timed out")), timeoutMs);
    }),
  ]).finally(() => {
    if (timer) clearTimeout(timer);
  });
}

function delay(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
