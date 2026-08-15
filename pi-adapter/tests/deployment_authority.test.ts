import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { create, toBinary } from "@bufbuild/protobuf";
import {
  AcceptedOperationSchema,
  AdapterDiagnosticReportResultSchema,
  AdapterIdSchema,
  AuthorityDomainIdSchema,
  CommandIdSchema,
  ContinuationAuthorityProvenanceSchema,
  ExternalRuntimeRefSchema,
  FreshSpawnSchema,
  GenerationSchema,
  GrantIdSchema,
  LogicalTargetIdSchema,
  OperationKind,
  OperationSchema,
  PayloadContentType,
  PayloadEnvelopeSchema,
  RuntimeGenerationRefSchema,
  RuntimeSessionIdSchema,
  SpawnClaimAcceptedSchema,
  SpawnContinuationSchema,
  SpawnGenerationClaimSchema,
  SpawnRequestSchema,
  SpawnTargetSpecSchema,
  TargetScopeKind,
  TargetScopeSchema,
  type AdapterDiagnosticReport,
  type RuntimeGenerationRef,
  type SpawnClaimAccepted,
  type SpawnRequest,
} from "@patchbay/contracts";
import {
  openAdapterDiagnostics,
  type AdapterDiagnosticInput,
  type AdapterDiagnostics,
} from "../src/adapter_diagnostics.js";
import {
  authorizeDeploymentIfRequired,
  ConfiguredDeploymentAuthorityResolver,
  DEPLOYMENT_AUTHORITY_ERROR_CODES,
  DeploymentAuthorityError,
  type DeploymentAuthorityBinding,
  type ConfiguredDeploymentTarget,
  type CredentialFreeDeploymentTarget,
  type CredentialRequiredDeploymentTarget,
  type DeploymentAuthorityRequest,
  type DeploymentAuthorityResolver,
  type DeploymentTargetIdentity,
} from "../src/deployment_authority.js";
import {
  composeAdapterDiagnostics,
  CoreDiagnosticsForwarder,
} from "../src/core_diagnostics_forwarder.js";
import { AdapterProcess } from "../src/main.js";

const encoder = new TextEncoder();
const target: DeploymentTargetIdentity = Object.freeze({
  adapterId: "pi",
  deploymentScope: "deployment-a",
  workspaceId: "workspace-a",
  projectId: "project-a",
  logicalTargetId: "logical-a",
});
const credentialRequiredTarget: CredentialRequiredDeploymentTarget = Object.freeze({
  ...target,
  credentialPolicy: "credential-required",
});
const credentialFreeTarget: CredentialFreeDeploymentTarget = Object.freeze({
  credentialPolicy: "credential-free",
  adapterId: target.adapterId,
  deploymentScope: target.deploymentScope,
  logicalTargetId: target.logicalTargetId,
});
const binding: DeploymentAuthorityBinding = Object.freeze({
  ...target,
  reference: "authority-ref-a",
  credentialHandle: "keyring://patchbay/workspace-a/key-a",
  shape: "session",
  expiresAt: new Date("2030-01-01T00:00:00.000Z"),
});
const now = new Date("2029-01-01T00:00:00.000Z");

test("configured policy distinguishes credential-required and credential-free launch paths", async () => {
  let lookups = 0;
  const resolver: DeploymentAuthorityResolver = {
    async authorize() {
      lookups += 1;
      return { credentialHandle: binding.credentialHandle };
    },
  };

  await assert.rejects(
    authorizeDeploymentIfRequired(
      resolver,
      request(freshAccepted({ reference: "" }), credentialRequiredTarget),
      now,
    ),
    authorityError("MISSING_REFERENCE"),
  );
  assert.equal(lookups, 0, "a required target must reject omission before resolver lookup");

  assert.equal(
    await authorizeDeploymentIfRequired(
      resolver,
      request(freshAccepted({ reference: "" }), credentialFreeTarget),
      now,
    ),
    undefined,
  );
  assert.equal(lookups, 0, "credential-free policy skips only the handle lookup");
  assert.equal("workspaceId" in credentialFreeTarget, false);
  assert.equal("projectId" in credentialFreeTarget, false);

  await assert.rejects(
    authorizeDeploymentIfRequired(
      resolver,
      request(freshAccepted(), credentialFreeTarget),
      now,
    ),
    authorityError("INVALID_TARGET_SPEC"),
    "credential-free policy must reject an unexpected authority reference",
  );
  assert.equal(lookups, 0);

  const builtIn = new ConfiguredDeploymentAuthorityResolver([binding]);
  const authorized = await authorizeDeploymentIfRequired(
    builtIn,
    request(freshAccepted(), credentialRequiredTarget),
    now,
  );
  assert.deepEqual(authorized, { credentialHandle: binding.credentialHandle });
  assert.equal(Object.isFrozen(authorized), true);
});

