import { fromBinary } from "@bufbuild/protobuf";
import {
  ObservationKind,
  PayloadContentType,
  PiPersistedProjectionReplacementSchema,
  PiPersistedProjectionSuffixSchema,
  PiVolatileProjectionSnapshotSchema,
  TargetScopeKind,
  type Observation,
  type PiPersistedProjectionEntry,
} from "@patchbay/contracts";

export const PI_PERSISTED_SUFFIX_SCHEMA_REF = "patchbay.PiPersistedProjectionSuffix.v1";
export const PI_PERSISTED_REPLACEMENT_SCHEMA_REF = "patchbay.PiPersistedProjectionReplacement.v1";
export const PI_VOLATILE_PROJECTION_SCHEMA_REF = "patchbay.PiVolatileProjectionSnapshot.v1";

export interface PiProjectionObservationScope {
  readonly key: string;
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly externalContinuityId: string;
  readonly authority: "persisted" | "volatile";
}

export interface PiPersistedPresentationItem {
  readonly membershipId: string;
  readonly transcriptEvent: Readonly<Record<string, unknown>> & { readonly kind: string };
}

export interface PiPersistedProjectionEntryView {
  readonly stableEntryId: string;
  readonly parentEntryId: string | null;
  readonly contentDigest: string;
  readonly presentationItems: readonly PiPersistedPresentationItem[];
}

export interface PiPersistedProjectionState {
  readonly scopeKey: string;
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly externalContinuityId: string;
  readonly replacementEpoch: bigint;
  readonly exactEntries: readonly PiPersistedProjectionEntryView[];
  readonly cursorEntryId: string | null;
  readonly leafEntryId: string | null;
  readonly treeDigest: string;
  readonly lastBatchId: string;
}

export interface PiPersistedProjectionFold {
  readonly kind: "suffix" | "replacement" | "idempotent";
  readonly state: PiPersistedProjectionState;
  readonly removedMembershipIds: readonly string[];
  readonly addedItems: readonly PiPersistedPresentationItem[];
}

export interface PiVolatileProjectionState {
  readonly scopeKey: string;
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly externalContinuityId: string;
  readonly exactEntries: readonly PiPersistedProjectionEntryView[];
  readonly cursorEntryId: string | null;
  readonly leafEntryId: string | null;
  readonly treeDigest: string;
  readonly lastBatchId: string;
}

export interface PiVolatileProjectionFold {
  readonly kind: "snapshot" | "idempotent";
  readonly state: PiVolatileProjectionState;
  readonly removedMembershipIds: readonly string[];
  readonly addedItems: readonly PiPersistedPresentationItem[];
}

export function piProjectionObservationScope(
  observation: Observation,
): PiProjectionObservationScope | undefined {
  const envelope = observation.payload;
  if (!envelope || envelope.contentType !== PayloadContentType.PROTOBUF) return undefined;
  let externalContinuityId: string;
  let authority: PiProjectionObservationScope["authority"];
  if (envelope.schemaRef === PI_PERSISTED_REPLACEMENT_SCHEMA_REF) {
    externalContinuityId = boundedContinuity(
      fromBinary(PiPersistedProjectionReplacementSchema, envelope.payload).externalContinuityId,
    );
    authority = "persisted";
  } else if (envelope.schemaRef === PI_PERSISTED_SUFFIX_SCHEMA_REF) {
    externalContinuityId = boundedContinuity(
      fromBinary(PiPersistedProjectionSuffixSchema, envelope.payload).externalContinuityId,
    );
    authority = "persisted";
  } else if (envelope.schemaRef === PI_VOLATILE_PROJECTION_SCHEMA_REF) {
    externalContinuityId = boundedContinuity(
      fromBinary(PiVolatileProjectionSnapshotSchema, envelope.payload).externalContinuityId,
    );
    authority = "volatile";
  } else {
    return undefined;
  }
  const source = validateObservationSource(observation);
  return Object.freeze({
    key: lengthFramedKey([source.adapterId, source.deploymentScope, externalContinuityId]),
    ...source,
    externalContinuityId,
    authority,
  });
}

