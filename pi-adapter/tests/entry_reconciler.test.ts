import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { create, fromBinary } from "@bufbuild/protobuf";
import {
  ActorEndpointRefSchema,
  ActorIdSchema,
  AdapterIdSchema,
  ExternalRuntimeRefSchema,
  GenerationSchema,
  LogicalTargetIdSchema,
  ObservationKind,
  ObservationSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  PiPersistedProjectionReplacementSchema,
  PiVolatileProjectionSnapshotSchema,
  RuntimeGenerationRefSchema,
  RuntimeSessionIdSchema,
  TargetScopeKind,
  TargetScopeSchema,
  type Observation,
  type RuntimeGenerationRef,
} from "@patchbay/contracts";
import {
  foldPiPersistedProjectionObservation,
  foldPiVolatileProjectionObservation,
  PI_PERSISTED_REPLACEMENT_SCHEMA_REF,
  PI_VOLATILE_PROJECTION_SCHEMA_REF,
  type PiPersistedProjectionState,
  type PiVolatileProjectionState,
} from "@patchbay/operator-domain";
import {
  FilePiCursorStore,
  derivePiSessionContinuityKey,
} from "../src/cursor_store.js";
import {
  PiEntryReconciler,
  serializePiStagedCursorPublication,
  restorePiStagedCursorPublication,
  type PiCursorReconciliationEvidence,
  type PiProjectionObservationPort,
} from "../src/entry_reconciler.js";
import { PiUnknownCursorError } from "../src/pi_session.js";
import {
  PI_PROJECTION_REPLACEMENT_SCHEMA_REF,
  projectCompletePiEntries,
} from "../src/pi_projection.js";
import type { MaterializedSessionSeal } from "../src/session_file.js";

const adapterId = "pi";
const deploymentScope = "machine-a";
const logicalTargetId = "logical-a";

class RecordingPublisher implements PiProjectionObservationPort {
  readonly publications: { runtime: RuntimeGenerationRef; schemaRef: string; payload: Uint8Array }[] = [];
  failAfterDurableAck = false;

  async publish(runtime: RuntimeGenerationRef, schemaRef: string, payload: Uint8Array): Promise<void> {
    this.publications.push({ runtime, schemaRef, payload: Uint8Array.from(payload) });
    if (this.failAfterDurableAck) {
      this.failAfterDurableAck = false;
      throw new Error("injected lost core acknowledgement");
    }
  }
}

class FailBeforeCasStore extends FilePiCursorStore {
  failBeforeCas = false;

  override async compareAndSwap(...args: Parameters<FilePiCursorStore["compareAndSwap"]>): Promise<void> {
    if (this.failBeforeCas) {
      this.failBeforeCas = false;
      throw new Error("injected crash after core ack before local CAS");
    }
    await super.compareAndSwap(...args);
  }
}

function runtime(generation: bigint): RuntimeGenerationRef {
  return create(RuntimeGenerationRefSchema, {
    logicalTargetId: create(LogicalTargetIdSchema, { value: logicalTargetId }),
    externalRuntime: create(ExternalRuntimeRefSchema, {
      adapterId: create(AdapterIdSchema, { value: adapterId }),
      deploymentScope,
      runtimeSessionId: create(RuntimeSessionIdSchema, { value: "pi-session" }),
      generation: create(GenerationSchema, { value: generation }),
    }),
  });
}

function projectionObservation(
  publication: RecordingPublisher["publications"][number],
): Observation {
  const external = publication.runtime.externalRuntime!;
  return create(ObservationSchema, {
    sender: create(ActorEndpointRefSchema, {
      actorId: create(ActorIdSchema, { value: external.adapterId!.value }),
    }),
    kind: ObservationKind.EVENT,
    targetScope: create(TargetScopeSchema, {
      kind: TargetScopeKind.RUNTIME_SESSION,
      adapterId: external.adapterId,
      deploymentScope: external.deploymentScope,
      runtimeSessionId: external.runtimeSessionId,
      sessionGeneration: external.generation,
    }),
    payload: create(PayloadEnvelopeSchema, {
      contentType: PayloadContentType.PROTOBUF,
      schemaRef: publication.schemaRef,
      payload: publication.payload,
    }),
  });
}

