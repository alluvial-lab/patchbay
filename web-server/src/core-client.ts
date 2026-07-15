import { createClient, type Client, type Interceptor } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import { ControlService } from "@patchbay/contracts";

export type CoreClient = Client<typeof ControlService>;

export function makeCoreClient(coreAddr: string, coreSecret: string): CoreClient {
  if (coreSecret.length === 0) {
    throw new Error("PATCHBAY_CORE_SECRET must be configured and non-empty");
  }

  const authenticateCorePrincipal: Interceptor = (next) => async (request) => {
    request.header.set("x-patchbay-core-secret", coreSecret);
    return next(request);
  };

  const transport = createGrpcTransport({
    baseUrl: coreAddr,
    interceptors: [authenticateCorePrincipal],
  });
  return createClient(ControlService, transport);
}