test("credential-free launch still validates fresh and continuation Grant/claim provenance", async () => {
  const credentialFreeFresh = request(
    freshAccepted({ reference: "" }),
    credentialFreeTarget,
  );
  assert.equal(
    await authorizeDeploymentIfRequired(undefined, credentialFreeFresh, now),
    undefined,
  );
  await assert.rejects(
    authorizeDeploymentIfRequired(
      undefined,
      request(freshAccepted({ reference: "", omitSpawnGrant: true }), credentialFreeTarget),
      now,
    ),
    authorityError("INVALID_CORE_EVIDENCE"),
  );

  assert.equal(
    await authorizeDeploymentIfRequired(
      undefined,
      request(continuationAccepted({ reference: "" }), credentialFreeTarget),
      now,
    ),
    undefined,
  );
  await assert.rejects(
    authorizeDeploymentIfRequired(
      undefined,
      request(
        continuationAccepted({ reference: "", omitCompoundAuthority: true }),
        credentialFreeTarget,
      ),
      now,
    ),
    authorityError("INVALID_CORE_EVIDENCE"),
  );
  await assert.rejects(
    authorizeDeploymentIfRequired(
      undefined,
      request(
        continuationAccepted({ reference: "", omitSpawnGrant: true }),
        credentialFreeTarget,
      ),
      now,
    ),
    authorityError("INVALID_CORE_EVIDENCE"),
  );
});

test("unknown, expired, revoked, and every configured target identity mismatch fail closed", async () => {
  await assert.rejects(
    new ConfiguredDeploymentAuthorityResolver([]).authorize(request(freshAccepted()), now),
    authorityError("UNKNOWN_REFERENCE"),
  );
  await assert.rejects(
    new ConfiguredDeploymentAuthorityResolver([
      { ...binding, expiresAt: new Date("2028-12-31T23:59:59.000Z") },
    ]).authorize(request(freshAccepted()), now),
    authorityError("EXPIRED_REFERENCE"),
  );
  await assert.rejects(
    new ConfiguredDeploymentAuthorityResolver([
      { ...binding, revokedAt: new Date("2028-12-31T23:59:59.000Z") },
    ]).authorize(request(freshAccepted()), now),
    authorityError("REVOKED_REFERENCE"),
  );

  const scopeMutations: readonly CredentialRequiredDeploymentTarget[] = [
    { ...credentialRequiredTarget, adapterId: "other-adapter" },
    { ...credentialRequiredTarget, deploymentScope: "deployment-b" },
    { ...credentialRequiredTarget, workspaceId: "workspace-b" },
    { ...credentialRequiredTarget, projectId: "project-b" },
  ];
  for (const mutated of scopeMutations) {
    await assert.rejects(
      resolverFor(binding).authorize(request(freshAccepted(), mutated), now),
      (error: unknown) =>
        error instanceof DeploymentAuthorityError &&
        (error.code === "SCOPE_MISMATCH" || error.code === "INVALID_CORE_EVIDENCE"),
    );
  }
  await assert.rejects(
    resolverFor(binding).authorize(
      request(freshAccepted({ shape: "worktree" })),
      now,
    ),
    authorityError("SCOPE_MISMATCH"),
  );
  await assert.rejects(
    resolverFor(binding).authorize(
      request(freshAccepted(), { ...credentialRequiredTarget, logicalTargetId: "logical-b" }),
      now,
    ),
    authorityError("INVALID_CORE_EVIDENCE"),
  );
});

test("paths and labels in opaque adapter payload cannot widen project or workspace scope", async () => {
  const hostilePayload = encoder.encode(JSON.stringify({
    workspaceId: binding.workspaceId,
    projectId: binding.projectId,
    cwd: "/raw/secret/workspace/path",
    label: "secret-project-label",
  }));
  const accepted = freshAccepted({ adapterPayload: hostilePayload });
  const mismatched = request(accepted, {
    ...credentialRequiredTarget,
    workspaceId: "workspace-b",
    projectId: "project-b",
  });

  await assert.rejects(
    resolverFor(binding).authorize(mismatched, now),
    authorityError("SCOPE_MISMATCH"),
  );
});

