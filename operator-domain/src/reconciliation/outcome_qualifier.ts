import {
  AdapterReconciliationStrength,
  IdempotencyStrength,
  ReconciliationAction,
  SubmissionOutcome,
  type AdapterAssuranceManifestV1,
  type AdapterCapabilitySummary,
} from "@patchbay/contracts";

export type UnknownOutcomeQualifier = "unknown" | "manual-required";

export function adapterAssuranceV1(
  capability: AdapterCapabilitySummary | undefined,
): AdapterAssuranceManifestV1 | undefined {
  const contract = capability?.assurance?.contract;
  if (contract?.case !== "v1") return undefined;
  const assurance = contract.value;
  if (assurance.continuationProofSupport === undefined
      || assurance.cursorSupport === undefined
      || assurance.generationFenceSupport === undefined
      || !knownNonSentinel(IdempotencyStrength, assurance.deduplicationStrength)
      || !knownNonSentinel(AdapterReconciliationStrength, assurance.reconciliationStrength)
      || !knownNonSentinel(ReconciliationAction, assurance.unprovenOutcomeAction)) {
    return undefined;
  }
  return assurance;
}

export function unprovenOutcomeQualifier(
  capability: AdapterCapabilitySummary | undefined,
): UnknownOutcomeQualifier {
  const assurance = adapterAssuranceV1(capability);
  if (!assurance) return "manual-required";
  return assurance.unprovenOutcomeAction === ReconciliationAction.NONE
    ? "unknown"
    : "manual-required";
}

export function submissionOutcomeQualifier(
  outcome: SubmissionOutcome,
  capability: AdapterCapabilitySummary | undefined,
): UnknownOutcomeQualifier | undefined {
  return outcome === SubmissionOutcome.UNKNOWN
    ? unprovenOutcomeQualifier(capability)
    : undefined;
}

function knownNonSentinel(
  registry: Record<string | number, string | number>,
  value: number,
): boolean {
  return value !== 0 && typeof registry[value] === "string";
}
