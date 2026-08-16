import { createHash } from "node:crypto";
import { toBinary } from "@bufbuild/protobuf";
import {
  PiPersistedPresentationItemSchema,
  PiPersistedProjectionEntrySchema,
  PiPersistedProjectionReplacementSchema,
  PiPersistedProjectionSuffixSchema,
  PiVolatileProjectionSnapshotSchema,
} from "@patchbay/contracts";
import { projectSessionEntries } from "./transcript_projection.js";
import type { TranscriptEvent } from "./transcript_event.js";

export const PI_PROJECTION_SUFFIX_SCHEMA_REF = "patchbay.PiPersistedProjectionSuffix.v1";
export const PI_PROJECTION_REPLACEMENT_SCHEMA_REF = "patchbay.PiPersistedProjectionReplacement.v1";
export const PI_VOLATILE_PROJECTION_SCHEMA_REF = "patchbay.PiVolatileProjectionSnapshot.v1";

const MAX_ENTRY_ID_BYTES = 1_024;
const DIGEST_PATTERN = /^[a-f0-9]{64}$/u;
const PI_ENTRY_ID_PATTERN = /^[A-Za-z0-9._:-]+$/u;

export interface PiProjectedPresentationItem {
  readonly membershipId: string;
  readonly transcriptEventJson: string;
}

/** Redacted persisted-entry evidence. Custom/control entries carry no presentation payload. */
export interface PiProjectedEntry {
  readonly stableEntryId: string;
  readonly parentEntryId: string | null;
  readonly contentDigest: string;
  readonly presentationItems: readonly PiProjectedPresentationItem[];
}

export type PiProjectionCursor = string | null;

export interface PiProjectionLeaf {
  readonly entryId: string | null;
  readonly treeDigest: string;
}

export interface ExactPiProjection {
  readonly entries: readonly PiProjectedEntry[];
  readonly cursor: PiProjectionCursor;
  readonly leaf: PiProjectionLeaf;
}

export interface EncodedPiProjectionEnvelope {
  readonly schemaRef:
    | typeof PI_PROJECTION_SUFFIX_SCHEMA_REF
    | typeof PI_PROJECTION_REPLACEMENT_SCHEMA_REF
    | typeof PI_VOLATILE_PROJECTION_SCHEMA_REF;
  readonly payload: Uint8Array;
  readonly batchId: string;
}

/** Project and validate one complete append-ordered Pi tree. */
export function projectCompletePiEntries(
  rawEntries: readonly unknown[],
  leafId: string | null,
  externalContinuityId: string,
): ExactPiProjection {
  const entries = projectRawEntries(rawEntries, externalContinuityId);
  validateCompleteTree(entries, leafId);
  return Object.freeze({
    entries,
    cursor: entries.at(-1)?.stableEntryId ?? null,
    leaf: Object.freeze({ entryId: leafId, treeDigest: piTreeDigest(entries) }),
  });
}

/** Validate a known-cursor suffix against the complete locally committed tree. */
export function projectKnownPiSuffix(
  existing: readonly PiProjectedEntry[],
  rawSuffix: readonly unknown[],
  leafId: string | null,
  externalContinuityId: string,
): ExactPiProjection & { readonly suffixEntries: readonly PiProjectedEntry[] } {
  const current = validateProjectedEntries(existing);
  const suffixEntries = projectRawEntries(rawSuffix, externalContinuityId);
  const combined = [...current];
  const indexes = new Map(combined.map((entry, index) => [entry.stableEntryId, index]));
  for (const entry of suffixEntries) {
    const existingIndex = indexes.get(entry.stableEntryId);
    if (existingIndex !== undefined) {
      if (!piProjectedEntriesEqual(combined[existingIndex]!, entry)) {
        throw new Error("Pi known suffix conflicts with committed stable entry content");
      }
      continue;
    }
    if (entry.parentEntryId === null) {
      if (combined.length !== 0) throw new Error("Pi known suffix introduces another tree root");
    } else if (!indexes.has(entry.parentEntryId)) {
      throw new Error("Pi known suffix parent is absent from the committed prefix");
    }
    indexes.set(entry.stableEntryId, combined.length);
    combined.push(entry);
  }
  validateCompleteTree(combined, leafId);
  return Object.freeze({
    entries: Object.freeze(combined),
    suffixEntries,
    cursor: combined.at(-1)?.stableEntryId ?? null,
    leaf: Object.freeze({ entryId: leafId, treeDigest: piTreeDigest(combined) }),
  });
}