test("continuations require both core Grant provenance records and exact claim evidence", async () => {
  const resolver = resolverFor(binding);
  await assert.doesNotReject(resolver.authorize(request(continuationAccepted()), now));

  await assert.rejects(
    resolver.authorize(request(continuationAccepted({ omitCompoundAuthority: true })), now),
    authorityError("INVALID_CORE_EVIDENCE"),
    "the local credential must not substitute for the exact-prior Grant",
  );
  await assert.rejects(
    resolver.authorize(request(continuationAccepted({ omitSpawnGrant: true })), now),
    authorityError("INVALID_CORE_EVIDENCE"),
    "the local credential must not substitute for the adapter-scoped spawn Grant",
  );
  await assert.rejects(
    resolver.authorize(request(continuationAccepted({ claimedGeneration: 7n })), now),
    authorityError("INVALID_CORE_EVIDENCE"),
  );
  await assert.rejects(
    resolver.authorize(request(continuationAccepted({ requestedPriorGeneration: 3n })), now),
    authorityError("INVALID_CORE_EVIDENCE"),
  );
  await assert.rejects(
    resolver.authorize(request(continuationAccepted({ replacementKind: OperationKind.SPAWN })), now),
    authorityError("INVALID_CORE_EVIDENCE"),
  );
});

test("continuation rejects reused Grants and missing runtime identity before resolver lookup", async () => {
  let lookups = 0;
  const resolver: DeploymentAuthorityResolver = {
    async authorize() {
      lookups += 1;
      return { credentialHandle: binding.credentialHandle };
    },
  };

  for (const accepted of [
    continuationAccepted({ replacementGrantId: "spawn-grant-a" }),
    continuationAccepted({ runtimeSessionId: null }),
    continuationAccepted({ runtimeSessionId: "" }),
  ]) {
    await assert.rejects(
      authorizeDeploymentIfRequired(resolver, request(accepted), now),
      authorityError("INVALID_CORE_EVIDENCE"),
    );
    assert.equal(lookups, 0, "invalid provenance must fail before credential lookup");
  }

  assert.deepEqual(
    await authorizeDeploymentIfRequired(resolver, request(continuationAccepted()), now),
    { credentialHandle: binding.credentialHandle },
  );
  assert.equal(lookups, 1);
});

test("each continuation attempt rechecks current revocation state instead of caching success", async () => {
  const resolver = resolverFor(binding);
  const continuation = request(continuationAccepted());
  assert.deepEqual(await resolver.authorize(continuation, now), {
    credentialHandle: binding.credentialHandle,
  });
  assert.equal(resolver.revoke(binding.reference, new Date("2029-01-01T00:00:01.000Z")), true);
  await assert.rejects(
    resolver.authorize(continuation, new Date("2029-01-01T00:00:02.000Z")),
    authorityError("REVOKED_REFERENCE"),
  );
});

test("supervisor integration records only bounded denial metadata on every redaction surface", async () => {
  const rawPath = "/raw/private/workspace";
  const rawLabel = "private-project-label";
  const rawKeyMaterial = "raw-provider-key-material";
  const credentialHandle = binding.credentialHandle;
  const authorityRef = binding.reference;
  const records: AdapterDiagnosticInput[] = [];
  const adapter = new AdapterProcess({
    coreAddress: "http://127.0.0.1:1",
    adapterId: "pi",
    authorityDomainId: "authority-test",
    attachmentEvidence: "attachment-secret",
    adapterGeneration: 1,
    sessions: [],
    deploymentAuthorityResolver: resolverFor(binding),
    diagnostics: {
      record(input) {
        records.push(input);
      },
      flush: async () => undefined,
      close: async () => undefined,
    },
  });
  const accepted = freshAccepted({
    adapterPayload: encoder.encode(JSON.stringify({
      cwd: rawPath,
      label: rawLabel,
      key: rawKeyMaterial,
    })),
  });

  await assert.rejects(
    adapter.authorizeDeployment(
      request(accepted, { ...credentialRequiredTarget, projectId: rawLabel }),
      now,
    ),
    authorityError("SCOPE_MISMATCH"),
  );

  const serializedDiagnostics = JSON.stringify(records);
  for (const forbidden of [rawPath, rawLabel, rawKeyMaterial, credentialHandle, authorityRef]) {
    assert.equal(serializedDiagnostics.includes(forbidden), false);
  }
  assert.deepEqual(records, [{
    event: "deployment.authority.denied",
    level: "warn",
    error: { name: "DeploymentAuthorityError", code: "SCOPE_MISMATCH" },
  }]);
});

