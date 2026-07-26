import { timingSafeEqual } from "node:crypto";
import type { FastifyReply, FastifyRequest, preHandlerAsyncHookHandler } from "fastify";

import { type OperatorSession, SessionStore } from "../sessions.js";

export const SESSION_COOKIE_NAME = "__Host-patchbay_session";
export const CSRF_HEADER_NAME = "x-patchbay-csrf";

export interface GuardOptions {
  requireCsrf?: boolean;
  trustedLoopbackProxy?: boolean;
  renewOnAuthenticated?: boolean;
  onIntegrityFailure?: (
    request: FastifyRequest,
    kind: "csrf_check_failed" | "origin_check_failed" | "fetch_metadata_check_failed",
    reasonCode: string,
  ) => Promise<void>;
  onSessionLifecycle?: (
    request: FastifyRequest,
    kind: "operator_session_renewed" | "operator_session_expired",
    reasonCode: string,
  ) => Promise<void>;
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
    // Keep verified attribution available for lifecycle failures too; this
    // value comes from the server-side session record, never the request.
    setVerifiedSession(request, session);
    if (session.status !== "active") {
      if (session.status === "expired") {
        await reportSessionLifecycle(options, request, "operator_session_expired", "session_ttl_expired");
      }
      await reply.code(403).send({ error: `session_${session.status}` });
      return;
    }
    // The active session is already the verified compound issuer. Integrity
    // failures remain durably attributable without trusting browser claims.
    if (options.renewOnAuthenticated) {
      await reportSessionLifecycle(options, request, "operator_session_renewed", "session_renewed");
    }
    if (requireCsrf) {
      const origin = request.headers.origin;
      if (origin !== undefined && !sameRequestOrigin(request, origin, options)) {
        await reportIntegrityFailure(options, request, "origin_check_failed", "origin_mismatch");
        await reply.code(403).send({ error: "origin_mismatch" });
        return;
      }
      if (request.headers["sec-fetch-site"] === "cross-site") {
        await reportIntegrityFailure(
          options,
          request,
          "fetch_metadata_check_failed",
          "cross_site_request",
        );
        await reply.code(403).send({ error: "cross_site_request" });
        return;
      }
      const proof = request.headers[CSRF_HEADER_NAME];
      if (typeof proof !== "string" || !safeTokenEqual(proof, session.csrfSecret)) {
        await reportIntegrityFailure(
          options,
          request,
          "csrf_check_failed",
          "csrf_proof_missing_or_invalid",
        );
        await reply.code(403).send({ error: "csrf_proof_missing_or_invalid" });
        return;
      }
    }
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

async function reportSessionLifecycle(
  options: GuardOptions,
  request: FastifyRequest,
  kind: "operator_session_renewed" | "operator_session_expired",
  reasonCode: string,
): Promise<void> {
  try {
    await options.onSessionLifecycle?.(request, kind, reasonCode);
  } catch {
    // Session enforcement remains local and fail-closed if the core is down.
  }
}

async function reportIntegrityFailure(
  options: GuardOptions,
  request: FastifyRequest,
  kind: "csrf_check_failed" | "origin_check_failed" | "fetch_metadata_check_failed",
  reasonCode: string,
): Promise<void> {
  try {
    await options.onIntegrityFailure?.(request, kind, reasonCode);
  } catch {
    // The request remains rejected. Core unavailability must not turn a
    // failed browser-integrity check into an accepted mutation.
  }
}

function sameRequestOrigin(
  request: FastifyRequest,
  origin: string,
  options: Pick<GuardOptions, "trustedLoopbackProxy">,
): boolean {
  try {
    const parsed = new URL(origin);
    const forwarded = request.headers["x-forwarded-proto"];
    const socket = request.raw.socket as typeof request.raw.socket & { encrypted?: boolean };
    const scheme = socket.encrypted === true
      ? "https"
      : options.trustedLoopbackProxy === true && forwarded === "https"
        ? "https"
        : "http";
    return parsed.protocol === `${scheme}:` && parsed.host === request.headers.host;
  } catch {
    return false;
  }
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
