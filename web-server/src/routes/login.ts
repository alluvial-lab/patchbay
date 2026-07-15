import type { FastifyInstance, FastifyRequest } from "fastify";

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

export function registerSessionRoutes(
  app: FastifyInstance,
  sessions: SessionStore,
  operator: OperatorRecord,
): void {
  app.post("/login", async (request, reply) => {
    if (!isSecureSessionRequest(request)) {
      return reply.code(400).send({ error: "https_required" });
    }

    const password = readPassword(request);
    if (password === null) {
      return reply.code(400).send({ error: "password_required" });
    }
    if (!(await verifyPassword(password, operator.passwordHash))) {
      return reply.code(401).send({ error: "invalid_credentials" });
    }

    const session = sessions.create(operator.actorId);
    reply.setCookie(SESSION_COOKIE_NAME, session.sessionId, SESSION_COOKIE_OPTIONS);
    return { csrfToken: session.csrfSecret };
  });

  app.post(
    "/logout",
    { preHandler: requireOperatorSession(sessions) },
    async (request, reply) => {
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