/** Decode and fold the two known Pi envelopes; unrelated Observations return undefined. */
export function foldPiPersistedProjectionObservation(
  current: PiPersistedProjectionState | undefined,
  observation: Observation,
): PiPersistedProjectionFold | undefined {
  const envelope = observation.payload;
  if (
    observation.kind !== ObservationKind.EVENT
    || !envelope
    || envelope.contentType !== PayloadContentType.PROTOBUF
    || (envelope.schemaRef !== PI_PERSISTED_SUFFIX_SCHEMA_REF
      && envelope.schemaRef !== PI_PERSISTED_REPLACEMENT_SCHEMA_REF)
  ) return undefined;
  const scope = piProjectionObservationScope(observation);
  if (!scope || scope.authority !== "persisted") {
    throw new Error("Pi persisted projection has no persisted observation scope");
  }

  if (envelope.schemaRef === PI_PERSISTED_REPLACEMENT_SCHEMA_REF) {
    const replacement = fromBinary(PiPersistedProjectionReplacementSchema, envelope.payload);
    const entries = decodeEntries(replacement.exactEntries);
    const state = validatedState({
      scopeKey: scope.key,
      adapterId: scope.adapterId,
      deploymentScope: scope.deploymentScope,
      externalContinuityId: scope.externalContinuityId,
      replacementEpoch: positiveEpoch(replacement.replacementEpoch),
      exactEntries: entries,
      cursorEntryId: optionalId(replacement.cursorEntryId),
      leafEntryId: optionalId(replacement.leafEntryId),
      treeDigest: digest(replacement.treeDigest, "tree digest"),
      lastBatchId: digest(replacement.batchId, "batch id"),
    });
    const expectedBatch = batchDigest([
      "replacement",
      state.externalContinuityId,
      state.replacementEpoch.toString(),
      canonicalJson(entriesForBatch(state.exactEntries)),
      state.cursorEntryId ?? "",
      state.leafEntryId ?? "",
      state.treeDigest,
    ]);
    if (expectedBatch !== state.lastBatchId) throw new Error("Pi replacement batch id is invalid");
    if (current) {
      if (current.scopeKey !== state.scopeKey) {
        throw new Error("Pi replacement scope differs from its selected state");
      }
      if (state.replacementEpoch === current.replacementEpoch) {
        if (!statesEqual(current, state)) throw new Error("Pi same-epoch replacement content conflicts");
        return { kind: "idempotent", state: current, removedMembershipIds: [], addedItems: [] };
      }
      if (state.replacementEpoch !== current.replacementEpoch + 1n) {
        throw new Error("Pi replacement epoch does not immediately follow current state");
      }
    }
    return {
      kind: "replacement",
      state,
      removedMembershipIds: current ? memberships(current.exactEntries) : [],
      addedItems: items(state.exactEntries),
    };
  }

  const suffix = fromBinary(PiPersistedProjectionSuffixSchema, envelope.payload);
  const continuityId = scope.externalContinuityId;
  const epoch = positiveEpoch(suffix.replacementEpoch);
  const batchId = digest(suffix.batchId, "batch id");
  if (!current) throw new Error("Pi known suffix has no current exact projection");
  if (current.scopeKey !== scope.key || current.replacementEpoch !== epoch) {
    throw new Error("Pi known suffix scope or epoch is stale");
  }
  if (current.lastBatchId === batchId) {
    return { kind: "idempotent", state: current, removedMembershipIds: [], addedItems: [] };
  }
  const baseCursor = requiredId(suffix.baseCursorEntryId, "base cursor");
  if (current.cursorEntryId !== baseCursor) throw new Error("Pi known suffix base cursor is stale");
  const suffixEntries = decodeEntries(suffix.entries);
  const exactEntries = mergeSuffix(current.exactEntries, suffixEntries);
  const state = validatedState({
    scopeKey: scope.key,
    adapterId: scope.adapterId,
    deploymentScope: scope.deploymentScope,
    externalContinuityId: continuityId,
    replacementEpoch: epoch,
    exactEntries,
    cursorEntryId: optionalId(suffix.cursorEntryId),
    leafEntryId: optionalId(suffix.leafEntryId),
    treeDigest: digest(suffix.treeDigest, "tree digest"),
    lastBatchId: batchId,
  });
  const expectedBatch = batchDigest([
    "suffix",
    continuityId,
    epoch.toString(),
    baseCursor,
    canonicalJson(entriesForBatch(suffixEntries)),
    state.cursorEntryId ?? "",
    state.leafEntryId ?? "",
    state.treeDigest,
  ]);
  if (expectedBatch !== batchId) throw new Error("Pi suffix batch id is invalid");
  const oldMemberships = new Set(memberships(current.exactEntries));
  return {
    kind: "suffix",
    state,
    removedMembershipIds: [],
    addedItems: items(suffixEntries).filter((item) => !oldMemberships.has(item.membershipId)),
  };
}

