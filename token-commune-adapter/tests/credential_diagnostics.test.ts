import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { create, fromBinary } from "@bufbuild/protobuf";
import { AdapterDiagnosticPayloadSchema, AdapterDiagnosticReportResultSchema, FailureCode, OperationKind } from "@patchbay/contracts";
import { openAdapterDiagnostics } from "../src/adapter_diagnostics.js";
import { loadGatewayCredential } from "../src/credential.js";
import { CoreDiagnosticsForwarder, TOKEN_COMMUNE_FORWARDED_DIAGNOSTIC_CODES } from "../src/core_diagnostics_forwarder.js";

function temporary(): string { return mkdtempSync(join(tmpdir(), "patchbay-token-credential-")); }

test("credential requires a non-symlink regular 0600 single-line file and applies bearer only", async () => {
  const directory = temporary();
  try {
    const path = join(directory, "member.key");
    writeFileSync(path, "member-key\n", { mode: 0o600 });
    const credential = await loadGatewayCredential(path);
    const headers = new Headers();
    credential.apply(headers);
    assert.equal(headers.get("authorization"), "Bearer member-key");
    assert.equal(headers.has("x-api-key"), false);
    assert.deepEqual(credential.redactionSecrets(), ["member-key", "Bearer member-key"]);
    credential.dispose();
    assert.throws(() => credential.apply(new Headers()), /disposed/);

    const unsafe = join(directory, "unsafe.key");
    writeFileSync(unsafe, "unsafe", { mode: 0o644 });
    chmodSync(unsafe, 0o644);
    await assert.rejects(loadGatewayCredential(unsafe), fixedError);
    const link = join(directory, "link.key");
    symlinkSync(path, link);
    await assert.rejects(loadGatewayCredential(link), fixedError);
    const multiline = join(directory, "multi.key");
    writeFileSync(multiline, "one\ntwo", { mode: 0o600 });
    await assert.rejects(loadGatewayCredential(multiline), fixedError);
    const empty = join(directory, "empty.key");
    writeFileSync(empty, "\n", { mode: 0o600 });
    await assert.rejects(loadGatewayCredential(empty), fixedError);
    await assert.rejects(loadGatewayCredential(directory), fixedError);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

function fixedError(error: unknown): boolean {
  return error instanceof Error && error.message === "token-commune member credential is unavailable or unsafe";
}

test("local diagnostics redact gateway key, bearer value, attachment evidence, and credential path", async () => {
  const directory = temporary();
  const path = join(directory, "adapter.log");
  const secrets = ["member-key", "attachment-evidence", "/private/member.key"];
  try {
    const diagnostics = await openAdapterDiagnostics({ path, adapterId: "token-commune", adapterGeneration: 1, secrets });
    diagnostics.record({
      event: "gateway.request.failed", level: "error",
      resource: { resourceKind: `Bearer member-key`, resourceId: `attachment-evidence /private/member.key` },
      commandId: "authorization=member-key", error: { name: "secret", code: "api_key=member-key" },
    });
    await diagnostics.close();
    const serialized = readFileSync(path, "utf8");
    for (const secret of secrets) assert.equal(serialized.includes(secret), false);
    assert.equal(serialized.toLowerCase().includes("authorization=member-key"), false);
  } finally { rmSync(directory, { recursive: true, force: true }); }
});

test("forwarder uses the shared registry and structurally discards arbitrary local strings", async () => {
  const reports: any[] = [];
  const forwarder = new CoreDiagnosticsForwarder(async (report) => {
    reports.push(report);
    return create(AdapterDiagnosticReportResultSchema, { accepted: true });
  }, { authorityDomainId: "default", adapterId: "token-commune", adapterGeneration: 2 }, { reportsPerSecond: 1_000 });
  forwarder.record({
    event: "delivery.unsupported", level: "warn", commandId: "command-1", operationKind: OperationKind.QUERY,
    failureCode: FailureCode.UNSUPPORTED_COMMAND, error: { name: "attachment-evidence", code: "Bearer member-key" },
  });
  await forwarder.flush();
  assert.equal(reports.length, 1);
  const payload = fromBinary(AdapterDiagnosticPayloadSchema, reports[0].payload.payload);
  assert.equal(payload.code, TOKEN_COMMUNE_FORWARDED_DIAGNOSTIC_CODES["delivery.unsupported"]);
  assert.equal(payload.operationKind, OperationKind.QUERY);
  const serialized = JSON.stringify(reports[0], (_key, value) => typeof value === "bigint" ? value.toString() : value);
  assert.equal(serialized.includes("member-key"), false);
  assert.equal(serialized.includes("attachment-evidence"), false);
});

test("forwarding failures are bounded, non-retrying, and sink isolation is non-interfering", async () => {
  let calls = 0;
  const forwarder = new CoreDiagnosticsForwarder(async () => {
    calls += 1;
    throw new Error("network unavailable");
  }, { authorityDomainId: "default", adapterId: "token-commune", adapterGeneration: 1 }, { reportTimeoutMs: 10, maxFlushMs: 50, reportsPerSecond: 1_000 });
  assert.doesNotThrow(() => forwarder.record({ event: "adapter.started", level: "info" }));
  await assert.doesNotReject(forwarder.flush());
  assert.equal(calls, 1);
  await assert.doesNotReject(forwarder.close());
});
