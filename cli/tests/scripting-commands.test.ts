import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { create, toBinary } from "@bufbuild/protobuf";
import {
  AdapterAssuranceManifestSchema,
  AdapterAssuranceManifestV1Schema,
  AdapterCapabilitySummarySchema,
  AdapterIdSchema,
  AdapterReconciliationStrength,
  AdapterStatusPageSchema,
  AdapterStatusSchema,
  CommandIdSchema,
  OperationKind,
  OperationSchema,
  IdempotencyStrength,
  OperationState,
  ReconciliationAction,
  StoredEventKind,
  StoredEventPayloadSchema,
  SubmissionOutcome,
  SubmissionResultSchema,
  type Operation,
} from "@patchbay/contracts";
import { cancelCommand } from "../src/commands/cancel.js";
import { instructCommand } from "../src/commands/instruct.js";
import { interruptCommand } from "../src/commands/interrupt.js";
import { sessionTargetScope } from "../src/commands/sessions.js";
import { CredentialStore } from "../src/credentials.js";
import {
  BEARER_SECRET,
  captureOutput,
  credentials,
  DOMAIN,
  session,
  snapshotResponse,
  diagnosticsResponse,
} from "./helpers.js";

async function credentialStore(): Promise<CredentialStore> {
  const directory = await mkdtemp(join(tmpdir(), "patchbay-cli-scripting-"));
  const store = new CredentialStore(join(directory, "credentials.json"));
  await store.write(credentials());
  return store;
}

function accepted(commandId = "accepted-command") {
  return create(SubmissionResultSchema, {
    outcome: SubmissionOutcome.ACCEPTED,
    commandId: create(CommandIdSchema, { value: commandId }),
    operationState: OperationState.ACCEPTED,
  });
}

test("instruct prints stable identity before submission and keeps JSON output secret-free", async () => {
  const store = await credentialStore();
  const events: string[] = [];
  const output = captureOutput(events);
  let submitted: Operation | undefined;
  const client = {
    async loadSnapshot() {
      return snapshotResponse();
    },
    async submit(request: { operation?: Operation }) {
      events.push("submit");
      submitted = request.operation;
      return accepted(request.operation?.commandId?.value);
    },
  };

  assert.equal(
    await instructCommand(
      client as never,
      store,
      DOMAIN,
      {
        target: "runtime-1",
        prompt: "Run the checks",
        json: true,
        idempotencyKey: "retry-safe-key",
      },
      output,
    ),
    0,
  );

  assert.equal(
    events[0],
    'stderr:{"target":"adapter=pi-adapter;scope=machine-a;runtime=runtime-1;generation=3"}',
  );
  assert.equal(events[1], "submit");
  assert.equal(submitted?.kind, OperationKind.INSTRUCT);
  assert.equal(new TextDecoder().decode(submitted?.payload?.payload), "Run the checks");
  assert.equal(submitted?.idempotencyKey, "retry-safe-key");
  assert.match(submitted?.commandId?.value ?? "", /^cli-[a-f0-9]{32}$/);
  assert.equal(submitted?.sender?.actorId?.value, "operator-primary");
  assert.equal(submitted?.sender?.endpointId?.value, "cli-endpoint");
  assertDefaultValidityWindow(submitted);
  assert.equal(JSON.parse(output.out[0]!).outcome, "accepted");
  assert.equal([...output.out, ...output.err].join("\n").includes(BEARER_SECRET), false);
});

test("UNKNOWN instruct fetches the limiting adapter declaration before rendering its qualifier", async () => {
  const store = await credentialStore();
  const output = captureOutput();
  let diagnosticsCalls = 0;
  const client = {
    async loadSnapshot() {
      return snapshotResponse();
    },
    async submit() {
      return create(SubmissionResultSchema, {
        outcome: SubmissionOutcome.UNKNOWN,
        operationState: OperationState.UNSPECIFIED,
      });
    },
    async queryDiagnostics() {
      diagnosticsCalls += 1;
      return diagnosticsResponse("adapters", create(AdapterStatusPageSchema, {
        adapters: [create(AdapterStatusSchema, {
          adapterId: create(AdapterIdSchema, { value: "pi-adapter" }),
          capability: create(AdapterCapabilitySummarySchema, {
            assurance: create(AdapterAssuranceManifestSchema, {
              contract: {
                case: "v1",
                value: create(AdapterAssuranceManifestV1Schema, {
                  deduplicationStrength: IdempotencyStrength.AT_PATCHBAY_BOUNDARY,
                  continuationProofSupport: false,
                  cursorSupport: false,
                  generationFenceSupport: false,
                  reconciliationStrength: AdapterReconciliationStrength.NONE,
                  unprovenOutcomeAction: ReconciliationAction.NONE,
                }),
              },
            }),
          }),
        })],
      }));
    },
  };

  assert.equal(
    await instructCommand(
      client as never,
      store,
      DOMAIN,
      { target: "runtime-1", prompt: "Run the checks", json: true },
      output,
    ),
    4,
  );
  assert.equal(diagnosticsCalls, 1);
  assert.equal(JSON.parse(output.out[0]!).outcomeQualifier, "unknown");
});

test("cancel and interrupt recover the stable target from command records", async () => {
  for (const [name, execute, expectedKind] of [
    ["cancel", cancelCommand, OperationKind.CANCEL],
    ["interrupt", interruptCommand, OperationKind.INTERRUPT],
  ] as const) {
    const store = await credentialStore();
    const original = create(OperationSchema, {
      commandId: create(CommandIdSchema, { value: "target-command" }),
      kind: OperationKind.INSTRUCT,
      targetScope: sessionTargetScope(session()),
    });
    let submitted: Operation | undefined;
    const client = {
      async *subscribe() {
        yield {
          payload: create(StoredEventPayloadSchema, {
            kind: StoredEventKind.OPERATION,
            payload: toBinary(OperationSchema, original),
          }),
        };
      },
      async submit(request: { operation?: Operation }) {
        submitted = request.operation;
        return accepted(`${name}-command`);
      },
    };
    const output = captureOutput();

    assert.equal(
      await execute(
        client as never,
        store,
        DOMAIN,
        { targetCommandId: "target-command", json: false },
        output,
      ),
      0,
    );
    assert.equal(submitted?.kind, expectedKind);
    assert.equal(submitted?.targetScope?.runtimeSessionId?.value, "runtime-1");
    assert.equal(submitted?.correlations[0]?.ref.case, "commandId");
    assertDefaultValidityWindow(submitted);
    assert.equal(
      submitted?.correlations[0]?.ref.case === "commandId"
        ? submitted.correlations[0].ref.value.value
        : undefined,
      "target-command",
    );
  }
});

function assertDefaultValidityWindow(operation: Operation | undefined): void {
  assert.ok(operation?.submittedAt);
  assert.ok(operation.validityWindow?.startsAt);
  assert.ok(operation.validityWindow.expiresAt);
  assert.equal(operation.validityWindow.startsAt.seconds, operation.submittedAt.seconds);
  assert.equal(operation.validityWindow.startsAt.nanos, operation.submittedAt.nanos);
  assert.equal(
    operation.validityWindow.expiresAt.seconds - operation.validityWindow.startsAt.seconds,
    300n,
  );
  assert.equal(operation.validityWindow.expiresAt.nanos, operation.validityWindow.startsAt.nanos);
}