/** Fold a non-authoritative memory-only snapshot without an epoch claim. */
export function foldPiVolatileProjectionObservation(
  current: PiVolatileProjectionState | undefined,
  observation: Observation,
): PiVolatileProjectionFold | undefined {
  const envelope = observation.payload;
  if (
    observation.kind !== ObservationKind.EVENT
    || !envelope
    || envelope.contentType !== PayloadContentType.PROTOBUF
    || envelope.schemaRef !== PI_VOLATILE_PROJECTION_SCHEMA_REF
  ) return undefined;
  const scope = piProjectionObservationScope(observation);
  if (!scope || scope.authority !== "volatile") {
    throw new Error("Pi volatile projection has no volatile observation scope");
  }
  const snapshot = fromBinary(PiVolatileProjectionSnapshotSchema, envelope.payload);
  const entries = decodeEntries(snapshot.exactEntries);
  const state = validatedVolatileState({
    scopeKey: scope.key,
    adapterId: scope.adapterId,
    deploymentScope: scope.deploymentScope,
    externalContinuityId: scope.externalContinuityId,
    exactEntries: entries,
    cursorEntryId: optionalId(snapshot.cursorEntryId),
    leafEntryId: optionalId(snapshot.leafEntryId),
    treeDigest: digest(snapshot.treeDigest, "tree digest"),
    lastBatchId: digest(snapshot.batchId, "batch id"),
  });
  const expectedBatch = batchDigest([
    "volatile",
    state.externalContinuityId,
    canonicalJson(entriesForBatch(state.exactEntries)),
    state.cursorEntryId ?? "",
    state.leafEntryId ?? "",
    state.treeDigest,
  ]);
  if (expectedBatch !== state.lastBatchId) throw new Error("Pi volatile snapshot batch id is invalid");
  if (current) {
    if (current.scopeKey !== state.scopeKey) {
      throw new Error("Pi volatile snapshot scope differs from its selected state");
    }
    if (volatileStatesEqual(current, state)) {
      return { kind: "idempotent", state: current, removedMembershipIds: [], addedItems: [] };
    }
  }
  return {
    kind: "snapshot",
    state,
    removedMembershipIds: current ? memberships(current.exactEntries) : [],
    addedItems: items(state.exactEntries),
  };
}

function validateObservationSource(
  observation: Observation,
): { readonly adapterId: string; readonly deploymentScope: string } {
  const target = observation.targetScope;
  const adapterId = target?.adapterId?.value;
  if (
    target?.kind !== TargetScopeKind.RUNTIME_SESSION
    || target.resource
    || target.actorId
    || target.projectOrGroup
    || target.legacyAuditResourceId
    || !adapterId
    || !target.deploymentScope
    || !target.runtimeSessionId?.value
    || !target.sessionGeneration?.value
    || observation.sender?.actorId?.value !== adapterId
  ) {
    throw new Error("Pi projection Observation source or runtime target is invalid");
  }
  return Object.freeze({ adapterId, deploymentScope: target.deploymentScope });
}

