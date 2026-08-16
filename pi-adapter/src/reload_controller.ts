import { randomBytes } from "node:crypto";
import { fromBinary, create } from "@bufbuild/protobuf";
import {
  FailureCode,
  OperationKind,
  PayloadContentType,
  PiProcessReplacementOnlyKind,
  PiReconfigureOutcome,
  PiReconfigureRequestSchema,
  PiReconfigureResultSchema,
  PiReloadableResourceKind,
  type Operation,
  type PiReconfigureResult,
  type RuntimeGenerationRef,
} from "@patchbay/contracts";
import {
  PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE,
  PATCHBAY_CONTROL_RELOAD_COMMAND,
  PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE,
  PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE,
} from "../extensions/patchbay-control.js";
import type { PiEntryReconciler } from "./entry_reconciler.js";
import type { ManagedPiRuntimePort, PiRpcRuntime } from "./pi_process.js";
import type { RpcPiSession } from "./pi_session.js";
import { classifyPiSessionMaterialization, type PiSessionMaterialization } from "./session_file.js";
import {
  RuntimeActionBusyError,
  RuntimeActionFencedError,
  type SettledRuntimeSnapshot,
} from "./runtime_action_gate.js";

const PI_RECONFIGURE_SCHEMA_REF = "patchbay.PiReconfigureRequest";
const COMMAND_ID_PATTERN = /^[A-Za-z0-9._:-]{1,128}$/u;
const BASE64URL_PATTERN = /^[A-Za-z0-9_-]+$/u;
const EPOCH_LENGTH = 22;
const DEFAULT_MAX_MARKER_POLLS = 40;
const DEFAULT_POLL_INTERVAL_MS = 25;
const ADMITTED_RELOAD_RESOURCES = new Set<PiReloadableResourceKind>([
  PiReloadableResourceKind.EXTENSION_ENTRYPOINT,
  PiReloadableResourceKind.SKILL,
  PiReloadableResourceKind.PROMPT,
  PiReloadableResourceKind.THEME,
  PiReloadableResourceKind.CONTEXT_FILE,
]);

export type PiReloadRejectionReason =
  | "busy_direct_rpc"
  | "busy_delivery"
  | "busy_streaming"
  | "busy_compacting"
  | "busy_queued"
  | "busy_unsettled"
  | "materialization_required"
  | "stale_runtime"
  | "invalid_request";

export class PiReloadRejectedError extends Error {
  readonly failureCode = FailureCode.DELIVERY_REJECTED;
  readonly reason: PiReloadRejectionReason;
  readonly diagnostic: string;

  constructor(reason: PiReloadRejectionReason) {
    super("Pi resource reload rejected before effect");
    this.name = "PiReloadRejectedError";
    this.reason = reason;
    this.diagnostic = `pi_reload_${reason}`;
  }
}

export class PiReloadAmbiguousError extends Error {
  readonly failureCode = FailureCode.EXECUTION_OUTCOME_UNKNOWN;
  readonly diagnostic = "pi_reload_rehydration_outcome_unknown";

  constructor() {
    super("Pi resource reload may have occurred but rehydration is incomplete");
    this.name = "PiReloadAmbiguousError";
  }
}

export interface ReloadController {
  reloadEnumeratedResources(
    operation: Operation,
    runtime: PiRpcRuntime,
  ): Promise<PiReconfigureResult>;
}

export interface PiReloadControllerOptions {
  readonly session: RpcPiSession;
  readonly runtimePort: ManagedPiRuntimePort;
  readonly reconciler: PiEntryReconciler;
  readonly runtimeReference: RuntimeGenerationRef;
  readonly logicalTargetId: string;
  readonly configuredSessionRoot: string;
  readonly expectedProjectCwd: string;
  readonly hasConflictingDelivery: () => boolean;
  readonly markRehydrating: () => Promise<void>;
  readonly markRehydrated: () => Promise<void>;
  readonly maxMarkerPolls?: number;
  readonly pollIntervalMs?: number;
  readonly randomBytes?: (size: number) => Uint8Array;
  readonly sleep?: (milliseconds: number) => Promise<void>;
}