export function encodePiProjectionSuffix(input: {
  readonly externalContinuityId: string;
  readonly replacementEpoch: bigint;
  readonly baseCursor: string;
  readonly entries: readonly PiProjectedEntry[];
  readonly cursor: PiProjectionCursor;
  readonly leaf: PiProjectionLeaf;
}): EncodedPiProjectionEnvelope {
  boundedText(input.externalContinuityId, "external continuity id", 512);
  boundedText(input.baseCursor, "base cursor", MAX_ENTRY_ID_BYTES);
  positiveEpoch(input.replacementEpoch);
  const entries = validateProjectedEntries(input.entries);
  const batchId = batchDigest([
    "suffix",
    input.externalContinuityId,
    input.replacementEpoch.toString(),
    input.baseCursor,
    canonicalJson(entries),
    input.cursor ?? "",
    input.leaf.entryId ?? "",
    input.leaf.treeDigest,
  ]);
  const message = {
    $typeName: "patchbay.PiPersistedProjectionSuffix" as const,
    externalContinuityId: input.externalContinuityId,
    replacementEpoch: input.replacementEpoch,
    batchId,
    baseCursorEntryId: input.baseCursor,
    entries: entries.map(projectedEntryToWire),
    cursorEntryId: input.cursor ?? "",
    leafEntryId: input.leaf.entryId ?? "",
    treeDigest: requiredDigest(input.leaf.treeDigest, "tree digest"),
  };
  return Object.freeze({
    schemaRef: PI_PROJECTION_SUFFIX_SCHEMA_REF,
    payload: toBinary(PiPersistedProjectionSuffixSchema, message),
    batchId,
  });
}

export function encodePiProjectionReplacement(input: {
  readonly externalContinuityId: string;
  readonly replacementEpoch: bigint;
  readonly exactEntries: readonly PiProjectedEntry[];
  readonly cursor: PiProjectionCursor;
  readonly leaf: PiProjectionLeaf;
}): EncodedPiProjectionEnvelope {
  boundedText(input.externalContinuityId, "external continuity id", 512);
  positiveEpoch(input.replacementEpoch);
  const exactEntries = validateProjectedEntries(input.exactEntries);
  validateCompleteTree(exactEntries, input.leaf.entryId);
  const computedTreeDigest = piTreeDigest(exactEntries);
  if (computedTreeDigest !== input.leaf.treeDigest) {
    throw new Error("Pi replacement tree digest disagrees with exact membership");
  }
  const expectedCursor = exactEntries.at(-1)?.stableEntryId ?? null;
  if (expectedCursor !== input.cursor) throw new Error("Pi replacement cursor is not the append-order tail");
  const batchId = batchDigest([
    "replacement",
    input.externalContinuityId,
    input.replacementEpoch.toString(),
    canonicalJson(exactEntries),
    input.cursor ?? "",
    input.leaf.entryId ?? "",
    input.leaf.treeDigest,
  ]);
  const message = {
    $typeName: "patchbay.PiPersistedProjectionReplacement" as const,
    externalContinuityId: input.externalContinuityId,
    replacementEpoch: input.replacementEpoch,
    batchId,
    exactEntries: exactEntries.map(projectedEntryToWire),
    cursorEntryId: input.cursor ?? "",
    leafEntryId: input.leaf.entryId ?? "",
    treeDigest: input.leaf.treeDigest,
  };
  return Object.freeze({
    schemaRef: PI_PROJECTION_REPLACEMENT_SCHEMA_REF,
    payload: toBinary(PiPersistedProjectionReplacementSchema, message),
    batchId,
  });
}

