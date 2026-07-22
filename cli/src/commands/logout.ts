import type { ControlClient } from "../core-client.js";
import type { CredentialStore } from "../credentials.js";
import type { CliOutput } from "../main.js";

export async function logoutCommand(
  client: Pick<ControlClient, "revokeOperatorSession">,
  store: CredentialStore,
  output: CliOutput,
): Promise<number> {
  const result = await client.revokeOperatorSession({});
  if (!result.revoked) throw new Error("core did not revoke the current operator session");
  await store.clear();
  output.stdout("Current operator session revoked; local credentials removed.");
  return 0;
}
