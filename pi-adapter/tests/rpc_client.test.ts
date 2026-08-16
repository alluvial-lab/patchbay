import assert from "node:assert/strict";
import { PassThrough } from "node:stream";
import test from "node:test";
import { PiRpcClient, PiRpcTransportError } from "../src/rpc_client.js";

function streams() {
  return {
    stdin: new PassThrough(),
    stdout: new PassThrough(),
    stderr: new PassThrough(),
  };
}

async function nextWrittenLine(stdin: PassThrough): Promise<Record<string, unknown>> {
  const chunk = await new Promise<Buffer>((resolve) => stdin.once("data", resolve));
  return JSON.parse(chunk.toString("utf8")) as Record<string, unknown>;
}

test("PiRpcClient assigns unique ids and keeps responses out of the event stream", async () => {
  const io = streams();
  const client = new PiRpcClient({ streams: io, requestPrefix: "request-prefix", requestTimeoutMs: 1_000 });
  const events: Record<string, unknown>[] = [];
  client.onEvent((event) => events.push(event));

  const first = client.request<{ value: number }>({ type: "get_state" });
  const firstRequest = await nextWrittenLine(io.stdin);
  const second = client.request<{ value: number }>({ type: "get_entries" });
  const secondRequest = await nextWrittenLine(io.stdin);
  assert.notEqual(firstRequest["id"], secondRequest["id"]);

  io.stdout.write(`${JSON.stringify({
    type: "response",
    id: secondRequest["id"],
    command: "get_entries",
    success: true,
    data: { value: 2 },
  })}\n`);
  io.stdout.write(`${JSON.stringify({ type: "agent_start" })}\n`);
  io.stdout.write(`${JSON.stringify({
    type: "response",
    id: firstRequest["id"],
    command: "get_state",
    success: true,
    data: { value: 1 },
  })}\n`);

  assert.deepEqual(await first, { value: 1 });
  assert.deepEqual(await second, { value: 2 });
  assert.deepEqual(events, [{ type: "agent_start" }]);
  client.close();
});

test("PiRpcClient fails closed on malformed framing and response correlation", async () => {
  const malformedIo = streams();
  const malformed = new PiRpcClient({ streams: malformedIo, requestPrefix: "malformed-rpc" });
  const malformedFailure = new Promise<PiRpcTransportError>((resolve) => malformed.onFailure(resolve));
  malformedIo.stdout.write("{not-json}\n");
  assert.equal((await malformedFailure).kind, "framing");

  const correlationIo = streams();
  const correlation = new PiRpcClient({ streams: correlationIo, requestPrefix: "correlation-rpc" });
  const pending = correlation.request({ type: "get_state" });
  const request = await nextWrittenLine(correlationIo.stdin);
  correlationIo.stdout.write(`${JSON.stringify({
    type: "response",
    id: request["id"],
    command: "get_entries",
    success: true,
    data: {},
  })}\n`);
  await assert.rejects(pending, (error: unknown) =>
    error instanceof PiRpcTransportError && error.kind === "protocol");
});

test("PiRpcClient tracks bounded extension errors, bounded stderr, EOF, and process exit evidence", async () => {
  const io = streams();
  const client = new PiRpcClient({
    streams: io,
    requestPrefix: "evidence-rpc",
    maxStderrBytes: 1_024,
  });
  io.stdout.write(`${JSON.stringify({ type: "extension_error", message: "safe" })}\n`);
  io.stderr.write("x".repeat(2_048));
  await new Promise<void>((resolve) => setImmediate(resolve));
  assert.equal(client.extensionErrors.length, 1);
  assert.equal(Buffer.byteLength(client.stderrSnapshot()), 1_024);

  const failure = new Promise<PiRpcTransportError>((resolve) => client.onFailure(resolve));
  client.markProcessExit({ code: 9, signal: null, expected: false });
  const error = await failure;
  assert.equal(error.kind, "process_exit");
  assert.deepEqual(error.processExit, { code: 9, signal: null, expected: false });
});

test("PiRpcClient proves pre-write rejection and marks every post-write response-loss boundary ambiguous", async () => {
  const preWriteIo = streams();
  preWriteIo.stdin.write = (() => {
    throw new Error("injected synchronous pre-write refusal");
  }) as typeof preWriteIo.stdin.write;
  const preWrite = new PiRpcClient({
    streams: preWriteIo,
    requestPrefix: "prewrite-rpc",
  });
  await assert.rejects(
    preWrite.request({ type: "prompt", message: "not written" }),
    (error: unknown) => error instanceof PiRpcTransportError &&
      error.kind === "pipe" && error.requestEffect === "proved_not_written",
  );
  preWrite.close();

  const cases = ["malformed", "exit", "eof", "timeout"] as const;
  for (const boundary of cases) {
    const io = streams();
    const client = new PiRpcClient({
      streams: io,
      requestPrefix: `${boundary}-loss-rpc`,
      requestTimeoutMs: boundary === "timeout" ? 5 : 1_000,
    });
    const pending = client.request({ type: "prompt", message: boundary });
    await nextWrittenLine(io.stdin);
    if (boundary === "malformed") io.stdout.write("{malformed}\n");
    if (boundary === "exit") client.markProcessExit({ code: 9, signal: null, expected: false });
    if (boundary === "eof") io.stdout.end();
    await assert.rejects(
      pending,
      (error: unknown) => error instanceof PiRpcTransportError &&
        error.requestEffect === "possibly_written" &&
        (boundary !== "timeout" || error.kind === "timeout"),
      `${boundary} after write must preserve execution ambiguity`,
    );
    client.close();
  }
});

test("PiRpcClient rejects EOF with an unterminated line", async () => {
  const io = streams();
  const client = new PiRpcClient({ streams: io, requestPrefix: "unterminated-rpc" });
  const failure = new Promise<PiRpcTransportError>((resolve) => client.onFailure(resolve));
  io.stdout.write("{}");
  io.stdout.end();
  assert.equal((await failure).kind, "framing");
});