function sessionRoot(timestamp = "2026-08-16T00:00:00.000Z") {
  return { type: "session_info", id: "root", parentId: null, timestamp, name: "private-label" };
}
function user(id: string, text: string, parentId = "root") {
  return {
    type: "message", id, parentId, timestamp: "2026-08-16T00:00:01.000Z",
    message: { role: "user", content: text, timestamp: 1_755_302_401_000 },
  };
}
function assistant(id: string, text: string, parentId: string) {
  return {
    type: "message", id, parentId, timestamp: "2026-08-16T00:00:02.000Z",
    message: {
      role: "assistant", content: [{ type: "text", text }], timestamp: 1_755_302_402_000,
      provider: "fixture", model: "offline", usage: {
        input: 1, output: 1, cacheRead: 0, cacheWrite: 0, totalTokens: 2,
        cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
      }, stopReason: "stop",
    },
  };
}

async function fixture(options: { storeClass?: typeof FilePiCursorStore } = {}) {
  const root = await mkdtemp(join(tmpdir(), "patchbay-pi-entry-reconcile-"));
  const sessionDirectory = join(root, "sessions");
  const cursorDirectory = join(root, "cursors");
  await mkdir(sessionDirectory);
  const sessionPath = join(sessionDirectory, "session.jsonl");
  await writeFile(sessionPath, "fixture\n", { mode: 0o600 });
  const Store = options.storeClass ?? FilePiCursorStore;
  const store = new Store(cursorDirectory);
  const publisher = new RecordingPublisher();
  const reconciler = new PiEntryReconciler(store, publisher);
  return { root, sessionDirectory, cursorDirectory, sessionPath, store, publisher, reconciler };
}

function sealFor(
  sessionPath: string,
  entries: readonly unknown[],
  leafId: string,
): MaterializedSessionSeal {
  const exact = projectCompletePiEntries(entries, leafId, "temporary-continuity");
  return {
    canonicalPath: sessionPath,
    sessionRootId: "root",
    sessionId: "pi-session",
    device: 1n,
    inode: 1n,
    size: 1n,
    contentDigest: "a".repeat(64),
    treeDigest: exact.leaf.treeDigest,
    orderedEntryIds: exact.entries.map((entry) => entry.stableEntryId),
    leafId,
  };
}

function evidence(
  sessionDirectory: string,
  sessionPath: string,
  entries: readonly unknown[],
  leafId: string,
  fetchKnown: PiCursorReconciliationEvidence["fetchKnown"],
): PiCursorReconciliationEvidence {
  return {
    logicalTargetId,
    configuredSessionRoot: sessionDirectory,
    piSessionId: "pi-session",
    declaredSessionPath: sessionPath,
    materialization: { kind: "materialized", seal: sealFor(sessionPath, entries, leafId) },
    completeEntries: entries,
    leafId,
    fetchKnown,
  };
}

async function scopeFor(sessionDirectory: string, sessionPath: string) {
  return (await derivePiSessionContinuityKey({
    adapterId,
    deploymentScope,
    piSessionId: "pi-session",
    configuredSessionRoot: sessionDirectory,
    canonicalSessionPath: sessionPath,
  })).scope;
}

