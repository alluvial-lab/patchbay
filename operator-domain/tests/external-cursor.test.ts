import assert from "node:assert/strict";
import test from "node:test";

import {
  ExternalCursorInvariantError,
  ExternalCursorProjectionMachine,
  externalCursorScopeKey,
  type AtomicExternalCursorProjectionStore,
  type AuthoritativeCursorReplacement,
  type ExternalCursorProjectionRecord,
  type ExternalCursorScope,
  type ProjectionReplacement,
} from "../src/reconciliation/external_cursor.js";

interface Entry {
  readonly id: string;
  readonly value: string;
}

type Cursor = string;
type Leaf = string;
type Record = ExternalCursorProjectionRecord<Entry, Cursor, Leaf>;

type DecoratedScope = ExternalCursorScope & { readonly patchbayGeneration: bigint };

const baseScope: ExternalCursorScope = {
  adapterId: "adapter-a",
  deploymentScope: "deployment-a",
  externalContinuityId: "verified-external-session-a",
};

const entry = (id: string, value = id): Entry => ({ id, value });

function initialRecord(entries = [entry("kept"), entry("stale")]): Record {
  return {
    recordVersion: 3n,
    freshness: "current",
    projection: {
      replacementEpoch: 7n,
      exactEntries: entries,
      cursor: "cursor-7",
      leaf: "leaf-7",
    },
  };
}

class MemoryStore implements AtomicExternalCursorProjectionStore<
  ExternalCursorScope,
  Entry,
  Cursor,
  Leaf
> {
  readonly writes: Record[] = [];
  failNext: "before" | "after" | undefined;
  onWrite: ((record: Record) => void) | undefined;
  private readonly records = new Map<string, Record>();

  seed(scope: ExternalCursorScope, record: Record): void {
    this.records.set(externalCursorScopeKey(scope), cloneRecord(record));
  }

  async load(scope: ExternalCursorScope): Promise<Record | undefined> {
    const record = this.records.get(externalCursorScopeKey(scope));
    return record ? cloneRecord(record) : undefined;
  }

  async compareAndSwap(
    scope: ExternalCursorScope,
    expectedRecordVersion: bigint,
    next: Record,
  ): Promise<void> {
    if (this.failNext === "before") {
      this.failNext = undefined;
      throw new Error("injected crash before atomic commit");
    }
    const key = externalCursorScopeKey(scope);
    const current = this.records.get(key);
    assert.equal(current?.recordVersion, expectedRecordVersion, "compare-and-swap version");
    const committed = cloneRecord(next);
    this.records.set(key, committed);
    this.writes.push(cloneRecord(committed));
    this.onWrite?.(cloneRecord(committed));
    if (this.failNext === "after") {
      this.failNext = undefined;
      throw new Error("injected crash after atomic commit");
    }
  }
}

const values = {
  entryIdentity: (candidate: Entry) => candidate.id,
  entriesEqual: (left: Entry, right: Entry) => left.id === right.id && left.value === right.value,
  cursorsEqual: (left: Cursor, right: Cursor) => left === right,
  leavesEqual: (left: Leaf, right: Leaf) => left === right,
};

function setup(record = initialRecord()) {
  const store = new MemoryStore();
  store.seed(baseScope, record);
  return {
    store,
    machine: new ExternalCursorProjectionMachine(store, values),
  };
}

test("cursor continuity scope ignores Patchbay generation replacement", async () => {
  const generationOne: DecoratedScope = { ...baseScope, patchbayGeneration: 1n };
  const generationTwo: DecoratedScope = { ...baseScope, patchbayGeneration: 2n };
  const store = new MemoryStore();
  store.seed(generationOne, initialRecord());
  const machine = new ExternalCursorProjectionMachine(store, values);

  assert.equal(externalCursorScopeKey(generationOne), externalCursorScopeKey(generationTwo));
  assert.equal((await machine.read(generationTwo))?.projection.cursor, "cursor-7");
  assert.equal(await machine.read({
    ...generationTwo,
    externalContinuityId: "different-verified-external-session",
  }), undefined);
});

test("known cursor suffix is idempotent and preserves unrelated projection members", async () => {
  const { store, machine } = setup(initialRecord([entry("kept"), entry("unrelated")]));
  const suffix = {
    baseCursor: "cursor-7",
    entries: [entry("kept"), entry("new")],
    cursor: "cursor-8",
    leaf: "leaf-8",
  } as const;

  await machine.applyKnownSuffix(baseScope, suffix);
  const first = await machine.read(baseScope);
  assert.deepEqual(first?.projection, {
    replacementEpoch: 7n,
    exactEntries: [entry("kept"), entry("unrelated"), entry("new")],
    cursor: "cursor-8",
    leaf: "leaf-8",
  });
  const writeCount = store.writes.length;

  await machine.applyKnownSuffix(baseScope, suffix);
  assert.equal(store.writes.length, writeCount, "response-loss retry is an inert suffix replay");
  assert.deepEqual((await machine.read(baseScope))?.projection.exactEntries, [
    entry("kept"), entry("unrelated"), entry("new"),
  ]);
});

