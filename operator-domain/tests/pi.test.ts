import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import test from "node:test";
import { create, toBinary } from "@bufbuild/protobuf";
import {
  ActorEndpointRefSchema,
  ActorIdSchema,
  AdapterIdSchema,
  GenerationSchema,
  ObservationKind,
  ObservationSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  PiPersistedPresentationItemSchema,
  PiPersistedProjectionEntrySchema,
  PiPersistedProjectionReplacementSchema,
  PiPersistedProjectionSuffixSchema,
  PiVolatileProjectionSnapshotSchema,
  RuntimeSessionIdSchema,
  TargetScopeKind,
  TargetScopeSchema,
} from "@patchbay/contracts";
import {
  PI_PERSISTED_REPLACEMENT_SCHEMA_REF,
  PI_PERSISTED_SUFFIX_SCHEMA_REF,
  PI_VOLATILE_PROJECTION_SCHEMA_REF,
  foldPiPersistedProjectionObservation,
  foldPiVolatileProjectionObservation,
  piProjectionObservationScope,
  type PiPersistedProjectionEntryView,
} from "../src/pi.js";

const continuityId = `pi1:${"a".repeat(43)}`;
const encoder = new TextEncoder();

function entry(
  stableEntryId: string,
  parentEntryId: string | null,
  membershipId?: string,
  text = stableEntryId,
): PiPersistedProjectionEntryView {
  return {
    stableEntryId,
    parentEntryId,
    contentDigest: createHash("sha256").update(`${stableEntryId}:${text}`).digest("hex"),
    presentationItems: membershipId ? [{
      membershipId,
      transcriptEvent: {
        kind: "user_confirmed",
        eventId: membershipId,
        sessionId: continuityId,
        ts: 1,
        messageId: stableEntryId,
        text,
      },
    }] : [],
  };
}

function replacement(
  epoch: bigint,
  entries: readonly PiPersistedProjectionEntryView[],
  leaf: string,
  target: { readonly adapterId?: string; readonly deploymentScope?: string } = {},
) {
  const treeDigest = tree(entries);
  const cursor = entries.at(-1)?.stableEntryId ?? "";
  const batchId = batch([
    "replacement", continuityId, epoch.toString(), canonical(entriesForBatch(entries)), cursor, leaf, treeDigest,
  ]);
  return observation(
    PI_PERSISTED_REPLACEMENT_SCHEMA_REF,
    toBinary(PiPersistedProjectionReplacementSchema, create(PiPersistedProjectionReplacementSchema, {
      externalContinuityId: continuityId,
      replacementEpoch: epoch,
      batchId,
      exactEntries: entries.map(wireEntry),
      cursorEntryId: cursor,
      leafEntryId: leaf,
      treeDigest,
    })),
    target,
  );
}

function suffix(
  epoch: bigint,
  baseCursor: string,
  suffixEntries: readonly PiPersistedProjectionEntryView[],
  combined: readonly PiPersistedProjectionEntryView[],
  leaf: string,
) {
  const treeDigest = tree(combined);
  const cursor = combined.at(-1)?.stableEntryId ?? "";
  const batchId = batch([
    "suffix", continuityId, epoch.toString(), baseCursor, canonical(entriesForBatch(suffixEntries)), cursor, leaf, treeDigest,
  ]);
  return observation(
    PI_PERSISTED_SUFFIX_SCHEMA_REF,
    toBinary(PiPersistedProjectionSuffixSchema, create(PiPersistedProjectionSuffixSchema, {
      externalContinuityId: continuityId,
      replacementEpoch: epoch,
      batchId,
      baseCursorEntryId: baseCursor,
      entries: suffixEntries.map(wireEntry),
      cursorEntryId: cursor,
      leafEntryId: leaf,
      treeDigest,
    })),
  );
}

function volatile(
  entries: readonly PiPersistedProjectionEntryView[],
  leaf: string,
  target: { readonly adapterId?: string; readonly deploymentScope?: string } = {},
) {
  const treeDigest = tree(entries);
  const cursor = entries.at(-1)?.stableEntryId ?? "";
  const batchId = batch([
    "volatile", continuityId, canonical(entriesForBatch(entries)), cursor, leaf, treeDigest,
  ]);
  return observation(
    PI_VOLATILE_PROJECTION_SCHEMA_REF,
    toBinary(PiVolatileProjectionSnapshotSchema, create(PiVolatileProjectionSnapshotSchema, {
      externalContinuityId: continuityId,
      batchId,
      exactEntries: entries.map(wireEntry),
      cursorEntryId: cursor,
      leafEntryId: leaf,
      treeDigest,
    })),
    target,
  );
}

test("Pi compositor exact replacement removes omitted membership while retry is inert", () => {
  const oldEntries = [entry("root", null), entry("old", "root", "membership-old", "stale")];
  const first = foldPiPersistedProjectionObservation(undefined, replacement(1n, oldEntries, "old"));
  assert.equal(first?.kind, "replacement");
  assert.deepEqual(first?.addedItems.map((item) => item.membershipId), ["membership-old"]);

  const nextEntries = [entry("root", null), entry("current", "root", "membership-current", "fresh")];
  const secondObservation = replacement(2n, nextEntries, "current");
  const second = foldPiPersistedProjectionObservation(first!.state, secondObservation);
  assert.equal(second?.kind, "replacement");
  assert.deepEqual(second?.removedMembershipIds, ["membership-old"]);
  assert.deepEqual(second?.addedItems.map((item) => item.membershipId), ["membership-current"]);

  const retry = foldPiPersistedProjectionObservation(second!.state, secondObservation);
  assert.equal(retry?.kind, "idempotent");
  assert.deepEqual(retry?.removedMembershipIds, []);
  assert.deepEqual(retry?.addedItems, []);
});

