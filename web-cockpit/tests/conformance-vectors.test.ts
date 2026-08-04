import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import path from "node:path";
import test from "node:test";

const RUNNER = "web-cockpit" as const;

interface ImplementationCheck {
  runner: string;
  case: string;
}

interface ConformanceVector {
  vector_id: string;
  property_id: string;
  promotion_status: string;
  implementation_checks?: readonly ImplementationCheck[];
  input: unknown;
  expected_outcome: unknown;
}

interface RequestedCheck {
  vector_id: string;
  case: string;
}

function vectorsForRunner(): ReadonlyMap<string, ConformanceVector> {
  const directory = path.resolve(process.cwd(), "../contracts/vectors");
  const vectors = readdirSync(directory)
    .filter((filename) => filename.endsWith(".json"))
    .sort()
    .map((filename) => JSON.parse(readFileSync(path.join(directory, filename), "utf8")) as ConformanceVector);
  return new Map(vectors.map((vector) => [vector.vector_id, vector]));
}

function requestedChecks(): readonly RequestedCheck[] {
  return process.env.PATCHBAY_CONFORMANCE_REQUESTS
    ? JSON.parse(process.env.PATCHBAY_CONFORMANCE_REQUESTS) as RequestedCheck[]
    : [];
}

async function executeVectorCase(vector: ConformanceVector, caseName: string): Promise<void> {
  void vector.property_id;
  void vector.promotion_status;
  void vector.input;
  void vector.expected_outcome;
  throw new Error(`unhandled ${RUNNER} conformance case ${vector.vector_id}:${caseName}`);
}

test("conformance vector runner", async () => {
  const vectors = vectorsForRunner();
  for (const request of requestedChecks()) {
    const vector = vectors.get(request.vector_id);
    assert.ok(vector, `unknown vector id ${request.vector_id}`);
    assert.ok(
      vector.implementation_checks?.some((check) => check.runner === RUNNER && check.case === request.case),
      `unregistered requested check ${request.vector_id}:${request.case}`,
    );
    await executeVectorCase(vector, request.case);
    console.log(`PATCHBAY_CONFORMANCE_EXECUTED=${request.vector_id}:${request.case}`);
  }
});