test("N+1 loads N's same-Pi-session cursor and response-loss retry is byte-idempotent", async () => {
  const f = await fixture();
  try {
    const beforeRaw = [sessionRoot(), user("user-1", "hello")];
    const afterRaw = [...beforeRaw, assistant("assistant-1", "world", "user-1")];
    const scope = await scopeFor(f.sessionDirectory, f.sessionPath);
    const before = projectCompletePiEntries(beforeRaw, "user-1", scope.externalContinuityId);
    await f.store.initializeCurrent(scope, logicalTargetId, {
      recordVersion: 1n,
      freshness: "current",
      projection: { replacementEpoch: 1n, exactEntries: before.entries, cursor: before.cursor, leaf: before.leaf },
    });

    let observedCursor = "";
    const staged = await f.reconciler.stageClaimedSuccessor(runtime(2n), evidence(
      f.sessionDirectory,
      f.sessionPath,
      afterRaw,
      "assistant-1",
      async (cursor) => {
        observedCursor = cursor;
        return { entries: [afterRaw[2]!], leafId: "assistant-1" };
      },
    ));
    assert.equal(staged.mode, "known");
    assert.equal(observedCursor, "user-1", "N+1 uses N's native cursor, not Patchbay generation");
    assert.equal(f.publisher.publications.length, 0, "claimed successor remains unpublished");
    assert.equal((await f.store.load(scope))?.projection.cursor, "user-1", "candidate loss leaves N current");
    assert.equal((await f.store.load(scope))?.freshness, "current");

    f.publisher.failAfterDurableAck = true;
    await assert.rejects(f.reconciler.publishAfterPromotion(staged), /lost core acknowledgement/);
    assert.equal((await f.store.load(scope))?.projection.cursor, "user-1");
    await f.reconciler.publishAfterPromotion(staged);
    assert.equal((await f.store.load(scope))?.projection.cursor, "assistant-1");
    assert.equal(f.publisher.publications.length, 2);
    assert.deepEqual(f.publisher.publications[0]!.payload, f.publisher.publications[1]!.payload);
  } finally {
    await rm(f.root, { recursive: true, force: true });
  }
});

test("unknown cursor stages old projection stale then one exact replacement removes omission", async () => {
  const f = await fixture();
  try {
    const oldRaw = [sessionRoot(), user("omitted-old", "stale")];
    const nextRaw = [sessionRoot(), user("current", "fresh")];
    const scope = await scopeFor(f.sessionDirectory, f.sessionPath);
    const old = projectCompletePiEntries(oldRaw, "omitted-old", scope.externalContinuityId);
    await f.store.initializeCurrent(scope, logicalTargetId, {
      recordVersion: 1n,
      freshness: "current",
      projection: { replacementEpoch: 4n, exactEntries: old.entries, cursor: old.cursor, leaf: old.leaf },
    });

    const staged = await f.reconciler.stageClaimedSuccessor(runtime(5n), evidence(
      f.sessionDirectory,
      f.sessionPath,
      nextRaw,
      "current",
      async (cursor) => { throw new PiUnknownCursorError(cursor); },
    ));
    assert.equal(staged.mode, "replacement");
    assert.equal(f.publisher.publications.length, 0);
    const stale = await f.store.load(scope);
    assert.equal(stale?.freshness, "stale");
    assert.equal(stale?.projection.exactEntries.some((entry) => entry.stableEntryId === "omitted-old"), true);

    await f.reconciler.publishAfterPromotion(staged);
    const current = await f.store.load(scope);
    assert.equal(current?.freshness, "current");
    assert.deepEqual(current?.projection.exactEntries.map((entry) => entry.stableEntryId), ["root", "current"]);
    assert.equal(f.publisher.publications.length, 1);
    assert.equal(f.publisher.publications[0]!.schemaRef, PI_PROJECTION_REPLACEMENT_SCHEMA_REF);
    const envelope = fromBinary(PiPersistedProjectionReplacementSchema, f.publisher.publications[0]!.payload);
    assert.deepEqual(envelope.exactEntries.map((entry) => entry.stableEntryId), ["root", "current"]);
  } finally {
    await rm(f.root, { recursive: true, force: true });
  }
});

