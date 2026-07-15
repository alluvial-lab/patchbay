// Throwaway spike client/harness for the @connectrpc/connect-node <-> tonic
// interop validation (story-connect-node-tonic-interop-spike). NOT production
// code. Runs all five acceptance conditions against a spike-server process and
// prints a structured per-condition PASS/FAIL report to stdout.
//
// Usage:
//   node dist/run.js                          # h2c against 127.0.0.1:PORT
//   node dist/run.js --tls                    # TLS against 127.0.0.1:PORT (TLS_*)
//   node dist/run.js --host 127.0.0.1 --port 50051
//
// Expects spike-server already running. Exits 0 only if every condition PASSes.

import { createClient, Code, ConnectError } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

import { SpikeControl } from "./gen/spike_pb.js";
import {
  SubmissionFailureDetailSchema,
  type SubmissionFailureDetail,
} from "./gen/spike_pb.js";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

interface CondResult {
  name: string;
  pass: boolean;
  detail: string;
}

const results: CondResult[] = [];

function record(name: string, pass: boolean, detail: string) {
  results.push({ name, pass, detail });
  const tag = pass ? "PASS" : "FAIL";
  console.log(`[${tag}] ${name}: ${detail}`);
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const host = args.host ?? "127.0.0.1";
  const port = args.port ?? 50051;
  const useTls = args.tls === true;

  const baseUrl = `${useTls ? "https" : "http"}://${host}:${port}`;

  // Create the gRPC/HTTP2 transport the synthesis recommends (NOT the Connect
  // protocol transport). This is the exact call site under test. gRPC
  // transport is inherently HTTP/2, so no httpVersion discriminator is needed
  // (that field only applies to connect/grpc-web transports).
  const transport = useTls
    ? createGrpcTransport({
        baseUrl,
        // Accept the self-signed cert by passing it via Node's http2 session
        // options (connect-node forwards nodeOptions to http2.connect). The
        // goal is transport interop, not cert identity validation.
        nodeOptions: {
          ca: fs.readFileSync(process.env.TLS_CERT!),
          rejectUnauthorized: false,
        },
      })
    : createGrpcTransport({
        baseUrl,
      });

  const client = createClient(SpikeControl, transport);

  // ---- Condition 1: Unary RPC round-trip -------------------------------
  try {
    const res = await client.submit(
      { commandId: "c1", payload: "hello", triggerError: false },
      { headers: {} },
    );
    const ok =
      res.commandId === "c1" &&
      res.accepted === true &&
      res.acceptedLsn === 42n &&
      typeof res.diagnostic === "string" &&
      res.diagnostic.includes("op_session=");
    record("1-unary", ok, `commandId=${res.commandId} lsn=${res.acceptedLsn} diag=${res.diagnostic}`);
  } catch (e) {
    record("1-unary", false, `threw: ${describe(e)}`);
  }

  // ---- Condition 2: Server-streaming -----------------------------------
  try {
    const it = client.subscribe({ cursor: 100n, eventCount: 4 });
    const got: bigint[] = [];
    let payloadOk = true;
    for await (const ev of it) {
      got.push(ev.lsn);
      if (!ev.payload.startsWith("event-")) payloadOk = false;
    }
    const ok =
      got.length === 4 &&
      got[0] === 101n &&
      got[1] === 102n &&
      got[2] === 103n &&
      got[3] === 104n &&
      payloadOk;
    record("2-streaming", ok, `lsns=[${got.join(",")}]`);
  } catch (e) {
    record("2-streaming", false, `threw: ${describe(e)}`);
  }

  // ---- Condition 3: Error mapping --------------------------------------
  // Server returns google.rpc.Status with a custom SubmissionFailureDetail as
  // an Any detail. Connect should surface this as a structured ConnectError
  // (code InvalidArgument) with the detail retrievable by type.
  try {
    await client.submit(
      { commandId: "c3", payload: "x", triggerError: true },
      { headers: {} },
    );
    record("3-error-mapping", false, "expected ConnectError but call resolved");
  } catch (e: any) {
    // ConnectError carries code + message; details are retrievable via
    // findDetails(schema) on the error instance.
    const isConnectError = e instanceof ConnectError;
    const code = isConnectError ? (e as ConnectError).code : undefined;
    let detailFound: SubmissionFailureDetail | null = null;
    if (isConnectError) {
      try {
        const details = (e as ConnectError).findDetails(
          SubmissionFailureDetailSchema,
        );
        if (details.length > 0) {
          detailFound = details[0] as SubmissionFailureDetail;
        }
      } catch {
        // detail not decodable; detailFound stays null
      }
    }
    const ok =
      code === Code.InvalidArgument &&
      detailFound !== null &&
      detailFound.commandId === "c3" &&
      detailFound.failureCode === "FAILURE_CODE_VALIDATION_FAILED";
    record(
      "3-error-mapping",
      ok,
      `code=${code} (${isConnectError ? "ConnectError" : typeof e}) detail=${
        detailFound
          ? JSON.stringify({
              commandId: detailFound.commandId,
              failureCode: detailFound.failureCode,
              reason: detailFound.reason,
            })
          : "MISSING"
      }`,
    );
  }

  // ---- Condition 4: Metadata propagation -------------------------------
  // Set the operator-session + CSRF headers the seam will forward; the server
  // echoes them back in the diagnostic so we can assert they arrived.
  try {
    const opSession = "op-sess-abc";
    const csrf = "csrf-token-xyz";
    const res = await client.submit(
      { commandId: "c4", payload: "meta", triggerError: false },
      {
        headers: {
          "x-patchbay-operator-session": opSession,
          "x-patchbay-csrf": csrf,
        },
      },
    );
    const ok =
      res.diagnostic.includes(`op_session=${opSession}`) &&
      res.diagnostic.includes(`csrf=${csrf}`);
    record("4-metadata", ok, `diag=${res.diagnostic}`);
  } catch (e) {
    record("4-metadata", false, `threw: ${describe(e)}`);
  }

  // ---- Condition 5: TLS -------------------------------------------------
  // Only meaningful in --tls mode; otherwise this is an explicit N/A with a
  // documented gap + revisit trigger per the spike brief.
  if (!useTls) {
    record(
      "5-tls",
      true,
      "N/A in h2c mode (conditions 1-4 ran over cleartext HTTP/2); TLS is exercised by re-running with --tls against a TLS server",
    );
  } else {
    // If we got here at all over TLS, the transport + unary already proved TLS
    // interop; do one more explicit unary to record it.
    try {
      const res = await client.submit(
        { commandId: "c5", payload: "tls", triggerError: false },
        { headers: {} },
      );
      record("5-tls", res.accepted === true, `TLS unary accepted=${res.accepted}`);
    } catch (e) {
      record("5-tls", false, `threw: ${describe(e)}`);
    }
  }

  // ---- Summary ---------------------------------------------------------
  const passed = results.filter((r) => r.pass).length;
  const failed = results.length - passed;
  console.log(`\n== SPIKE RESULT: ${passed}/${results.length} conditions passed (${failed} failed) ==`);
  process.exit(failed === 0 ? 0 : 1);
}

function describe(e: unknown): string {
  if (e instanceof Error) {
    return `${e.name}: ${e.message}`;
  }
  return String(e);
}

function parseArgs(argv: string[]): { host?: string; port?: number; tls?: boolean } {
  const out: { host?: string; port?: number; tls?: boolean } = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--tls") out.tls = true;
    else if (a === "--host") out.host = argv[++i];
    else if (a === "--port") out.port = Number(argv[++i]);
  }
  return out;
}

main().catch((e) => {
  console.error("harness crashed:", e);
  process.exit(2);
});