interface ReloadCorrelation {
  readonly commandId: string;
  readonly nonce: string;
  readonly priorExtensionEpoch: string;
  readonly resources: readonly PiReloadableResourceKind[];
}

interface RpcEntries {
  readonly entries: readonly unknown[];
  readonly leafId: string | null;
}

interface ReloadMarkerPair {
  readonly requestEntryId: string;
  readonly completionEntryId: string;
  readonly extensionEpoch: string;
}

/** Idle-only, same-process reload followed by complete in-generation rehydration. */
export class PiReloadController implements ReloadController {
  readonly #options: PiReloadControllerOptions;

  constructor(options: PiReloadControllerOptions) {
    this.#options = options;
  }

  async reloadEnumeratedResources(
    operation: Operation,
    runtime: PiRpcRuntime,
  ): Promise<PiReconfigureResult> {
    const request = decodeReloadRequest(operation);
    const unsupported = request.reloadResources.some(
      (resource) => !ADMITTED_RELOAD_RESOURCES.has(resource),
    );
    if (unsupported || request.reloadResources.length === 0) {
      return processReplacementRequired();
    }
    if (new Set(request.reloadResources).size !== request.reloadResources.length) {
      throw new PiReloadRejectedError("invalid_request");
    }

    const commandId = operation.commandId?.value;
    if (!commandId || !COMMAND_ID_PATTERN.test(commandId)) {
      throw new PiReloadRejectedError("invalid_request");
    }
    if (runtime !== this.#options.session.runtimeForReload()) {
      throw new PiReloadRejectedError("stale_runtime");
    }

    try {
      return await this.#options.session.actionGate.withExclusiveCurrent(
        runtime,
        async (snapshot) => this.#reloadOwned(operation, runtime, snapshot, request.reloadResources),
      );
    } catch (error) {
      if (error instanceof RuntimeActionBusyError) {
        throw new PiReloadRejectedError("busy_direct_rpc");
      }
      if (error instanceof RuntimeActionFencedError) {
        throw new PiReloadRejectedError("stale_runtime");
      }
      throw error;
    }
  }

  async #reloadOwned(
    operation: Operation,
    runtime: PiRpcRuntime,
    snapshot: SettledRuntimeSnapshot,
    resources: readonly PiReloadableResourceKind[],
  ): Promise<PiReconfigureResult> {
    this.#requireIdleAdmission(runtime, snapshot);
    const control = this.#options.session.controlContextForReload();
    if (
      control.handshake.sessionId !== snapshot.sessionId
      || control.handshake.sessionFile !== snapshot.sessionFile
    ) {
      throw new PiReloadRejectedError("stale_runtime");
    }

    const baseline = await getEntries(runtime);
    const initialMaterialization = await classify(snapshot, this.#options.configuredSessionRoot, baseline);
    if (initialMaterialization.kind !== "materialized") {
      throw new PiReloadRejectedError("materialization_required");
    }
    requireMaterializedHandshake(baseline.entries, initialMaterialization, control.handshake);

    let recovered: { readonly correlation: ReloadCorrelation; readonly pair: ReloadMarkerPair } | undefined;
    try {
      recovered = recoverPersistedReload(
        baseline.entries,
        initialMaterialization,
        operation.commandId!.value,
        resources,
      );
    } catch (error) {
      await this.#reportPersistedAmbiguity(error);
    }
    if (recovered) {
      try {
        await this.#options.markRehydrating();
        return await this.#rehydrate(
          runtime,
          snapshot,
          control.extensionPath,
          recovered.correlation,
          recovered.pair,
          new Set(),
        );
      } catch (error) {
        if (error instanceof PiReloadRejectedError) throw error;
        if (error instanceof PiReloadAmbiguousError) throw error;
        throw new PiReloadAmbiguousError();
      }
    }

    const correlation: ReloadCorrelation = Object.freeze({
      commandId: operation.commandId!.value,
      nonce: generateNonce(this.#options.randomBytes ?? randomBytes),
      priorExtensionEpoch: control.handshake.extensionEpoch,
      resources: Object.freeze([...resources]),
    });
    const baselineIds = new Set(
      baseline.entries.flatMap((entry) => isRecord(entry) && typeof entry["id"] === "string" ? [entry["id"]] : []),
    );
    const argument = Buffer.from(JSON.stringify({
      commandId: correlation.commandId,
      nonce: correlation.nonce,
      priorExtensionEpoch: correlation.priorExtensionEpoch,
      resources: correlation.resources,
    }), "utf8").toString("base64url");

    let commandPossiblyWritten = false;
    try {
      await runtime.rpc.request({
        type: "prompt",
        message: `/${PATCHBAY_CONTROL_RELOAD_COMMAND} ${argument}`,
      });
      commandPossiblyWritten = true;
      await this.#options.markRehydrating();

      const completed = await this.#awaitMaterializedMarkers(
        runtime,
        snapshot,
        correlation,
        baselineIds,
      );
      return await this.#rehydrate(
        runtime,
        snapshot,
        control.extensionPath,
        correlation,
        completed.pair,
        baselineIds,
      );
    } catch (error) {
      if (
        error instanceof PiReloadRejectedError
        || (!commandPossiblyWritten && error instanceof RuntimeActionFencedError)
      ) {
        throw error;
      }
      if (!commandPossiblyWritten && isProvedNotWritten(error)) throw error;
      if (error instanceof PiReloadAmbiguousError) throw error;
      throw new PiReloadAmbiguousError();
    }
  }

  async #rehydrate(
    runtime: PiRpcRuntime,
    snapshot: SettledRuntimeSnapshot,
    extensionPath: string,
    correlation: ReloadCorrelation,
    completedPair: ReloadMarkerPair,
    baselineIds: ReadonlySet<string>,
  ): Promise<PiReconfigureResult> {
    try {
      const handshake = await this.#options.runtimePort.handshake(runtime, {
        expectedProjectCwd: this.#options.expectedProjectCwd,
        expectedExtensionPath: extensionPath,
        requiredExtensionEpoch: completedPair.extensionEpoch,
        previousExtensionEpoch: correlation.priorExtensionEpoch,
      });
      const rehydratedEntries = await getEntries(runtime);
      const rehydratedMaterialization = await classify(
        snapshot,
        this.#options.configuredSessionRoot,
        rehydratedEntries,
      );
      if (rehydratedMaterialization.kind !== "materialized") {
        throw new PiReloadAmbiguousError();
      }
      const pair = requireReloadMarkerPair(
        rehydratedEntries.entries,
        rehydratedMaterialization,
        correlation,
        baselineIds,
      );
      if (
        pair.requestEntryId !== completedPair.requestEntryId
        || pair.completionEntryId !== completedPair.completionEntryId
        || pair.extensionEpoch !== handshake.extensionEpoch
      ) {
        throw new PiReloadAmbiguousError();
      }
      requireMaterializedHandshake(rehydratedEntries.entries, rehydratedMaterialization, handshake);

      this.#options.session.installControlHandshake(handshake, extensionPath);
      this.#options.session.rebindAfterReload(runtime.processToken);
      await this.#options.reconciler.reconcileAfterReload(
        this.#options.runtimeReference,
        {
          logicalTargetId: this.#options.logicalTargetId,
          configuredSessionRoot: this.#options.configuredSessionRoot,
          piSessionId: snapshot.sessionId,
          declaredSessionPath: snapshot.sessionFile,
          materialization: rehydratedMaterialization,
          completeEntries: rehydratedEntries.entries,
          leafId: rehydratedEntries.leafId,
          fetchKnown: (cursor) => getEntries(runtime, cursor),
        },
      );
      if (
        runtime.pid !== snapshot.pid
        || runtime.processToken !== snapshot.processToken
        || this.#options.runtimeReference.externalRuntime?.generation?.value
          !== BigInt(this.#options.session.generation)
      ) {
        throw new PiReloadAmbiguousError();
      }
      await this.#options.markRehydrated();
      return create(PiReconfigureResultSchema, {
        outcome: PiReconfigureOutcome.RELOADED,
        processReplacementReasons: [],
      });
    } catch (error) {
      if (error instanceof PiReloadAmbiguousError) throw error;
      throw new PiReloadAmbiguousError();
    }
  }

  async #reportPersistedAmbiguity(error: unknown): Promise<never> {
    await this.#options.markRehydrating().catch(() => undefined);
    if (error instanceof PiReloadRejectedError) throw error;
    if (error instanceof PiReloadAmbiguousError) throw error;
    throw new PiReloadAmbiguousError();
  }

  #requireIdleAdmission(runtime: PiRpcRuntime, snapshot: SettledRuntimeSnapshot): void {
    if (
      runtime.pid !== snapshot.pid
      || runtime.processToken !== snapshot.processToken
      || this.#options.runtimeReference.externalRuntime?.runtimeSessionId?.value
        !== this.#options.session.runtimeSessionId
      || this.#options.runtimeReference.externalRuntime?.generation?.value
        !== BigInt(this.#options.session.generation)
    ) {
      throw new PiReloadRejectedError("stale_runtime");
    }
    if (this.#options.hasConflictingDelivery()) {
      throw new PiReloadRejectedError("busy_delivery");
    }
    if (snapshot.isStreaming) throw new PiReloadRejectedError("busy_streaming");
    if (snapshot.isCompacting) throw new PiReloadRejectedError("busy_compacting");
    if (snapshot.pendingMessageCount !== 0) throw new PiReloadRejectedError("busy_queued");
    if (!snapshot.noActivityStarted && !snapshot.settledAfterLatestActivity) {
      throw new PiReloadRejectedError("busy_unsettled");
    }
  }

  async #awaitMaterializedMarkers(
    runtime: PiRpcRuntime,
    snapshot: SettledRuntimeSnapshot,
    correlation: ReloadCorrelation,
    baselineIds: ReadonlySet<string>,
  ): Promise<{ readonly pair: ReloadMarkerPair }> {
    const maxPolls = boundedInteger(
      this.#options.maxMarkerPolls ?? DEFAULT_MAX_MARKER_POLLS,
      1,
      1_000,
    );
    const pollIntervalMs = boundedInteger(
      this.#options.pollIntervalMs ?? DEFAULT_POLL_INTERVAL_MS,
      0,
      60_000,
    );
    const sleep = this.#options.sleep ?? defaultSleep;
    for (let attempt = 0; attempt < maxPolls; attempt += 1) {
      const entries = await getEntries(runtime);
      const materialization = await classify(snapshot, this.#options.configuredSessionRoot, entries);
      if (materialization.kind === "materialized") {
        const pair = findReloadMarkerPair(
          entries.entries,
          materialization,
          correlation,
          baselineIds,
        );
        if (pair) return { pair };
      }
      if (attempt + 1 < maxPolls) await sleep(pollIntervalMs);
    }
    throw new PiReloadAmbiguousError();
  }
}