function decodeEntries(entries: readonly PiPersistedProjectionEntry[]): readonly PiPersistedProjectionEntryView[] {
  const stableIds = new Set<string>();
  const membershipIds = new Set<string>();
  return Object.freeze(entries.map((entry) => {
    const stableEntryId = requiredId(entry.stableEntryId, "stable entry id");
    if (stableIds.has(stableEntryId)) throw new Error("Pi projection repeats a stable entry id");
    stableIds.add(stableEntryId);
    const presentationItems = Object.freeze(entry.presentationItems.map((item) => {
      const membershipId = bounded(item.membershipId, "membership id", 256);
      if (membershipIds.has(membershipId)) throw new Error("Pi projection repeats presentation membership");
      membershipIds.add(membershipId);
      const parsed: unknown = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(item.transcriptEventJson));
      if (!isRecord(parsed) || typeof parsed.kind !== "string") {
        throw new Error("Pi projected transcript event is malformed");
      }
      return Object.freeze({
        membershipId,
        transcriptEvent: Object.freeze(parsed) as Readonly<Record<string, unknown>> & { readonly kind: string },
      });
    }));
    return Object.freeze({
      stableEntryId,
      parentEntryId: optionalId(entry.parentEntryId),
      contentDigest: digest(entry.contentDigest, "entry content digest"),
      presentationItems,
    });
  }));
}

function mergeSuffix(
  existing: readonly PiPersistedProjectionEntryView[],
  suffix: readonly PiPersistedProjectionEntryView[],
): readonly PiPersistedProjectionEntryView[] {
  const result = [...existing];
  const indexes = new Map(result.map((entry, index) => [entry.stableEntryId, index]));
  for (const entry of suffix) {
    const index = indexes.get(entry.stableEntryId);
    if (index === undefined) {
      indexes.set(entry.stableEntryId, result.length);
      result.push(entry);
    } else if (!entriesEqual(result[index]!, entry)) {
      throw new Error("Pi suffix conflicts with a stable entry identity");
    }
  }
  return Object.freeze(result);
}

function validatedState(state: PiPersistedProjectionState): PiPersistedProjectionState {
  validateExactState(state);
  return Object.freeze({ ...state, exactEntries: Object.freeze([...state.exactEntries]) });
}

function validatedVolatileState(state: PiVolatileProjectionState): PiVolatileProjectionState {
  validateExactState(state);
  return Object.freeze({ ...state, exactEntries: Object.freeze([...state.exactEntries]) });
}

function validateExactState(state: {
  readonly exactEntries: readonly PiPersistedProjectionEntryView[];
  readonly cursorEntryId: string | null;
  readonly leafEntryId: string | null;
  readonly treeDigest: string;
}): void {
  const seen = new Set<string>();
  const seenMemberships = new Set<string>();
  let roots = 0;
  for (const entry of state.exactEntries) {
    if (seen.has(entry.stableEntryId)) throw new Error("Pi exact projection repeats stable identity");
    if (entry.parentEntryId === null) roots += 1;
    else if (!seen.has(entry.parentEntryId) || entry.parentEntryId === entry.stableEntryId) {
      throw new Error("Pi exact projection has an invalid append-order parent");
    }
    seen.add(entry.stableEntryId);
    for (const item of entry.presentationItems) {
      if (seenMemberships.has(item.membershipId)) {
        throw new Error("Pi exact projection repeats presentation membership");
      }
      seenMemberships.add(item.membershipId);
    }
  }
  if (state.exactEntries.length === 0) {
    if (state.cursorEntryId !== null || state.leafEntryId !== null) {
      throw new Error("empty Pi projection has a cursor or leaf");
    }
  } else {
    if (roots !== 1 || state.leafEntryId === null || !seen.has(state.leafEntryId)) {
      throw new Error("Pi exact projection root or leaf is invalid");
    }
    if (state.cursorEntryId !== state.exactEntries.at(-1)!.stableEntryId) {
      throw new Error("Pi projection cursor is not its append-order tail");
    }
  }
  if (piTreeDigest(state.exactEntries) !== state.treeDigest) {
    throw new Error("Pi projection tree digest disagrees with exact membership");
  }
}

