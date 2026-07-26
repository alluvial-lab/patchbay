import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import { timestampFromMs } from "@bufbuild/protobuf/wkt";
import { Code, ConnectError } from "@connectrpc/connect";
import {
  ActorEndpointRefSchema,
  AuditEventKind,
  LoadSnapshotRequestSchema,
  LoadSnapshotResponseSchema,
  QueryDiagnosticsRequestSchema,
  QueryDiagnosticsResponseSchema,
  RecordControlSurfaceAuditRequestSchema,
  SubmissionResultSchema,
  SubmitRequestSchema,
  SubscribeEventSchema,
  SubscribeRequestSchema,
  TimeWindowSchema,
} from "@patchbay/contracts";
import type { FastifyInstance, FastifyReply, FastifyRequest } from "fastify";
import type { Operation } from "@patchbay/contracts";

import { type GuardOptions, requireOperatorSession } from "../middleware/csrf-auth.js";

const GRPC_WEB_CONTENT_TYPE = "application/grpc-web+proto";
const DEFAULT_OPERATION_VALIDITY_MS = 5 * 60 * 1_000;

export function registerRpcRoutes(
  app: FastifyInstance,
  coreSecret: string,
  options: Pick<GuardOptions, "trustedLoopbackProxy" | "onSessionLifecycle"> = {},
): void {
  app.addContentTypeParser(GRPC_WEB_CONTENT_TYPE, { parseAs: "buffer" }, (_request, body, done) => {
    done(null, body);
  });

  const guardOptions: GuardOptions = {
    ...options,
    onIntegrityFailure: async (request, kind, reasonCode) => {
      const kindValue = {
        csrf_check_failed: AuditEventKind.CSRF_CHECK_FAILED,
        origin_check_failed: AuditEventKind.ORIGIN_CHECK_FAILED,
        fetch_metadata_check_failed: AuditEventKind.FETCH_METADATA_CHECK_FAILED,
      }[kind];
      await app.coreClient.recordControlSurfaceAudit(
        create(RecordControlSurfaceAuditRequestSchema, {
          kind: kindValue,
          reasonCode,
        }),
        { headers: coreHeaders(app, request, coreSecret) },
      );
    },
  };

  app.post(
    "/patchbay.ControlService/Submit",
    { preHandler: requireOperatorSession(app.sessions, guardOptions) },
    async (request, reply) => {
      try {
        const input = fromBinary(SubmitRequestSchema, decodeRequestFrame(request.body));
        stampVerifiedOperation(input.operation, request);
        const output = await app.coreClient.submit(input, {
          headers: coreHeaders(app, request, coreSecret),
        });
        return sendUnary(reply, toBinary(SubmissionResultSchema, output));
      } catch (error) {
        return sendRpcError(app, request, reply, error);
      }
    },
  );

  app.post(
    "/patchbay.ControlService/QueryDiagnostics",
    { preHandler: requireOperatorSession(app.sessions, guardOptions) },
    async (request, reply) => {
      try {
        const input = fromBinary(QueryDiagnosticsRequestSchema, decodeRequestFrame(request.body));
        stampVerifiedOperation(input.operation, request);
        const output = await app.coreClient.queryDiagnostics(input, {
          headers: coreHeaders(app, request, coreSecret),
        });
        return sendUnary(reply, toBinary(QueryDiagnosticsResponseSchema, output));
      } catch (error) {
        return sendRpcError(app, request, reply, error);
      }
    },
  );

  app.post(
    "/patchbay.ControlService/LoadSnapshot",
    { preHandler: requireOperatorSession(app.sessions, { ...guardOptions, requireCsrf: false }) },
    async (request, reply) => {
      try {
        const input = fromBinary(LoadSnapshotRequestSchema, decodeRequestFrame(request.body));
        const output = await app.coreClient.loadSnapshot(input, {
          headers: coreHeaders(app, request, coreSecret),
        });
        return sendUnary(reply, toBinary(LoadSnapshotResponseSchema, output));
      } catch (error) {
        return sendRpcError(app, request, reply, error);
      }
    },
  );

  app.post(
    "/patchbay.ControlService/Subscribe",
    { preHandler: requireOperatorSession(app.sessions, { ...guardOptions, requireCsrf: false }) },
    async (request, reply) => {
      let input;
      try {
        input = fromBinary(SubscribeRequestSchema, decodeRequestFrame(request.body));
      } catch (error) {
        return sendRpcError(app, request, reply, error);
      }

      reply.hijack();
      reply.raw.statusCode = 200;
      reply.raw.setHeader("content-type", GRPC_WEB_CONTENT_TYPE);
      reply.raw.setHeader("x-content-type-options", "nosniff");
      reply.raw.flushHeaders();
      try {
        const stream = app.coreClient.subscribe(input, {
          headers: coreHeaders(app, request, coreSecret),
        });
        for await (const event of stream) {
          await writeFrame(reply, dataFrame(toBinary(SubscribeEventSchema, event)));
        }
        reply.raw.end(trailerFrame(0, ""));
      } catch (error) {
        const rpcError = ConnectError.from(error, Code.Internal);
        invalidateDeadCoreSession(app, request, rpcError);
        reply.raw.end(trailerFrame(rpcError.code, rpcError.rawMessage));
      }
    },
  );
}