test("core ack followed by local-CAS crash resends the same replacement and commits once", async () => {
  const f = await fixture({ storeClass: FailBeforeCasStore });
  try {
    const nextRaw = [sessionRoot(), user("current", "fresh")];
    const staged = await f.reconciler.stageClaimedSuccessor(runtime(1n), evidence(
      f.sessionDirectory,
      f.sessionPath,
      nextRaw,
      "current",
      async (cursor) => { throw new PiUnknownCursorError(cursor); },
    ));
    const store = f.store as FailBeforeCasStore;
    store.failBeforeCas = true;
    await assert.rejects(f.reconciler.publishAfterPromotion(staged), /after core ack before local CAS/);
    const scope = staged.scope!;
    assert.equal((await store.load(scope))?.freshness, "stale");

    const recovered = restorePiStagedCursorPublication(
      serializePiStagedCursorPublication(staged),
      staged.runtime,
    );
    await f.reconciler.publishRecoveredAfterPromotion(recovered);
    assert.equal((await store.load(scope))?.freshness, "current");
    assert.equal(f.publisher.publications.length, 2);
    assert.deepEqual(f.publisher.publications[0]!.payload, f.publisher.publications[1]!.payload);
  } finally {
    await rm(f.root, { recursive: true, force: true });
  }
});

test("adapter restart replays volatile snapshots non-authoritatively then materializes through epoch one", async () => {
  const f = await fixture();
  try {
    const firstRaw = [sessionRoot(), user("first", "volatile before restart")];
    const secondRaw = [sessionRoot(), user("second", "volatile after restart")];
    const memoryEvidence = (raw: readonly unknown[], leafId: string): PiCursorReconciliationEvidence => ({
      logicalTargetId,
      configuredSessionRoot: f.sessionDirectory,
      piSessionId: "pi-session",
      declaredSessionPath: f.sessionPath,
      materialization: { kind: "memory_only", sessionId: "pi-session", declaredPath: f.sessionPath },
      completeEntries: raw,
      leafId,
      fetchKnown: async () => { throw new Error("memory-only state must not claim a durable cursor"); },
    });

    const beforeRestart = await f.reconciler.stageClaimedSuccessor(
      runtime(1n),
      memoryEvidence(firstRaw, "first"),
    );
    assert.equal(beforeRestart.mode, "volatile-snapshot");
    assert.equal(beforeRestart.replacementEpoch, null);
    assert.equal(beforeRestart.restartStable, false);
    await f.reconciler.publishAfterPromotion(beforeRestart);

    // A new reconciler is a new adapter process: it has no recoverable volatile
    // epoch, and therefore emits a distinct non-authoritative snapshot instead.
    const restarted = new PiEntryReconciler(new FilePiCursorStore(f.cursorDirectory), f.publisher);
    const afterRestart = await restarted.stageClaimedSuccessor(
      runtime(2n),
      memoryEvidence(secondRaw, "second"),
    );
    assert.equal(afterRestart.mode, "volatile-snapshot");
    assert.equal(afterRestart.replacementEpoch, null);
    await restarted.publishAfterPromotion(afterRestart);
    await assert.rejects(
      stat(f.cursorDirectory),
      (error: unknown) => (error as NodeJS.ErrnoException).code === "ENOENT",
    );
    assert.deepEqual(
      f.publisher.publications.slice(0, 2).map((publication) => publication.schemaRef),
      [PI_VOLATILE_PROJECTION_SCHEMA_REF, PI_VOLATILE_PROJECTION_SCHEMA_REF],
    );
    assert.equal(
      "replacementEpoch" in fromBinary(
        PiVolatileProjectionSnapshotSchema,
        f.publisher.publications[0]!.payload,
      ),
      false,
    );

    let volatileState: PiVolatileProjectionState | undefined;
    for (const publication of f.publisher.publications.slice(0, 2)) {
      volatileState = foldPiVolatileProjectionObservation(
        volatileState,
        projectionObservation(publication),
      )!.state;
    }
    assert.ok(volatileState);
    assert.deepEqual(
      volatileState.exactEntries.map((entry) => entry.stableEntryId),
      ["root", "second"],
    );

    const materialized = await restarted.stageClaimedSuccessor(runtime(2n), evidence(
      f.sessionDirectory,
      f.sessionPath,
      secondRaw,
      "second",
      async (cursor) => { throw new PiUnknownCursorError(cursor); },
    ));
    assert.equal(materialized.mode, "replacement");
    assert.equal(materialized.replacementEpoch, 1n, "volatile state cannot seed a durable epoch");
    assert.equal(materialized.restartStable, true);
    await restarted.publishAfterPromotion(materialized);
    assert.equal(f.publisher.publications[2]!.schemaRef, PI_PERSISTED_REPLACEMENT_SCHEMA_REF);
    let persistedState: PiPersistedProjectionState | undefined;
    persistedState = foldPiPersistedProjectionObservation(
      persistedState,
      projectionObservation(f.publisher.publications[2]!),
    )!.state;
    assert.equal(persistedState.replacementEpoch, 1n);

    const scope = await scopeFor(f.sessionDirectory, f.sessionPath);
    const current = await f.store.load(scope);
    assert.equal(current?.freshness, "current");
    assert.equal(current?.projection.replacementEpoch, 1n);
    assert.equal(current?.projection.cursor, "second");
  } finally {
    await rm(f.root, { recursive: true, force: true });
  }
});

