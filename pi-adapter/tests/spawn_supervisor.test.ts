import assert from "node:assert/strict";
import { mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import test from "node:test";
import { create, toBinary } from "@bufbuild/protobuf";
import {
  AcceptedOperationSchema,
  AdapterIdSchema,
  AuthorityDomainIdSchema,
  CommandIdSchema,
  ContinuationAuthorityProvenanceSchema,
  ContinuationContextStatus,
  ExternalEffectDisposition,
  ExternalRuntimeRefSchema,
  FailureCode,
  FreshSpawnSchema,
  GenerationSchema,
  GrantIdSchema,
  LogicalTargetIdSchema,
  OperationKind,
  OperationSchema,
  OperationState,
  PayloadContentType,
  PayloadEnvelopeSchema,
  PiContinuationMode,
  PiSpawnTargetSpecSchema,
  RuntimeGenerationRefSchema,
  RuntimeSessionIdSchema,
  SpawnClaimAcceptedSchema,
  SpawnContinuationSchema,
  SpawnExecutionPhase,
  SpawnGenerationClaimSchema,
  SpawnPendingReplacementFenceSchema,
  SpawnPriorWorkDisposition,
  SpawnPriorWorkEffectSchema,
  SpawnPromotionCommittedSchema,
  SpawnRequestSchema,
  SpawnTargetSpecSchema,
  TargetScopeKind,
  TargetScopeSchema,
  type RuntimeGenerationRef,
  type SpawnClaimAccepted,
} from "@patchbay/contracts";
import type { PiControlHandshake } from "../src/control_handshake.js";
import { PiRpcTransportError } from "../src/rpc_client.js";
import type { ConfiguredDeploymentTarget } from "../src/deployment_authority.js";
import {
  type ManagedPiRuntimePort,
  type PiHandshakeChallenge,
  type PiLaunchSpec,
  type PiRpcRuntime,
  type ProcessExit,
} from "../src/pi_process.js";
import { RpcPiSession } from "../src/pi_session.js";
import { RuntimeActionGate } from "../src/runtime_action_gate.js";
import { SessionRegistry } from "../src/session_registry.js";
import {
  ClaimAwareSpawnSupervisor,
  LocalStagedPiReconciler,
  PI_RPC_TARGET_SHAPE,
  PI_SPAWN_TARGET_SCHEMA_REF,
  type PiAuthoritativeReconciler,
  type SpawnSupervisorCorePort,
} from "../src/spawn_supervisor.js";
import {
  FileSpawnEffectJournal,
  type SpawnEffectJournal,
} from "../src/spawn_journal.js";

const cwd = process.cwd();
const logicalTargetId = "logical-pi-test";
const deploymentScope = "machine-test";
const adapterId = "pi";

class FakeRpc {
  readonly events = new Set<(event: Record<string, unknown>) => void>();
  readonly failures = new Set<(error: PiRpcTransportError) => void>();
  closed = false;
  streaming = false;
  abortError: Error | undefined;

  constructor(
    readonly sessionId: string,
    readonly sessionFile: string,
    readonly entries: readonly unknown[] = [],
    readonly leafId: string | null = null,
  ) {}

  async request<T>(command: Record<string, unknown> & { readonly type: string }): Promise<T> {
    switch (command.type) {
      case "get_state":
        return {
          sessionId: this.sessionId,
          sessionFile: this.sessionFile,
          isStreaming: this.streaming,
          isCompacting: false,
          pendingMessageCount: 0,
          model: null,
          thinkingLevel: "off",
        } as T;
      case "get_entries":
        return { entries: this.entries, leafId: this.leafId } as T;
      case "abort":
        if (this.abortError) throw this.abortError;
        return {} as T;
      default:
        return {} as T;
    }
  }

  onEvent(listener: (event: Record<string, unknown>) => void): () => void {
    this.events.add(listener);
    return () => this.events.delete(listener);
  }

  emit(event: Record<string, unknown>): void {
    for (const listener of this.events) listener(event);
  }

  emitFailure(): void {
    const error = new PiRpcTransportError("pipe", "injected candidate transport loss");
    for (const listener of this.failures) listener(error);
  }

  close(): void {
    this.closed = true;
  }
}

class FakeRuntimePort implements ManagedPiRuntimePort {
  constructor(readonly events?: string[]) {}
  launchCalls = 0;
  terminateCalls = 0;
  readonly launches: PiLaunchSpec[] = [];
  readonly runtimes: PiRpcRuntime[] = [];
  failLaunch = false;
  nextSessionId = "pi-successor";
  nextSessionFile = join(cwd, "missing-pi-successor.jsonl");

  async launch(spec: PiLaunchSpec): Promise<PiRpcRuntime> {
    this.events?.push("process-launch");
    this.launchCalls += 1;
    this.launches.push(spec);
    if (this.failLaunch) throw new Error("injected launch ambiguity");
    const runtime = fakeRuntime(
      this.nextSessionId,
      this.nextSessionFile,
      `process-${this.launchCalls}`,
    );
    this.runtimes.push(runtime);
    return runtime;
  }

  async handshake(runtime: PiRpcRuntime, _challenge: PiHandshakeChallenge): Promise<PiControlHandshake> {
    const rpc = runtime.rpc as unknown as FakeRpc;
    return {
      challenge: "c".repeat(43),
      launchNonce: "n".repeat(43),
      extensionEpoch: "e".repeat(43),
      cwd,
      sessionId: rpc.sessionId,
      sessionFile: rpc.sessionFile,
      markerEntryId: rpc.leafId ?? "marker",
    };
  }

  async terminate(runtime: PiRpcRuntime): Promise<ProcessExit> {
    this.terminateCalls += 1;
    runtime.rpc.close();
    return {
      pid: runtime.pid,
      processToken: runtime.processToken,
      code: 0,
      signal: null,
      expected: true,
      terminatedBySupervisor: true,
    };
  }
}

function fakeRuntime(
  sessionId: string,
  sessionFile: string,
  processToken: string,
  entries: readonly unknown[] = [],
  leafId: string | null = null,
): PiRpcRuntime {
  const rpc = new FakeRpc(sessionId, sessionFile, entries, leafId);
  return {
    pid: 10_000 + processToken.length,
    processToken,
    rpc: rpc as never,
    exit: new Promise<ProcessExit>(() => undefined),
    child: {} as PiRpcRuntime["child"],
    markExpectedTermination() {},
    onTransportFailure(listener) {
      rpc.failures.add(listener);
      return () => rpc.failures.delete(listener);
    },
  };
}

function target(): ConfiguredDeploymentTarget {
  return {
    credentialPolicy: "credential-free",
    adapterId,
    deploymentScope,
    logicalTargetId,
  };
}

function managedTarget() {
  return {
    projectContextRef: "project-context-test",
    deploymentTarget: target(),
    cwd,
    sessionRoot: cwd,
    executable: process.execPath,
    cliPath: process.execPath,
    controlExtensionPath: process.execPath,
  } as const;
}

function runtimeRef(runtimeSessionId: string, generation: bigint): RuntimeGenerationRef {
  return create(RuntimeGenerationRefSchema, {
    logicalTargetId: create(LogicalTargetIdSchema, { value: logicalTargetId }),
    externalRuntime: create(ExternalRuntimeRefSchema, {
      adapterId: create(AdapterIdSchema, { value: adapterId }),
      deploymentScope,
      runtimeSessionId: create(RuntimeSessionIdSchema, { value: runtimeSessionId }),
      generation: create(GenerationSchema, { value: generation }),
    }),
  });
}

function acceptedSpawn(options: {
  readonly commandId?: string;
  readonly generation: bigint;
  readonly continuation?: RuntimeGenerationRef;
  readonly continuationMode?: PiContinuationMode;
  readonly priorWorkEffects?: SpawnClaimAccepted["priorWorkEffects"];
  readonly omitFence?: boolean;
}): SpawnClaimAccepted {
  const commandId = options.commandId ?? `spawn-${options.generation}`;
  const piTarget = create(PiSpawnTargetSpecSchema, {
    projectContextRef: "project-context-test",
    continuationMode: options.continuationMode ?? PiContinuationMode.UNSPECIFIED,
  });
  const request = create(SpawnRequestSchema, {
    intent: options.continuation
      ? { case: "continuation", value: create(SpawnContinuationSchema, { prior: options.continuation }) }
      : { case: "fresh", value: create(FreshSpawnSchema) },
    targetSpec: create(SpawnTargetSpecSchema, {
      shape: PI_RPC_TARGET_SHAPE,
      deploymentAuthorityRef: "authority-ref-test",
      adapterPayload: create(PayloadEnvelopeSchema, {
        contentType: PayloadContentType.PROTOBUF,
        schemaRef: PI_SPAWN_TARGET_SCHEMA_REF,
        payload: toBinary(PiSpawnTargetSpecSchema, piTarget),
      }),
    }),
  });
  const operation = create(OperationSchema, {
    commandId: create(CommandIdSchema, { value: commandId }),
    authorityDomainId: create(AuthorityDomainIdSchema, { value: "authority-test" }),
    kind: OperationKind.SPAWN,
    targetScope: create(TargetScopeSchema, {
      kind: TargetScopeKind.ADAPTER,
      adapterId: create(AdapterIdSchema, { value: adapterId }),
    }),
    payload: create(PayloadEnvelopeSchema, {
      contentType: PayloadContentType.PROTOBUF,
      schemaRef: "patchbay.SpawnRequest",
      payload: toBinary(SpawnRequestSchema, request),
    }),
  });
  const claim = create(SpawnGenerationClaimSchema, {
    authorityDomainId: create(AuthorityDomainIdSchema, { value: "authority-test" }),
    claimOperationId: create(CommandIdSchema, { value: commandId }),
    logicalTargetId: create(LogicalTargetIdSchema, { value: logicalTargetId }),
    claimedGeneration: create(GenerationSchema, { value: options.generation }),
    ...(options.continuation ? { expectedPrior: options.continuation } : {}),
  });
  return create(SpawnClaimAcceptedSchema, {
    acceptedOperation: create(AcceptedOperationSchema, {
      operation,
      authorizingGrantId: create(GrantIdSchema, { value: "spawn-grant" }),
    }),
    claim,
    ...(options.continuation
      ? {
          compoundAuthority: create(ContinuationAuthorityProvenanceSchema, {
            exactPrior: options.continuation,
            replacementGrantId: create(GrantIdSchema, { value: "replacement-grant" }),
            replacementAuthorityKind: OperationKind.SESSION_MANAGEMENT,
          }),
          ...(!options.omitFence
            ? {
                pendingReplacement: create(SpawnPendingReplacementFenceSchema, {
                  exactPrior: options.continuation,
                  failureCode: FailureCode.SUPERSEDED,
                  reasonCode: "replacement_pending",
                }),
              }
            : {}),
          priorWorkEffects: options.priorWorkEffects ?? [],
        }
      : {}),
  });
}

function priorWorkEffect(
  commandId: string,
  priorState: OperationState,
  disposition: SpawnPriorWorkDisposition,
  failureCode: FailureCode,
) {
  return create(SpawnPriorWorkEffectSchema, {
    commandId: create(CommandIdSchema, { value: commandId }),
    priorState,
    disposition,
    failureCode,
    reasonCode: "replacement_pending",
  });
}

function testReconciler(): PiAuthoritativeReconciler {
  return new LocalStagedPiReconciler(async () => undefined);
}

function createCore(
  events: string[],
  onStage?: (accepted: SpawnClaimAccepted, runtime: RuntimeGenerationRef) => void,
): SpawnSupervisorCorePort {
  return {
    adapterId,
    adapterGeneration: 1,
    async authorizeDeployment() { events.push("authorized"); },
    async flushObservations() { events.push("observations-flushed"); },
    async reportSpawnEvidence(input) { events.push(`evidence:${input.phase}:${input.disposition}`); },
    async reportSessionState(_entry, connectivity, activity) {
      events.push(`session:${connectivity}:${activity}`);
    },
    async reportRecoveredSessionState(_runtime, connectivity, activity) {
      events.push(`recovered-session:${connectivity}:${activity}`);
    },
    async resolvePriorWorkEffects({ effects }) {
      for (const effect of effects) {
        if (effect.disposition === SpawnPriorWorkDisposition.QUIESCE_OUTCOME_RECONCILIATION) {
          events.push(`prior-unknown:${effect.commandId?.value}`);
        } else {
          events.push(`prior-superseded:${effect.commandId?.value}`);
        }
      }
    },
    async stageSuccessor({
      acceptedSpawn: accepted,
      runtime,
      presentationConnectivity,
      presentationActivity,
    }) {
      events.push("successor-staged");
      events.push(`successor-presentation:${presentationConnectivity}:${presentationActivity}`);
      onStage?.(accepted, runtime);
    },
    async reportSpawnResult() { events.push("result-reported"); },
    async reportSpawnFailure(_operation, failureCode) { events.push(`failure:${failureCode}`); },
  };
}

class OrderedJournal implements SpawnEffectJournal {
  constructor(readonly inner: SpawnEffectJournal, readonly events: string[]) {}
  async beginClaim(record: Parameters<SpawnEffectJournal["beginClaim"]>[0]) {
    this.events.push("journal-claim");
    return this.inner.beginClaim(record);
  }
  async recordPhase(record: Parameters<SpawnEffectJournal["recordPhase"]>[0]) {
    this.events.push(`journal-phase:${record.phase}`);
    return this.inner.recordPhase(record);
  }
  recordExternalIdentity(record: Parameters<SpawnEffectJournal["recordExternalIdentity"]>[0]) {
    return this.inner.recordExternalIdentity(record);
  }
  recordStagedPublication(record: Parameters<SpawnEffectJournal["recordStagedPublication"]>[0]) {
    return this.inner.recordStagedPublication(record);
  }
  reconcile(claimOperationId: string) { return this.inner.reconcile(claimOperationId); }
  reconcileAll() { return this.inner.reconcileAll(); }
  markPromotionObserved(
    claimOperationId: string,
    runtime: Parameters<SpawnEffectJournal["markPromotionObserved"]>[1],
  ) {
    return this.inner.markPromotionObserved(claimOperationId, runtime);
  }
  async markPublicationCommitted(claimOperationId: string) {
    await this.inner.markPublicationCommitted(claimOperationId);
    this.events.push("journal-publication-committed");
  }
  abandonClaim(claimOperationId: string) {
    return this.inner.abandonClaim(claimOperationId);
  }
}

async function journalFixture(events: string[] = []) {
  const directory = await mkdtemp(join(cwd, "tmp-pi-spawn-journal-"));
  return {
    directory,
    journal: new OrderedJournal(new FileSpawnEffectJournal(directory), events),
  };
}

async function seedRecoverableJournal(
  journal: SpawnEffectJournal,
  accepted: SpawnClaimAccepted,
  staged: boolean,
): Promise<RuntimeGenerationRef> {
  const claimOperationId = accepted.claim!.claimOperationId!.value;
  const runtime = runtimeRef("pi-recovered", accepted.claim!.claimedGeneration!.value);
  await journal.beginClaim({
    exactClaim: accepted.claim!,
    launchNonce: "n".repeat(43),
    targetFingerprint: "a".repeat(64),
    createdAt: new Date().toISOString(),
  });
  await journal.recordPhase({
    claimOperationId,
    phase: SpawnExecutionPhase.LAUNCH_ATTEMPTED,
    externalEffectDisposition: ExternalEffectDisposition.MAY_EXIST,
    recordedAt: new Date().toISOString(),
    poisoned: true,
  });
  await journal.recordExternalIdentity({
    claimOperationId,
    runtime,
    processToken: "recovered-process",
    pid: 42,
    recordedAt: new Date().toISOString(),
  });
  await journal.recordPhase({
    claimOperationId,
    phase: SpawnExecutionPhase.EXTERNAL_IDENTITY_KNOWN,
    externalEffectDisposition: ExternalEffectDisposition.IDENTIFIED,
    recordedAt: new Date().toISOString(),
  });
  if (staged) {
    await journal.recordPhase({
      claimOperationId,
      phase: SpawnExecutionPhase.HANDSHAKE_RECONCILING,
      externalEffectDisposition: ExternalEffectDisposition.IDENTIFIED,
      recordedAt: new Date().toISOString(),
    });
    const projection = await new LocalStagedPiReconciler(async () => undefined)
      .stageClaimedSuccessor(runtime, [], null);
    await journal.recordStagedPublication({
      claimOperationId,
      runtime,
      readinessDigest: projection.readinessDigest,
      entryCount: projection.entryCount,
      continuationContextStatus: ContinuationContextStatus.UNSPECIFIED,
      entries: projection.recoveryEntries,
      leafId: projection.recoveryLeafId,
    });
    await journal.recordPhase({
      claimOperationId,
      phase: SpawnExecutionPhase.SUCCESS_EVIDENCE_REPORTED,
      externalEffectDisposition: ExternalEffectDisposition.IDENTIFIED,
      recordedAt: new Date().toISOString(),
    });
  }
  return runtime;
}

interface MutableStoredJournal {
  phases: Array<{
    phase: SpawnExecutionPhase;
    externalEffectDisposition: ExternalEffectDisposition;
    recordedAt: string;
  }>;
  poisoned: boolean;
}

async function mutateStoredJournal(
  directory: string,
  mutate: (stored: MutableStoredJournal) => void,
): Promise<void> {
  const [name] = await readdir(directory);
  assert.ok(name);
  const path = join(directory, name);
  const stored = JSON.parse(await readFile(path, "utf8")) as MutableStoredJournal;
  mutate(stored);
  await writeFile(path, `${JSON.stringify(stored)}\n`);
}

test("repeated journal phase effect claims follow the monotonicity table", async () => {
  const initialAt = "2026-08-16T12:00:00.000Z";
  const repeatedAt = "2026-08-16T12:00:01.000Z";
  const cases = [
    {
      name: "no-effect to may-exist strengthens",
      initial: ExternalEffectDisposition.PROVED_NONE,
      repeated: ExternalEffectDisposition.MAY_EXIST,
      accepted: true,
      expected: ExternalEffectDisposition.MAY_EXIST,
      expectedAt: repeatedAt,
      poisoned: true,
    },
    {
      name: "may-exist to no-effect downgrades",
      initial: ExternalEffectDisposition.MAY_EXIST,
      repeated: ExternalEffectDisposition.PROVED_NONE,
      accepted: false,
      expected: ExternalEffectDisposition.MAY_EXIST,
      expectedAt: initialAt,
      poisoned: true,
    },
    {
      name: "identical no-effect is idempotent",
      initial: ExternalEffectDisposition.PROVED_NONE,
      repeated: ExternalEffectDisposition.PROVED_NONE,
      accepted: true,
      expected: ExternalEffectDisposition.PROVED_NONE,
      expectedAt: initialAt,
      poisoned: false,
    },
    {
      name: "identical may-exist is idempotent",
      initial: ExternalEffectDisposition.MAY_EXIST,
      repeated: ExternalEffectDisposition.MAY_EXIST,
      accepted: true,
      expected: ExternalEffectDisposition.MAY_EXIST,
      expectedAt: initialAt,
      poisoned: true,
    },
  ] as const;

  for (const candidate of cases) {
    const fixture = await journalFixture();
    const commandId = `phase-monotonicity-${candidate.name.replaceAll(" ", "-")}`;
    const accepted = acceptedSpawn({
      commandId,
      generation: 8n,
      continuation: runtimeRef(`prior-${commandId}`, 7n),
    });
    try {
      await fixture.journal.beginClaim({
        exactClaim: accepted.claim!,
        launchNonce: "n".repeat(43),
        targetFingerprint: "a".repeat(64),
        createdAt: initialAt,
      });
      await fixture.journal.recordPhase({
        claimOperationId: commandId,
        phase: SpawnExecutionPhase.QUIESCING_PRIOR,
        externalEffectDisposition: candidate.initial,
        recordedAt: initialAt,
      });
      const repeat = fixture.journal.recordPhase({
        claimOperationId: commandId,
        phase: SpawnExecutionPhase.QUIESCING_PRIOR,
        externalEffectDisposition: candidate.repeated,
        recordedAt: repeatedAt,
      });
      if (candidate.accepted) await repeat;
      else await assert.rejects(repeat, /effect claim cannot weaken or contradict/);

      const state = await fixture.journal.reconcile(commandId);
      assert.equal(state?.phases.length, 1, candidate.name);
      assert.equal(
        state?.phases[0]?.externalEffectDisposition,
        candidate.expected,
        candidate.name,
      );
      assert.equal(state?.phases[0]?.recordedAt, candidate.expectedAt, candidate.name);
      assert.equal(state?.poisoned, candidate.poisoned, candidate.name);
    } finally {
      await rm(fixture.directory, { recursive: true, force: true });
    }
  }
});

test("direct journal reads reject a durable repeated phase with a stronger effect claim", async () => {
  const fixture = await journalFixture();
  const commandId = "phase-duplicate-direct-read";
  const accepted = acceptedSpawn({
    commandId,
    generation: 8n,
    continuation: runtimeRef("prior-phase-duplicate-direct-read", 7n),
  });
  try {
    await fixture.journal.beginClaim({
      exactClaim: accepted.claim!,
      launchNonce: "n".repeat(43),
      targetFingerprint: "a".repeat(64),
      createdAt: "2026-08-16T12:00:00.000Z",
    });
    await fixture.journal.recordPhase({
      claimOperationId: commandId,
      phase: SpawnExecutionPhase.QUIESCING_PRIOR,
      externalEffectDisposition: ExternalEffectDisposition.PROVED_NONE,
      recordedAt: "2026-08-16T12:00:00.000Z",
    });
    await mutateStoredJournal(fixture.directory, (stored) => {
      stored.phases.push({
        phase: SpawnExecutionPhase.QUIESCING_PRIOR,
        externalEffectDisposition: ExternalEffectDisposition.MAY_EXIST,
        recordedAt: "2026-08-16T12:00:01.000Z",
      });
      stored.poisoned = true;
    });

    await assert.rejects(
      fixture.journal.reconcile(commandId),
      /spawn journal phase is duplicated or contradictory/,
    );
    await assert.rejects(
      fixture.journal.reconcileAll(),
      /spawn journal phase is duplicated or contradictory/,
    );
  } finally {
    await rm(fixture.directory, { recursive: true, force: true });
  }
});

test("local staged reconciler replays the exact durable projection after restart promotion", async () => {
  const runtime = runtimeRef("pi-local-recovery", 1n);
  const entries = [{ type: "message", id: "entry-1", message: { role: "user", content: "durable" } }];
  let published: readonly unknown[] | undefined;
  const reconciler = new LocalStagedPiReconciler(async (publishedRuntime, recovered) => {
    assert.equal(publishedRuntime.externalRuntime?.runtimeSessionId?.value, "pi-local-recovery");
    published = recovered;
  });
  const staged = await reconciler.stageClaimedSuccessor(runtime, entries, "entry-1");
  entries[0]!.id = "mutated-after-stage";
  await reconciler.publishRecoveredAfterPromotion(staged, runtime);
  assert.deepEqual(published, [{
    type: "message",
    id: "entry-1",
    message: { role: "user", content: "durable" },
  }]);
});

test("fresh spawn journals the exact generation before launch and publishes only after exact promotion", async () => {
  const events: string[] = [];
  const runtimePort = new FakeRuntimePort(events);
  const registry = new SessionRegistry();
  const fixture = await journalFixture(events);
  let supervisor!: ClaimAwareSpawnSupervisor;
  let transcriptCount = 0;
  const reconciler: PiAuthoritativeReconciler = {
    async stageClaimedSuccessor(runtime) {
      events.push("projection-staged");
      return {
        runtime,
        readinessDigest: "a".repeat(64),
        entryCount: 0,
        recoveryEntries: [],
        recoveryLeafId: null,
      };
    },
    async publishAfterPromotion(staged, session) {
      assert.equal(registry.resolve(session.runtimeSessionId)?.session, session);
      events.push("projection-published");
      session.publishStagedTranscript();
      events.push("cursor-record-current");
      assert.equal(staged.readinessDigest, "a".repeat(64));
    },
    async publishRecoveredAfterPromotion() {
      events.push("projection-recovered");
    },
  };
  const accepted = acceptedSpawn({ generation: 1n });
  const core = createCore(events, (acceptedClaim, runtime) => {
    assert.equal(registry.resolve(runtime.externalRuntime!.runtimeSessionId!.value), undefined);
    (runtimePort.runtimes[0]!.rpc as unknown as FakeRpc).emit({
      type: "entry_appended",
      entry: {
        type: "message",
        id: "staged-user",
        timestamp: new Date().toISOString(),
        message: { role: "user", content: "staged" },
      },
    });
    assert.equal(transcriptCount, 0, "claimed output is not ordinary output");
    setImmediate(() => {
      supervisor.acceptPromotion(create(SpawnPromotionCommittedSchema, {
        acceptedClaim,
        promotedRuntime: runtime,
      }));
    });
  });
  supervisor = new ClaimAwareSpawnSupervisor({
    runtimePort,
    journal: fixture.journal,
    registry,
    core,
    targets: [managedTarget()],
    reconciler,
    observeTranscript: () => { transcriptCount += 1; },
  });

  try {
    const successor = await supervisor.handleAcceptedSpawn(accepted);
    assert.equal(successor.runtime.externalRuntime?.generation?.value, 1n);
    assert.equal(runtimePort.launchCalls, 1);
    assert.equal(runtimePort.launches[0]?.argv.includes("--session"), false);
    assert.ok(events.indexOf("journal-claim") < events.findIndex((event) => event.startsWith("journal-phase:")));
    assert.ok(
      events.indexOf(`journal-phase:${SpawnExecutionPhase.LAUNCH_ATTEMPTED}`) < events.indexOf("process-launch"),
      "launch_attempted is durable before invoking the process port",
    );
    assert.ok(events.findIndex((event) => event.startsWith("journal-phase:")) < events.indexOf("successor-staged"));
    assert.ok(events.indexOf("successor-staged") < events.indexOf("result-reported"));
    assert.ok(events.includes("successor-presentation:stale:unknown"));
    assert.ok(events.indexOf("result-reported") < events.indexOf("projection-published"));
    assert.equal(transcriptCount, 1, "staged transcript publishes exactly once after promotion");
    assert.ok(events.indexOf("cursor-record-current") < events.indexOf("journal-publication-committed"));
    assert.ok(events.indexOf("journal-publication-committed") < events.findIndex((event) => event.startsWith("session:live:")));
    assert.equal(registry.resolve(successor.entry.runtimeSessionId)?.session, successor.entry.session);
  } finally {
    await registry.dispose();
    await rm(fixture.directory, { recursive: true, force: true });
  }
});

test("promotion crash windows never report live before projection, cursor, and journal commit", async () => {
  for (const crashPoint of ["before-envelope", "before-local-cas"] as const) {
    const events: string[] = [];
    const runtimePort = new FakeRuntimePort(events);
    const registry = new SessionRegistry();
    const fixture = await journalFixture(events);
    const accepted = acceptedSpawn({ commandId: `publication-crash-${crashPoint}`, generation: 1n });
    let supervisor!: ClaimAwareSpawnSupervisor;
    let cursorRecordCurrent = false;
    const reconciler: PiAuthoritativeReconciler = {
      async stageClaimedSuccessor(runtime) {
        return {
          runtime,
          readinessDigest: "b".repeat(64),
          entryCount: 0,
          recoveryEntries: [],
          recoveryLeafId: null,
        };
      },
      async publishAfterPromotion() {
        if (crashPoint === "before-envelope") throw new Error("injected before envelope");
        events.push("projection-envelope-durable");
        if (crashPoint === "before-local-cas") throw new Error("injected before local CAS");
        cursorRecordCurrent = true;
      },
      async publishRecoveredAfterPromotion() {
        throw new Error("not used");
      },
    };
    const core = createCore(events, (acceptedClaim, runtime) => {
      setImmediate(() => {
        supervisor.acceptPromotion(create(SpawnPromotionCommittedSchema, {
          acceptedClaim,
          promotedRuntime: runtime,
        }));
      });
    });
    supervisor = new ClaimAwareSpawnSupervisor({
      runtimePort,
      journal: fixture.journal,
      registry,
      core,
      targets: [managedTarget()],
      reconciler,
    });

    try {
      await assert.rejects(
        supervisor.handleAcceptedSpawn(accepted),
        /successor publication failed after core promotion/u,
      );
      const journal = await fixture.journal.reconcile(accepted.claim!.claimOperationId!.value);
      assert.equal(journal?.promotionObserved, true, crashPoint);
      assert.equal(journal?.publicationCommitted, false, crashPoint);
      assert.equal(cursorRecordCurrent, false, crashPoint);
      assert.ok(events.includes("successor-presentation:stale:unknown"), crashPoint);
      assert.ok(events.includes("session:stale:unknown"), crashPoint);
      assert.equal(events.some((event) => event.startsWith("session:live:")), false, crashPoint);
      assert.equal(
        events.includes("projection-envelope-durable"),
        crashPoint === "before-local-cas",
        crashPoint,
      );
    } finally {
      await registry.dispose();
      await rm(fixture.directory, { recursive: true, force: true });
    }
  }
});

test("target mutex structurally precedes validation, authority, journal, and fence activation", async () => {
  const events: string[] = [];
  const runtimePort = new FakeRuntimePort(events);
  const registry = new SessionRegistry();
  const fixture = await journalFixture(events);
  const heldTarget = await registry.gateFor(logicalTargetId).acquireReplacementTarget("held-prefix");
  const accepted = acceptedSpawn({ commandId: "serialized-prefix", generation: 1n });
  let supervisor!: ClaimAwareSpawnSupervisor;
  const core = createCore(events, (acceptedClaim, runtime) => {
    setImmediate(() => {
      void supervisor.acceptPromotion(create(SpawnPromotionCommittedSchema, {
        acceptedClaim,
        promotedRuntime: runtime,
      }));
    });
  });
  supervisor = new ClaimAwareSpawnSupervisor({
    runtimePort,
    journal: fixture.journal,
    registry,
    core,
    targets: [managedTarget()],
    reconciler: testReconciler(),
  });
  let targetReleased = false;
  let pending: Promise<unknown> | undefined;
  try {
    pending = supervisor.handleAcceptedSpawn(accepted);
    await new Promise<void>((resolve) => setTimeout(resolve, 50));
    assert.equal(events.length, 0, "no prefix work crosses a held target mutex");
    assert.equal(runtimePort.launchCalls, 0);
    heldTarget.release();
    targetReleased = true;
    await pending;
    assert.ok(events.indexOf("authorized") < events.indexOf("journal-claim"));
    assert.ok(events.indexOf("journal-claim") < events.indexOf("process-launch"));
  } finally {
    if (!targetReleased) heldTarget.release();
    await pending?.catch(() => undefined);
    await registry.dispose();
    await rm(fixture.directory, { recursive: true, force: true });
  }
});

test("a launch-attempt ambiguity poisons the claim and duplicate delivery never relaunches", async () => {
  const events: string[] = [];
  const runtimePort = new FakeRuntimePort();
  runtimePort.failLaunch = true;
  const registry = new SessionRegistry();
  const fixture = await journalFixture(events);
  const supervisor = new ClaimAwareSpawnSupervisor({
    runtimePort,
    journal: fixture.journal,
    registry,
    core: createCore(events),
    targets: [managedTarget()],
    reconciler: testReconciler(),
  });
  const accepted = acceptedSpawn({ commandId: "ambiguous-launch", generation: 1n });
  try {
    await assert.rejects(supervisor.handleAcceptedSpawn(accepted), { failureCode: FailureCode.EXECUTION_OUTCOME_UNKNOWN });
    assert.equal(registry.gateFor(logicalTargetId).poisoned, true);
    await assert.rejects(supervisor.handleAcceptedSpawn(accepted), { failureCode: FailureCode.EXECUTION_OUTCOME_UNKNOWN });
    assert.equal(runtimePort.launchCalls, 1, "the exact generation receives at most one launch attempt");
    const state = await fixture.journal.reconcile("ambiguous-launch");
    assert.equal(
      state?.phases.some((phase) => phase.phase === SpawnExecutionPhase.LAUNCH_ATTEMPTED),
      true,
    );
  } finally {
    await registry.dispose();
    await rm(fixture.directory, { recursive: true, force: true });
  }
});

test("restart poisons every unpromoted journal launch and never auto-launches it", async () => {
  const events: string[] = [];
  const runtimePort = new FakeRuntimePort(events);
  const registry = new SessionRegistry();
  const fixture = await journalFixture(events);
  const accepted = acceptedSpawn({ commandId: "restart-pending", generation: 1n });
  await seedRecoverableJournal(fixture.journal, accepted, false);
  const supervisor = new ClaimAwareSpawnSupervisor({
    runtimePort,
    journal: fixture.journal,
    registry,
    core: createCore(events),
    targets: [managedTarget()],
    reconciler: testReconciler(),
  });
  try {
    await supervisor.recoverOnStart();
    assert.equal(runtimePort.launchCalls, 0);
    assert.equal(registry.gateFor(logicalTargetId).poisoned, true);
    assert.ok(events.some((event) => event.startsWith(
      `evidence:${SpawnExecutionPhase.EXTERNAL_IDENTITY_KNOWN}:`,
    )));
    assert.ok(events.includes("recovered-session:stale:unknown"));
  } finally {
    await registry.dispose();
    await rm(fixture.directory, { recursive: true, force: true });
  }
});

test("replayed promotion recovers and commits staged publication without launching", async () => {
  const events: string[] = [];
  const runtimePort = new FakeRuntimePort(events);
  const registry = new SessionRegistry();
  const fixture = await journalFixture(events);
  const accepted = acceptedSpawn({ commandId: "restart-promoted", generation: 1n });
  const runtime = await seedRecoverableJournal(fixture.journal, accepted, true);
  let recovered = 0;
  const reconciler: PiAuthoritativeReconciler = {
    async stageClaimedSuccessor() {
      throw new Error("restart recovery must not restage through a process");
    },
    async publishAfterPromotion() {
      throw new Error("restart recovery has no live process session");
    },
    async publishRecoveredAfterPromotion(staged, recoveredRuntime) {
      recovered += 1;
      assert.match(staged.readinessDigest, /^[a-f0-9]{64}$/u);
      assert.equal(recoveredRuntime.externalRuntime?.runtimeSessionId?.value, "pi-recovered");
    },
  };
  const supervisor = new ClaimAwareSpawnSupervisor({
    runtimePort,
    journal: fixture.journal,
    registry,
    core: createCore(events),
    targets: [managedTarget()],
    reconciler,
  });
  try {
    assert.equal(await supervisor.acceptPromotion(create(SpawnPromotionCommittedSchema, {
      acceptedClaim: accepted,
      promotedRuntime: runtime,
    })), true);
    assert.equal(recovered, 1);
    assert.equal(runtimePort.launchCalls, 0);
    const state = await fixture.journal.reconcile("restart-promoted");
    assert.equal(state?.promotionObserved, true);
    assert.equal(state?.publicationCommitted, true);
    assert.ok(events.includes("recovered-session:stale:unknown"));
  } finally {
    await registry.dispose();
    await rm(fixture.directory, { recursive: true, force: true });
  }
});

test("promotion replay rejects missing or duplicated launch-attempt phases before every effect", async () => {
  const corruptions = ["missing", "duplicated"] as const;
  for (const corruption of corruptions) {
    const events: string[] = [];
    const runtimePort = new FakeRuntimePort(events);
    const registry = new SessionRegistry();
    const fixture = await journalFixture(events);
    const accepted = acceptedSpawn({
      commandId: `restart-corrupt-${corruption}`,
      generation: 1n,
    });
    const runtime = await seedRecoverableJournal(fixture.journal, accepted, true);
    await mutateStoredJournal(fixture.directory, (stored) => {
      const launchIndex = stored.phases.findIndex(
        (phase) => phase.phase === SpawnExecutionPhase.LAUNCH_ATTEMPTED,
      );
      assert.notEqual(launchIndex, -1);
      if (corruption === "missing") stored.phases.splice(launchIndex, 1);
      else stored.phases.splice(launchIndex + 1, 0, structuredClone(stored.phases[launchIndex]!));
    });
    let publications = 0;
    const supervisor = new ClaimAwareSpawnSupervisor({
      runtimePort,
      journal: fixture.journal,
      registry,
      core: createCore(events),
      targets: [managedTarget()],
      reconciler: new LocalStagedPiReconciler(async () => {
        publications += 1;
      }),
    });
    try {
      await assert.rejects(
        supervisor.acceptPromotion(create(SpawnPromotionCommittedSchema, {
          acceptedClaim: accepted,
          promotedRuntime: runtime,
        })),
        /spawn journal (semantic phase chain|phase is duplicated)/,
      );
      assert.equal(publications, 0);
      assert.equal(runtimePort.launchCalls, 0);
      assert.equal(events.some((event) => event.startsWith("recovered-session:")), false);
      const [name] = await readdir(fixture.directory);
      assert.ok(name);
      const stored = JSON.parse(await readFile(join(fixture.directory, name), "utf8")) as {
        promotionObserved: boolean;
        publicationCommitted: boolean;
      };
      assert.equal(stored.promotionObserved, false);
      assert.equal(stored.publicationCommitted, false);
    } finally {
      await registry.dispose();
      await rm(fixture.directory, { recursive: true, force: true });
    }
  }
});

test("claimed successor transport loss before promotion stays unpublished and poisons the generation", async () => {
  const events: string[] = [];
  const runtimePort = new FakeRuntimePort();
  const registry = new SessionRegistry();
  const fixture = await journalFixture(events);
  const accepted = acceptedSpawn({ commandId: "candidate-crash", generation: 1n });
  const core = createCore(events, () => {
    queueMicrotask(() => {
      (runtimePort.runtimes[0]!.rpc as unknown as FakeRpc).emitFailure();
    });
  });
  const supervisor = new ClaimAwareSpawnSupervisor({
    runtimePort,
    journal: fixture.journal,
    registry,
    core,
    targets: [managedTarget()],
    reconciler: testReconciler(),
  });
  try {
    await assert.rejects(
      supervisor.handleAcceptedSpawn(accepted),
      { failureCode: FailureCode.EXECUTION_OUTCOME_UNKNOWN },
    );
    assert.equal(registry.resolve("pi-successor"), undefined);
    assert.equal(registry.candidate("candidate-crash"), undefined);
    assert.equal(registry.gateFor(logicalTargetId).poisoned, true);
    assert.equal(runtimePort.terminateCalls, 1);
  } finally {
    await registry.dispose();
    await rm(fixture.directory, { recursive: true, force: true });
  }
});

test("fresh generation two is rejected instead of allocating or repairing current plus one", async () => {
  const events: string[] = [];
  const runtimePort = new FakeRuntimePort();
  runtimePort.failLaunch = true;
  const registry = new SessionRegistry();
  const fixture = await journalFixture(events);
  const supervisor = new ClaimAwareSpawnSupervisor({
    runtimePort,
    journal: fixture.journal,
    registry,
    core: createCore(events),
    targets: [managedTarget()],
    reconciler: testReconciler(),
  });
  try {
    await assert.rejects(
      supervisor.handleAcceptedSpawn(acceptedSpawn({ generation: 2n })),
      { failureCode: FailureCode.DELIVERY_REJECTED },
    );
    assert.equal(runtimePort.launchCalls, 0);
  } finally {
    await registry.dispose();
    await rm(fixture.directory, { recursive: true, force: true });
  }
});

test("continuation rejects each omitted or mutated authority, fence, and prior-work dimension", async () => {
  const events: string[] = [];
  const runtimePort = new FakeRuntimePort();
  const registry = new SessionRegistry();
  const fixture = await journalFixture(events);
  const prior = runtimeRef("pi-prior-validation", 7n);
  const omittedFence = acceptedSpawn({
    commandId: "invalid-omitted-fence",
    generation: 8n,
    continuation: prior,
    continuationMode: PiContinuationMode.ALLOW_NEW_CONTEXT,
    omitFence: true,
  });
  const mutatedFence = acceptedSpawn({
    commandId: "invalid-mutated-fence",
    generation: 8n,
    continuation: prior,
    continuationMode: PiContinuationMode.ALLOW_NEW_CONTEXT,
  });
  mutatedFence.pendingReplacement!.reasonCode = "not_replacement_pending";
  const omittedAuthority = acceptedSpawn({
    commandId: "invalid-omitted-authority",
    generation: 8n,
    continuation: prior,
    continuationMode: PiContinuationMode.ALLOW_NEW_CONTEXT,
  });
  omittedAuthority.compoundAuthority = undefined;
  const malformedEffect = acceptedSpawn({
    commandId: "invalid-prior-effect",
    generation: 8n,
    continuation: prior,
    continuationMode: PiContinuationMode.ALLOW_NEW_CONTEXT,
    priorWorkEffects: [priorWorkEffect(
      "running-but-superseded",
      OperationState.RUNNING,
      SpawnPriorWorkDisposition.SUPERSEDED_BEFORE_OFFER,
      FailureCode.SUPERSEDED,
    )],
  });
  const supervisor = new ClaimAwareSpawnSupervisor({
    runtimePort,
    journal: fixture.journal,
    registry,
    core: createCore(events),
    targets: [managedTarget()],
    reconciler: testReconciler(),
  });
  try {
    for (const candidate of [omittedFence, mutatedFence, omittedAuthority, malformedEffect]) {
      const authorizationCount = events.filter((event) => event === "authorized").length;
      await assert.rejects(
        supervisor.handleAcceptedSpawn(candidate),
        { failureCode: FailureCode.DELIVERY_REJECTED },
      );
      assert.equal(
        events.filter((event) => event === "authorized").length,
        authorizationCount,
        "invalid continuation rejects before deployment authority or journal responsibility",
      );
      assert.equal(
        await fixture.journal.reconcile(candidate.claim!.claimOperationId!.value),
        undefined,
        "invalid continuation cannot acquire journal responsibility",
      );
    }
    assert.equal(runtimePort.launchCalls, 0);
  } finally {
    await registry.dispose();
    await rm(fixture.directory, { recursive: true, force: true });
  }
});

test("possibly-written quiesce abort loss reports unknown outcome and unproved prior state", async () => {
  const cases = [
    {
      name: "timeout",
      error: new PiRpcTransportError(
        "timeout",
        "injected post-write abort timeout",
        undefined,
        "possibly_written",
      ),
      connectivity: "stale",
    },
    {
      name: "unclean-exit",
      error: new PiRpcTransportError(
        "process_exit",
        "injected post-write abort process exit",
        { code: 9, signal: null, expected: false },
        "possibly_written",
      ),
      connectivity: "failed",
    },
  ] as const;
  for (const candidate of cases) {
    const events: string[] = [];
    const runtimePort = new FakeRuntimePort();
    const registry = new SessionRegistry();
    const fixture = await journalFixture(events);
    const priorRef = runtimeRef(`pi-prior-abort-${candidate.name}`, 7n);
    const priorRuntime = fakeRuntime(
      `pi-prior-abort-${candidate.name}`,
      join(cwd, `missing-prior-abort-${candidate.name}.jsonl`),
      `prior-abort-${candidate.name}-process`,
    );
    const priorRpc = priorRuntime.rpc as unknown as FakeRpc;
    priorRpc.streaming = true;
    priorRpc.abortError = candidate.error;
    const prior = await RpcPiSession.bind({
      runtimeSessionId: `pi-prior-abort-${candidate.name}`,
      generation: 7,
      runtime: priorRuntime,
      runtimePort,
      actionGate: registry.gateFor(logicalTargetId),
      publication: "current",
    });
    registry.register(
      {
        runtimeSessionId: prior.runtimeSessionId,
        deploymentScope,
        cwd,
        logicalTargetId,
      },
      prior,
      () => undefined,
      () => undefined,
    );
    const accepted = acceptedSpawn({
      commandId: `quiesce-abort-${candidate.name}`,
      generation: 8n,
      continuation: priorRef,
      continuationMode: PiContinuationMode.ALLOW_NEW_CONTEXT,
    });
    const supervisor = new ClaimAwareSpawnSupervisor({
      runtimePort,
      journal: fixture.journal,
      registry,
      core: createCore(events),
      targets: [managedTarget()],
      reconciler: testReconciler(),
    });
    try {
      await assert.rejects(
        supervisor.handleAcceptedSpawn(accepted),
        { failureCode: FailureCode.EXECUTION_OUTCOME_UNKNOWN },
      );
      assert.equal(runtimePort.launchCalls, 0);
      assert.equal(registry.gateFor(logicalTargetId).poisoned, true);
      assert.ok(events.includes(`session:${candidate.connectivity}:unknown`));
      assert.equal(events.includes("session:live:idle"), false);
      assert.ok(events.includes(`failure:${FailureCode.EXECUTION_OUTCOME_UNKNOWN}`));
      assert.ok(events.includes(
        `evidence:${SpawnExecutionPhase.QUIESCING_PRIOR}:${ExternalEffectDisposition.MAY_EXIST}`,
      ));
      const state = await fixture.journal.reconcile(accepted.claim!.claimOperationId!.value);
      assert.equal(state?.poisoned, true);
      assert.equal(
        state?.phases.at(-1)?.externalEffectDisposition,
        ExternalEffectDisposition.MAY_EXIST,
      );
    } finally {
      await registry.dispose();
      await rm(fixture.directory, { recursive: true, force: true });
    }
  }
});

test("explicit allow_new_context consumes exact N+1 but never reports resumed", async () => {
  const events: string[] = [];
  const runtimePort = new FakeRuntimePort();
  const registry = new SessionRegistry();
  const fixture = await journalFixture(events);
  const priorRef = runtimeRef("pi-prior-new-context", 7n);
  const priorRuntime = fakeRuntime(
    "pi-prior-new-context",
    join(cwd, "missing-allow-new-prior.jsonl"),
    "prior-new-context-process",
  );
  const prior = await RpcPiSession.bind({
    runtimeSessionId: "pi-prior-new-context",
    generation: 7,
    runtime: priorRuntime,
    runtimePort,
    actionGate: registry.gateFor(logicalTargetId),
    publication: "current",
  });
  registry.register(
    {
      runtimeSessionId: "pi-prior-new-context",
      deploymentScope,
      cwd,
      logicalTargetId,
    },
    prior,
    () => undefined,
    () => undefined,
  );
  let supervisor!: ClaimAwareSpawnSupervisor;
  const accepted = acceptedSpawn({
    commandId: "allow-new-context",
    generation: 8n,
    continuation: priorRef,
    continuationMode: PiContinuationMode.ALLOW_NEW_CONTEXT,
    priorWorkEffects: [
      priorWorkEffect(
        "accepted-n-work",
        OperationState.ACCEPTED,
        SpawnPriorWorkDisposition.SUPERSEDED_BEFORE_OFFER,
        FailureCode.SUPERSEDED,
      ),
      priorWorkEffect(
        "running-n-work",
        OperationState.RUNNING,
        SpawnPriorWorkDisposition.QUIESCE_OUTCOME_RECONCILIATION,
        FailureCode.UNSPECIFIED,
      ),
    ],
  });
  const core = createCore(events, (acceptedClaim, runtime) => {
    setImmediate(() => supervisor.acceptPromotion(create(SpawnPromotionCommittedSchema, {
      acceptedClaim,
      promotedRuntime: runtime,
    })));
  });
  supervisor = new ClaimAwareSpawnSupervisor({
    runtimePort,
    journal: fixture.journal,
    registry,
    core,
    targets: [managedTarget()],
    reconciler: testReconciler(),
  });
  try {
    const successor = await supervisor.handleAcceptedSpawn(accepted);
    assert.equal(successor.runtime.externalRuntime?.generation?.value, 8n);
    assert.equal(successor.continuationContextStatus, ContinuationContextStatus.NEW_CONTEXT);
    assert.notEqual(successor.continuationContextStatus, ContinuationContextStatus.RESUMED);
    assert.equal(runtimePort.launches[0]?.argv.includes("--session"), false);
    assert.ok(events.includes("prior-superseded:accepted-n-work"));
    assert.ok(events.includes("prior-unknown:running-n-work"));
    assert.ok(events.indexOf("prior-unknown:running-n-work") < events.indexOf("observations-flushed"));
    assert.ok(events.includes(
      `evidence:${SpawnExecutionPhase.QUIESCING_PRIOR}:${ExternalEffectDisposition.PROVED_NONE}`,
    ));
    assert.ok(events.includes(
      `evidence:${SpawnExecutionPhase.PRIOR_TERMINATED}:${ExternalEffectDisposition.PROVED_NONE}`,
    ));
    assert.equal(runtimePort.terminateCalls, 1, "the exact prior is terminated once before launch");
  } finally {
    await registry.dispose();
    await rm(fixture.directory, { recursive: true, force: true });
  }
});

test("require_resume refuses a memory-only prior before termination or successor launch", async () => {
  const events: string[] = [];
  const runtimePort = new FakeRuntimePort();
  const registry = new SessionRegistry();
  const fixture = await journalFixture(events);
  const priorRef = runtimeRef("pi-prior", 7n);
  const priorRuntime = fakeRuntime(
    "pi-prior",
    join(cwd, "missing-memory-only-prior.jsonl"),
    "prior-process",
  );
  const prior = await RpcPiSession.bind({
    runtimeSessionId: "pi-prior",
    generation: 7,
    runtime: priorRuntime,
    runtimePort,
    actionGate: registry.gateFor(logicalTargetId),
    publication: "current",
  });
  registry.register(
    {
      runtimeSessionId: "pi-prior",
      deploymentScope,
      cwd,
      logicalTargetId,
    },
    prior,
    () => undefined,
    () => undefined,
  );
  const supervisor = new ClaimAwareSpawnSupervisor({
    runtimePort,
    journal: fixture.journal,
    registry,
    core: createCore(events),
    targets: [managedTarget()],
    reconciler: testReconciler(),
  });
  try {
    await assert.rejects(
      supervisor.handleAcceptedSpawn(acceptedSpawn({
        commandId: "resume-memory-only",
        generation: 8n,
        continuation: priorRef,
        continuationMode: PiContinuationMode.REQUIRE_RESUME,
      })),
      { failureCode: FailureCode.EXECUTION_FAILED },
    );
    assert.equal(runtimePort.launchCalls, 0);
    assert.equal(runtimePort.terminateCalls, 0, "the prior remains alive when resume proof is absent");
    assert.equal(
      await fixture.journal.reconcile("resume-memory-only"),
      undefined,
      "the terminal pre-launch refusal abandons its no-effect journal",
    );
    assert.ok(events.includes("session:live:idle"));
  } finally {
    await registry.dispose();
    await rm(fixture.directory, { recursive: true, force: true });
  }
});
