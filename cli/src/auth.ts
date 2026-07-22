import type { Interceptor } from "@connectrpc/connect";
import type { CliCredentials, CredentialReader } from "./credentials.js";

export const AUTH_HEADERS = {
  principalId: "x-patchbay-principal-id",
  principalSecret: "x-patchbay-principal-secret",
  operatorId: "x-patchbay-operator-id",
  operatorSessionId: "x-patchbay-operator-session-id",
} as const;

export function authInterceptor(credentials: CredentialReader): Interceptor {
  return (next) => async (request) => {
    applyAuthHeaders(request.header, await credentials.readRequired());
    return next(request);
  };
}

export function applyAuthHeaders(headers: Headers, credentials: CliCredentials): void {
  headers.set(AUTH_HEADERS.principalId, credentials.principal.principalId);
  headers.set(AUTH_HEADERS.principalSecret, credentials.principal.secret);
  headers.set(AUTH_HEADERS.operatorId, credentials.operatorActorId);
  headers.set(AUTH_HEADERS.operatorSessionId, credentials.sessionId);
}
