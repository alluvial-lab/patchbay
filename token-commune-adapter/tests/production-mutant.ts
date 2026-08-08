import assert from "node:assert/strict";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

export interface ProductionReplacement {
  file: string;
  from: string;
  to: string;
}

export class ProductionMutantHarnessError extends Error {
  constructor(
    readonly phase: "setup" | "load" | "cleanup",
    message: string,
    options?: ErrorOptions,
  ) {
    super(message, options);
    this.name = "ProductionMutantHarnessError";
  }
}

/**
 * Loads a byte-for-byte copy of the compiled production module graph after
 * applying one claim-breaking source replacement. The caller drives the same
 * input and oracle through baseline production first, then this mutant graph.
 */
export async function withProductionMutant<T>(
  replacements: readonly ProductionReplacement[],
  entry: string,
  run: (module: Record<string, any>) => Promise<T> | T,
): Promise<T> {
  const packageRoot = process.cwd();
  let directory: string | undefined;
  try {
    // Keep the copied graph below the package root so bare imports resolve
    // through this package's real node_modules ancestry.
    directory = mkdtempSync(path.join(packageRoot, ".patchbay-token-production-mutant-"));
    const mutantRoot = path.join(directory, "src");
    cpSync(path.join(packageRoot, "dist/src"), mutantRoot, { recursive: true });
    cpSync(path.join(packageRoot, "schemas"), path.join(directory, "schemas"), { recursive: true });
    for (const replacement of replacements) {
      const file = path.join(mutantRoot, replacement.file);
      const source = readFileSync(file, "utf8");
      const first = source.indexOf(replacement.from);
      assert.notEqual(first, -1, `production mutation anchor missing in ${replacement.file}`);
      assert.equal(source.indexOf(replacement.from, first + replacement.from.length), -1, `production mutation anchor is not unique in ${replacement.file}`);
      writeFileSync(file, source.slice(0, first) + replacement.to + source.slice(first + replacement.from.length));
    }
  } catch (error) {
    if (directory !== undefined) {
      try { rmSync(directory, { recursive: true, force: false }); }
      catch (cleanupError) {
        throw new ProductionMutantHarnessError("cleanup", "failed to clean up an incomplete production mutant graph", { cause: cleanupError });
      }
    }
    throw new ProductionMutantHarnessError("setup", "production mutant graph setup failed", { cause: error });
  }

  assert.ok(directory, "production mutant directory was not created");
  const mutantRoot = path.join(directory, "src");
  let loaded: Record<string, any>;
  try {
    loaded = await import(`${pathToFileURL(path.join(mutantRoot, entry)).href}?mutant=${Date.now()}-${Math.random()}`);
  } catch (error) {
    try { rmSync(directory, { recursive: true, force: false }); }
    catch (cleanupError) {
      throw new ProductionMutantHarnessError("cleanup", "failed to clean up an unloadable production mutant graph", { cause: cleanupError });
    }
    throw new ProductionMutantHarnessError("load", "production mutant module failed to load", { cause: error });
  }

  let result: T | undefined;
  let oracleFailed = false;
  let oracleError: unknown;
  try {
    result = await run(loaded);
  } catch (error) {
    oracleFailed = true;
    oracleError = error;
  }
  try {
    rmSync(directory, { recursive: true, force: false });
  } catch (error) {
    throw new ProductionMutantHarnessError("cleanup", "production mutant graph cleanup failed", { cause: error });
  }
  if (oracleFailed) throw oracleError;
  return result as T;
}
