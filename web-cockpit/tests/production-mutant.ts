import assert from "node:assert/strict";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";

export interface ProductionReplacement { file: string; from: string; to: string }

export async function withProductionMutant<T>(
  packageRoot: string,
  replacements: readonly ProductionReplacement[],
  entry: string,
  run: (module: Record<string, any>) => Promise<T> | T,
): Promise<T> {
  const directory = mkdtempSync(path.join(tmpdir(), "patchbay-web-production-mutant-"));
  const mutantRoot = path.join(directory, "src");
  cpSync(path.resolve(packageRoot, "dist/src"), mutantRoot, { recursive: true });
  try {
    for (const replacement of replacements) {
      const file = path.join(mutantRoot, replacement.file);
      const source = readFileSync(file, "utf8");
      const first = source.indexOf(replacement.from);
      assert.notEqual(first, -1, `production mutation anchor missing in ${replacement.file}`);
      assert.equal(source.indexOf(replacement.from, first + replacement.from.length), -1, `production mutation anchor is not unique in ${replacement.file}`);
      writeFileSync(file, source.slice(0, first) + replacement.to + source.slice(first + replacement.from.length));
    }
    return await run(await import(`${pathToFileURL(path.join(mutantRoot, entry)).href}?mutant=${Date.now()}-${Math.random()}`));
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}
