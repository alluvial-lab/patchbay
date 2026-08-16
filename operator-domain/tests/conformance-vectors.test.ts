import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import test from "node:test";
import { ContinuationContextStatus } from "@patchbay/contracts";

import {
  AuthoritativeCursorReplacement,
  externalCursorScopeKey,
  type AtomicExternalCursorProjectionStore,
  type ExternalCursorFetchPort,
  type ExternalCursorProjectionRecord,
  type ExternalCursorPublishPort,
  type ExternalCursorScope,
  type ExternalCursorValueContract,
  type ProjectionReplacement,
} from "../src/reconciliation/external_cursor.js";
import { continuationContextStatusName } from "../src/spawn.js";

const RUNNER = "operator-domain" as const;

type Entry = { readonly id: string; readonly value: string };
type Cursor = string;
type Leaf = string;
type CursorRecord = ExternalCursorProjectionRecord<Entry, Cursor, Leaf>;

interface ImplementationCheck { runner: string; case: string }
interface Vector {
  vector_id: string;
  property_id: string;
  promotion_status: string;
  implementation_checks?: readonly ImplementationCheck[];
  input: unknown;
  expected_outcome: unknown;
}
interface RequestedCheck { vector_id: string; case: string }

class MemoryStore implements AtomicExternalCursorProjectionStore<ExternalCursorScope, Entry, Cursor, Leaf> {
  readonly records = new Map<string, CursorRecord>();

  async load(scope: ExternalCursorScope): Promise<CursorRecord | undefined> {
    return this.records.get(externalCursorScopeKey(scope));
  }

  async compareAndSwap(scope: ExternalCursorScope, expectedRecordVersion: bigint, next: CursorRecord): Promise<void> {
    const key = externalCursorScopeKey(scope);
    const current = this.records.get(key);
    if (!current || current.recordVersion !== expectedRecordVersion) throw new Error("stale record version");
    this.records.set(key, structuredClone(next));
  }
}

function vectors(): ReadonlyMap<string, Vector> {
  const directory = path.resolve(process.cwd(), "../contracts/vectors");
  return new Map(
    readdirSync(directory)
      .filter((filename) => filename.endsWith(".json"))
      .sort()
      .map((filename) => JSON.parse(readFileSync(path.join(directory, filename), "utf8")) as Vector)
      .map((vector) => [vector.vector_id, vector]),
  );
}

function requestedChecks(): readonly RequestedCheck[] {
  return process.env.PATCHBAY_CONFORMANCE_REQUESTS
    ? JSON.parse(process.env.PATCHBAY_CONFORMANCE_REQUESTS) as RequestedCheck[]
    : [];
}

function object(value: unknown, name: string): Record<string, unknown> {
  assert.ok(value && typeof value === "object" && !Array.isArray(value), `${name} must be an object`);
  return value as Record<string, unknown>;
}

function text(value: unknown, name: string): string {
  assert.equal(typeof value, "string", `${name} must be a string`);
  return value as string;
}

function integer(value: unknown, name: string): bigint {
  if (typeof value !== "number") throw new Error(`${name} must be a number`);
  assert.ok(Number.isSafeInteger(value) && value >= 0, `${name} must be a non-negative safe integer`);
  return BigInt(value);
}

function textList(value: unknown, name: string): readonly string[] {
  assert.ok(Array.isArray(value) && value.every((entry) => typeof entry === "string"), `${name} must be a string array`);
  return value;
}

function entries(ids: readonly string[]): readonly Entry[] {
  return ids.map((id) => ({ id, value: `value:${id}` }));
}