test("unknown cursor marks and retains the old projection stale before complete fetch", async () => {
  const { machine } = setup();
  let observedDuringFetch: Record | undefined;

  const staged = await machine.stageAuthoritativeReplacement(baseScope, async () => {
    observedDuringFetch = await machine.read(baseScope);
    return { entries: [entry("kept"), entry("replacement")], leaf: "leaf-8" };
  });

  assert.equal(observedDuringFetch?.freshness, "stale");
  assert.deepEqual(observedDuringFetch?.projection, initialRecord().projection);
  assert.deepEqual(observedDuringFetch?.pendingReplacement, {
    kind: "fetching",
    replacementEpoch: 8n,
  });
  assert.deepEqual(staged, {
    replacementEpoch: 8n,
    exactEntries: [entry("kept"), entry("replacement")],
    leaf: "leaf-8",
  });

  const afterFetch = await machine.read(baseScope);
  assert.equal(afterFetch?.freshness, "stale");
  assert.deepEqual(afterFetch?.projection, initialRecord().projection);
  assert.deepEqual(afterFetch?.pendingReplacement, { kind: "staged", ...staged });
});

test("atomic authoritative replacement removes omissions and installs projection leaf cursor and epoch together", async () => {
  const { store, machine } = setup();
  const staged = await machine.stageAuthoritativeReplacement(baseScope, async () => ({
    entries: [entry("kept"), entry("replacement")],
    leaf: "leaf-8",
  }));
  const observedCommits: Record[] = [];
  store.onWrite = (record) => observedCommits.push(record);

  await machine.commitReplacement(baseScope, {
    ...staged,
    cursor: "cursor-8",
  });

  assert.equal(observedCommits.length, 1);
  assert.deepEqual(observedCommits[0], {
    recordVersion: 6n,
    freshness: "current",
    projection: {
      replacementEpoch: 8n,
      exactEntries: [entry("kept"), entry("replacement")],
      cursor: "cursor-8",
      leaf: "leaf-8",
    },
  });
  assert.equal(
    observedCommits[0]!.projection.exactEntries.some((candidate) => candidate.id === "stale"),
    false,
    "authoritative omission removes the old member instead of upserting",
  );
});

test("injected crashes expose either the old stale record or the complete replacement", async () => {
  const { store, machine } = setup();
  const staged = await machine.stageAuthoritativeReplacement(baseScope, async () => ({
    entries: [entry("complete-a"), entry("complete-b")],
    leaf: "leaf-8",
  }));
  const replacement: ProjectionReplacement<Entry, Cursor, Leaf> = {
    ...staged,
    cursor: "cursor-8",
  };

  store.failNext = "before";
  await assert.rejects(machine.commitReplacement(baseScope, replacement), /before atomic commit/);
  const before = await machine.read(baseScope);
  assert.equal(before?.freshness, "stale");
  assert.deepEqual(before?.projection, initialRecord().projection);
  assert.deepEqual(before?.pendingReplacement, { kind: "staged", ...staged });

  store.failNext = "after";
  await assert.rejects(machine.commitReplacement(baseScope, replacement), /after atomic commit/);
  assert.deepEqual(await machine.read(baseScope), {
    recordVersion: 6n,
    freshness: "current",
    projection: replacement,
  });

  await machine.commitReplacement(baseScope, replacement);
  assert.deepEqual((await machine.read(baseScope))?.projection, replacement);
});

test("staged exact identity conflicts and known suffix content conflicts fail closed", async () => {
  const { machine } = setup();
  await assert.rejects(
    machine.applyKnownSuffix(baseScope, {
      baseCursor: "cursor-7",
      entries: [entry("kept", "conflict")],
      cursor: "cursor-8",
      leaf: "leaf-8",
    }),
    ExternalCursorInvariantError,
  );

  await assert.rejects(
    machine.stageAuthoritativeReplacement(baseScope, async () => ({
      entries: [entry("duplicate"), entry("duplicate")],
      leaf: "leaf-8",
    })),
    /duplicate exact entry identity/,
  );
  const after = await machine.read(baseScope);
  assert.equal(after?.freshness, "stale");
  assert.deepEqual(after?.projection, initialRecord().projection);
  assert.deepEqual(after?.pendingReplacement, { kind: "fetching", replacementEpoch: 8n });
});

test("required adapter contract retains the designed method surface", async () => {
  const replacementPort: AuthoritativeCursorReplacement<
    ExternalCursorScope,
    Entry,
    Cursor,
    Leaf
  > = {
    async reconcileKnown() { return [entry("suffix")]; },
    async stageReplacement() { return { entries: [entry("full")], leaf: "leaf" }; },
    async commitReplacement() {},
  };
  assert.deepEqual(await replacementPort.reconcileKnown(baseScope, "cursor"), [entry("suffix")]);
});

function cloneRecord(record: Record): Record {
  const pending = record.pendingReplacement;
  return {
    recordVersion: record.recordVersion,
    freshness: record.freshness,
    projection: {
      ...record.projection,
      exactEntries: record.projection.exactEntries.map((candidate) => ({ ...candidate })),
    },
    ...(pending
      ? {
          pendingReplacement: pending.kind === "fetching"
            ? { ...pending }
            : {
                ...pending,
                exactEntries: pending.exactEntries.map((candidate) => ({ ...candidate })),
              },
        }
      : {}),
  };
}