function statesEqual(left: PiPersistedProjectionState, right: PiPersistedProjectionState): boolean {
  return left.scopeKey === right.scopeKey
    && left.adapterId === right.adapterId
    && left.deploymentScope === right.deploymentScope
    && left.externalContinuityId === right.externalContinuityId
    && left.replacementEpoch === right.replacementEpoch
    && left.cursorEntryId === right.cursorEntryId
    && left.leafEntryId === right.leafEntryId
    && left.treeDigest === right.treeDigest
    && left.lastBatchId === right.lastBatchId
    && left.exactEntries.length === right.exactEntries.length
    && left.exactEntries.every((entry, index) => entriesEqual(entry, right.exactEntries[index]!));
}

function volatileStatesEqual(left: PiVolatileProjectionState, right: PiVolatileProjectionState): boolean {
  return left.scopeKey === right.scopeKey
    && left.adapterId === right.adapterId
    && left.deploymentScope === right.deploymentScope
    && left.externalContinuityId === right.externalContinuityId
    && left.cursorEntryId === right.cursorEntryId
    && left.leafEntryId === right.leafEntryId
    && left.treeDigest === right.treeDigest
    && left.lastBatchId === right.lastBatchId
    && left.exactEntries.length === right.exactEntries.length
    && left.exactEntries.every((entry, index) => entriesEqual(entry, right.exactEntries[index]!));
}

function entriesEqual(left: PiPersistedProjectionEntryView, right: PiPersistedProjectionEntryView): boolean {
  return left.stableEntryId === right.stableEntryId
    && left.parentEntryId === right.parentEntryId
    && left.contentDigest === right.contentDigest
    && left.presentationItems.length === right.presentationItems.length
    && left.presentationItems.every((item, index) => {
      const candidate = right.presentationItems[index]!;
      return item.membershipId === candidate.membershipId
        && canonicalJson(item.transcriptEvent) === canonicalJson(candidate.transcriptEvent);
    });
}

function entriesForBatch(entries: readonly PiPersistedProjectionEntryView[]): readonly Record<string, unknown>[] {
  return entries.map((entry) => ({
    stableEntryId: entry.stableEntryId,
    parentEntryId: entry.parentEntryId,
    contentDigest: entry.contentDigest,
    presentationItems: entry.presentationItems.map((item) => ({
      membershipId: item.membershipId,
      transcriptEventJson: JSON.stringify(item.transcriptEvent),
    })),
  }));
}

function items(entries: readonly PiPersistedProjectionEntryView[]): readonly PiPersistedPresentationItem[] {
  return Object.freeze(entries.flatMap((entry) => entry.presentationItems));
}

function memberships(entries: readonly PiPersistedProjectionEntryView[]): readonly string[] {
  return items(entries).map((item) => item.membershipId);
}

function piTreeDigest(entries: readonly PiPersistedProjectionEntryView[]): string {
  return sha256Hex(JSON.stringify(entries.map((entry) => [entry.stableEntryId, entry.parentEntryId])));
}

function lengthFramedKey(parts: readonly string[]): string {
  const encoder = new TextEncoder();
  return parts.map((part) => `${encoder.encode(part).byteLength}:${part}\0`).join("");
}

function batchDigest(parts: readonly string[]): string {
  const encoder = new TextEncoder();
  return sha256Hex(parts.map((part) => `${encoder.encode(part).byteLength}:${part}\0`).join(""));
}

function canonicalJson(value: unknown): string {
  if (value === null || typeof value === "string" || typeof value === "boolean") return JSON.stringify(value);
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error("Pi projection contains a non-finite number");
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (!isRecord(value)) throw new Error("Pi projection contains unsupported data");
  return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
}