async function externalCursorAuthoritativeReplacement(vector: Vector): Promise<void> {
  const input = object(vector.input, "input");
  const expected = object(vector.expected_outcome, "expected outcome");
  const scope: ExternalCursorScope & { readonly patchbayGeneration: bigint } = {
    adapterId: text(input.adapter_id, "adapter id"),
    deploymentScope: text(input.deployment_scope, "deployment scope"),
    externalContinuityId: text(input.external_continuity_id, "external continuity id"),
    patchbayGeneration: BigInt((input.patchbay_generations as number[])[0]!),
  };
  const successorScope = {
    ...scope,
    patchbayGeneration: BigInt((input.patchbay_generations as number[])[1]!),
  };
  const crossNativeScope: ExternalCursorScope = {
    adapterId: scope.adapterId,
    deploymentScope: scope.deploymentScope,
    externalContinuityId: text(input.cross_native_continuity_id, "cross-native continuity id"),
  };
  assert.equal(
    externalCursorScopeKey(scope) === externalCursorScopeKey(successorScope),
    expected.external_scope_survives_patchbay_generation,
  );
  assert.notEqual(externalCursorScopeKey(scope), externalCursorScopeKey(crossNativeScope));

  const store = new MemoryStore();
  const oldProjection: ProjectionReplacement<Entry, Cursor, Leaf> = {
    replacementEpoch: integer(input.old_epoch, "old epoch"),
    exactEntries: entries(textList(input.old_projection_ids, "old projection ids")),
    cursor: text(input.old_cursor, "old cursor"),
    leaf: text(input.old_leaf, "old leaf"),
  };
  store.records.set(externalCursorScopeKey(scope), {
    recordVersion: 1n,
    freshness: "current",
    projection: oldProjection,
  });

  const replacementEntries = entries(textList(input.replacement_projection_ids, "replacement projection ids"));
  const fetch: ExternalCursorFetchPort<ExternalCursorScope, Entry, Cursor, Leaf> = {
    async fetchKnown() { throw new Error("known-cursor fetch is not part of this unknown-cursor vector"); },
    async fetchComplete() {
      return { entries: replacementEntries, leaf: text(input.replacement_leaf, "replacement leaf") };
    },
  };
  const publications: ProjectionReplacement<Entry, Cursor, Leaf>[] = [];
  const publish: ExternalCursorPublishPort<ExternalCursorScope, Entry, Cursor, Leaf> = {
    async publishKnownSuffix() { throw new Error("known suffix publication is not expected"); },
    async publishReplacement(_scope, replacement) { publications.push(structuredClone(replacement)); },
  };
  const values: ExternalCursorValueContract<Entry, Cursor, Leaf> = {
    entryIdentity: (entry) => entry.id,
    entriesEqual: (left, right) => left.id === right.id && left.value === right.value,
    cursorsEqual: (left, right) => left === right,
    leavesEqual: (left, right) => left === right,
  };
  const replacement = AuthoritativeCursorReplacement.create(store, fetch, publish, values);
  const staged = await replacement.stageReplacement(successorScope);
  const beforeCommit = await replacement.read(scope);
  assert.equal(beforeCommit?.freshness, "stale");
  assert.deepEqual(beforeCommit?.projection, oldProjection, "staging retains the complete old projection as stale");
  assert.equal(staged.replacementEpoch, integer(input.replacement_epoch, "replacement epoch"));

  const committed: ProjectionReplacement<Entry, Cursor, Leaf> = {
    replacementEpoch: staged.replacementEpoch,
    exactEntries: staged.entries,
    cursor: text(input.replacement_cursor, "replacement cursor"),
    leaf: staged.leaf,
  };
  await replacement.commitReplacement(scope, committed);
  const current = await replacement.read(successorScope);
  assert.equal(current?.freshness, "current");
  assert.deepEqual(current?.projection.exactEntries.map((entry) => entry.id), expected.external_projection_ids);
  assert.equal(current?.projection.cursor, expected.external_cursor);
  assert.equal(current?.projection.leaf, expected.external_leaf);
  assert.equal(current?.projection.replacementEpoch, BigInt(expected.external_epoch as number));
  assert.equal(current?.projection.exactEntries.some((entry) => entry.id === "omitted-stale"), !expected.omitted_stale_entry_removed);
  assert.deepEqual(publications, [committed], "publication carries one complete replacement, never a cursor-only prefix");

  await assert.rejects(
    replacement.reconcileKnown(crossNativeScope, text(input.old_cursor, "old cursor")),
    /not initialized/,
  );
  assert.equal(expected.cross_native_cursor_reuse_rejected, true);
  assert.equal(expected.cursor_visible_before_exact_projection, false);
}

function continuationContextStatusPresentation(vector: Vector): void {
  const input = object(vector.input, "input");
  const expected = object(vector.expected_outcome, "expected outcome");
  const statuses = textList(input.adapter_context_statuses, "adapter context statuses");
  const values = statuses.map((status) => {
    switch (status) {
      case "CONTINUATION_CONTEXT_STATUS_RESUMED": return ContinuationContextStatus.RESUMED;
      case "CONTINUATION_CONTEXT_STATUS_NEW_CONTEXT": return ContinuationContextStatus.NEW_CONTEXT;
      case "CONTINUATION_CONTEXT_STATUS_UNKNOWN": return ContinuationContextStatus.UNKNOWN;
      default: throw new Error(`unknown continuation-context status ${status}`);
    }
  });
  assert.deepEqual(
    values.map(continuationContextStatusName),
    textList(expected.presentation_statuses, "presentation statuses"),
  );
  assert.throws(
    () => continuationContextStatusName(ContinuationContextStatus.UNSPECIFIED),
    /unspecified/,
    "the shared presentation boundary fails closed on sentinel status",
  );
}

async function execute(vector: Vector, caseName: string): Promise<void> {
  assert.ok(vector.property_id);
  assert.ok(vector.promotion_status === "draft" || vector.promotion_status === "promoted");
  switch (caseName) {
    case "external_cursor_authoritative_replacement":
      await externalCursorAuthoritativeReplacement(vector);
      return;
    case "continuation_context_status_presentation":
      continuationContextStatusPresentation(vector);
      return;
    default:
      throw new Error(`unhandled ${RUNNER} conformance case ${vector.vector_id}:${caseName}`);
  }
}

test("conformance vector runner", async () => {
  const available = vectors();
  for (const request of requestedChecks()) {
    const vector = available.get(request.vector_id);
    assert.ok(vector, `unknown vector id ${request.vector_id}`);
    assert.ok(
      vector.implementation_checks?.some((check) => check.runner === RUNNER && check.case === request.case),
      `unregistered requested check ${request.vector_id}:${request.case}`,
    );
    await execute(vector, request.case);
    console.log(`PATCHBAY_CONFORMANCE_EXECUTED=${request.vector_id}:${request.case}`);
  }
});