test("hostile resolver exception metadata is closed before every diagnostics surface", async () => {
  assert.equal(Object.isFrozen(DEPLOYMENT_AUTHORITY_ERROR_CODES), true);
  const directory = mkdtempSync(join(tmpdir(), "patchbay-deployment-authority-"));
  const path = join(directory, "adapter.log");
  const records: AdapterDiagnosticInput[] = [];
  const forwarded: AdapterDiagnosticReport[] = [];
  const forbidden = [
    "resolver-secret-name",
    "resolver-secret-message",
    "/resolver/private/code",
    "resolver-secret-cause",
    binding.credentialHandle,
    "/resolver/private/path",
    "resolver-private-label",
    binding.reference,
    "resolver-raw-key-material",
  ] as const;
  const hostileError = Object.assign(new Error(forbidden[1]), {
    name: forbidden[0],
    code: forbidden[2],
    cause: forbidden[3],
    credentialHandle: forbidden[4],
    path: forbidden[5],
    label: forbidden[6],
    reference: forbidden[7],
    keyMaterial: forbidden[8],
  });
  const resolver: DeploymentAuthorityResolver = {
    async authorize() {
      throw hostileError;
    },
  };
  const fileDiagnostics = await openAdapterDiagnostics({
    path,
    adapterId: "pi",
    adapterGeneration: 1,
    now: () => new Date("2026-08-14T12:00:00.000Z"),
  });
  const forwarder = new CoreDiagnosticsForwarder(
    async (report) => {
      forwarded.push(report);
      return create(AdapterDiagnosticReportResultSchema, { accepted: true });
    },
    { authorityDomainId: "authority-test", adapterId: "pi", adapterGeneration: 1 },
    { reportsPerSecond: 1_000 },
  );
  const capture: AdapterDiagnostics = {
    record(input) {
      records.push(input);
    },
    flush: async () => undefined,
    close: async () => undefined,
  };
  const diagnostics = composeAdapterDiagnostics([fileDiagnostics, forwarder, capture]);
  const adapter = new AdapterProcess({
    coreAddress: "http://127.0.0.1:1",
    adapterId: "pi",
    authorityDomainId: "authority-test",
    attachmentEvidence: "attachment-secret",
    adapterGeneration: 1,
    sessions: [],
    deploymentAuthorityResolver: resolver,
    diagnostics,
  });

  try {
    await assert.rejects(
      adapter.authorizeDeployment(request(freshAccepted()), now),
      (error: unknown) => error === hostileError,
    );
    await diagnostics.flush();

    const fileRecord = readFileSync(path, "utf8");
    const capturedSurface = JSON.stringify(records);
    const forwardedSurface = JSON.stringify(forwarded, (_key, value) =>
      typeof value === "bigint" ? value.toString() : value
    );
    for (const value of forbidden) {
      assert.equal(fileRecord.includes(value), false, `file diagnostics leaked ${value}`);
      assert.equal(capturedSurface.includes(value), false, `diagnostic input leaked ${value}`);
      assert.equal(forwardedSurface.includes(value), false, `core forwarding leaked ${value}`);
    }
    assert.deepEqual(records, [{
      event: "deployment.authority.denied",
      level: "warn",
      error: {
        name: "DeploymentAuthorityResolverError",
        code: "RESOLVER_FAILURE",
      },
    }]);
    const parsedFileRecord = JSON.parse(fileRecord) as Record<string, unknown>;
    assert.deepEqual(parsedFileRecord["error"], {
      name: "DeploymentAuthorityResolverError",
      code: "RESOLVER_FAILURE",
    });
  } finally {
    await diagnostics.close();
    rmSync(directory, { recursive: true, force: true });
  }
});

function resolverFor(candidate: DeploymentAuthorityBinding) {
  return new ConfiguredDeploymentAuthorityResolver([candidate]);
}

function request(
  acceptedSpawn: SpawnClaimAccepted,
  requestTarget: ConfiguredDeploymentTarget = credentialRequiredTarget,
): DeploymentAuthorityRequest {
  return { acceptedSpawn, target: requestTarget };
}

interface FreshAcceptedOptions {
  reference?: string;
  shape?: string;
  adapterPayload?: Uint8Array;
  omitSpawnGrant?: boolean;
}

function freshAccepted(options: FreshAcceptedOptions = {}): SpawnClaimAccepted {
  const spawnRequest = create(SpawnRequestSchema, {
    intent: { case: "fresh", value: create(FreshSpawnSchema, {}) },
    targetSpec: create(SpawnTargetSpecSchema, {
      shape: options.shape ?? binding.shape,
      deploymentAuthorityRef: options.reference ?? binding.reference,
      ...(options.adapterPayload
        ? {
            adapterPayload: create(PayloadEnvelopeSchema, {
              payload: options.adapterPayload,
              contentType: PayloadContentType.JSON,
              schemaRef: "patchbay.pi.TestTarget.v1",
            }),
          }
        : {}),
    }),
  });
  return acceptedEnvelope(
    spawnRequest,
    1n,
    undefined,
    options.omitSpawnGrant ? { omitSpawnGrant: true } : {},
  );
}

