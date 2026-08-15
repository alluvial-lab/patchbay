import { fromBinary } from "@bufbuild/protobuf";
import {
  OperationKind,
  PayloadContentType,
  SpawnRequestSchema,
  TargetScopeKind,
  type ContinuationAuthorityProvenance,
  type RuntimeGenerationRef,
  type SpawnClaimAccepted,
  type SpawnGenerationClaim,
  type SpawnTargetSpec,
} from "@patchbay/contracts";

export interface DeploymentAuthorityResolver {
  authorize(
    request: DeploymentAuthorityRequest,
    now: Date,
  ): Promise<{ readonly credentialHandle: string }>;
}

/** Adapter-configured identities only. Paths and display labels are deliberately absent. */
export interface DeploymentTargetIdentity {
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly workspaceId: string;
  readonly projectId: string;
  readonly logicalTargetId: string;
}

export interface CredentialRequiredDeploymentTarget extends DeploymentTargetIdentity {
  readonly credentialPolicy: "credential-required";
}

/** Credential-free policy still binds the core target, but needs no Workspace/project object. */
export interface CredentialFreeDeploymentTarget {
  readonly credentialPolicy: "credential-free";
  readonly adapterId: string;
  readonly deploymentScope: string;
  readonly logicalTargetId: string;
}

export type ConfiguredDeploymentTarget =
  | CredentialRequiredDeploymentTarget
  | CredentialFreeDeploymentTarget;

export interface DeploymentAuthorityRequest {
  readonly acceptedSpawn: SpawnClaimAccepted;
  readonly target: ConfiguredDeploymentTarget;
}

export interface DeploymentAuthorityBinding extends DeploymentTargetIdentity {
  readonly reference: string;
  readonly credentialHandle: string;
  readonly shape: string;
  readonly expiresAt?: Date;
  readonly revokedAt?: Date;
}

export const DEPLOYMENT_AUTHORITY_ERROR_CODES = Object.freeze([
  "MISSING_REFERENCE",
  "UNKNOWN_REFERENCE",
  "EXPIRED_REFERENCE",
  "REVOKED_REFERENCE",
  "SCOPE_MISMATCH",
  "INVALID_CORE_EVIDENCE",
  "INVALID_TARGET_SPEC",
] as const);

export type DeploymentAuthorityErrorCode =
  (typeof DEPLOYMENT_AUTHORITY_ERROR_CODES)[number];

/** Safe for diagnostics: neither message nor code contains request/configuration values. */
export class DeploymentAuthorityError extends Error {
  readonly code: DeploymentAuthorityErrorCode;

  constructor(code: DeploymentAuthorityErrorCode) {
    super(deploymentAuthorityErrorMessage(code));
    this.name = "DeploymentAuthorityError";
    this.code = code;
  }
}

/**
 * Mutable adapter-local keyring. Every authorize call reads current binding state;
 * successful decisions are never cached across fresh or continuation attempts.
 */
export class ConfiguredDeploymentAuthorityResolver implements DeploymentAuthorityResolver {
  readonly #bindings = new Map<string, DeploymentAuthorityBinding>();

  constructor(bindings: readonly DeploymentAuthorityBinding[]) {
    for (const binding of bindings) {
      const copy = validatedBinding(binding);
      if (this.#bindings.has(copy.reference)) {
        throw new Error("deployment authority references must be unique");
      }
      this.#bindings.set(copy.reference, copy);
    }
  }

  async authorize(
    request: DeploymentAuthorityRequest,
    now: Date,
  ): Promise<{ readonly credentialHandle: string }> {
    const { target, evidence } = validatedDeploymentRequest(request, now);
    if (target.credentialPolicy !== "credential-required") {
      throw new DeploymentAuthorityError("INVALID_TARGET_SPEC");
    }
    const reference = evidence.targetSpec.deploymentAuthorityRef;

    // This lookup is intentionally inside every call. Revocation and expiry are
    // launch-time adapter preconditions, never inherited continuation authority.
    const binding = this.#bindings.get(reference);
    if (!binding) throw new DeploymentAuthorityError("UNKNOWN_REFERENCE");
    if (binding.revokedAt && binding.revokedAt.getTime() <= now.getTime()) {
      throw new DeploymentAuthorityError("REVOKED_REFERENCE");
    }
    if (binding.expiresAt && binding.expiresAt.getTime() <= now.getTime()) {
      throw new DeploymentAuthorityError("EXPIRED_REFERENCE");
    }
    if (!bindingMatches(binding, target, evidence.targetSpec)) {
      throw new DeploymentAuthorityError("SCOPE_MISMATCH");
    }

    return Object.freeze({ credentialHandle: binding.credentialHandle });
  }

  /** Adapter-local revocation. Existing core Grants are neither read nor changed. */
  revoke(reference: string, revokedAt: Date): boolean {
    if (!Number.isFinite(revokedAt.getTime())) throw new Error("revokedAt must be a valid date");
    const binding = this.#bindings.get(reference);
    if (!binding) return false;
    this.#bindings.set(reference, Object.freeze({ ...binding, revokedAt: new Date(revokedAt) }));
    return true;
  }
}

