import assert from "node:assert/strict";
import { readFile, rename, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { mkdtemp } from "node:fs/promises";
import test from "node:test";
import { PiSessionIntegrityFailure } from "@patchbay/contracts";
import { PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE } from "../extensions/patchbay-control.js";
import type { PiControlHandshake } from "../src/control_handshake.js";
import {
  classifyPiSessionMaterialization,
  verifyMaterializedSessionSeal,
  verifyResumedSessionExtension,
  type MaterializedSessionSeal,
  type PiSessionFileValidationOptions,
  type PiSessionMaterialization,
} from "../src/session_file.js";

const fixturePath = join(process.cwd(), "tests", "fixtures", "session-valid.jsonl");
const sessionId = "session-fixture";

test("valid raw JSONL and exact RPC tree materialize to a physical prefix seal", async () => {
  await withFixture(async ({ path, root, content, entries }) => {
    const result = await classifyPiSessionMaterialization(
      options(path, root, entries, "leaf0002"),
    );
    assert.equal(result.kind, "materialized");
    if (result.kind !== "materialized") return;
    assert.equal(result.seal.sessionId, sessionId);
    assert.equal(result.seal.sessionRootId, "root0001");
    assert.deepEqual(result.seal.orderedEntryIds, ["root0001", "leaf0002"]);
    assert.equal(result.seal.leafId, "leaf0002");
    assert.equal(result.seal.size, BigInt(Buffer.byteLength(content)));
    assert.match(result.seal.contentDigest, /^[a-f0-9]{64}$/u);
    assert.match(result.seal.treeDigest, /^[a-f0-9]{64}$/u);

    const unchanged = await verifyMaterializedSessionSeal({
      ...options(path, root, entries, "leaf0002"),
      seal: result.seal,
    });
    assert.equal(unchanged.kind, "materialized");
  });
});

test("declared path without a regular non-empty file stays memory_only despite in-memory entries", async () => {
  const root = await mkdtemp(join(tmpdir(), "patchbay-session-memory-"));
  try {
    const missing = join(root, "not-created.jsonl");
    const inMemoryEntries = [
      {
        type: "custom",
        id: "memory01",
        parentId: null,
        timestamp: "2026-08-12T00:00:00.000Z",
        customType: PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE,
        data: { marker: "not durable" },
      },
    ];
    const absent = await classifyPiSessionMaterialization(
      options(missing, root, inMemoryEntries, "memory01"),
    );
    assert.deepEqual(absent, { kind: "memory_only", sessionId, declaredPath: missing });

    await writeFile(missing, "", { mode: 0o600 });
    const empty = await classifyPiSessionMaterialization(
      options(missing, root, inMemoryEntries, "memory01"),
    );
    assert.equal(empty.kind, "memory_only");

    const materializedContent = await readFile(fixturePath, "utf8");
    const materializedEntries = parseEntries(materializedContent);
    await writeFile(missing, materializedContent);
    const afterFirstAssistant = await classifyPiSessionMaterialization(
      options(missing, root, materializedEntries, "leaf0002"),
    );
    assert.equal(afterFirstAssistant.kind, "materialized");
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("strict parser rejects malformed lines, duplicate ids, orphan/forward/self parents, and multiple roots", async () => {
  await withFixture(async ({ path, root, content, entries }) => {
    const lines = content.trimEnd().split("\n");
    const malformed = `${lines[0]}\n{malformed\n${lines.slice(1).join("\n")}\n`;
    await writeFile(path, malformed);
    await expectInvalid(
      classifyPiSessionMaterialization(options(path, root, entries, "leaf0002")),
      PiSessionIntegrityFailure.JSON_INVALID,
    );
  });

  await mutationCase((objects) => {
    objects.push(structuredClone(objects[2]!));
  }, PiSessionIntegrityFailure.DUPLICATE_ENTRY_ID);
  await mutationCase((objects) => {
    objects[2]!.parentId = "missing-parent";
  }, PiSessionIntegrityFailure.PARENT_INVALID);
  await mutationCase((objects) => {
    objects[1]!.parentId = "leaf0002";
  }, PiSessionIntegrityFailure.PARENT_INVALID);
  await mutationCase((objects) => {
    objects[2]!.parentId = "leaf0002";
  }, PiSessionIntegrityFailure.PARENT_INVALID);
  await mutationCase((objects) => {
    objects[2]!.parentId = null;
  }, PiSessionIntegrityFailure.TREE_INVALID);
});

test("strict parser rejects bad secondary references, unsupported versions/types, and truncation", async () => {
  await mutationCase((objects) => {
    objects.push({
      type: "label",
      id: "label001",
      parentId: "leaf0002",
      timestamp: "2026-08-12T00:00:03.000Z",
      targetId: "missing-target",
      label: "bad",
    });
  }, PiSessionIntegrityFailure.REFERENCE_INVALID, "label001");
  await mutationCase((objects) => {
    objects[0]!.version = 2;
  }, PiSessionIntegrityFailure.HEADER_INVALID);
  await mutationCase((objects) => {
    objects[2]!.type = "future_entry";
  }, PiSessionIntegrityFailure.ENTRY_TYPE_UNSUPPORTED);
  await withFixture(async ({ path, root, content, entries }) => {
    await writeFile(path, content.trimEnd());
    await expectInvalid(
      classifyPiSessionMaterialization(options(path, root, entries, "leaf0002")),
      PiSessionIntegrityFailure.FRAMING_INVALID,
    );
  });
});

test("safe open rejects symlinks, allowed-root escape, and path inode swap", async () => {
  const root = await mkdtemp(join(tmpdir(), "patchbay-session-safe-open-"));
  const outsideRoot = await mkdtemp(join(tmpdir(), "patchbay-session-outside-"));
  try {
    const content = await readFile(fixturePath, "utf8");
    const target = join(root, "target.jsonl");
    const link = join(root, "link.jsonl");
    await writeFile(target, content);
    await symlink(target, link);
    const entries = parseEntries(content);
    await expectInvalid(
      classifyPiSessionMaterialization(options(link, root, entries, "leaf0002")),
      PiSessionIntegrityFailure.SYMLINK,
    );

    const outside = join(outsideRoot, "outside.jsonl");
    await writeFile(outside, content);
    await expectInvalid(
      classifyPiSessionMaterialization(options(outside, root, entries, "leaf0002")),
      PiSessionIntegrityFailure.PATH_OUTSIDE_ALLOWED_ROOT,
    );

    const raced = join(root, "raced.jsonl");
    const old = join(root, "old.jsonl");
    await writeFile(raced, content);
    await expectInvalid(
      classifyPiSessionMaterialization({
        ...options(raced, root, entries, "leaf0002"),
        afterOpen: async () => {
          await rename(raced, old);
          await writeFile(raced, content);
        },
      }),
      PiSessionIntegrityFailure.UNSTABLE_FILE,
    );
  } finally {
    await rm(root, { recursive: true, force: true });
    await rm(outsideRoot, { recursive: true, force: true });
  }
});

test("raw and RPC entries plus RPC leaf must agree exactly", async () => {
  await withFixture(async ({ path, root, entries }) => {
    const divergent = structuredClone(entries);
    const message = divergent[0]?.message as Record<string, unknown>;
    message.content = "RPC-only rewrite";
    await expectInvalid(
      classifyPiSessionMaterialization(options(path, root, divergent, "leaf0002")),
      PiSessionIntegrityFailure.RPC_ENTRIES_MISMATCH,
    );
    await expectInvalid(
      classifyPiSessionMaterialization(options(path, root, entries, "missing-leaf")),
      PiSessionIntegrityFailure.RPC_LEAF_MISMATCH,
    );
  });
});

test("resume proof preserves the exact seal and admits only a linear bounded control suffix", async () => {
  await withFixture(async ({ path, root, content, entries }) => {
    const initial = await classifyPiSessionMaterialization(
      options(path, root, entries, "leaf0002"),
    );
    assert.equal(initial.kind, "materialized");
    if (initial.kind !== "materialized") return;
    const handshake = appendHandshake(path, content, initial.seal);
    await writeFile(path, handshake.content);
    const resumed = await verifyResumedSessionExtension({
      ...options(path, root, handshake.entries, handshake.handshake.markerEntryId),
      seal: initial.seal,
      handshake: handshake.handshake,
    });
    assert.equal(resumed.kind, "materialized");

    const changedPrefix = handshake.content.replace(
      "offline fixture prompt",
      "offline fixture pr0mpt",
    );
    assert.equal(Buffer.byteLength(changedPrefix), Buffer.byteLength(handshake.content));
    await writeFile(path, changedPrefix);
    const changedEntries = parseEntries(changedPrefix);
    await expectInvalid(
      verifyResumedSessionExtension({
        ...options(path, root, changedEntries, handshake.handshake.markerEntryId),
        seal: initial.seal,
        handshake: handshake.handshake,
      }),
      PiSessionIntegrityFailure.SEALED_PREFIX_MISMATCH,
    );
  });
});

test("seal verification rejects physical replacement and invalid failures stay path-redacted", async () => {
  await withFixture(async ({ path, root, content, entries }) => {
    const initial = await classifyPiSessionMaterialization(
      options(path, root, entries, "leaf0002"),
    );
    assert.equal(initial.kind, "materialized");
    if (initial.kind !== "materialized") return;
    const replacement = join(root, "replacement.jsonl");
    await writeFile(replacement, content);
    await rename(replacement, path);
    const result = await verifyMaterializedSessionSeal({
      ...options(path, root, entries, "leaf0002"),
      seal: initial.seal,
    });
    assert.equal(result.kind, "invalid");
    assert.equal(
      result.kind === "invalid" ? result.failure : undefined,
      PiSessionIntegrityFailure.SEAL_IDENTITY_MISMATCH,
    );
    assert.equal(JSON.stringify(result).includes(path), false);
    assert.equal(JSON.stringify(result).includes("/workspace/patchbay"), false);
  });
});

async function mutationCase(
  mutate: (objects: Array<Record<string, unknown>>) => void,
  failure: PiSessionIntegrityFailure,
  leafId = "leaf0002",
): Promise<void> {
  await withFixture(async ({ path, root, content }) => {
    const objects = parseObjects(content);
    mutate(objects);
    const mutated = `${objects.map((value) => JSON.stringify(value)).join("\n")}\n`;
    await writeFile(path, mutated);
    await expectInvalid(
      classifyPiSessionMaterialization(options(path, root, objects.slice(1), leafId)),
      failure,
    );
  });
}

function appendHandshake(
  path: string,
  content: string,
  seal: MaterializedSessionSeal,
): { content: string; entries: Array<Record<string, unknown>>; handshake: PiControlHandshake } {
  const challenge = Buffer.alloc(32, 10).toString("base64url");
  const launchNonce = Buffer.alloc(32, 11).toString("base64url");
  const extensionEpoch = Buffer.alloc(16, 12).toString("base64url");
  const markerEntryId = "marker003";
  const marker = {
    type: "custom",
    id: markerEntryId,
    parentId: seal.leafId,
    timestamp: "2026-08-12T00:00:03.000Z",
    customType: PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE,
    data: {
      challenge,
      launchNonce,
      extensionEpoch,
      cwd: process.cwd(),
      sessionId,
      sessionFile: path,
    },
  };
  const nextContent = `${content}${JSON.stringify(marker)}\n`;
  return {
    content: nextContent,
    entries: parseEntries(nextContent),
    handshake: {
      challenge,
      launchNonce,
      extensionEpoch,
      cwd: process.cwd(),
      sessionId,
      sessionFile: path,
      markerEntryId,
    },
  };
}

function options(
  declaredPath: string,
  allowedRoot: string,
  rpcEntries: readonly unknown[],
  rpcLeafId: string | null,
): PiSessionFileValidationOptions {
  return { sessionId, declaredPath, allowedRoot, rpcEntries, rpcLeafId };
}

async function withFixture(
  action: (fixture: {
    path: string;
    root: string;
    content: string;
    entries: Array<Record<string, unknown>>;
  }) => Promise<void>,
): Promise<void> {
  const root = await mkdtemp(join(tmpdir(), "patchbay-session-fixture-"));
  try {
    const content = await readFile(fixturePath, "utf8");
    const path = join(root, "session.jsonl");
    await writeFile(path, content, { mode: 0o600 });
    await action({ path, root, content, entries: parseEntries(content) });
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

function parseEntries(content: string): Array<Record<string, unknown>> {
  return parseObjects(content).slice(1);
}

function parseObjects(content: string): Array<Record<string, unknown>> {
  return content
    .trimEnd()
    .split("\n")
    .map((line) => JSON.parse(line) as Record<string, unknown>);
}

async function expectInvalid(
  promise: Promise<PiSessionMaterialization>,
  failure: PiSessionIntegrityFailure,
): Promise<void> {
  const result = await promise;
  assert.equal(result.kind, "invalid");
  assert.equal(result.kind === "invalid" ? result.failure : undefined, failure);
}
