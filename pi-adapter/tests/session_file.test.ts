import assert from "node:assert/strict";
import { readFile, rename, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { mkdtemp } from "node:fs/promises";
import test from "node:test";
import {
  ContinuationContextStatus,
  ExternalEffectDisposition,
  FailureCode,
  PiReloadableResourceKind,
  PiSessionIntegrityFailure,
  SpawnExecutionPhase,
} from "@patchbay/contracts";
import {
  PATCHBAY_CONTROL_HANDSHAKE_CUSTOM_TYPE,
  PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE,
  PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE,
} from "../extensions/patchbay-control.js";
import type { PiControlHandshake } from "../src/control_handshake.js";
import {
  admitPiContinuationBeforeLaunch,
  classifyPiSessionMaterialization,
  verifyMaterializedSessionSeal,
  verifyResumedSessionExtension,
  type MaterializedSessionSeal,
  type PiContinuationLaunchPlan,
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

test("strict Pi v3 validator rejects invalid assistant optional fields and nested signatures", async () => {
  await assistantMutationCase((message) => {
    message.responseId = 42;
  });

  const invalidOptionalMutations: Array<
    [string, (message: Record<string, unknown>) => void]
  > = [
    ["responseModel", (message) => { message.responseModel = false; }],
    ["diagnostics", (message) => {
      message.diagnostics = [{ type: "provider", timestamp: 1, error: { message: 42 } }];
    }],
    ["deferred", (message) => {
      message.deferred = {
        provider: "faux",
        modelId: "offline",
        api: "faux",
        id: "deferred-1",
        pollAfterMs: "later",
      };
    }],
    ["errorMessage", (message) => { message.errorMessage = 42; }],
    ["rawStopReason", (message) => { message.rawStopReason = 42; }],
    ["textSignature", (message) => {
      message.content = [{ type: "text", text: "signed", textSignature: 42 }];
    }],
    ["thinkingSignature", (message) => {
      message.content = [{ type: "thinking", thinking: "reason", thinkingSignature: 42 }];
    }],
    ["thinking redacted", (message) => {
      message.content = [{ type: "thinking", thinking: "reason", redacted: "yes" }];
    }],
    ["tool thoughtSignature", (message) => {
      message.content = [{
        type: "toolCall",
        id: "tool001",
        name: "lookup",
        arguments: {},
        thoughtSignature: 42,
      }];
    }],
    ["complete Usage", (message) => {
      const usage = message.usage as Record<string, unknown>;
      delete usage.totalTokens;
    }],
    ["complete Usage.cost", (message) => {
      const usage = message.usage as Record<string, unknown>;
      const cost = usage.cost as Record<string, unknown>;
      delete cost.total;
    }],
  ];
  for (const [label, mutate] of invalidOptionalMutations) {
    await assistantMutationCase(mutate, label);
  }
});

test("strict Pi v3 validator accepts installed optional assistant fields when well formed", async () => {
  await withFixture(async ({ path, root, content }) => {
    const objects = parseObjects(content);
    const message = objects[2]!.message as Record<string, unknown>;
    message.responseModel = "offline-v2";
    message.responseId = "response-1";
    message.diagnostics = [{
      type: "provider_retry",
      timestamp: 1776124802001,
      error: { name: "RetryError", message: "retry", stack: "stack", code: 429 },
      details: { attempt: 1 },
    }];
    message.deferred = {
      provider: "faux",
      modelId: "offline",
      api: "faux",
      id: "deferred-1",
      expiresAt: 1776124900000,
      pollAfterMs: 0,
      data: { row: 1, values: ["x", true, null] },
    };
    message.errorMessage = "";
    message.rawStopReason = "provider_stop";
    message.content = [
      { type: "text", text: "signed", textSignature: "text-signature" },
      {
        type: "thinking",
        thinking: "reason",
        thinkingSignature: "thinking-signature",
        redacted: false,
      },
      {
        type: "toolCall",
        id: "tool001",
        name: "lookup",
        arguments: { key: "value" },
        thoughtSignature: "thought-signature",
      },
    ];
    const usage = message.usage as Record<string, unknown>;
    usage.cacheWrite1h = 0;
    usage.reasoning = 1;
    const mutated = serializeObjects(objects);
    await writeFile(path, mutated);
    const result = await classifyPiSessionMaterialization(
      options(path, root, objects.slice(1), "leaf0002"),
    );
    assert.equal(result.kind, "materialized");
  });
});

test("strict Pi v3 validator rejects invalid bash optionals and incomplete summary Usage", async () => {
  const bashMessage = {
    role: "bashExecution",
    command: "pwd",
    output: "/workspace/patchbay",
    exitCode: 0,
    cancelled: false,
    truncated: false,
    timestamp: 1776124803000,
  };
  await appendedMessageMutationCase(
    { ...bashMessage, fullOutputPath: 42 },
    "bash-full-output-path",
  );
  await appendedMessageMutationCase(
    { ...bashMessage, excludeFromContext: "false" },
    "bash-exclude-from-context",
  );
  await mutationCase((objects) => {
    objects.push({
      type: "compaction",
      id: "compact03",
      parentId: "leaf0002",
      timestamp: "2026-08-12T00:00:03.000Z",
      summary: "summary",
      firstKeptEntryId: "root0001",
      tokensBefore: 2,
      usage: {},
    });
  }, PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID, "compact03");
});

test("reload completions require an earlier exactly matching request entry", async () => {
  await withFixture(async ({ path, root, content, entries }) => {
    const initial = await classifyPiSessionMaterialization(
      options(path, root, entries, "leaf0002"),
    );
    assert.equal(initial.kind, "materialized");
    if (initial.kind !== "materialized") return;

    const completion = reloadCompletion("missing-request");
    const withCompletion = `${content}${JSON.stringify(completion)}\n`;
    const handshake = appendHandshake(path, withCompletion, initial.seal, completion.id);
    await writeFile(path, handshake.content);
    await expectInvalid(
      verifyResumedSessionExtension({
        ...options(path, root, handshake.entries, handshake.handshake.markerEntryId),
        seal: initial.seal,
        handshake: handshake.handshake,
      }),
      PiSessionIntegrityFailure.REFERENCE_INVALID,
    );
  });

  await withFixture(async ({ path, root, content, entries }) => {
    const initial = await classifyPiSessionMaterialization(
      options(path, root, entries, "leaf0002"),
    );
    assert.equal(initial.kind, "materialized");
    if (initial.kind !== "materialized") return;

    const request = reloadRequest();
    const completion = { ...reloadCompletion(request.id), data: {
      ...reloadCompletion(request.id).data,
      commandId: "another-command",
    } };
    const suffix = `${content}${JSON.stringify(request)}\n${JSON.stringify(completion)}\n`;
    const handshake = appendHandshake(path, suffix, initial.seal, completion.id);
    await writeFile(path, handshake.content);
    await expectInvalid(
      verifyResumedSessionExtension({
        ...options(path, root, handshake.entries, handshake.handshake.markerEntryId),
        seal: initial.seal,
        handshake: handshake.handshake,
      }),
      PiSessionIntegrityFailure.REFERENCE_INVALID,
    );
  });

  await withFixture(async ({ path, root, content, entries }) => {
    const initial = await classifyPiSessionMaterialization(
      options(path, root, entries, "leaf0002"),
    );
    assert.equal(initial.kind, "materialized");
    if (initial.kind !== "materialized") return;

    const request = reloadRequest();
    const completion = reloadCompletion(request.id);
    const suffix = `${content}${JSON.stringify(request)}\n${JSON.stringify(completion)}\n`;
    const handshake = appendHandshake(path, suffix, initial.seal, completion.id);
    await writeFile(path, handshake.content);
    const resumed = await verifyResumedSessionExtension({
      ...options(path, root, handshake.entries, handshake.handshake.markerEntryId),
      seal: initial.seal,
      handshake: handshake.handshake,
    });
    assert.equal(resumed.kind, "materialized");
  });
});

test("require_resume admission refuses before launch without a fresh materialized seal", async () => {
  const root = await mkdtemp(join(tmpdir(), "patchbay-resume-admission-"));
  try {
    const missing = join(root, "memory-only.jsonl");
    const launches: PiContinuationLaunchPlan[] = [];
    const memoryOnly = await admitPiContinuationBeforeLaunch(
      { ...options(missing, root, [], null), mode: "require_resume" },
      (plan) => { launches.push(plan); return "launched"; },
    );
    assert.equal(memoryOnly.kind, "refused");
    assert.equal(memoryOnly.kind === "refused" ? memoryOnly.materialization : undefined, "memory_only");
    assert.deepEqual(
      memoryOnly.kind === "refused" ? memoryOnly.evidence : undefined,
      {
        phase: SpawnExecutionPhase.QUIESCING_PRIOR,
        externalEffectDisposition: ExternalEffectDisposition.PROVED_NONE,
        failureCode: FailureCode.EXECUTION_FAILED,
        noExternalEffectProof: "exact_supervisor_pre_launch_failure",
        successorLaunchEffect: "not_attempted",
      },
    );
    assert.equal(launches.length, 0);

    const content = await readFile(fixturePath, "utf8");
    const objects = parseObjects(content);
    const message = objects[2]!.message as Record<string, unknown>;
    message.responseId = 42;
    await writeFile(missing, serializeObjects(objects));
    const invalid = await admitPiContinuationBeforeLaunch(
      { ...options(missing, root, objects.slice(1), "leaf0002"), mode: "require_resume" },
      (plan) => { launches.push(plan); return "launched"; },
    );
    assert.equal(invalid.kind, "refused");
    assert.equal(invalid.kind === "refused" ? invalid.materialization : undefined, "invalid");
    assert.equal(
      invalid.kind === "refused" ? invalid.integrityFailure : undefined,
      PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID,
    );
    assert.equal(launches.length, 0);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});

test("continuation admission selects resume only for require_resume and fresh context only when allowed", async () => {
  await withFixture(async ({ path, root, entries }) => {
    const launches: PiContinuationLaunchPlan[] = [];
    const resumed = await admitPiContinuationBeforeLaunch(
      { ...options(path, root, entries, "leaf0002"), mode: "require_resume" },
      (plan) => { launches.push(plan); return "resume-launch"; },
    );
    assert.equal(resumed.kind, "launch_admitted");
    if (resumed.kind !== "launch_admitted") return;
    assert.equal(resumed.launchResult, "resume-launch");
    assert.equal(resumed.plan.mode, "require_resume");
    assert.equal(resumed.plan.resumeSelector, path);
    assert.equal(
      resumed.plan.onlyAllowedContextStatus,
      ContinuationContextStatus.RESUMED,
    );
    assert.equal(launches.length, 1);
  });

  const root = await mkdtemp(join(tmpdir(), "patchbay-new-context-admission-"));
  try {
    const missing = join(root, "intentionally-new.jsonl");
    const launches: PiContinuationLaunchPlan[] = [];
    const fresh = await admitPiContinuationBeforeLaunch(
      { ...options(missing, root, [], null), mode: "allow_new_context" },
      (plan) => { launches.push(plan); return "fresh-launch"; },
    );
    assert.equal(fresh.kind, "launch_admitted");
    if (fresh.kind !== "launch_admitted") return;
    assert.equal(fresh.launchResult, "fresh-launch");
    assert.deepEqual(fresh.plan, {
      mode: "allow_new_context",
      resumeSelector: null,
      onlyAllowedContextStatus: ContinuationContextStatus.NEW_CONTEXT,
    });
    assert.deepEqual(launches, [fresh.plan]);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
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
  label?: string,
): Promise<void> {
  await withFixture(async ({ path, root, content }) => {
    const objects = parseObjects(content);
    mutate(objects);
    const mutated = serializeObjects(objects);
    await writeFile(path, mutated);
    await expectInvalid(
      classifyPiSessionMaterialization(options(path, root, objects.slice(1), leafId)),
      failure,
      label,
    );
  });
}

function appendHandshake(
  path: string,
  content: string,
  seal: MaterializedSessionSeal,
  parentId = seal.leafId,
): { content: string; entries: Array<Record<string, unknown>>; handshake: PiControlHandshake } {
  const challenge = Buffer.alloc(32, 10).toString("base64url");
  const launchNonce = Buffer.alloc(32, 11).toString("base64url");
  const extensionEpoch = Buffer.alloc(16, 12).toString("base64url");
  const markerEntryId = "marker003";
  const marker = {
    type: "custom",
    id: markerEntryId,
    parentId,
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

function reloadRequest(): Record<string, unknown> & { id: string } {
  return {
    type: "custom",
    id: "reload004",
    parentId: "leaf0002",
    timestamp: "2026-08-12T00:00:03.000Z",
    customType: PATCHBAY_CONTROL_RELOAD_REQUEST_CUSTOM_TYPE,
    data: {
      commandId: "reload-command",
      nonce: Buffer.alloc(32, 21).toString("base64url"),
      priorExtensionEpoch: Buffer.alloc(16, 22).toString("base64url"),
      resources: [PiReloadableResourceKind.SKILL],
    },
  };
}

function reloadCompletion(requestEntryId: string): Record<string, unknown> & {
  id: string;
  data: Record<string, unknown>;
} {
  return {
    type: "custom",
    id: "reload005",
    parentId: requestEntryId === "missing-request" ? "leaf0002" : requestEntryId,
    timestamp: "2026-08-12T00:00:04.000Z",
    customType: PATCHBAY_CONTROL_RELOAD_COMPLETION_CUSTOM_TYPE,
    data: {
      commandId: "reload-command",
      nonce: Buffer.alloc(32, 21).toString("base64url"),
      requestEntryId,
      priorExtensionEpoch: Buffer.alloc(16, 22).toString("base64url"),
      extensionEpoch: Buffer.alloc(16, 23).toString("base64url"),
    },
  };
}

async function assistantMutationCase(
  mutate: (message: Record<string, unknown>) => void,
  label = "assistant-response-id",
): Promise<void> {
  await mutationCase((objects) => {
    const message = objects[2]!.message as Record<string, unknown>;
    mutate(message);
  }, PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID, "leaf0002", label);
}

async function appendedMessageMutationCase(
  message: Record<string, unknown>,
  label: string,
): Promise<void> {
  await mutationCase((objects) => {
    objects.push({
      type: "message",
      id: "message03",
      parentId: "leaf0002",
      timestamp: "2026-08-12T00:00:03.000Z",
      message,
    });
  }, PiSessionIntegrityFailure.ENTRY_SHAPE_INVALID, "message03", label);
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

function serializeObjects(objects: readonly Record<string, unknown>[]): string {
  return `${objects.map((value) => JSON.stringify(value)).join("\n")}\n`;
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
  label?: string,
): Promise<void> {
  const result = await promise;
  assert.equal(result.kind, "invalid", label);
  assert.equal(result.kind === "invalid" ? result.failure : undefined, failure, label);
}