/**
 * Supervisor integration point. Credential-free target specs require no local
 * Workspace object; credential-bearing specs always fail closed through a resolver.
 */
export async function authorizeDeploymentIfRequired(
  resolver: DeploymentAuthorityResolver | undefined,
  request: DeploymentAuthorityRequest,
  now: Date,
): Promise<{ readonly credentialHandle: string } | undefined> {
  const { target } = validatedDeploymentRequest(request, now);
  if (target.credentialPolicy === "credential-free") return undefined;
  if (!resolver) throw new DeploymentAuthorityError("UNKNOWN_REFERENCE");
  return resolver.authorize(request, now);
}

interface ValidatedDeploymentRequest {
  readonly target: ConfiguredDeploymentTarget;
  readonly evidence: ValidatedSpawnEvidence;
}

function validatedDeploymentRequest(
  request: DeploymentAuthorityRequest,
  now: Date,
): ValidatedDeploymentRequest {
  if (!Number.isFinite(now.getTime())) {
    throw new DeploymentAuthorityError("INVALID_CORE_EVIDENCE");
  }
  const target = validatedConfiguredTarget(request.target);
  const evidence = validatedSpawnEvidence(request.acceptedSpawn, target);
  const reference = evidence.targetSpec.deploymentAuthorityRef;
  if (target.credentialPolicy === "credential-required" && !reference) {
    throw new DeploymentAuthorityError("MISSING_REFERENCE");
  }
  if (target.credentialPolicy === "credential-free" && reference) {
    throw new DeploymentAuthorityError("INVALID_TARGET_SPEC");
  }
  return { target, evidence };
}

interface ValidatedSpawnEvidence {
  readonly targetSpec: SpawnTargetSpec;
}

function validatedSpawnEvidence(
  acceptedSpawn: SpawnClaimAccepted,
  target: ConfiguredDeploymentTarget,
): ValidatedSpawnEvidence {
  const accepted = acceptedSpawn.acceptedOperation;
  const operation = accepted?.operation;
  const claim = acceptedSpawn.claim;
  const spawnGrantId = accepted?.authorizingGrantId?.value;
  if (
    !spawnGrantId ||
    !operation?.commandId?.value ||
    operation.kind !== OperationKind.SPAWN ||
    !claim?.claimOperationId?.value ||
    claim.claimOperationId.value !== operation.commandId.value ||
    !claim.authorityDomainId?.value ||
    !claim.logicalTargetId?.value ||
    claim.logicalTargetId.value !== target.logicalTargetId ||
    !claim.claimedGeneration?.value ||
    claim.claimedGeneration.value <= 0n ||
    operation.authorityDomainId?.value !== claim.authorityDomainId.value ||
    operation.targetScope?.kind !== TargetScopeKind.ADAPTER ||
    operation.targetScope.adapterId?.value !== target.adapterId
  ) {
    throw new DeploymentAuthorityError("INVALID_CORE_EVIDENCE");
  }

  const spawnRequest = decodeSpawnRequest(acceptedSpawn);
  const targetSpec = spawnRequest.targetSpec;
  if (!targetSpec?.shape) throw new DeploymentAuthorityError("INVALID_TARGET_SPEC");
  if (spawnRequest.intent.case === "fresh") {
    if (claim.expectedPrior || acceptedSpawn.compoundAuthority || claim.claimedGeneration.value !== 1n) {
      throw new DeploymentAuthorityError("INVALID_CORE_EVIDENCE");
    }
  } else if (spawnRequest.intent.case === "continuation") {
    validateContinuationEvidence(
      claim,
      acceptedSpawn.compoundAuthority,
      spawnGrantId,
      target,
      spawnRequest.intent.value.prior,
    );
  } else {
    throw new DeploymentAuthorityError("INVALID_TARGET_SPEC");
  }

  return { targetSpec };
}

function validateContinuationEvidence(
  claim: SpawnGenerationClaim,
  authority: ContinuationAuthorityProvenance | undefined,
  spawnGrantId: string,
  target: ConfiguredDeploymentTarget,
  requestedPrior: RuntimeGenerationRef | undefined,
): void {
  const prior = claim.expectedPrior;
  const runtime = prior?.externalRuntime;
  if (
    !prior?.logicalTargetId?.value ||
    !runtime?.adapterId?.value ||
    !runtime.deploymentScope ||
    !runtime.runtimeSessionId?.value ||
    !runtime.generation?.value ||
    runtime.generation.value <= 0n ||
    !authority?.replacementGrantId?.value ||
    authority.replacementGrantId.value === spawnGrantId ||
    authority.replacementAuthorityKind !== OperationKind.SESSION_MANAGEMENT ||
    !sameRuntimeGenerationRef(prior, requestedPrior) ||
    !sameRuntimeGenerationRef(prior, authority.exactPrior) ||
    prior.logicalTargetId.value !== claim.logicalTargetId?.value ||
    runtime.adapterId.value !== target.adapterId ||
    runtime.deploymentScope !== target.deploymentScope ||
    claim.claimedGeneration?.value !== runtime.generation.value + 1n
  ) {
    throw new DeploymentAuthorityError("INVALID_CORE_EVIDENCE");
  }
}