export function isPiReloadRequest(operation: Operation): boolean {
  return operation.kind === OperationKind.RECONFIGURE
    && operation.payload?.contentType === PayloadContentType.PROTOBUF
    && operation.payload.schemaRef === PI_RECONFIGURE_SCHEMA_REF;
}

function decodeReloadRequest(operation: Operation) {
  if (!isPiReloadRequest(operation) || !operation.payload) {
    throw new PiReloadRejectedError("invalid_request");
  }
  try {
    return fromBinary(PiReconfigureRequestSchema, operation.payload.payload);
  } catch {
    throw new PiReloadRejectedError("invalid_request");
  }
}

function processReplacementRequired(): PiReconfigureResult {
  return create(PiReconfigureResultSchema, {
    outcome: PiReconfigureOutcome.PROCESS_REPLACEMENT_REQUIRED,
    processReplacementReasons: [PiProcessReplacementOnlyKind.UNKNOWN_SCOPE],
  });
}

async function getEntries(runtime: PiRpcRuntime, since?: string): Promise<RpcEntries> {
  const value = await runtime.rpc.request<Record<string, unknown>>({
    type: "get_entries",
    ...(since ? { since } : {}),
  });
  if (!Array.isArray(value["entries"]) || !(value["leafId"] === null || typeof value["leafId"] === "string")) {
    throw new Error("Pi reload get_entries response is malformed");
  }
  return Object.freeze({ entries: value["entries"], leafId: value["leafId"] });
}