export function encodePiVolatileProjectionSnapshot(input: {
  readonly externalContinuityId: string;
  readonly exactEntries: readonly PiProjectedEntry[];
  readonly cursor: PiProjectionCursor;
  readonly leaf: PiProjectionLeaf;
}): EncodedPiProjectionEnvelope {
  boundedText(input.externalContinuityId, "external continuity id", 512);
  const exactEntries = validateProjectedEntries(input.exactEntries);
  validateCompleteTree(exactEntries, input.leaf.entryId);
  const computedTreeDigest = piTreeDigest(exactEntries);
  if (computedTreeDigest !== input.leaf.treeDigest) {
    throw new Error("Pi volatile snapshot tree digest disagrees with exact membership");
  }
  const expectedCursor = exactEntries.at(-1)?.stableEntryId ?? null;
  if (expectedCursor !== input.cursor) {
    throw new Error("Pi volatile snapshot cursor is not the append-order tail");
  }
  const batchId = batchDigest([
    "volatile",
    input.externalContinuityId,
    canonicalJson(exactEntries),
    input.cursor ?? "",
    input.leaf.entryId ?? "",
    input.leaf.treeDigest,
  ]);
  const message = {
    $typeName: "patchbay.PiVolatileProjectionSnapshot" as const,
    externalContinuityId: input.externalContinuityId,
    batchId,
    exactEntries: exactEntries.map(projectedEntryToWire),
    cursorEntryId: input.cursor ?? "",
    leafEntryId: input.leaf.entryId ?? "",
    treeDigest: input.leaf.treeDigest,
  };
  return Object.freeze({
    schemaRef: PI_VOLATILE_PROJECTION_SCHEMA_REF,
    payload: toBinary(PiVolatileProjectionSnapshotSchema, message),
    batchId,
  });
}

export function piProjectedEntriesEqual(left: PiProjectedEntry, right: PiProjectedEntry): boolean {
  return left.stableEntryId === right.stableEntryId
    && left.parentEntryId === right.parentEntryId
    && left.contentDigest === right.contentDigest
    && left.presentationItems.length === right.presentationItems.length
    && left.presentationItems.every((item, index) => {
      const candidate = right.presentationItems[index]!;
      return item.membershipId === candidate.membershipId
        && item.transcriptEventJson === candidate.transcriptEventJson;
    });
}

export function piProjectionLeavesEqual(left: PiProjectionLeaf, right: PiProjectionLeaf): boolean {
  return left.entryId === right.entryId && left.treeDigest === right.treeDigest;
}

export function piTreeDigest(entries: readonly PiProjectedEntry[]): string {
  return createHash("sha256")
    .update(JSON.stringify(entries.map((entry) => [entry.stableEntryId, entry.parentEntryId])))
    .digest("hex");
}

export function validateProjectedEntries(entries: readonly PiProjectedEntry[]): readonly PiProjectedEntry[] {
  const identities = new Set<string>();
  return Object.freeze(entries.map((entry) => {
    const stableEntryId = boundedText(entry.stableEntryId, "stable entry id", MAX_ENTRY_ID_BYTES);
    if (!PI_ENTRY_ID_PATTERN.test(stableEntryId)) throw new Error("Pi projected entry id is invalid");
    if (identities.has(stableEntryId)) throw new Error("Pi projection repeats a stable entry id");
    identities.add(stableEntryId);
    const parentEntryId = entry.parentEntryId === null
      ? null
      : boundedText(entry.parentEntryId, "parent entry id", MAX_ENTRY_ID_BYTES);
    if (parentEntryId !== null && !PI_ENTRY_ID_PATTERN.test(parentEntryId)) {
      throw new Error("Pi projected parent id is invalid");
    }
    const contentDigest = requiredDigest(entry.contentDigest, "entry content digest");
    const itemIds = new Set<string>();
    const presentationItems = Object.freeze(entry.presentationItems.map((item) => {
      const membershipId = boundedText(item.membershipId, "presentation membership id", 256);
      if (itemIds.has(membershipId)) throw new Error("Pi projection repeats presentation membership");
      itemIds.add(membershipId);
      const parsed: unknown = JSON.parse(item.transcriptEventJson);
      if (!isRecord(parsed) || typeof parsed["kind"] !== "string") {
        throw new Error("Pi projected transcript event is malformed");
      }
      return Object.freeze({ membershipId, transcriptEventJson: item.transcriptEventJson });
    }));
    return Object.freeze({ stableEntryId, parentEntryId, contentDigest, presentationItems });
  }));
}