function decodeSpawnRequest(acceptedSpawn: SpawnClaimAccepted) {
  const payload = acceptedSpawn.acceptedOperation?.operation?.payload;
  if (
    !payload ||
    payload.contentType !== PayloadContentType.PROTOBUF ||
    payload.schemaRef !== "patchbay.SpawnRequest"
  ) {
    throw new DeploymentAuthorityError("INVALID_TARGET_SPEC");
  }
  try {
    return fromBinary(SpawnRequestSchema, payload.payload);
  } catch {
    throw new DeploymentAuthorityError("INVALID_TARGET_SPEC");
  }
}

function sameRuntimeGenerationRef(
  left: RuntimeGenerationRef,
  right: RuntimeGenerationRef | undefined,
): boolean {
  return (
    !!right &&
    left.logicalTargetId?.value === right.logicalTargetId?.value &&
    left.externalRuntime?.adapterId?.value === right.externalRuntime?.adapterId?.value &&
    left.externalRuntime?.deploymentScope === right.externalRuntime?.deploymentScope &&
    left.externalRuntime?.runtimeSessionId?.value === right.externalRuntime?.runtimeSessionId?.value &&
    left.externalRuntime?.generation?.value === right.externalRuntime?.generation?.value
  );
}

function bindingMatches(
  binding: DeploymentAuthorityBinding,
  target: CredentialRequiredDeploymentTarget,
  targetSpec: SpawnTargetSpec,
): boolean {
  return (
    binding.adapterId === target.adapterId &&
    binding.deploymentScope === target.deploymentScope &&
    binding.workspaceId === target.workspaceId &&
    binding.projectId === target.projectId &&
    binding.logicalTargetId === target.logicalTargetId &&
    binding.shape === targetSpec.shape
  );
}

function validatedConfiguredTarget(
  target: ConfiguredDeploymentTarget | undefined,
): ConfiguredDeploymentTarget {
  if (!target) throw new DeploymentAuthorityError("SCOPE_MISMATCH");
  for (const value of [target.adapterId, target.deploymentScope, target.logicalTargetId]) {
    if (!isBoundedIdentityValue(value)) {
      throw new DeploymentAuthorityError("SCOPE_MISMATCH");
    }
  }
  if (target.credentialPolicy === "credential-required") {
    if (!isBoundedIdentityValue(target.workspaceId) || !isBoundedIdentityValue(target.projectId)) {
      throw new DeploymentAuthorityError("SCOPE_MISMATCH");
    }
    return target;
  }
  if (target.credentialPolicy === "credential-free") return target;
  throw new DeploymentAuthorityError("INVALID_TARGET_SPEC");
}

function isBoundedIdentityValue(value: unknown): value is string {
  return typeof value === "string" && value.length > 0 && value.length <= 1_024;
}

function validatedBinding(binding: DeploymentAuthorityBinding): DeploymentAuthorityBinding {
  for (const value of [
    binding.reference,
    binding.credentialHandle,
    binding.adapterId,
    binding.deploymentScope,
    binding.workspaceId,
    binding.projectId,
    binding.logicalTargetId,
    binding.shape,
  ]) {
    if (!value || value.length > 1_024) {
      throw new Error("deployment authority binding fields must be non-empty and bounded");
    }
  }
  if (binding.expiresAt && !Number.isFinite(binding.expiresAt.getTime())) {
    throw new Error("deployment authority expiry must be a valid date");
  }
  if (binding.revokedAt && !Number.isFinite(binding.revokedAt.getTime())) {
    throw new Error("deployment authority revocation must be a valid date");
  }
  return Object.freeze({
    reference: binding.reference,
    credentialHandle: binding.credentialHandle,
    adapterId: binding.adapterId,
    deploymentScope: binding.deploymentScope,
    workspaceId: binding.workspaceId,
    projectId: binding.projectId,
    logicalTargetId: binding.logicalTargetId,
    shape: binding.shape,
    ...(binding.expiresAt ? { expiresAt: new Date(binding.expiresAt) } : {}),
    ...(binding.revokedAt ? { revokedAt: new Date(binding.revokedAt) } : {}),
  });
}

function deploymentAuthorityErrorMessage(code: DeploymentAuthorityErrorCode): string {
  switch (code) {
    case "MISSING_REFERENCE":
      return "deployment authority reference is required";
    case "UNKNOWN_REFERENCE":
      return "deployment authority reference is not configured";
    case "EXPIRED_REFERENCE":
      return "deployment authority reference has expired";
    case "REVOKED_REFERENCE":
      return "deployment authority reference is revoked";
    case "SCOPE_MISMATCH":
      return "deployment authority scope does not match the configured target";
    case "INVALID_CORE_EVIDENCE":
      return "accepted spawn evidence is incomplete or inconsistent";
    case "INVALID_TARGET_SPEC":
      return "accepted spawn target specification is invalid";
  }
}