function classify(
  snapshot: SettledRuntimeSnapshot,
  allowedRoot: string,
  entries: RpcEntries,
): Promise<PiSessionMaterialization> {
  return classifyPiSessionMaterialization({
    sessionId: snapshot.sessionId,
    declaredPath: snapshot.sessionFile,
    allowedRoot,
    rpcEntries: entries.entries,
    rpcLeafId: entries.leafId,
  });
}

function recoverPersistedReload(
  entries: readonly unknown[],
  materialization: PiSessionMaterialization,
  commandId: string,
  requestedResources: readonly PiReloadableResourceKind[],
): { readonly correlation: ReloadCorrelation; readonly pair: ReloadMarkerPair } | undefined {
  const requests = entries.filter(
    (entry) => markerHasCommand(entry, PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE, commandId),
  );
  const completions = entries.filter(
    (entry) => markerHasCommand(entry, PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE, commandId),
  );
  if (requests.length === 0 && completions.length === 0) return undefined;
  if (requests.length !== 1 || completions.length !== 1) throw new PiReloadAmbiguousError();
  const request = requests[0];
  if (!isRecord(request) || !isRecord(request["data"])) throw new PiReloadAmbiguousError();
  const data = request["data"];
  if (
    !isExactBase64Url(data["nonce"], 43)
    || !isExactBase64Url(data["priorExtensionEpoch"], EPOCH_LENGTH)
    || !sameResources(data["resources"], requestedResources)
  ) {
    throw new PiReloadAmbiguousError();
  }
  const correlation: ReloadCorrelation = Object.freeze({
    commandId,
    nonce: data["nonce"],
    priorExtensionEpoch: data["priorExtensionEpoch"],
    resources: Object.freeze([...requestedResources]),
  });
  return {
    correlation,
    pair: requireReloadMarkerPair(entries, materialization, correlation, new Set()),
  };
}

