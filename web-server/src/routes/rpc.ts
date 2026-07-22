import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import { Code, ConnectError } from "@connectrpc/connect";
import {
  ActorEndpointRefSchema,
  LoadSnapshotRequestSchema,
  LoadSnapshotResponseSchema,
  SubmissionResultSchema,
  SubmitRequestSchema,
  SubscribeEventSchema,
  SubscribeRequestSchema,
} from "@patchbay/contracts";
import type { FastifyInstance, FastifyReply, FastifyRequest } from "fastify";

import { requireOperatorSession } from "../middleware/csrf-auth.js";

const GRPC_WEB_CONTENT_TYPE = "application/grpc-web+proto";

export function registerRpcRoutes(app: FastifyInstance, coreSecret: string): void {
  app.addContentTypeParser(GRPC_WEB_CONTENT_TYPE, { parseAs: "buffer" }, (_request, body, done) => {
    done(null, body);
  });

  app.post(
    "/patchbay.ControlService/Submit",
    { preHandler: requireOperatorSession(app.sessions) },
    async (request, reply) => {
      try {
        const input = fromBinary(SubmitRequestSchema, decodeRequestFrame(request.body));
        if (input.operation) {
          // The browser's sender claim is audit input, never authority. Replace
          // it with the actor established by the server-side session record.
          input.operation.sender = create(ActorEndpointRefSchema, {
            actorId: { value: verified(request.verifiedOperator, "operator actor") },
          });
        }
        const output = await app.coreClient.submit(input, {
          headers: coreHeaders(app, request, coreSecret),
        });
        return sendUnary(reply, toBinary(SubmissionResultSchema, output));
      } catch (error) {
        return sendRpcError(reply, error);
      }
    },
  );

  app.post(
    "/patchbay.ControlService/LoadSnapshot",
    { preHandler: requireOperatorSession(app.sessions, { requireCsrf: false }) },
    async (request, reply) => {
      try {
        const input = fromBinary(LoadSnapshotRequestSchema, decodeRequestFrame(request.body));
        const output = await app.coreClient.loadSnapshot(input, {
          headers: coreHeaders(app, request, coreSecret),
        });
        return sendUnary(reply, toBinary(LoadSnapshotResponseSchema, output));
      } catch (error) {
        return sendRpcError(reply, error);
      }
    },
  );

  app.post(
    "/patchbay.ControlService/Subscribe",
    { preHandler: requireOperatorSession(app.sessions, { requireCsrf: false }) },
    async (request, reply) => {
      let input;
      try {
        input = fromBinary(SubscribeRequestSchema, decodeRequestFrame(request.body));
      } catch (error) {
        return sendRpcError(reply, error);
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
        reply.raw.end(trailerFrame(rpcError.code, rpcError.rawMessage));
      }
    },
  );
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

function sendRpcError(reply: FastifyReply, error: unknown): FastifyReply {
  const rpcError = ConnectError.from(error, Code.Internal);
  return reply
    .code(200)
    .header("content-type", GRPC_WEB_CONTENT_TYPE)
    .header("x-content-type-options", "nosniff")
    .send(trailerFrame(rpcError.code, rpcError.rawMessage));
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
