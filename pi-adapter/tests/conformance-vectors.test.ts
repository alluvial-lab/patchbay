import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import test from "node:test";
import { create, fromBinary } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  ExternalRuntimeRefSchema,
  GenerationSchema,
  LogicalTargetIdSchema,
  PiPersistedProjectionReplacementSchema,
  RuntimeGenerationRefSchema,
  RuntimeSessionIdSchema,
} from "@patchbay/contracts";
import { FilePiCursorStore, derivePiSessionContinuityKey } from "../src/cursor_store.js";
import { PiEntryReconciler } from "../src/entry_reconciler.js";
import { PiUnknownCursorError } from "../src/pi_session.js";
import { projectCompletePiEntries } from "../src/pi_projection.js";

const RUNNER = "pi-adapter";
const VECTOR_ID = "spawn-reconnect-cursor-convergence";
const CASE = "pi_cursor_authoritative_replacement";

interface Vector {
  readonly vector_id: string;
  readonly implementation_checks?: readonly { readonly runner: string; readonly case: string }[];
  readonly input: Record<string, unknown>;
  readonly expected_outcome: Record<string, unknown>;
}

test("conformance vector runner", async () => {
  const requests = process.env.PATCHBAY_CONFORMANCE_REQUESTS
    ? JSON.parse(process.env.PATCHBAY_CONFORMANCE_REQUESTS) as { vector_id: string; case: string }[]
    : [];
  for (const request of requests) {
    if (request.vector_id !== VECTOR_ID || request.case !== CASE) {
      throw new Error(`unhandled ${RUNNER} conformance case ${request.vector_id}:${request.case}`);
    }
    const vector = JSON.parse(await readFile(
      resolve(process.cwd(), "../contracts/vectors/spawn-reconnect-cursor-convergence.json"),
      "utf8",
    )) as Vector;
    assert.ok(vector.implementation_checks?.some((check) => check.runner === RUNNER && check.case === CASE));
    await execute(vector);
    console.log(`PATCHBAY_CONFORMANCE_EXECUTED=${request.vector_id}:${request.case}`);
  }
});

async function execute(vector: Vector): Promise<void> {
  const oldIds = stringList(vector.input.old_projection_ids);
  const replacementIds = stringList(vector.input.replacement_projection_ids);
  const expectedIds = stringList(vector.expected_outcome.external_projection_ids);
  const root = await mkdtemp(join(tmpdir(), "patchbay-pi-vector-"));
  try {
    const sessionRoot = join(root, "sessions");
    const sessionPath = join(sessionRoot, "session.jsonl");
    await mkdir(sessionRoot);
    await writeFile(sessionPath, "fixture\n", { mode: 0o600 });
    const scope = (await derivePiSessionContinuityKey({
      adapterId: String(vector.input.adapter_id),
      deploymentScope: String(vector.input.deployment_scope),
      piSessionId: "pi-native-session-a",
      sessionRootId: oldIds[0]!,
      configuredSessionRoot: sessionRoot,
      canonicalSessionPath: sessionPath,
    })).scope;
    const oldRaw = rawTree(oldIds);
    const replacementRaw = rawTree(replacementIds);
    const oldProjection = projectCompletePiEntries(oldRaw, oldIds.at(-1)!, scope.externalContinuityId);
    const store = new FilePiCursorStore(join(root, "cursor-store"));
    await store.initializeCurrent(scope, "logical-a", {
      recordVersion: 1n,
      freshness: "current",
      projection: {
        replacementEpoch: BigInt(Number(vector.input.old_epoch)),
        exactEntries: oldProjection.entries,
        cursor: oldProjection.cursor,
        leaf: oldProjection.leaf,
      },
    });
    const publications: Uint8Array[] = [];
    const reconciler = new PiEntryReconciler(store, {
      async publish(_runtime, _schemaRef, payload) { publications.push(Uint8Array.from(payload)); },
    });
    const exact = projectCompletePiEntries(replacementRaw, replacementIds.at(-1)!, scope.externalContinuityId);
    const runtime = create(RuntimeGenerationRefSchema, {
      logicalTargetId: create(LogicalTargetIdSchema, { value: "logical-a" }),
      externalRuntime: create(ExternalRuntimeRefSchema, {
        adapterId: create(AdapterIdSchema, { value: String(vector.input.adapter_id) }),
        deploymentScope: String(vector.input.deployment_scope),
        runtimeSessionId: create(RuntimeSessionIdSchema, { value: "runtime-n-plus-one" }),
        generation: create(GenerationSchema, { value: 2n }),
      }),
    });
    const staged = await reconciler.stageClaimedSuccessor(runtime, {
      logicalTargetId: "logical-a",
      configuredSessionRoot: sessionRoot,
      piSessionId: "pi-native-session-a",
      declaredSessionPath: sessionPath,
      materialization: {
        kind: "materialized",
        seal: {
          canonicalPath: sessionPath,
          sessionRootId: oldIds[0]!,
          sessionId: "pi-native-session-a",
          device: 1n,
          inode: 1n,
          size: 1n,
          contentDigest: "a".repeat(64),
          treeDigest: exact.leaf.treeDigest,
          orderedEntryIds: exact.entries.map((entry) => entry.stableEntryId),
          leafId: replacementIds.at(-1)!,
        },
      },
      completeEntries: replacementRaw,
      leafId: replacementIds.at(-1)!,
      fetchKnown: async (cursor) => { throw new PiUnknownCursorError(cursor); },
    });
    assert.equal(publications.length, 0, "claimed successor is unpublished before promotion");
    assert.equal((await store.load(scope))?.freshness, "stale");
    assert.deepEqual(
      (await store.load(scope))?.projection.exactEntries.map((entry) => entry.stableEntryId),
      oldIds,
    );
    await reconciler.publishAfterPromotion(staged);
    assert.deepEqual(
      (await store.load(scope))?.projection.exactEntries.map((entry) => entry.stableEntryId),
      expectedIds,
    );
    assert.equal(
      (await store.load(scope))?.projection.exactEntries.some((entry) => entry.stableEntryId === "omitted-stale"),
      !Boolean(vector.expected_outcome.omitted_stale_entry_removed),
    );
    assert.deepEqual(
      fromBinary(PiPersistedProjectionReplacementSchema, publications[0]!).exactEntries.map((entry) => entry.stableEntryId),
      expectedIds,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

function rawTree(ids: readonly string[]): readonly Record<string, unknown>[] {
  return ids.map((id, index) => index === 0
    ? { type: "session_info", id, parentId: null, timestamp: "2026-08-16T00:00:00.000Z" }
    : {
      type: "message", id, parentId: ids[0], timestamp: "2026-08-16T00:00:01.000Z",
      message: { role: "user", content: id, timestamp: 1_755_302_401_000 + index },
    });
}

function stringList(value: unknown): readonly string[] {
  assert.ok(Array.isArray(value) && value.every((item) => typeof item === "string"));
  return value;
}