function boundedContinuity(value: string): string {
  const boundedValue = bounded(value, "external continuity id", 512);
  if (!boundedValue.startsWith("pi1:")) throw new Error("Pi external continuity id has an unknown version");
  return boundedValue;
}

function requiredId(value: string, field: string): string {
  const id = bounded(value, field, 1_024);
  if (!/^[A-Za-z0-9._:-]+$/u.test(id)) throw new Error(`Pi ${field} is invalid`);
  return id;
}

function optionalId(value: string): string | null {
  return value === "" ? null : requiredId(value, "optional entry id");
}

function bounded(value: string, field: string, max: number): string {
  if (!value || new TextEncoder().encode(value).byteLength > max || value.includes("\0")) throw new Error(`Pi ${field} is invalid`);
  return value;
}

function digest(value: string, field: string): string {
  if (!/^[a-f0-9]{64}$/u.test(value)) throw new Error(`Pi ${field} is invalid`);
  return value;
}

function positiveEpoch(value: bigint): bigint {
  if (value <= 0n) throw new Error("Pi projection epoch must be positive");
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

// Small browser-safe SHA-256 used only to verify adapter-provided projection
// digests. It keeps the pure presentation fold synchronous and dependency-free.
function sha256Hex(text: string): string {
  const source = new TextEncoder().encode(text);
  const bitLength = BigInt(source.byteLength) * 8n;
  const paddedLength = Math.ceil((source.byteLength + 9) / 64) * 64;
  const bytes = new Uint8Array(paddedLength);
  bytes.set(source);
  bytes[source.byteLength] = 0x80;
  for (let index = 0; index < 8; index += 1) {
    bytes[paddedLength - 1 - index] = Number((bitLength >> BigInt(index * 8)) & 0xffn);
  }
  const h = new Uint32Array([
    0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
    0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
  ]);
  const k = new Uint32Array([
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
  ]);
  const w = new Uint32Array(64);
  const rotate = (value: number, count: number): number => (value >>> count) | (value << (32 - count));
  for (let offset = 0; offset < bytes.length; offset += 64) {
    const view = new DataView(bytes.buffer, bytes.byteOffset + offset, 64);
    for (let index = 0; index < 16; index += 1) w[index] = view.getUint32(index * 4, false);
    for (let index = 16; index < 64; index += 1) {
      const x = w[index - 15]!;
      const y = w[index - 2]!;
      const s0 = rotate(x, 7) ^ rotate(x, 18) ^ (x >>> 3);
      const s1 = rotate(y, 17) ^ rotate(y, 19) ^ (y >>> 10);
      w[index] = (w[index - 16]! + s0 + w[index - 7]! + s1) >>> 0;
    }
    let [a, b, c, d, e, f, g, hh] = h;
    for (let index = 0; index < 64; index += 1) {
      const sum1 = rotate(e!, 6) ^ rotate(e!, 11) ^ rotate(e!, 25);
      const choice = (e! & f!) ^ (~e! & g!);
      const t1 = (hh! + sum1 + choice + k[index]! + w[index]!) >>> 0;
      const sum0 = rotate(a!, 2) ^ rotate(a!, 13) ^ rotate(a!, 22);
      const majority = (a! & b!) ^ (a! & c!) ^ (b! & c!);
      const t2 = (sum0 + majority) >>> 0;
      hh = g;
      g = f;
      f = e;
      e = (d! + t1) >>> 0;
      d = c;
      c = b;
      b = a;
      a = (t1 + t2) >>> 0;
    }
    h[0] = (h[0]! + a!) >>> 0;
    h[1] = (h[1]! + b!) >>> 0;
    h[2] = (h[2]! + c!) >>> 0;
    h[3] = (h[3]! + d!) >>> 0;
    h[4] = (h[4]! + e!) >>> 0;
    h[5] = (h[5]! + f!) >>> 0;
    h[6] = (h[6]! + g!) >>> 0;
    h[7] = (h[7]! + hh!) >>> 0;
  }
  return [...h].map((value) => value.toString(16).padStart(8, "0")).join("");
}
