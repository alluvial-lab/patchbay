import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { assertAtomicExternalCursorProjectionStoreConformance } from "@patchbay/operator-domain/reconciliation/external-cursor";
import {
  FilePiCursorStore,
  derivePiSessionContinuityKey,
  type PiCursorProjectionRecord,
  type PiExternalCursorScope,
} from "../src/cursor_store.js";
import { piTreeDigest, type PiProjectedEntry } from "../src/pi_projection.js";

const scope: PiExternalCursorScope = {
  adapterId: "pi",
  deploymentScope: "machine-a",
  externalContinuityId: `pi1:${"a".repeat(43)}`,
};
const emptyTree = piTreeDigest([]);
const entry = (id: string, parentEntryId: string | null = null): PiProjectedEntry => ({
  stableEntryId: id,
  parentEntryId,
  contentDigest: "b".repeat(64),
  presentationItems: [],
});

function initialRecord(): PiCursorProjectionRecord {
  const exactEntries = [entry("root")];
  return {
    recordVersion: 2n,
    freshness: "current",
    projection: {
      replacementEpoch: 3n,
      exactEntries,
      cursor: "root",
      leaf: { entryId: "root", treeDigest: piTreeDigest(exactEntries) },
    },
  };
}

function nextRecord(): PiCursorProjectionRecord {
  const exactEntries = [entry("root"), entry("next", "root")];
  return {
    recordVersion: 3n,
    freshness: "current",
    projection: {
      replacementEpoch: 3n,
      exactEntries,
      cursor: "next",
      leaf: { entryId: "next", treeDigest: piTreeDigest(exactEntries) },
    },
  };
}

test("verified Pi continuity ignores Patchbay generation and never exposes the raw path", async () => {
  const root = await mkdtemp(join(tmpdir(), "patchbay-pi-continuity-"));
  try {
    await mkdir(join(root, "nested"));
    const first = await derivePiSessionContinuityKey({
      adapterId: "pi",
      deploymentScope: "machine-a",
      piSessionId: "pi-session-1",
      sessionRootId: "root-entry-1",
      configuredSessionRoot: root,
      canonicalSessionPath: join(root, "nested", "session.jsonl"),
    });
    const second = await derivePiSessionContinuityKey({
      adapterId: "pi",
      deploymentScope: "machine-a",
      piSessionId: "pi-session-1",
      sessionRootId: "root-entry-1",
      configuredSessionRoot: root,
      canonicalSessionPath: join(root, "nested", "session.jsonl"),
      // A caller may carry generation elsewhere; it is not accepted by or fed
      // into the continuity derivation.
    });
    assert.equal(first.scope.externalContinuityId, second.scope.externalContinuityId);
    assert.equal(first.key.rootRelativePath, join("nested", "session.jsonl"));
    assert.doesNotMatch(first.scope.externalContinuityId, /nested|session\.jsonl|pi-session-1/);

    const different = await derivePiSessionContinuityKey({
      adapterId: "pi",
      deploymentScope: "machine-a",
      piSessionId: "pi-session-2",
      sessionRootId: "root-entry-1",
      configuredSessionRoot: root,
      canonicalSessionPath: join(root, "nested", "session.jsonl"),
    });
    assert.notEqual(different.scope.externalContinuityId, first.scope.externalContinuityId);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("cursor store persists one reverse logical-target binding in a 0600 atomic record", async () => {
  const root = await mkdtemp(join(tmpdir(), "patchbay-pi-cursor-store-"));
  try {
    const store = new FilePiCursorStore(root);
    await store.bindLogicalTarget(scope, "logical-a");
    await assert.rejects(
      store.bindLogicalTarget(scope, "logical-b"),
      /already bound to another logical target/,
    );
    await store.ensureReplacementBaseline(scope, "logical-a", {
      replacementEpoch: 0n,
      exactEntries: [],
      cursor: null,
      leaf: { entryId: null, treeDigest: emptyTree },
    });
    const loaded = await store.load(scope);
    assert.equal(loaded?.freshness, "stale");
    assert.equal(loaded?.projection.cursor, null);
    assert.deepEqual(loaded?.pendingReplacement, { kind: "fetching", replacementEpoch: 1n });
    assert.equal((await stat(root)).mode & 0o077, 0, "cursor directory is private");
    const files = (await import("node:fs/promises")).readdir(root);
    for (const name of await files) {
      const metadata = await stat(join(root, name));
      assert.equal(metadata.mode & 0o077, 0);
    }
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("FilePiCursorStore passes the exported CAS suite including the overlapping reader", async () => {
  const roots: string[] = [];
  try {
    const initial = initialRecord();
    const cases = await assertAtomicExternalCursorProjectionStoreConformance({
      createStore: async () => {
        const root = await mkdtemp(join(tmpdir(), "patchbay-pi-cursor-conformance-"));
        roots.push(root);
        const store = new FilePiCursorStore(root);
        await store.initializeCurrent(scope, "logical-a", initial);
        return store;
      },
      scope,
      initialRecord: initial,
      firstNextRecord: nextRecord(),
      secondNextRecord: {
        recordVersion: 3n,
        freshness: "stale",
        projection: initial.projection,
        pendingReplacement: { kind: "fetching", replacementEpoch: 4n },
      },
      assertSnapshot: (actual, expected, context) => assert.deepEqual(actual, expected, context),
    });
    assert.deepEqual(cases, [
      "stale-expected-version rejection",
      "complete settled snapshot",
      "single overlapping-reader snapshot",
      "ambiguous post-commit retry",
      "racing-writer behavior",
    ]);
  } finally {
    await Promise.all(roots.map((root) => rm(root, { recursive: true, force: true })));
  }
});