test("consumer scope key separates forced digest collisions by adapter and deployment", () => {
  const observations = [
    replacement(1n, [entry("root-a", null), entry("a", "root-a", "member-a")], "a"),
    replacement(
      1n,
      [entry("root-b", null), entry("b", "root-b", "member-b")],
      "b",
      { adapterId: "other-pi" },
    ),
    replacement(
      1n,
      [entry("root-c", null), entry("c", "root-c", "member-c")],
      "c",
      { deploymentScope: "machine-b" },
    ),
  ];
  const states = new Map<string, NonNullable<ReturnType<typeof foldPiPersistedProjectionObservation>>["state"]>();
  for (const candidate of observations) {
    const scope = piProjectionObservationScope(candidate)!;
    const folded = foldPiPersistedProjectionObservation(states.get(scope.key), candidate)!;
    states.set(scope.key, folded.state);
  }
  assert.equal(states.size, 3);
  assert.deepEqual(
    [...states.values()].map((state) => [state.adapterId, state.deploymentScope]),
    [["pi", "machine-a"], ["other-pi", "machine-a"], ["pi", "machine-b"]],
  );
});

test("volatile replay is last-observation-wins and later materialization starts authoritative epoch one", () => {
  const first = foldPiVolatileProjectionObservation(undefined, volatile([
    entry("root", null),
    entry("old", "root", "membership-old"),
  ], "old"))!;
  const second = foldPiVolatileProjectionObservation(first.state, volatile([
    entry("root", null),
    entry("new", "root", "membership-new"),
  ], "new"))!;
  assert.equal(second.kind, "snapshot");
  assert.deepEqual(second.removedMembershipIds, ["membership-old"]);
  assert.deepEqual(second.addedItems.map((item) => item.membershipId), ["membership-new"]);

  const materialized = foldPiPersistedProjectionObservation(undefined, replacement(1n, [
    entry("root", null),
    entry("new", "root", "membership-new"),
  ], "new"))!;
  assert.equal(materialized.state.replacementEpoch, 1n);
});

test("Pi compositor known suffix is stable-id idempotent and same-epoch conflicts fail closed", () => {
  const root = entry("root", null);
  const current = foldPiPersistedProjectionObservation(undefined, replacement(4n, [root], "root"))!;
  const appended = entry("new", "root", "membership-new");
  const suffixObservation = suffix(4n, "root", [appended], [root, appended], "new");
  const applied = foldPiPersistedProjectionObservation(current.state, suffixObservation)!;
  assert.equal(applied.kind, "suffix");
  assert.deepEqual(applied.addedItems.map((item) => item.membershipId), ["membership-new"]);
  assert.equal(foldPiPersistedProjectionObservation(applied.state, suffixObservation)?.kind, "idempotent");

  const conflict = replacement(4n, [root, entry("different", "root", "membership-other")], "different");
  assert.throws(
    () => foldPiPersistedProjectionObservation(applied.state, conflict),
    /same-epoch replacement content conflicts/,
  );
});

function observation(
  schemaRef: string,
  payload: Uint8Array,
  target: { readonly adapterId?: string; readonly deploymentScope?: string } = {},
) {
  const targetAdapterId = target.adapterId ?? "pi";
  return create(ObservationSchema, {
    sender: create(ActorEndpointRefSchema, { actorId: create(ActorIdSchema, { value: targetAdapterId }) }),
    kind: ObservationKind.EVENT,
    targetScope: create(TargetScopeSchema, {
      kind: TargetScopeKind.RUNTIME_SESSION,
      adapterId: create(AdapterIdSchema, { value: targetAdapterId }),
      deploymentScope: target.deploymentScope ?? "machine-a",
      runtimeSessionId: create(RuntimeSessionIdSchema, { value: "pi-session" }),
      sessionGeneration: create(GenerationSchema, { value: 2n }),
    }),
    payload: create(PayloadEnvelopeSchema, {
      schemaRef,
      contentType: PayloadContentType.PROTOBUF,
      payload,
    }),
  });
}

function wireEntry(value: PiPersistedProjectionEntryView) {
  return create(PiPersistedProjectionEntrySchema, {
    stableEntryId: value.stableEntryId,
    parentEntryId: value.parentEntryId ?? "",
    contentDigest: value.contentDigest,
    presentationItems: value.presentationItems.map((item) => create(PiPersistedPresentationItemSchema, {
      membershipId: item.membershipId,
      transcriptEventJson: encoder.encode(JSON.stringify(item.transcriptEvent)),
    })),
  });
}

function entriesForBatch(entries: readonly PiPersistedProjectionEntryView[]) {
  return entries.map((value) => ({
    stableEntryId: value.stableEntryId,
    parentEntryId: value.parentEntryId,
    contentDigest: value.contentDigest,
    presentationItems: value.presentationItems.map((item) => ({
      membershipId: item.membershipId,
      transcriptEventJson: JSON.stringify(item.transcriptEvent),
    })),
  }));
}

function tree(entries: readonly PiPersistedProjectionEntryView[]): string {
  return createHash("sha256")
    .update(JSON.stringify(entries.map((value) => [value.stableEntryId, value.parentEntryId])))
    .digest("hex");
}

function batch(parts: readonly string[]): string {
  const hash = createHash("sha256");
  for (const part of parts) hash.update(`${Buffer.byteLength(part)}:${part}\0`);
  return hash.digest("hex");
}

function canonical(value: unknown): string {
  if (value === null || typeof value === "string" || typeof value === "boolean" || typeof value === "number") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  const object = value as Record<string, unknown>;
  return `{${Object.keys(object).sort().map((key) => `${JSON.stringify(key)}:${canonical(object[key])}`).join(",")}}`;
}
