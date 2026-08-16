import assert from "node:assert/strict";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import {
  AdapterAssuranceManifestSchema,
  AdapterAssuranceManifestV1Schema,
  AdapterCapabilitySummarySchema,
  AdapterReconciliationStrength,
  IdempotencyStrength,
  ReconciliationAction,
  SubmissionOutcome,
} from "@patchbay/contracts";
import { submissionOutcomeQualifier } from "../src/index.js";

function capability(action: ReconciliationAction) {
  return create(AdapterCapabilitySummarySchema, {
    assurance: create(AdapterAssuranceManifestSchema, {
      contract: {
        case: "v1",
        value: create(AdapterAssuranceManifestV1Schema, {
          deduplicationStrength: IdempotencyStrength.NONE,
          continuationProofSupport: false,
          cursorSupport: false,
          generationFenceSupport: false,
          reconciliationStrength: AdapterReconciliationStrength.NONE,
          unprovenOutcomeAction: action,
        }),
      },
    }),
  });
}

test("submission unknown consumes the generated action and uncertainty defaults conservatively", () => {
  assert.equal(
    submissionOutcomeQualifier(
      SubmissionOutcome.UNKNOWN,
      capability(ReconciliationAction.NONE),
    ),
    "unknown",
  );
  assert.equal(
    submissionOutcomeQualifier(
      SubmissionOutcome.UNKNOWN,
      capability(ReconciliationAction.MANUAL_REQUIRED),
    ),
    "manual-required",
  );
  assert.equal(
    submissionOutcomeQualifier(SubmissionOutcome.UNKNOWN, undefined),
    "manual-required",
  );
  assert.equal(
    submissionOutcomeQualifier(
      SubmissionOutcome.ACCEPTED,
      capability(ReconciliationAction.MANUAL_REQUIRED),
    ),
    undefined,
    "capability alone cannot qualify a proven submission outcome",
  );
});
