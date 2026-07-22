import type { CliOutput } from "../main.js";

export function inspectCommandCommand(output: CliOutput): number {
  output.stderr("requires core-diagnostics (not yet implemented); see feature-v0-cli Unit 3b");
  return 1;
}
