import { TargetScopeKind, type TargetScope } from "@patchbay/contracts";

import type { ResourceIdentityView } from "../domain/model.js";

export function formatTargetScope(scope: TargetScope | undefined): string {
  if (!scope) return "unknown scope";
  switch (scope.kind) {
    case TargetScopeKind.AUTHORITY_DOMAIN:
      return hasAnyTargetFields(scope) ? "incomplete authority-domain scope" : "authority domain";
    case TargetScopeKind.FLEET_SUPERVISOR:
      return hasAnyTargetFields(scope) ? "incomplete fleet/supervisor scope" : "fleet/supervisor";
    case TargetScopeKind.ADAPTER:
      return scope.adapterId?.value && !hasFieldsOtherThan(scope, "adapter")
        ? `adapter ${scope.adapterId.value}`
        : "incomplete adapter scope";
    case TargetScopeKind.RUNTIME_SESSION:
      return scope.adapterId?.value
          && scope.deploymentScope
          && scope.runtimeSessionId?.value
          && scope.sessionGeneration?.value !== undefined
          && !hasFieldsOtherThan(scope, "runtime")
        ? `${scope.adapterId.value}/${scope.deploymentScope}/${scope.runtimeSessionId.value}/gen-${scope.sessionGeneration.value}`
        : "incomplete runtime-session scope";
    case TargetScopeKind.RESOURCE: {
      const identity = !hasFieldsOtherThan(scope, "resource") ? resourceIdentity(scope) : undefined;
      return identity
        ? `adapter=${identity.adapterId};resource-kind=${identity.resourceKind};resource=${identity.resourceId}`
        : "incomplete operational-resource scope";
    }
    case TargetScopeKind.ACTOR:
      return scope.actorId?.value ? `actor ${scope.actorId.value}` : "incomplete actor scope";
    case TargetScopeKind.PROJECT_SESSION_GROUP:
      return scope.projectOrGroup ? `project/session group ${scope.projectOrGroup}` : "incomplete project/session-group scope";
    case TargetScopeKind.UNSPECIFIED:
    default:
      return `scope kind ${scope.kind}`;
  }
}

/** Explanatory only. Core grant evaluation remains authoritative. */
export function scopeMayContainResource(
  scope: TargetScope | undefined,
  identity: ResourceIdentityView,
): boolean {
  if (!scope) return false;
  switch (scope.kind) {
    case TargetScopeKind.AUTHORITY_DOMAIN:
    case TargetScopeKind.FLEET_SUPERVISOR:
      return !hasAnyTargetFields(scope);
    case TargetScopeKind.ADAPTER:
      return !hasFieldsOtherThan(scope, "adapter") && scope.adapterId?.value === identity.adapterId;
    case TargetScopeKind.RESOURCE: {
      const scoped = !hasFieldsOtherThan(scope, "resource") ? resourceIdentity(scope) : undefined;
      return Boolean(
        scoped
        && scoped.adapterId === identity.adapterId
        && scoped.resourceKind === identity.resourceKind
        && scoped.resourceId === identity.resourceId,
      );
    }
    default:
      return false;
  }
}

function hasAnyTargetFields(scope: TargetScope): boolean {
  return Boolean(
    scope.actorId
    || scope.adapterId
    || scope.runtimeSessionId
    || scope.sessionGeneration
    || scope.deploymentScope
    || scope.projectOrGroup
    || scope.legacyAuditResourceId
    || scope.resource,
  );
}

function hasFieldsOtherThan(
  scope: TargetScope,
  allowed: "adapter" | "runtime" | "resource",
): boolean {
  if (scope.actorId || scope.projectOrGroup || scope.legacyAuditResourceId) return true;
  if (allowed === "adapter") {
    return Boolean(scope.runtimeSessionId || scope.sessionGeneration || scope.deploymentScope || scope.resource);
  }
  if (allowed === "runtime") return Boolean(scope.resource);
  return Boolean(scope.adapterId || scope.runtimeSessionId || scope.sessionGeneration || scope.deploymentScope);
}

function resourceIdentity(scope: TargetScope): ResourceIdentityView | undefined {
  const resource = scope.resource;
  if (
    !resource?.adapterId?.value
    || !resource.resourceKind?.value
    || !resource.resourceId?.value
  ) return undefined;
  return {
    adapterId: resource.adapterId.value,
    resourceKind: resource.resourceKind.value,
    resourceId: resource.resourceId.value,
  };
}