interface ContinuationAcceptedOptions {
  omitCompoundAuthority?: boolean;
  omitSpawnGrant?: boolean;
  claimedGeneration?: bigint;
  replacementKind?: OperationKind;
  replacementGrantId?: string;
  requestedPriorGeneration?: bigint;
  runtimeSessionId?: string | null;
  reference?: string;
}

function continuationAccepted(options: ContinuationAcceptedOptions = {}): SpawnClaimAccepted {
  const runtimeSessionId = options.runtimeSessionId === undefined
    ? "runtime-a"
    : options.runtimeSessionId;
  const prior = runtimeGenerationRef(4n, runtimeSessionId);
  const requestedPrior = runtimeGenerationRef(
    options.requestedPriorGeneration ?? 4n,
    runtimeSessionId,
  );
  const spawnRequest = create(SpawnRequestSchema, {
    intent: {
      case: "continuation",
      value: create(SpawnContinuationSchema, { prior: requestedPrior }),
    },
    targetSpec: create(SpawnTargetSpecSchema, {
      shape: binding.shape,
      deploymentAuthorityRef: options.reference ?? binding.reference,
    }),
  });
  return acceptedEnvelope(
    spawnRequest,
    options.claimedGeneration ?? 5n,
    prior,
    options,
  );
}

function acceptedEnvelope(
  spawnRequest: SpawnRequest,
  claimedGeneration: bigint,
  expectedPrior?: RuntimeGenerationRef,
  options: ContinuationAcceptedOptions = {},
): SpawnClaimAccepted {
  const commandId = create(CommandIdSchema, { value: "spawn-command-a" });
  return create(SpawnClaimAcceptedSchema, {
    acceptedOperation: create(AcceptedOperationSchema, {
      operation: create(OperationSchema, {
        commandId,
        authorityDomainId: create(AuthorityDomainIdSchema, { value: "authority-a" }),
        kind: OperationKind.SPAWN,
        targetScope: create(TargetScopeSchema, {
          kind: TargetScopeKind.ADAPTER,
          adapterId: create(AdapterIdSchema, { value: target.adapterId }),
        }),
        payload: create(PayloadEnvelopeSchema, {
          payload: toBinary(SpawnRequestSchema, spawnRequest),
          contentType: PayloadContentType.PROTOBUF,
          schemaRef: "patchbay.SpawnRequest",
        }),
      }),
      ...(options.omitSpawnGrant
        ? {}
        : { authorizingGrantId: create(GrantIdSchema, { value: "spawn-grant-a" }) }),
    }),
    claim: create(SpawnGenerationClaimSchema, {
      authorityDomainId: create(AuthorityDomainIdSchema, { value: "authority-a" }),
      claimOperationId: commandId,
      logicalTargetId: create(LogicalTargetIdSchema, { value: target.logicalTargetId }),
      ...(expectedPrior ? { expectedPrior } : {}),
      claimedGeneration: create(GenerationSchema, { value: claimedGeneration }),
    }),
    ...(expectedPrior && !options.omitCompoundAuthority
      ? {
          compoundAuthority: create(ContinuationAuthorityProvenanceSchema, {
            exactPrior: expectedPrior,
            replacementGrantId: create(GrantIdSchema, {
              value: options.replacementGrantId ?? "replacement-grant-a",
            }),
            replacementAuthorityKind:
              options.replacementKind ?? OperationKind.SESSION_MANAGEMENT,
          }),
        }
      : {}),
  });
}

function runtimeGenerationRef(generation: bigint, runtimeSessionId: string | null) {
  return create(RuntimeGenerationRefSchema, {
    logicalTargetId: create(LogicalTargetIdSchema, { value: target.logicalTargetId }),
    externalRuntime: create(ExternalRuntimeRefSchema, {
      adapterId: create(AdapterIdSchema, { value: target.adapterId }),
      deploymentScope: target.deploymentScope,
      ...(runtimeSessionId === null
        ? {}
        : { runtimeSessionId: create(RuntimeSessionIdSchema, { value: runtimeSessionId }) }),
      generation: create(GenerationSchema, { value: generation }),
    }),
  });
}

function authorityError(code: DeploymentAuthorityError["code"]) {
  return (error: unknown) => error instanceof DeploymentAuthorityError && error.code === code;
}
