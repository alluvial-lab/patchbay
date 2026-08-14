import assert from "node:assert/strict";
import test from "node:test";

import {
  AuthoritativeCursorReplacement,
  ExternalCursorInvariantError,
  assertAtomicExternalCursorProjectionStoreConformance,
  externalCursorScopeKey,
  type AtomicExternalCursorProjectionStore,
  type ExternalCursorFetchPort,
  type ExternalCursorProjectionRecord,
  type ExternalCursorPublishPort,
  type ExternalCursorScope,
  type KnownCursorFetch,
  type KnownCursorSuffix,
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

function fetchingRecord(entries = [entry("kept"), entry("stale")]): Record {
  const initial = initialRecord(entries);
  return {
    recordVersion: initial.recordVersion + 1n,
    freshness: "stale",
    projection: initial.projection,
    pendingReplacement: { kind: "fetching", replacementEpoch: 8n },
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
  beforeSwap: (() => Promise<void>) | undefined;
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
    await this.beforeSwap?.();
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

class ScriptedFetch implements ExternalCursorFetchPort<
  ExternalCursorScope,
  Entry,
  Cursor,
  Leaf
> {
  known: (
    scope: ExternalCursorScope,
    cursor: Cursor,
  ) => Promise<KnownCursorFetch<Entry, Cursor, Leaf>> = async () => {
    throw new Error("known fetch was not configured");
  };

  complete: (
    scope: ExternalCursorScope,
  ) => Promise<{ readonly entries: readonly Entry[]; readonly leaf: Leaf }> = async () => {
    throw new Error("complete fetch was not configured");
  };

  fetchKnown(
    scope: ExternalCursorScope,
    cursor: Cursor,
  ): Promise<KnownCursorFetch<Entry, Cursor, Leaf>> {
    return this.known(scope, cursor);
  }

  fetchComplete(
    scope: ExternalCursorScope,
  ): Promise<{ readonly entries: readonly Entry[]; readonly leaf: Leaf }> {
    return this.complete(scope);
  }
}

class RecordingPublisher implements ExternalCursorPublishPort<
  ExternalCursorScope,
  Entry,
  Cursor,
  Leaf
> {
  readonly knownSuffixes: KnownCursorSuffix<Entry, Cursor, Leaf>[] = [];
  readonly replacements: ProjectionReplacement<Entry, Cursor, Leaf>[] = [];

  async publishKnownSuffix(
    _scope: ExternalCursorScope,
    suffix: KnownCursorSuffix<Entry, Cursor, Leaf>,
  ): Promise<void> {
    this.knownSuffixes.push({ ...suffix, entries: [...suffix.entries] });
  }

  async publishReplacement(
    _scope: ExternalCursorScope,
    replacement: ProjectionReplacement<Entry, Cursor, Leaf>,
  ): Promise<void> {
    this.replacements.push({
      ...replacement,
      exactEntries: [...replacement.exactEntries],
    });
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
  const fetch = new ScriptedFetch();
  const publisher = new RecordingPublisher();
  store.seed(baseScope, record);
  return {
    store,
    fetch,
    publisher,
    replacement: new AuthoritativeCursorReplacement(store, fetch, publisher, values),
  };
}

test("cursor continuity scope ignores Patchbay generation replacement", async () => {
  const generationOne: DecoratedScope = { ...baseScope, patchbayGeneration: 1n };
  const generationTwo: DecoratedScope = { ...baseScope, patchbayGeneration: 2n };
  const store = new MemoryStore();
  const fetch = new ScriptedFetch();
  const publisher = new RecordingPublisher();
  store.seed(generationOne, initialRecord());
  const replacement = new AuthoritativeCursorReplacement(store, fetch, publisher, values);

  assert.equal(externalCursorScopeKey(generationOne), externalCursorScopeKey(generationTwo));
  assert.equal((await replacement.read(generationTwo))?.projection.cursor, "cursor-7");
  assert.equal(await replacement.read({
    ...generationTwo,
    externalContinuityId: "different-verified-external-session",
  }), undefined);
});

test("exported contract reconciles a known suffix idempotently and preserves unrelated members", async () => {
  const { store, fetch, publisher, replacement } = setup(
    initialRecord([entry("kept"), entry("unrelated")]),
  );
  fetch.known = async () => ({
    entries: [entry("kept"), entry("new")],
    cursor: "cursor-8",
    leaf: "leaf-8",
  });

  assert.deepEqual(
    await replacement.reconcileKnown(baseScope, "cursor-7"),
    [entry("kept"), entry("new")],
  );
  assert.deepEqual((await replacement.read(baseScope))?.projection, {
    replacementEpoch: 7n,
    exactEntries: [entry("kept"), entry("unrelated"), entry("new")],
    cursor: "cursor-8",
    leaf: "leaf-8",
  });
  assert.deepEqual(publisher.knownSuffixes, [{
    baseCursor: "cursor-7",
    entries: [entry("kept"), entry("new")],
    cursor: "cursor-8",
    leaf: "leaf-8",
  }]);
  const writeCount = store.writes.length;

  await replacement.reconcileKnown(baseScope, "cursor-7");
  assert.equal(store.writes.length, writeCount, "response-loss retry is an inert suffix replay");
  assert.equal(publisher.knownSuffixes.length, 1, "a locally committed retry is not republished");
  assert.deepEqual((await replacement.read(baseScope))?.projection.exactEntries, [
    entry("kept"), entry("unrelated"), entry("new"),
  ]);
});

test("exported contract marks and retains the old projection stale before complete fetch", async () => {
  const { fetch, replacement } = setup();
  let observedDuringFetch: Record | undefined;
  fetch.complete = async () => {
    observedDuringFetch = await replacement.read(baseScope);
    return { entries: [entry("kept"), entry("replacement")], leaf: "leaf-8" };
  };

  const staged = await replacement.stageReplacement(baseScope);

  assert.equal(observedDuringFetch?.freshness, "stale");
  assert.deepEqual(observedDuringFetch?.projection, initialRecord().projection);
  assert.deepEqual(observedDuringFetch?.pendingReplacement, {
    kind: "fetching",
    replacementEpoch: 8n,
  });
  assert.deepEqual(staged, {
    replacementEpoch: 8n,
    entries: [entry("kept"), entry("replacement")],
    leaf: "leaf-8",
  });

  const afterFetch = await replacement.read(baseScope);
  assert.equal(afterFetch?.freshness, "stale");
  assert.deepEqual(afterFetch?.projection, initialRecord().projection);
  assert.deepEqual(afterFetch?.pendingReplacement, {
    kind: "staged",
    replacementEpoch: 8n,
    exactEntries: staged.entries,
    leaf: "leaf-8",
  });
});

test("exported contract atomically replaces omissions and installs projection leaf cursor and epoch", async () => {
  const { store, fetch, publisher, replacement } = setup();
  fetch.complete = async () => ({
    entries: [entry("kept"), entry("replacement")],
    leaf: "leaf-8",
  });
  const staged = await replacement.stageReplacement(baseScope);
  const observedCommits: Record[] = [];
  store.onWrite = (record) => observedCommits.push(record);

  await replacement.commitReplacement(baseScope, {
    replacementEpoch: staged.replacementEpoch,
    exactEntries: staged.entries,
    cursor: "cursor-8",
    leaf: staged.leaf,
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
  assert.deepEqual(publisher.replacements, [observedCommits[0]!.projection]);
  assert.equal(
    observedCommits[0]!.projection.exactEntries.some((candidate) => candidate.id === "stale"),
    false,
    "authoritative omission removes the old member instead of upserting",
  );
});

test("exported contract exposes old-stale or complete-new under injected commit crashes", async () => {
  const { store, fetch, publisher, replacement } = setup();
  fetch.complete = async () => ({
    entries: [entry("complete-a"), entry("complete-b")],
    leaf: "leaf-8",
  });
  const staged = await replacement.stageReplacement(baseScope);
  const committed: ProjectionReplacement<Entry, Cursor, Leaf> = {
    replacementEpoch: staged.replacementEpoch,
    exactEntries: staged.entries,
    cursor: "cursor-8",
    leaf: staged.leaf,
  };

  store.failNext = "before";
  await assert.rejects(replacement.commitReplacement(baseScope, committed), /before atomic commit/);
  const before = await replacement.read(baseScope);
  assert.equal(before?.freshness, "stale");
  assert.deepEqual(before?.projection, initialRecord().projection);
  assert.deepEqual(before?.pendingReplacement, {
    kind: "staged",
    replacementEpoch: staged.replacementEpoch,
    exactEntries: staged.entries,
    leaf: staged.leaf,
  });

  store.failNext = "after";
  await assert.rejects(replacement.commitReplacement(baseScope, committed), /after atomic commit/);
  assert.deepEqual(await replacement.read(baseScope), {
    recordVersion: 6n,
    freshness: "current",
    projection: committed,
  });

  await replacement.commitReplacement(baseScope, committed);
  assert.deepEqual((await replacement.read(baseScope))?.projection, committed);
  assert.equal(publisher.replacements.length, 2, "only pre-commit ambiguity republishes");
});

test("duplicate exact identities and known suffix content conflicts fail closed", async () => {
  const { fetch, publisher, replacement } = setup();
  fetch.known = async () => ({
    entries: [entry("kept", "conflict")],
    cursor: "cursor-8",
    leaf: "leaf-8",
  });
  await assert.rejects(
    replacement.reconcileKnown(baseScope, "cursor-7"),
    ExternalCursorInvariantError,
  );
  assert.equal(publisher.knownSuffixes.length, 0);

  fetch.complete = async () => ({
    entries: [entry("duplicate"), entry("duplicate")],
    leaf: "leaf-8",
  });
  await assert.rejects(
    replacement.stageReplacement(baseScope),
    /duplicate exact entry identity/,
  );
  const after = await replacement.read(baseScope);
  assert.equal(after?.freshness, "stale");
  assert.deepEqual(after?.projection, initialRecord().projection);
  assert.deepEqual(after?.pendingReplacement, { kind: "fetching", replacementEpoch: 8n });
});

test("same-epoch and post-commit conflicts preserve the authoritative pre-attempt record", async () => {
  const { fetch, publisher, replacement } = setup();
  fetch.complete = async () => ({
    entries: [entry("complete-a"), entry("complete-b")],
    leaf: "leaf-8",
  });
  const staged = await replacement.stageReplacement(baseScope);
  const valid: ProjectionReplacement<Entry, Cursor, Leaf> = {
    replacementEpoch: staged.replacementEpoch,
    exactEntries: staged.entries,
    cursor: "cursor-8",
    leaf: staged.leaf,
  };
  const stagedBeforeAttempts = await replacement.read(baseScope);

  const stagedConflicts: readonly ProjectionReplacement<Entry, Cursor, Leaf>[] = [
    { ...valid, exactEntries: [entry("different-entry")] },
    { ...valid, leaf: "different-leaf" },
  ];
  for (const conflict of stagedConflicts) {
    await assert.rejects(
      replacement.commitReplacement(baseScope, conflict),
      /differs from its staged exact set/,
    );
    assert.deepEqual(
      await replacement.read(baseScope),
      stagedBeforeAttempts,
      "same-epoch conflict leaves the complete staged record unchanged",
    );
  }
  assert.equal(publisher.replacements.length, 0, "invalid staged evidence is never published");

  await replacement.commitReplacement(baseScope, valid);
  const committedBeforeAttempts = await replacement.read(baseScope);
  const postCommitConflicts: readonly ProjectionReplacement<Entry, Cursor, Leaf>[] = [
    { ...valid, cursor: "different-cursor" },
    { ...valid, exactEntries: [entry("complete-a", "different-content"), entry("complete-b")] },
  ];
  for (const conflict of postCommitConflicts) {
    await assert.rejects(
      replacement.commitReplacement(baseScope, conflict),
      /no staged authoritative replacement exists/,
    );
    assert.deepEqual(
      await replacement.read(baseScope),
      committedBeforeAttempts,
      "conflicting post-commit retry leaves the complete current record unchanged",
    );
  }
  assert.equal(publisher.replacements.length, 1, "only the correlated replacement is published");
});

test("barrier-controlled racing suffixes have one stale-version loser and preserve the winner", async () => {
  const { store, fetch, replacement } = setup(
    initialRecord([entry("kept"), entry("unrelated")]),
  );
  let fetchIndex = 0;
  const suffixes: readonly KnownCursorFetch<Entry, Cursor, Leaf>[] = [
    { entries: [entry("suffix-a")], cursor: "cursor-a", leaf: "leaf-a" },
    { entries: [entry("suffix-b")], cursor: "cursor-b", leaf: "leaf-b" },
  ];
  fetch.known = async () => suffixes[fetchIndex++]!;
  const barrier = blockNextSwaps(store, 2);

  const attempts = [
    replacement.reconcileKnown(baseScope, "cursor-7"),
    replacement.reconcileKnown(baseScope, "cursor-7"),
  ];
  await barrier.allArrived;
  barrier.release();
  const settled = await Promise.allSettled(attempts);
  const winner = assertExactlyOneCasWinner(settled);
  const final = await replacement.read(baseScope);

  assert.deepEqual(final, {
    recordVersion: 4n,
    freshness: "current",
    projection: {
      replacementEpoch: 7n,
      exactEntries: [entry("kept"), entry("unrelated"), suffixes[winner]!.entries[0]!],
      cursor: suffixes[winner]!.cursor,
      leaf: suffixes[winner]!.leaf,
    },
  });
});

test("barrier-controlled known suffix versus replacement staging has one CAS winner", async () => {
  const { store, fetch, replacement } = setup();
  fetch.known = async () => ({
    entries: [entry("known-winner")],
    cursor: "cursor-known",
    leaf: "leaf-known",
  });
  fetch.complete = async () => ({
    entries: [entry("replacement-winner")],
    leaf: "leaf-replacement",
  });
  const barrier = blockNextSwaps(store, 2);

  const attempts = [
    replacement.reconcileKnown(baseScope, "cursor-7"),
    replacement.stageReplacement(baseScope),
  ];
  await barrier.allArrived;
  barrier.release();
  const settled = await Promise.allSettled(attempts);
  const winner = assertExactlyOneCasWinner(settled);
  const final = await replacement.read(baseScope);

  if (winner === 0) {
    assert.deepEqual(final, {
      recordVersion: 4n,
      freshness: "current",
      projection: {
        replacementEpoch: 7n,
        exactEntries: [entry("kept"), entry("stale"), entry("known-winner")],
        cursor: "cursor-known",
        leaf: "leaf-known",
      },
    });
  } else {
    assert.deepEqual(final, {
      recordVersion: 5n,
      freshness: "stale",
      projection: initialRecord().projection,
      pendingReplacement: {
        kind: "staged",
        replacementEpoch: 8n,
        exactEntries: [entry("replacement-winner")],
        leaf: "leaf-replacement",
      },
    });
  }
});

test("barrier-controlled replacement fetches have one stale-version loser and one exact stage", async () => {
  const { store, fetch, publisher, replacement } = setup(fetchingRecord());
  let fetchIndex = 0;
  const candidates = [
    { entries: [entry("replacement-a")], leaf: "leaf-a" },
    { entries: [entry("replacement-b")], leaf: "leaf-b" },
  ] as const;
  fetch.complete = async () => candidates[fetchIndex++]!;
  const barrier = blockNextSwaps(store, 2);

  const attempts = [
    replacement.stageReplacement(baseScope),
    replacement.stageReplacement(baseScope),
  ];
  await barrier.allArrived;
  barrier.release();
  const settled = await Promise.allSettled(attempts);
  const winner = assertExactlyOneCasWinner(settled);

  assert.deepEqual(await replacement.read(baseScope), {
    recordVersion: 5n,
    freshness: "stale",
    projection: initialRecord().projection,
    pendingReplacement: {
      kind: "staged",
      replacementEpoch: 8n,
      exactEntries: candidates[winner]!.entries,
      leaf: candidates[winner]!.leaf,
    },
  });
  assert.equal(publisher.replacements.length, 0, "fetch races cannot publish before commit");
});

test("MemoryStore runs the reusable atomic store conformance suite", async () => {
  const initial = initialRecord();
  const firstNext: Record = {
    recordVersion: 4n,
    freshness: "current",
    projection: {
      replacementEpoch: 7n,
      exactEntries: [entry("first")],
      cursor: "cursor-first",
      leaf: "leaf-first",
    },
  };
  const secondNext: Record = {
    recordVersion: 4n,
    freshness: "stale",
    projection: initial.projection,
    pendingReplacement: { kind: "fetching", replacementEpoch: 8n },
  };

  const cases = await assertAtomicExternalCursorProjectionStoreConformance({
    createStore: () => {
      const store = new MemoryStore();
      store.seed(baseScope, initial);
      return store;
    },
    scope: baseScope,
    initialRecord: initial,
    firstNextRecord: firstNext,
    secondNextRecord: secondNext,
    assertSnapshot: (actual, expected, context) => assert.deepEqual(actual, expected, context),
  });

  assert.deepEqual(cases, [
    "stale-expected-version rejection",
    "all-or-nothing snapshots",
    "ambiguous post-commit retry",
    "racing-writer behavior",
  ]);
});

function assertExactlyOneCasWinner<T>(settled: readonly PromiseSettledResult<T>[]): number {
  const winners = settled
    .map((result, index) => result.status === "fulfilled" ? index : -1)
    .filter((index) => index >= 0);
  assert.equal(winners.length, 1, "exactly one racing writer must win");
  const loser = settled[winners[0] === 0 ? 1 : 0]!;
  assert.equal(loser.status, "rejected");
  if (loser.status === "rejected") {
    assert.match(String(loser.reason), /compare-and-swap version/);
  }
  return winners[0]!;
}

function blockNextSwaps(
  store: MemoryStore,
  count: number,
): { readonly allArrived: Promise<void>; release(): void } {
  const allArrived = deferred();
  const released = deferred();
  let arrived = 0;
  store.beforeSwap = async () => {
    arrived += 1;
    if (arrived === count) {
      store.beforeSwap = undefined;
      allArrived.resolve();
    }
    await released.promise;
  };
  return { allArrived: allArrived.promise, release: released.resolve };
}

function deferred(): { readonly promise: Promise<void>; resolve(): void } {
  let resolve!: () => void;
  const promise = new Promise<void>((accept) => {
    resolve = accept;
  });
  return { promise, resolve };
}

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