function findReloadMarkerPair(
  entries: readonly unknown[],
  materialization: PiSessionMaterialization,
  correlation: ReloadCorrelation,
  baselineIds: ReadonlySet<string>,
): ReloadMarkerPair | undefined {
  if (materialization.kind !== "materialized") return undefined;
  const requests = entries.filter((entry) => markerHasCommand(entry, PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE, correlation.commandId));
  const completions = entries.filter((entry) => markerHasCommand(entry, PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE, correlation.commandId));
  if (requests.length === 0 && completions.length === 0) return undefined;
  if (requests.length !== 1 || completions.length !== 1) throw new PiReloadAmbiguousError();
  const request = requests[0];
  const completion = completions[0];
  if (!isRecord(request) || !isRecord(request["data"]) || !isRecord(completion) || !isRecord(completion["data"])) {
    throw new PiReloadAmbiguousError();
  }
  const requestId = request["id"];
  const completionId = completion["id"];
  const requestData = request["data"];
  const completionData = completion["data"];
  if (
    typeof requestId !== "string"
    || typeof completionId !== "string"
    || baselineIds.has(requestId)
    || baselineIds.has(completionId)
    || requestData["nonce"] !== correlation.nonce
    || requestData["priorExtensionEpoch"] !== correlation.priorExtensionEpoch
    || !sameResources(requestData["resources"], correlation.resources)
    || completionData["nonce"] !== correlation.nonce
    || completionData["requestEntryId"] !== requestId
    || completionData["priorExtensionEpoch"] !== correlation.priorExtensionEpoch
    || !isExactBase64Url(completionData["extensionEpoch"], EPOCH_LENGTH)
    || completionData["extensionEpoch"] === correlation.priorExtensionEpoch
  ) {
    throw new PiReloadAmbiguousError();
  }
  const durableIds = new Set(materialization.seal.orderedEntryIds);
  if (!durableIds.has(requestId) || !durableIds.has(completionId)) {
    throw new PiReloadAmbiguousError();
  }
  return Object.freeze({
    requestEntryId: requestId,
    completionEntryId: completionId,
    extensionEpoch: completionData["extensionEpoch"],
  });
}

