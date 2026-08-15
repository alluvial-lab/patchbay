import assert from "node:assert/strict";
import test from "node:test";
import { create, toBinary } from "@bufbuild/protobuf";
import {
  AcceptedOperationSchema,
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
  type RuntimeGenerationRef,
  type SpawnClaimAccepted,
  type SpawnRequest,
} from "@patchbay/contracts";
import type { AdapterDiagnosticInput } from "../src/adapter_diagnostics.js";
import {
  authorizeDeploymentIfRequired,
  ConfiguredDeploymentAuthorityResolver,
  DeploymentAuthorityError,
  type DeploymentAuthorityBinding,
  type DeploymentAuthorityRequest,
  type DeploymentTargetIdentity,
} from "../src/deployment_authority.js";
import { AdapterProcess } from "../src/main.js";

const encoder = new TextEncoder();
const target: DeploymentTargetIdentity = Object.freeze({
  adapterId: "pi",
  deploymentScope: "deployment-a",
  workspaceId: "workspace-a",
  projectId: "project-a",
  logicalTargetId: "logical-a",
});
const binding: DeploymentAuthorityBinding = Object.freeze({
  ...target,
  reference: "authority-ref-a",
  credentialHandle: "keyring://patchbay/workspace-a/key-a",
  shape: "session",
  expiresAt: new Date("2030-01-01T00:00:00.000Z"),
});
const now = new Date("2029-01-01T00:00:00.000Z");

test("exact configured scope returns only a credential handle and credential-free specs need no workspace", async () => {
  const resolver = new ConfiguredDeploymentAuthorityResolver([binding]);
  const authorized = await resolver.authorize(request(freshAccepted()), now);
  assert.deepEqual(authorized, { credentialHandle: binding.credentialHandle });
  assert.equal(Object.isFrozen(authorized), true);

  const credentialFree: DeploymentAuthorityRequest = {
    acceptedSpawn: freshAccepted({ reference: "" }),
  };
  assert.equal(await authorizeDeploymentIfRequired(undefined, credentialFree, now), undefined);
  await assert.rejects(
    resolver.authorize(request(credentialFree.acceptedSpawn), now),
    authorityError("MISSING_REFERENCE"),
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

  const scopeMutations: readonly DeploymentTargetIdentity[] = [
    { ...target, adapterId: "other-adapter" },
    { ...target, deploymentScope: "deployment-b" },
    { ...target, workspaceId: "workspace-b" },
    { ...target, projectId: "project-b" },
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
      request(freshAccepted(), { ...target, logicalTargetId: "logical-b" }),
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
    ...target,
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
      request(accepted, { ...target, projectId: rawLabel }),
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

function resolverFor(candidate: DeploymentAuthorityBinding) {
  return new ConfiguredDeploymentAuthorityResolver([candidate]);
}

function request(
  acceptedSpawn: SpawnClaimAccepted,
  requestTarget: DeploymentTargetIdentity = target,
): DeploymentAuthorityRequest {
  return { acceptedSpawn, target: requestTarget };
}

interface FreshAcceptedOptions {
  reference?: string;
  shape?: string;
  adapterPayload?: Uint8Array;
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
  return acceptedEnvelope(spawnRequest, 1n);
}

interface ContinuationAcceptedOptions {
  omitCompoundAuthority?: boolean;
  omitSpawnGrant?: boolean;
  claimedGeneration?: bigint;
  replacementKind?: OperationKind;
  requestedPriorGeneration?: bigint;
}

function continuationAccepted(options: ContinuationAcceptedOptions = {}): SpawnClaimAccepted {
  const prior = runtimeGenerationRef(4n);
  const requestedPrior = runtimeGenerationRef(options.requestedPriorGeneration ?? 4n);
  const spawnRequest = create(SpawnRequestSchema, {
    intent: {
      case: "continuation",
      value: create(SpawnContinuationSchema, { prior: requestedPrior }),
    },
    targetSpec: create(SpawnTargetSpecSchema, {
      shape: binding.shape,
      deploymentAuthorityRef: binding.reference,
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
            replacementGrantId: create(GrantIdSchema, { value: "replacement-grant-a" }),
            replacementAuthorityKind:
              options.replacementKind ?? OperationKind.SESSION_MANAGEMENT,
          }),
        }
      : {}),
  });
}

function runtimeGenerationRef(generation: bigint) {
  return create(RuntimeGenerationRefSchema, {
    logicalTargetId: create(LogicalTargetIdSchema, { value: target.logicalTargetId }),
    externalRuntime: create(ExternalRuntimeRefSchema, {
      adapterId: create(AdapterIdSchema, { value: target.adapterId }),
      deploymentScope: target.deploymentScope,
      runtimeSessionId: create(RuntimeSessionIdSchema, { value: "runtime-a" }),
      generation: create(GenerationSchema, { value: generation }),
    }),
  });
}

function authorityError(code: DeploymentAuthorityError["code"]) {
  return (error: unknown) => error instanceof DeploymentAuthorityError && error.code === code;
}
