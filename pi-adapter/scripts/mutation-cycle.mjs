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
    find: `if (\n      options.services.modelCatalogAuthStub.kind !== "offline-injected" ||\n      !offlineFixtureModelRuntimes.has(options.services.modelRuntime)\n    )`,
    replace: `if (false)`,
    test: "offline fixture rejects a missing injected catalog",
    testFile: "dist/tests/pi_session.test.js",
  },
  {
    name: "perform continuation prefix before acquiring the target mutex",
    file: "src/spawn_supervisor.ts",
    find: `    const targetLock = await gate.acquireReplacementTarget(preliminaryClaimOperationId);`,
    replace: `    const targetLock = {\n      activateFence: () => gate.acquireReplacement(preliminaryClaimOperationId),\n      release() {},\n    };`,
    test: "target mutex structurally precedes",
    testFile: "dist/tests/spawn_supervisor.test.js",
  },
  {
    name: "treat continuation pending-replacement fence as optional",
    file: "src/spawn_supervisor.ts",
    find: `    validateAcceptedContinuationEnvelope(\n      acceptedSpawn,\n      continuationMode,\n      request.intent.case === "continuation" ? request.intent.value.prior : undefined,\n    );`,
    replace: `    if (acceptedSpawn.pendingReplacement) {\n      validateAcceptedContinuationEnvelope(\n        acceptedSpawn,\n        continuationMode,\n        request.intent.case === "continuation" ? request.intent.value.prior : undefined,\n      );\n    }`,
    test: "continuation rejects each omitted",
    testFile: "dist/tests/spawn_supervisor.test.js",
  },
  {
    name: "ignore accepted prior-work effects during continuation",
    file: "src/spawn_supervisor.ts",
    find: `    await this.#core.resolvePriorWorkEffects({\n      exactPrior: validated.claim.expectedPrior!,\n      effects: validated.acceptedSpawn.priorWorkEffects,\n    });`,
    replace: `    // mutant: accepted prior-work effects ignored`,
    test: "explicit allow_new_context",
    testFile: "dist/tests/spawn_supervisor.test.js",
  },
  {
    name: "classify possibly-written RPC response loss as execution_failed",
    file: "src/main.ts",
    find: `      failureCode: outcomeUnknown\n        ? FailureCode.EXECUTION_OUTCOME_UNKNOWN\n        : FailureCode.EXECUTION_FAILED,`,
    replace: `      failureCode: FailureCode.EXECUTION_FAILED,`,
    test: "delivery classification preserves post-write RPC ambiguity",
    testFile: "dist/tests/delivery.test.js",
  },
  {
    name: "auto-launch a managed preprovisioned target outside the journal",
    file: "src/main.ts",
    find: `    if (configured.logicalTargetId && !this.#options.createSession) {`,
    replace: `    if (false) {`,
    test: "never preprovisions a managed logical target",
    testFile: "dist/tests/delivery.test.js",
  },
  {
    name: "silently ignore a replayed promotion without an in-memory waiter",
    file: "src/spawn_supervisor.ts",
    find: `      await this.#reconciler.publishRecoveredAfterPromotion(\n        stagedProjection(state.stagedPublication),\n        state.externalIdentity.runtime,\n      );\n      await this.#journal.markPublicationCommitted(claimOperationId);`,
    replace: `      // mutant: replayed promotion publication is silently ignored\n      await this.#journal.markPublicationCommitted(claimOperationId);`,
    test: "replayed promotion recovers and commits",
    testFile: "dist/tests/spawn_supervisor.test.js",
  },
  {
    name: "bypass RpcPiSession action gate for ordinary requests",
    file: "src/pi_session.ts",
    find: `  return gate.runAction(kind, () => runtime.rpc.request<T>(command));`,
    replace: `  return runtime.rpc.request<T>(command);`,
    test: "RpcPiSession production requests cannot bypass",
    testFile: "dist/tests/runtime_supervision_primitives.test.js",
  },
  {
    name: "replace SIGKILL escalation with a second SIGTERM",
    file: "src/pi_process.ts",
    find: `    signalProcessGroup(runtime.pid, "SIGKILL");`,
    replace: `    signalProcessGroup(runtime.pid, "SIGTERM");`,
    test: "stubborn process group forces bounded SIGKILL",
    testFile: "dist/tests/rpc_process_e2e.test.js",
  },
  {
    name: "construct offline fixture runtime through ambient discovery",
    file: "tests/offline_agent_fixture.ts",
    find: `  const runtime = await ModelRuntime.create({\n    credentials: new InMemoryCredentialStore(),\n    modelsStore: new InMemoryModelsStore(),\n    modelsPath: null,\n    refreshOnCreate: false,\n    allowModelNetwork: false,\n  });`,
    replace: `  const runtime = await ModelRuntime.create({ refreshOnCreate: false });`,
    test: "offline model factory passes only in-memory stores",
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
      { cwd: root, encoding: "utf8", timeout: 60_000 },
    );
    if (result.status === 0) {
      throw new Error(`SURVIVED: ${mutation.name}`);
    }
    killed += 1;
    console.log(`KILLED ${killed}/${mutations.length}: ${mutation.name}`);
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
