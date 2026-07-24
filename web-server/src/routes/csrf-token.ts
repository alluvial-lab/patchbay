import type { FastifyInstance } from "fastify";

import { type GuardOptions, requireOperatorSession } from "../middleware/csrf-auth.js";

export function registerCsrfTokenRoute(
  app: FastifyInstance,
  options: Pick<GuardOptions, "trustedLoopbackProxy"> = {},
): void {
  app.get(
    "/csrf-token",
    { preHandler: requireOperatorSession(app.sessions, { ...options, requireCsrf: false }) },
    async (request, reply) => {
      const session = app.sessions.lookup(request.verifiedSessionId!);
      if (!session || session.status !== "active") {
        return reply.code(403).send({ error: "session_not_active" });
      }
      return { csrfToken: session.csrfSecret };
    },
  );
}