test("different Pi continuity does not load another cursor and reverse binding rejects a second target", async () => {
  const f = await fixture();
  try {
    const original = await scopeFor(f.sessionDirectory, f.sessionPath);
    const projection = projectCompletePiEntries([sessionRoot()], "root", original.externalContinuityId);
    await f.store.initializeCurrent(original, logicalTargetId, {
      recordVersion: 1n,
      freshness: "current",
      projection: { replacementEpoch: 1n, exactEntries: projection.entries, cursor: projection.cursor, leaf: projection.leaf },
    });
    const different = (await derivePiSessionContinuityKey({
      adapterId,
      deploymentScope,
      piSessionId: "different-pi-session",
      configuredSessionRoot: f.sessionDirectory,
      canonicalSessionPath: f.sessionPath,
    })).scope;
    assert.equal(await f.store.load(different), undefined);
    await assert.rejects(f.store.bindLogicalTarget(original, "logical-b"), /another logical target/);
  } finally {
    await rm(f.root, { recursive: true, force: true });
  }
});

test("exact tree retains abandoned branches and pre-compaction entries while current leaf stays structural", () => {
  const raw = [
    sessionRoot(),
    user("abandoned", "older branch"),
    user("kept", "current branch"),
    {
      type: "compaction", id: "compact", parentId: "kept",
      timestamp: "2026-08-16T00:00:03.000Z", summary: "private compaction summary",
      firstKeptEntryId: "root", tokensBefore: 2,
      usage: { input: 1, output: 1, cacheRead: 0, cacheWrite: 0, totalTokens: 2, cost: {
        input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0,
      } },
    },
  ];
  const exact = projectCompletePiEntries(raw, "compact", `pi1:${"a".repeat(43)}`);
  assert.deepEqual(exact.entries.map((entry) => entry.stableEntryId), ["root", "abandoned", "kept", "compact"]);
  assert.equal(exact.cursor, "compact");
  assert.equal(exact.leaf.entryId, "compact");
  assert.equal(exact.entries.at(-1)?.presentationItems.length, 1);
  assert.equal(exact.entries.some((entry) => entry.stableEntryId === "abandoned"), true);
});

test("generated replacement carries no raw path or Pi label entry content", async () => {
  const f = await fixture();
  try {
    const raw = [
      sessionRoot(),
      user("current", "safe transcript"),
      {
        type: "custom", id: "control-marker", parentId: "current",
        timestamp: "2026-08-16T00:00:03.000Z", customType: "private-control-kind",
        data: { localPath: f.sessionPath, label: "private-custom-label" },
      },
    ];
    const staged = await f.reconciler.stageClaimedSuccessor(runtime(1n), evidence(
      f.sessionDirectory,
      f.sessionPath,
      raw,
      "control-marker",
      async (cursor) => { throw new PiUnknownCursorError(cursor); },
    ));
    await f.reconciler.publishAfterPromotion(staged);
    const bytes = Buffer.from(f.publisher.publications[0]!.payload).toString("utf8");
    assert.doesNotMatch(bytes, new RegExp(f.sessionPath.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")));
    assert.doesNotMatch(bytes, /private-label|private-control-kind|private-custom-label/);
  } finally {
    await rm(f.root, { recursive: true, force: true });
  }
});
