import assert from "node:assert/strict";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import {
  OperationState,
  SubmissionOutcome,
  SessionSchema,
  SubmissionResultSchema,
} from "@patchbay/contracts";
import { sessionHealthCommand } from "../src/commands/session-health.js";
import { parseArguments, run, usage } from "../src/main.js";
import { exitCodeForSubmission, printSubmissionResult } from "../src/output.js";
import { captureOutput, DOMAIN, session, snapshotResponse } from "./helpers.js";

test("SubmissionOutcome has stable script-facing exit codes", () => {
  assert.equal(exitCodeForSubmission(SubmissionOutcome.ACCEPTED), 0);
  assert.equal(exitCodeForSubmission(SubmissionOutcome.REJECTED), 2);
  assert.equal(exitCodeForSubmission(SubmissionOutcome.FAILED), 3);
  assert.equal(exitCodeForSubmission(SubmissionOutcome.UNKNOWN), 4);
  assert.equal(exitCodeForSubmission(SubmissionOutcome.UNSPECIFIED), 1);
});

test("UNKNOWN output directs reconciliation through core command records", () => {
  const output = captureOutput();
  const result = create(SubmissionResultSchema, {
    outcome: SubmissionOutcome.UNKNOWN,
    operationState: OperationState.UNSPECIFIED,
  });

  assert.equal(printSubmissionResult(result, true, output), 4);
  assert.equal(JSON.parse(output.out[0]!).outcome, "unknown");
  assert.match(output.err.join("\n"), /reconcile via the core's command records/);
});

test("session-health emits canonical connectivity and activity as JSON", async () => {
  const output = captureOutput();
  assert.equal(
    await sessionHealthCommand(
      { async loadSnapshot() { return snapshotResponse(); } } as never,
      DOMAIN,
      { json: true },
      output,
    ),
    0,
  );

  const rows = JSON.parse(output.out[0]!) as Array<Record<string, unknown>>;
  assert.equal(rows[0]?.["connectivity"], "live");
  assert.equal(rows[0]?.["activity"], "working");
  assert.equal(rows[0]?.["model"], "provider/model-1");
  assert.match(String(rows[0]?.["identity"]), /adapter=pi-adapter.*generation=3/);
});

test("session-health renders unavailable model as null in JSON and unknown in tables", async () => {
  const output = captureOutput();
  const unavailable = create(SessionSchema, { ...session(), model: "" });
  assert.equal(
    await sessionHealthCommand(
      { async loadSnapshot() { return snapshotResponse([unavailable]); } } as never,
      DOMAIN,
      { json: true },
      output,
    ),
    0,
  );
  assert.equal(JSON.parse(output.out[0]!)[0].model, null);

  const table = captureOutput();
  await sessionHealthCommand(
    { async loadSnapshot() { return snapshotResponse([unavailable]); } } as never,
    DOMAIN,
    { json: false },
    table,
  );
  assert.match(table.out[0]!, /MODEL/);
  assert.match(table.out[1]!, /Model unknown/);
});

test("secret-bearing arguments are rejected without echoing their value", async () => {
  const secret = "argv-secret-must-not-appear";
  const output = captureOutput();
  const exit = await run(
    ["login", `--password=${secret}`],
    output,
    { env: { PATCHBAY_CORE_SECRET: "configured" } },
  );

  assert.equal(exit, 1);
  assert.match(output.err.join("\n"), /unknown option: --password/);
  assert.doesNotMatch([...output.out, ...output.err].join("\n"), new RegExp(secret));
  assert.throws(() => parseArguments(["--setup-secret", secret]), /unknown option: --setup-secret/);
  assert.doesNotMatch(usage(), /--(?:password|setup-secret)/);
  assert.match(usage(), /PATCHBAY_OPERATOR_PASSWORD/);
});

test("state-changing dispatch refuses a missing credential store before submission", async () => {
  const output = captureOutput();
  const exit = await run(
    ["instruct", "runtime-1", "hello"],
    output,
    {
      env: {
        PATCHBAY_CORE_SECRET: "configured",
        PATCHBAY_CREDENTIALS_PATH: "/tmp/patchbay-cli-test-definitely-missing/credentials.json",
      },
    },
  );
  assert.equal(exit, 1);
  assert.match(output.err.join("\n"), /run patchbay-cli login/);
});