function stampVerifiedOperation(
  operation: Operation | undefined,
  request: FastifyRequest,
  nowMs = Date.now(),
): void {
  if (!operation) return;
  // Browser sender and time are untrusted. Both lifecycle RPCs share the same
  // verified compound-issuer stamping boundary.
  operation.sender = create(ActorEndpointRefSchema, {
    actorId: { value: verified(request.verifiedOperator, "operator actor") },
  });
  const submittedAt = timestampFromMs(nowMs);
  operation.submittedAt = submittedAt;
  operation.validityWindow = create(TimeWindowSchema, {
    startsAt: submittedAt,
    expiresAt: timestampFromMs(nowMs + DEFAULT_OPERATION_VALIDITY_MS),
  });
}

function decodeRequestFrame(body: unknown): Uint8Array {
  if (!Buffer.isBuffer(body) || body.length < 5) {
    throw new ConnectError("invalid gRPC-Web request frame", Code.InvalidArgument);
  }
  const flags = body[0];
  const length = body.readUInt32BE(1);
  if (flags !== 0 || length !== body.length - 5) {
    throw new ConnectError("unsupported gRPC-Web request frame", Code.InvalidArgument);
  }
  return body.subarray(5);
}

function coreHeaders(
  app: FastifyInstance,
  request: FastifyRequest,
  coreSecret: string,
): Headers {
  const principal = app.corePrincipals.get();
  if (!principal) {
    throw new ConnectError("control-surface principal enrollment is required", Code.Unauthenticated);
  }
  const headers = new Headers();
  headers.set("x-patchbay-core-secret", coreSecret);
  headers.set("x-patchbay-principal-id", principal.principalId);
  headers.set("x-patchbay-principal-secret", principal.secret);
  headers.set("x-patchbay-operator-id", verified(request.verifiedOperator, "operator actor"));
  headers.set(
    "x-patchbay-operator-session-id",
    verified(request.verifiedCoreSessionId, "core operator session"),
  );
  return headers;
}

function verified(value: string | undefined, description: string): string {
  if (!value) throw new ConnectError(`missing verified ${description}`, Code.Internal);
  return value;
}

function sendUnary(reply: FastifyReply, protobuf: Uint8Array): FastifyReply {
  return reply
    .code(200)
    .header("content-type", GRPC_WEB_CONTENT_TYPE)
    .header("x-content-type-options", "nosniff")
    .send(Buffer.concat([dataFrame(protobuf), trailerFrame(0, "")]));
}

function sendRpcError(
  app: FastifyInstance,
  request: FastifyRequest,
  reply: FastifyReply,
  error: unknown,
): FastifyReply {
  const rpcError = ConnectError.from(error, Code.Internal);
  invalidateDeadCoreSession(app, request, rpcError);
  return reply
    .code(200)
    .header("content-type", GRPC_WEB_CONTENT_TYPE)
    .header("x-content-type-options", "nosniff")
    .send(trailerFrame(rpcError.code, rpcError.rawMessage));
}

function invalidateDeadCoreSession(
  app: FastifyInstance,
  request: FastifyRequest,
  error: ConnectError,
): void {
  if (error.code === Code.Unauthenticated && request.verifiedSessionId) {
    app.sessions.invalidate(request.verifiedSessionId);
  }
}

function dataFrame(payload: Uint8Array): Buffer {
  return frame(0, payload);
}

function trailerFrame(code: number, message: string): Buffer {
  const trailers = Buffer.from(
    `grpc-status: ${code}\r\ngrpc-message: ${encodeURIComponent(message)}\r\n`,
    "ascii",
  );
  return frame(0x80, trailers);
}

function frame(flags: number, payload: Uint8Array): Buffer {
  const header = Buffer.allocUnsafe(5);
  header[0] = flags;
  header.writeUInt32BE(payload.byteLength, 1);
  return Buffer.concat([header, payload]);
}

async function writeFrame(reply: FastifyReply, payload: Buffer): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    reply.raw.write(payload, (error) => (error ? reject(error) : resolve()));
  });
}
