import type { FastifyInstance, FastifyRequest } from "fastify";

import { LoginLimiter } from "../login-limiter.js";
import {
  isSecureSessionRequest,
  requireOperatorSession,
  SESSION_COOKIE_NAME,
} from "../middleware/csrf-auth.js";
import {
  type OperatorRecord,
  SessionStore,
  verifyPassword,
} from "../sessions.js";

const SESSION_COOKIE_OPTIONS = {
  path: "/",
  secure: true,
  httpOnly: true,
  sameSite: "strict" as const,
};

export type PasswordVerifier = (password: string, passwordHash: string) => Promise<boolean>;
export interface OperatorAuthentication {
  actorId: string;
  coreSessionId?: string;
}
export type OperatorAuthenticator = (password: string) => Promise<OperatorAuthentication | null>;
export type OperatorSessionRevoker = (coreSessionId: string) => Promise<void>;

export interface SessionRouteOptions {
  loginLimiter?: LoginLimiter;
  passwordVerifier?: PasswordVerifier;
  operatorAuthenticator?: OperatorAuthenticator;
  operatorSessionRevoker?: OperatorSessionRevoker;
}

export function registerSessionRoutes(
  app: FastifyInstance,
  sessions: SessionStore,
  operator: OperatorRecord,
  options: SessionRouteOptions = {},
): void {
  const limiter = options.loginLimiter ?? new LoginLimiter();
  const passwordVerifier = options.passwordVerifier ?? verifyPassword;
  const operatorAuthenticator = options.operatorAuthenticator;

  app.post("/login", async (request, reply) => {
    const networkAddress = directSocketAddress(request);
    if (!isSecureSessionRequest(request)) {
      auditLogin(request, operator.actorId, networkAddress, "failure", "https_required");
      return reply.code(400).send({ error: "https_required" });
    }

    const password = readPassword(request);
    if (password === null) {
      auditLogin(request, operator.actorId, networkAddress, "failure", "password_required");
      return reply.code(400).send({ error: "password_required" });
    }

    const limit = limiter.beginAttempt(networkAddress);
    if (!limit.allowed) {
      auditLogin(
        request,
        operator.actorId,
        networkAddress,
        "failure",
        "login_throttled",
        limit.blockedDimensions,
      );
      return reply
        .header("retry-after", String(Math.max(1, Math.ceil(limit.retryAfterMs / 1_000))))
        .code(429)
        .send({ error: "login_throttled" });
    }

    let authentication: OperatorAuthentication | null;
    try {
      if (operatorAuthenticator) {
        authentication = await operatorAuthenticator(password);
      } else {
        authentication = (await passwordVerifier(password, operator.passwordHash))
          ? { actorId: operator.actorId }
          : null;
      }
    } catch (error) {
      limiter.recordFailure(networkAddress);
      auditLogin(request, operator.actorId, networkAddress, "failure", "verification_error");
      throw error;
    }

    if (authentication === null) {
      limiter.recordFailure(networkAddress);
      auditLogin(request, operator.actorId, networkAddress, "failure", "invalid_credentials");
      return reply.code(401).send({ error: "invalid_credentials" });
    }

    limiter.recordSuccess(networkAddress);
    const session = sessions.create(authentication.actorId, authentication.coreSessionId);
    auditLogin(request, authentication.actorId, networkAddress, "success", "authenticated");
    reply.setCookie(SESSION_COOKIE_NAME, session.sessionId, SESSION_COOKIE_OPTIONS);
    return { csrfToken: session.csrfSecret };
  });

  app.post(
    "/logout",
    { preHandler: requireOperatorSession(sessions) },
    async (request, reply) => {
      const coreSessionId = request.verifiedCoreSessionId;
      if (coreSessionId && options.operatorSessionRevoker) {
        try {
          await options.operatorSessionRevoker(coreSessionId);
        } catch (error) {
          request.log.warn({ err: error }, "core operator-session revocation failed during logout");
        }
      }
      sessions.revoke(request.verifiedSessionId!);
      reply.clearCookie(SESSION_COOKIE_NAME, SESSION_COOKIE_OPTIONS);
      return { loggedOut: true };
    },
  );
}

function readPassword(request: FastifyRequest): string | null {
  if (typeof request.body !== "object" || request.body === null) return null;
  const password = (request.body as Record<string, unknown>).password;
  return typeof password === "string" && password.length > 0 ? password : null;
}

function directSocketAddress(request: FastifyRequest): string {
  return request.raw.socket.remoteAddress ?? "unknown";
}

function auditLogin(
  request: FastifyRequest,
  operatorActorId: string,
  networkAddress: string,
  outcome: "success" | "failure",
  reason: string,
  blockedDimensions?: readonly ("account" | "network")[],
): void {
  request.log.info(
    {
      audit_event: "interactive_login",
      outcome,
      reason,
      operator_actor_id: operatorActorId,
      direct_socket_address: networkAddress,
      ...(blockedDimensions ? { blocked_dimensions: blockedDimensions } : {}),
    },
    "interactive login audit",
  );
}
