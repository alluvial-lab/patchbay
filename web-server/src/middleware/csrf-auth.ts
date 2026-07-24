import { timingSafeEqual } from "node:crypto";
import type { FastifyReply, FastifyRequest, preHandlerAsyncHookHandler } from "fastify";

import { type OperatorSession, SessionStore } from "../sessions.js";

export const SESSION_COOKIE_NAME = "__Host-patchbay_session";
export const CSRF_HEADER_NAME = "x-patchbay-csrf";

export interface GuardOptions {
  requireCsrf?: boolean;
  trustedLoopbackProxy?: boolean;
}

export function requireOperatorSession(
  sessions: SessionStore,
  options: GuardOptions = {},
): preHandlerAsyncHookHandler {
  const requireCsrf = options.requireCsrf ?? true;

  return async (request, reply) => {
    if (!isSecureSessionRequest(request, options)) {
      await reply.code(400).send({ error: "https_required" });
      return;
    }

    const sessionId = request.cookies[SESSION_COOKIE_NAME];
    const session = sessionId ? sessions.lookup(sessionId) : null;
    if (!session) {
      await reply.code(401).send({ error: "unauthenticated" });
      return;
    }
    if (session.status !== "active") {
      await reply.code(403).send({ error: `session_${session.status}` });
      return;
    }

    if (requireCsrf) {
      if (request.headers["sec-fetch-site"] === "cross-site") {
        await reply.code(403).send({ error: "cross_site_request" });
        return;
      }
      const proof = request.headers[CSRF_HEADER_NAME];
      if (typeof proof !== "string" || !safeTokenEqual(proof, session.csrfSecret)) {
        await reply.code(403).send({ error: "csrf_proof_missing_or_invalid" });
        return;
      }
    }

    setVerifiedSession(request, session);
  };
}

export function isSecureSessionRequest(
  request: FastifyRequest,
  options: Pick<GuardOptions, "trustedLoopbackProxy"> = {},
): boolean {
  const socket = request.raw.socket as typeof request.raw.socket & { encrypted?: boolean };
  if (socket.encrypted === true) return true;
  if (!isLoopbackAddress(socket.remoteAddress)) return false;

  const forwardedScheme = request.headers["x-forwarded-proto"];
  if (forwardedScheme === undefined) return options.trustedLoopbackProxy !== true;
  return options.trustedLoopbackProxy === true && isHttpsForwardedScheme(forwardedScheme);
}

function isLoopbackAddress(address: string | undefined): boolean {
  return address === "127.0.0.1" || address === "::1" || address === "::ffff:127.0.0.1";
}

function isHttpsForwardedScheme(value: string | string[]): boolean {
  return typeof value === "string" && value.trim().toLowerCase() === "https";
}

function safeTokenEqual(actual: string, expected: string): boolean {
  const actualBytes = Buffer.from(actual, "utf8");
  const expectedBytes = Buffer.from(expected, "utf8");
  if (actualBytes.length !== expectedBytes.length) {
    timingSafeEqual(expectedBytes, expectedBytes);
    return false;
  }
  return timingSafeEqual(actualBytes, expectedBytes);
}

function setVerifiedSession(request: FastifyRequest, session: OperatorSession): void {
  request.verifiedOperator = session.operatorActorId;
  request.verifiedSessionId = session.sessionId;
  request.verifiedCoreSessionId = session.coreSessionId;
}

declare module "fastify" {
  interface FastifyRequest {
    verifiedOperator?: string;
    verifiedSessionId?: string;
    verifiedCoreSessionId?: string;
  }
}