function projectRawEntries(
  rawEntries: readonly unknown[],
  externalContinuityId: string,
): readonly PiProjectedEntry[] {
  boundedText(externalContinuityId, "external continuity id", 512);
  return Object.freeze(rawEntries.map((raw) => {
    if (!isRecord(raw)) throw new Error("Pi persisted entry is not an object");
    const stableEntryId = boundedText(raw["id"], "persisted entry id", MAX_ENTRY_ID_BYTES);
    if (!PI_ENTRY_ID_PATTERN.test(stableEntryId)) throw new Error("Pi persisted entry id is invalid");
    const parent = raw["parentId"];
    const parentEntryId = parent === null
      ? null
      : boundedText(parent, "persisted parent id", MAX_ENTRY_ID_BYTES);
    if (parentEntryId !== null && !PI_ENTRY_ID_PATTERN.test(parentEntryId)) {
      throw new Error("Pi persisted parent id is invalid");
    }
    const transcript = projectSessionEntries([raw as never], externalContinuityId);
    const presentationItems = Object.freeze(transcript.map((event, index) => Object.freeze({
      membershipId: presentationMembershipId(externalContinuityId, stableEntryId, event, index),
      transcriptEventJson: JSON.stringify(event),
    })));
    return Object.freeze({
      stableEntryId,
      parentEntryId,
      contentDigest: createHash("sha256").update(canonicalJson(raw)).digest("hex"),
      presentationItems,
    });
  }));
}

function validateCompleteTree(entries: readonly PiProjectedEntry[], leafId: string | null): void {
  const seen = new Set<string>();
  let roots = 0;
  for (const entry of entries) {
    if (seen.has(entry.stableEntryId)) throw new Error("Pi exact projection repeats a stable entry id");
    if (entry.parentEntryId === null) roots += 1;
    else if (entry.parentEntryId === entry.stableEntryId || !seen.has(entry.parentEntryId)) {
      throw new Error("Pi exact projection has an invalid append-order parent");
    }
    seen.add(entry.stableEntryId);
  }
  if (entries.length === 0) {
    if (leafId !== null) throw new Error("empty Pi projection has a leaf");
    return;
  }
  if (roots !== 1) throw new Error("Pi exact projection must contain exactly one tree root");
  if (leafId === null || !seen.has(leafId)) throw new Error("Pi current leaf is absent from the exact tree");
}

function projectedEntryToWire(entry: PiProjectedEntry) {
  return {
    $typeName: PiPersistedProjectionEntrySchema.typeName,
    stableEntryId: entry.stableEntryId,
    parentEntryId: entry.parentEntryId ?? "",
    contentDigest: entry.contentDigest,
    presentationItems: entry.presentationItems.map((item) => ({
      $typeName: PiPersistedPresentationItemSchema.typeName,
      membershipId: item.membershipId,
      transcriptEventJson: new TextEncoder().encode(item.transcriptEventJson),
    })),
  };
}

function presentationMembershipId(
  continuityId: string,
  entryId: string,
  event: TranscriptEvent,
  index: number,
): string {
  return `pi:${batchDigest([continuityId, entryId, event.kind, String(index)])}`;
}

function batchDigest(parts: readonly string[]): string {
  const hash = createHash("sha256");
  for (const part of parts) {
    hash.update(String(Buffer.byteLength(part)));
    hash.update(":");
    hash.update(part);
    hash.update("\0");
  }
  return hash.digest("hex");
}

function canonicalJson(value: unknown): string {
  if (value === null || typeof value === "string" || typeof value === "boolean") {
    return JSON.stringify(value);
  }
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new Error("Pi persisted entry contains a non-finite number");
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (!isRecord(value)) throw new Error("Pi persisted entry contains unsupported data");
  return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`).join(",")}}`;
}

function positiveEpoch(value: bigint): void {
  if (typeof value !== "bigint" || value <= 0n) throw new Error("Pi replacement epoch must be positive");
}

function requiredDigest(value: string, field: string): string {
  if (!DIGEST_PATTERN.test(value)) throw new Error(`Pi ${field} is invalid`);
  return value;
}

function boundedText(value: unknown, field: string, maxBytes: number): string {
  if (
    typeof value !== "string"
    || value.length === 0
    || Buffer.byteLength(value) > maxBytes
    || value.includes("\0")
  ) {
    throw new Error(`Pi ${field} is invalid`);
  }
  return value;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}
