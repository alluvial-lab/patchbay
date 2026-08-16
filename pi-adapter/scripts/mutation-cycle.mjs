import { readFile, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";

const root = resolve(import.meta.dirname, "..");
const mutations = [
  {
    name: "accept fresh generation other than exact claimed one",
    file: "src/spawn_supervisor.ts",
    find: "claim.claimedGeneration.value !== 1n",
    replace: "claim.claimedGeneration.value === 1n",
    test: "fresh generation two",
    testFile: "dist/tests/spawn_supervisor.test.js",
  },
  {
    name: "invoke process launch before durable launch-attempt phase",
    file: "src/spawn_supervisor.ts",
    find: `      const launchSpec = await this.#launchSpec(validated.target, launchNonce, resumeSelector);\n      await this.#journal.recordPhase({\n        claimOperationId,\n        phase: SpawnExecutionPhase.LAUNCH_ATTEMPTED,\n        externalEffectDisposition: ExternalEffectDisposition.MAY_EXIST,\n        recordedAt: new Date().toISOString(),\n        poisoned: true,\n      });\n      lastPhase = SpawnExecutionPhase.LAUNCH_ATTEMPTED;\n      lastPhaseHasNoSuccessorProof = false;\n      launchAttempted = true;\n      launched = await this.#runtimePort.launch(launchSpec);`,
    replace: `      const launchSpec = await this.#launchSpec(validated.target, launchNonce, resumeSelector);\n      launchAttempted = true;\n      launched = await this.#runtimePort.launch(launchSpec);\n      await this.#journal.recordPhase({\n        claimOperationId,\n        phase: SpawnExecutionPhase.LAUNCH_ATTEMPTED,\n        externalEffectDisposition: ExternalEffectDisposition.MAY_EXIST,\n        recordedAt: new Date().toISOString(),\n        poisoned: true,\n      });\n      lastPhase = SpawnExecutionPhase.LAUNCH_ATTEMPTED;\n      lastPhaseHasNoSuccessorProof = false;`,
    test: "journals the exact generation before launch",
    testFile: "dist/tests/spawn_supervisor.test.js",
  },
  {
    name: "release instead of poison after launch ambiguity",
    file: "src/spawn_supervisor.ts",
    find: `          if (gate.fencedClaimOperationId === claimOperationId) lease.poison();\n          supervisorError.terminalReported = await this.#core`,
    replace: `          if (gate.fencedClaimOperationId === claimOperationId) lease.release();\n          supervisorError.terminalReported = await this.#core`,
    test: "launch-attempt ambiguity poisons",
    testFile: "dist/tests/spawn_supervisor.test.js",
  },
  {
    name: "permit require_resume from memory-only prior",
    file: "src/spawn_supervisor.ts",
    find: `if (validated.continuationMode === "require_resume" && materialization.kind !== "materialized")`,
    replace: `if (validated.continuationMode === "require_resume" && materialization.kind === "invalid")`,
    test: "require_resume refuses a memory-only prior",
    testFile: "dist/tests/spawn_supervisor.test.js",
  },
  {
    name: "publish successor before installing exact promoted candidate",
    file: "src/spawn_supervisor.ts",
    find: `      const promotedEntry = this.#registry.promoteCandidate(validated.claim, externalRuntime);\n      try {\n        await this.#reconciler.publishAfterPromotion(projection, successor);`,
    replace: `      let promotedEntry!: RuntimeSessionEntry;\n      try {\n        await this.#reconciler.publishAfterPromotion(projection, successor);\n        promotedEntry = this.#registry.promoteCandidate(validated.claim, externalRuntime);`,
    test: "publishes only after exact promotion",
    testFile: "dist/tests/spawn_supervisor.test.js",
  },
  {
    name: "allow SDK fixture without offline catalog/auth injection marker",
    file: "src/pi_session.ts",
    find: `if (options.services.modelCatalogAuthStub.kind !== "offline-injected")`,
    replace: `if (false)`,
    test: "offline fixture rejects a missing injected catalog",
    testFile: "dist/tests/pi_session.test.js",
  },
];

let killed = 0;
for (const mutation of mutations) {
  const path = resolve(root, mutation.file);
  const original = await readFile(path, "utf8");
  if (!original.includes(mutation.find)) {
    throw new Error(`mutation anchor missing: ${mutation.name}`);
  }
  await writeFile(path, original.replace(mutation.find, mutation.replace));
  try {
    const build = spawnSync("npm", ["run", "build"], { cwd: root, encoding: "utf8" });
    if (build.status !== 0) {
      throw new Error(`mutation did not compile: ${mutation.name}\n${build.stderr}${build.stdout}`);
    }
    const result = spawnSync(
      process.execPath,
      ["--test", `--test-name-pattern=${mutation.test}`, mutation.testFile],
      { cwd: root, encoding: "utf8" },
    );
    if (result.status === 0) {
      throw new Error(`SURVIVED: ${mutation.name}`);
    }
    killed += 1;
    console.log(`KILLED ${killed}/6: ${mutation.name}`);
  } finally {
    await writeFile(path, original);
  }
}

const rebuild = spawnSync("npm", ["run", "build"], { cwd: root, encoding: "utf8" });
if (rebuild.status !== 0) {
  throw new Error(`failed to restore clean build after mutations\n${rebuild.stderr}${rebuild.stdout}`);
}
if (killed !== mutations.length) throw new Error(`mutation score incomplete: ${killed}/${mutations.length}`);
console.log(`Mutation score: ${killed}/${mutations.length} killed`);
