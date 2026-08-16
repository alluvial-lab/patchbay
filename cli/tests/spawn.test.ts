import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";

import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import {
  AbandonSpawnTargetResultSchema,
  AdapterIdSchema,
  CommandIdSchema,
  ContinuationContextStatus,
  ExternalRuntimeRefSchema,
  GenerationSchema,
  LogicalTargetIdSchema,
  OperationKind,
  OperationSchema,
  OperationState,
  RuntimeGenerationRefSchema,
  RuntimeSessionIdSchema,
  SpawnClaimDisposition,
  SpawnPromotionCommittedSchema,
  SpawnPromotionStagedEvidenceSchema,
  SpawnRequestSchema,
  SpawnSuccessorEvidenceStagedSchema,
  StoredEventKind,
  StoredEventPayloadSchema,
  SubmissionOutcome,
  SubmissionResultSchema,
  SubscribeEventSchema,
  type Operation,
} from "@patchbay/contracts";
import {
  abandonSpawnTargetCommand,
  restartCommand,
  spawnCommand,
} from "../src/commands/spawn.js";
import type { ControlClient } from "../src/core-client.js";
import { CredentialStore } from "../src/credentials.js";
import { captureOutput, credentials, DOMAIN, snapshotResponse } from "./helpers.js";

async function store(): Promise<CredentialStore> {
  const directory = await mkdtemp(join(tmpdir(), "patchbay-cli-spawn-"));
  const result = new CredentialStore(join(directory, "credentials.json"));
  await result.write(credentials());
  return result;
}

function accepted(commandId: string) {
  return create(SubmissionResultSchema, {
    outcome: SubmissionOutcome.ACCEPTED,
    commandId: create(CommandIdSchema, { value: commandId }),
    operationState: OperationState.ACCEPTED,
  });
}

test("spawn target abandonment uses the typed RPC and canonical disposition presentation", async () => {
  let request: Parameters<ControlClient["abandonSpawnTarget"]>[0] | undefined;
  const output = captureOutput();
  const exit = await abandonSpawnTargetCommand(
    {
      async abandonSpawnTarget(candidate) {
        request = candidate;
        return create(AbandonSpawnTargetResultSchema, {
          changed: true,
          disposition: SpawnClaimDisposition.TARGET_ABANDONED,
          logicalTargetId: { value: "logical-primary" },
          abandonmentEventId: { authorityDomainId: { value: DOMAIN }, lsn: { value: 9n } },
          auditEventId: { authorityDomainId: { value: DOMAIN }, lsn: { value: 10n } },
          authorizingGrantId: { value: "abandon-grant" },
        });
      },
    },
    await store(),
    DOMAIN,
    {
      claimOperationId: "spawn-restart",
      logicalTargetId: "logical-primary",
      reasonCode: "operator_recovery",
      json: true,
    },
    output,
  );
  assert.equal(exit, 0);
  assert.equal(request?.claimOperationId?.value, "spawn-restart");
  assert.equal(request?.logicalTargetId?.value, "logical-primary");
  assert.deepEqual(JSON.parse(output.out[0]!), {
    changed: true,
    alreadyAbandoned: false,
    disposition: "target_abandoned",
    claimOperationId: "spawn-restart",
    logicalTargetId: "logical-primary",
    abandonmentEventLsn: "9",
    auditEventLsn: "10",
    authorizingGrantId: "abandon-grant",
  });
});

test("spawn submits the shared fresh SpawnRequest to one explicit adapter", async () => {
  let submitted: Operation | undefined;
  const output = captureOutput();
  const exit = await spawnCommand(
    {
      async submit(request) {
        submitted = request.operation ? create(OperationSchema, request.operation) : undefined;
        return accepted("spawn-fresh");
      },
    },
    await store(),
    DOMAIN,
    {
      adapterId: "pi-adapter",
      shape: "session",
      commandId: "spawn-fresh",
      idempotencyKey: "spawn-fresh-key",
      json: false,
    },
    output,
  );
  assert.equal(exit, 0);
  assert.equal(submitted?.kind, OperationKind.SPAWN);
  assert.equal(submitted?.targetScope?.adapterId?.value, "pi-adapter");
  const request = fromBinary(SpawnRequestSchema, submitted!.payload!.payload);
  assert.equal(request.intent.case, "fresh");
  assert.match(output.err.join("\n"), /intent=fresh/);
});

test("restart resolves durable managed identity and submits exact continuation without restoration claims", async () => {
  const prior = create(RuntimeGenerationRefSchema, {
    logicalTargetId: create(LogicalTargetIdSchema, { value: "logical-primary" }),
    externalRuntime: create(ExternalRuntimeRefSchema, {
      adapterId: create(AdapterIdSchema, { value: "pi-adapter" }),
      deploymentScope: "machine-a",
      runtimeSessionId: create(RuntimeSessionIdSchema, { value: "runtime-1" }),
      generation: create(GenerationSchema, { value: 3n }),
    }),
  });
  const promotion = create(SpawnPromotionCommittedSchema, {
    promotedRuntime: prior,
    stagedSuccessor: create(SpawnPromotionStagedEvidenceSchema, {
      staged: create(SpawnSuccessorEvidenceStagedSchema, {
        continuationContextStatus: ContinuationContextStatus.RESUMED,
      }),
    }),
  });
  let submitted: Operation | undefined;
  const output = captureOutput();
  const client: Pick<ControlClient, "loadSnapshot" | "subscribe" | "submit"> = {
    async loadSnapshot() {
      return snapshotResponse();
    },
    async *subscribe() {
      yield create(SubscribeEventSchema, {
        payload: create(StoredEventPayloadSchema, {
          kind: StoredEventKind.SPAWN_PROMOTION_COMMITTED,
          payload: toBinary(SpawnPromotionCommittedSchema, promotion),
        }),
      });
    },
    async submit(request) {
      submitted = request.operation ? create(OperationSchema, request.operation) : undefined;
      return accepted("spawn-restart");
    },
  };
  const exit = await restartCommand(
    client,
    await store(),
    DOMAIN,
    {
      target: "runtime-1",
      shape: "session",
      commandId: "spawn-restart",
      idempotencyKey: "spawn-restart-key",
      json: false,
    },
    output,
  );
  assert.equal(exit, 0);
  assert.equal(submitted?.kind, OperationKind.SPAWN);
  assert.equal(submitted?.targetScope?.adapterId?.value, "pi-adapter");
  const request = fromBinary(SpawnRequestSchema, submitted!.payload!.payload);
  assert.equal(request.intent.case, "continuation");
  if (request.intent.case !== "continuation") assert.fail("continuation intent expected");
  assert.deepEqual(request.intent.value.prior, prior);
  assert.match(output.err.join("\n"), /logical context resumed/);
  assert.doesNotMatch(output.err.join("\n"), /context continuity is unknown/);
  assert.doesNotMatch(output.err.join("\n"), /process state (was|is) restored/i);
});