function requireReloadMarkerPair(
  entries: readonly unknown[],
  materialization: PiSessionMaterialization,
  correlation: ReloadCorrelation,
  baselineIds: ReadonlySet<string>,
): ReloadMarkerPair {
  const pair = findReloadMarkerPair(entries, materialization, correlation, baselineIds);
  if (!pair) throw new PiReloadAmbiguousError();
  return pair;
}

function requireMaterializedHandshake(
  entries: readonly unknown[],
  materialization: PiSessionMaterialization,
  handshake: {
    readonly challenge: string;
    readonly launchNonce: string;
    readonly extensionEpoch: string;
    readonly cwd: string;
    readonly sessionId: string;
    readonly sessionFile: string;
    readonly markerEntryId: string;
  },
): void {
  if (materialization.kind !== "materialized") throw new PiReloadRejectedError("materialization_required");
  const matching = entries.filter((entry) => {
    if (!isRecord(entry) || entry["type"] !== "custom" || entry["customType"] !== PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE) {
      return false;
    }
    if (entry["id"] !== handshake.markerEntryId || !isRecord(entry["data"])) return false;
    const data = entry["data"];
    return data["challenge"] === handshake.challenge
      && data["launchNonce"] === handshake.launchNonce
      && data["extensionEpoch"] === handshake.extensionEpoch
      && data["cwd"] === handshake.cwd
      && data["sessionId"] === handshake.sessionId
      && data["sessionFile"] === handshake.sessionFile;
  });
  if (
    matching.length !== 1
    || !materialization.seal.orderedEntryIds.includes(handshake.markerEntryId)
  ) {
    throw new PiReloadRejectedError("materialization_required");
  }
}

function markerHasCommand(entry: unknown, customType: string, commandId: string): boolean {
  return isRecord(entry)
    && entry["type"] === "custom"
    && entry["customType"] === customType
    && isRecord(entry["data"])
    && entry["data"]["commandId"] === commandId;
}

function sameResources(value: unknown, expected: readonly PiReloadableResourceKind[]): boolean {
  return Array.isArray(value)
    && value.length === expected.length
    && value.every((resource, index) => resource === expected[index]);
}

function generateNonce(source: (size: number) => Uint8Array): string {
  const bytes = source(32);
  if (bytes.byteLength !== 32) throw new PiReloadRejectedError("invalid_request");
  return Buffer.from(bytes).toString("base64url");
}

function isExactBase64Url(value: unknown, length: number): value is string {
  return typeof value === "string" && value.length === length && BASE64URL_PATTERN.test(value);
}

function isProvedNotWritten(error: unknown): boolean {
  return isRecord(error) && error["requestEffect"] === "proved_not_written";
}

function boundedInteger(value: number, minimum: number, maximum: number): number {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new PiReloadRejectedError("invalid_request");
  }
  return value;
}

function defaultSleep(milliseconds: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
