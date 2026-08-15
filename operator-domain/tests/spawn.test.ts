import assert from "node:assert/strict";
import test from "node:test";
import { create, fromBinary } from "@bufbuild/protobuf";
import {
  AdapterIdSchema,
  ContinuationContextStatus,
  ExternalRuntimeRefSchema,
  GenerationSchema,
  LogicalTargetIdSchema,
  RuntimeGenerationRefSchema,
  RuntimeSessionIdSchema,
  SpawnRequestSchema,
  TargetScopeKind,
} from "@patchbay/contracts";
import {
  continuationContextExplanation,
  continuationContextStatusName,
  continuationSpawnPayload,
  freshSpawnPayload,
  spawnAdapterTarget,
} from "../src/spawn.js";

function prior(generation = 7n) {
  return create(RuntimeGenerationRefSchema, {
    logicalTargetId: create(LogicalTargetIdSchema, { value: "logical-a" }),
    externalRuntime: create(ExternalRuntimeRefSchema, {
      adapterId: create(AdapterIdSchema, { value: "pi" }),
      deploymentScope: "machine-a",
      runtimeSessionId: create(RuntimeSessionIdSchema, { value: "runtime-a" }),
      generation: create(GenerationSchema, { value: generation }),
    }),
  });
}

test("fresh and continuation encode as one spawn payload with disjoint exact intents", () => {
  const fresh = fromBinary(SpawnRequestSchema, freshSpawnPayload({ shape: "session" }).payload);
  const continuation = fromBinary(
    SpawnRequestSchema,
    continuationSpawnPayload(prior(), {
      shape: "session",
      deploymentAuthorityRef: "workspace-key",
    }).payload,
  );

  assert.equal(fresh.intent.case, "fresh");
  assert.equal(continuation.intent.case, "continuation");
  assert.deepEqual(
    continuation.intent.case === "continuation" ? continuation.intent.value.prior : undefined,
    prior(),
  );
  assert.equal(continuation.targetSpec?.deploymentAuthorityRef, "workspace-key");
  const target = spawnAdapterTarget("pi");
  assert.equal(target.kind, TargetScopeKind.ADAPTER);
  assert.equal(target.adapterId?.value, "pi");
  assert.equal(target.runtimeSessionId, undefined);
});

test("continuation rejects wildcard, zero, and unadvanceable prior identity", () => {
  const missingLogical = prior();
  missingLogical.logicalTargetId = undefined;
  assert.throws(() => continuationSpawnPayload(missingLogical, { shape: "session" }), /logical target/);
  assert.throws(() => continuationSpawnPayload(prior(0n), { shape: "session" }), /positive/);
  assert.throws(
    () => continuationSpawnPayload(prior((1n << 64n) - 1n), { shape: "session" }),
    /cannot advance/,
  );
});

test("generated context vocabulary never claims arbitrary process-state restoration", () => {
  assert.equal(continuationContextStatusName(ContinuationContextStatus.RESUMED), "resumed");
  assert.equal(continuationContextStatusName(ContinuationContextStatus.NEW_CONTEXT), "new_context");
  assert.equal(continuationContextStatusName(ContinuationContextStatus.UNKNOWN), "unknown");
  assert.match(
    continuationContextExplanation(ContinuationContextStatus.RESUMED),
    /logical context resumed/,
  );
  assert.match(
    continuationContextExplanation(ContinuationContextStatus.RESUMED),
    /process state was not restored/,
  );
  assert.match(
    continuationContextExplanation(ContinuationContextStatus.NEW_CONTEXT),
    /new adapter-native logical context/,
  );
  assert.match(
    continuationContextExplanation(ContinuationContextStatus.UNKNOWN),
    /no process-state continuity is claimed/,
  );
  assert.throws(
    () => continuationContextExplanation(ContinuationContextStatus.UNSPECIFIED),
    /unspecified/,
  );
});
