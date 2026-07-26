import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, statSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  diagnosticError,
  openAdapterDiagnostics,
  resolveAdapterLogPath,
} from "../src/adapter_diagnostics.js";
import { FailureCode, OperationKind } from "@patchbay/contracts";

function temporaryDirectory(): string {
  return mkdtempSync(join(tmpdir(), "patchbay-adapter-diagnostics-"));
}

test("diagnostics writes ordered JSONL with generated enum names and structural redaction", async () => {
  const directory = temporaryDirectory();
  const path = join(directory, "nested", "adapter.log");
  try {
    const diagnostics = await openAdapterDiagnostics({
      path,
      adapterId: "pi",
      adapterGeneration: 4,
      secrets: ["attachment-secret"],
      now: () => new Date("2026-01-02T03:04:05.000Z"),
    });
    diagnostics.record({
      event: "delivery.received",
      level: "info",
      commandId: "command-1",
      operationKind: OperationKind.INSTRUCT,
      reason: "bearer=attachment-secret prompt=do-not-log",
      session: {
        runtimeSessionId: "runtime-1",
        deploymentScope: "machine-a",
        generation: 2,
      },
    });
    diagnostics.record({
      event: "delivery.failed",
      level: "error",
      failureCode: FailureCode.EXECUTION_FAILED,
      error: Object.assign(new Error("secret message must not appear"), {
        code: "EFAIL",
        cause: "secret cause",
      }),
    });
    await diagnostics.close();

    const lines = readFileSync(path, "utf8").trimEnd().split("\n").map((line) => JSON.parse(line) as Record<string, unknown>);
    assert.equal(lines.length, 2);
    assert.deepEqual(lines.map((line) => line["event"]), ["delivery.received", "delivery.failed"]);
    assert.equal(lines[0]?.["operation_kind"], "INSTRUCT");
    assert.equal(lines[1]?.["failure_code"], "EXECUTION_FAILED");
    assert.equal(JSON.stringify(lines).includes("attachment-secret"), false);
    assert.equal(JSON.stringify(lines).includes("secret message"), false);
    assert.equal(JSON.stringify(lines).includes("secret cause"), false);
    assert.deepEqual(lines[1]?.["error"], { name: "Error", code: "EFAIL" });
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("diagnostics appends below the threshold and rotates at startup", async () => {
  const directory = temporaryDirectory();
  const path = join(directory, "adapter.log");
  try {
    writeFileSync(path, "old-content\n");
    const first = await openAdapterDiagnostics({
      path,
      adapterId: "pi",
      adapterGeneration: 1,
      rotateAtBytes: 1_000,
    });
    first.record({ event: "adapter.started", level: "info" });
    await first.close();
    assert.equal(statSync(path).isFile(), true);
    assert.equal(readFileSync(path, "utf8").startsWith("old-content"), true);

    writeFileSync(path, "x".repeat(1_000));
    const rotated = await openAdapterDiagnostics({
      path,
      adapterId: "pi",
      adapterGeneration: 2,
      rotateAtBytes: 1_000,
    });
    rotated.record({ event: "adapter.started", level: "info" });
    await rotated.close();
    assert.equal(readFileSync(join(directory, "adapter.log.1"), "utf8"), "x".repeat(1_000));
    assert.equal(JSON.parse(readFileSync(path, "utf8")).event, "adapter.started");
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("diagnostics bounds pending records and reports drops on flush", async () => {
  const directory = temporaryDirectory();
  const path = join(directory, "adapter.log");
  try {
    const diagnostics = await openAdapterDiagnostics({
      path,
      adapterId: "pi",
      adapterGeneration: 1,
      maxPendingRecords: 1,
    });
    for (let index = 0; index < 20; index += 1) {
      diagnostics.record({ event: "adapter.started", level: "info", count: index });
    }
    await diagnostics.close();
    const lines = readFileSync(path, "utf8").trimEnd().split("\n").map((line) => JSON.parse(line) as Record<string, unknown>);
    const dropped = lines.find((line) => line["event"] === "log.records_dropped");
    assert.ok(dropped);
    assert.equal(typeof dropped["count"], "number");
    assert.ok(Number(dropped["count"]) > 0);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("diagnostics failures and arbitrary errors are non-throwing", async () => {
  const directory = temporaryDirectory();
  const parentFile = join(directory, "not-a-directory");
  writeFileSync(parentFile, "file");
  const failures: string[] = [];
  try {
    const diagnostics = await openAdapterDiagnostics({
      path: join(parentFile, "adapter.log"),
      adapterId: "pi",
      adapterGeneration: 1,
      reportFailure: (code) => failures.push(code),
    });
    diagnostics.record({ event: "adapter.started", level: "info" });
    await assert.doesNotReject(diagnostics.flush());
    await assert.doesNotReject(diagnostics.close());
    assert.equal(failures[0], "open");
    assert.deepEqual(diagnosticError(Object.assign(new Error("message"), { code: 7, cause: "cause" })), {
      name: "Error",
      code: "7",
    });
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("diagnostics paths follow the XDG and override rules", () => {
  assert.equal(
    resolveAdapterLogPath({ XDG_STATE_HOME: "/state" }, "/home/test"),
    "/state/patchbay/adapter.log",
  );
  assert.equal(
    resolveAdapterLogPath({ XDG_STATE_HOME: "relative" }, "/home/test"),
    "/home/test/.local/state/patchbay/adapter.log",
  );
  assert.equal(
    resolveAdapterLogPath({ PATCHBAY_ADAPTER_LOG: "relative.log" }, "/home/test"),
    join(process.cwd(), "relative.log"),
  );
});
