---
id: idea-proto-prose-registry-consistency-check
kind: backlog
created: 2026-07-09
updated: 2026-07-09
tags: [protocol, verification, foundation, testing]
research_refs: []
---

# Backlog: Automated check for prose-registry ↔ proto enum drift

Filed from the epic-level deep review of `epic-foundation-hardening`. The three existing contract checks (`contracts/scripts/check-vectors.mjs`, `check-generated-drift.mjs`, `check-models.mjs`) all pass while a known drift exists: `execution_outcome_unknown` is canonical in the `docs/PROTOCOL.md` failure vocabulary (`docs/PROTOCOL.md:356`) and load-bearing in `docs/UX.md` retry-safety, but the generated `FailureCode` proto enum (`contracts/proto/patchbay/operations.proto`) had no matching value until `story-fix-failurecode-execution-outcome-unknown` closes it.

## The gap

- `check-generated-drift.mjs` only verifies generated Rust/TS matches `.proto` (gen-vs-proto). It does NOT verify `.proto` matches the prose SSOT registries.
- `check-models.mjs` checks model-property traceability against `docs/VERIFICATION.md`, not proto-vs-prose.
- `check-vectors.mjs` checks vector metadata against model properties, not proto-vs-prose.

So a later feature that adds a failure-vocabulary term (or any registry value) to `docs/PROTOCOL.md` without updating the `.proto` enum will drift silently — exactly the temporal cross-child failure that the epic-level review caught. `feature-protocol-idl-and-conformance` already noted that the CI script catches missing properties but not naming drift, leaving review as the mitigation (`.work/active/features/feature-protocol-idl-and-conformance.md:151`).

## Proposal

Add or extend a check that parses the canonical prose registries from `docs/PROTOCOL.md` (OperationKind, OperationState/CommandState, SessionState axes, ElicitationState, SubmissionOutcome, response_contract.contract_kind, FailureCode) and asserts the corresponding `.proto` enums contain a matching value for every committed (non-reserved) prose entry — and that reserved prose entries map to reserved proto values. This closes the class so it cannot recur after any later prose registry edit.

## Scope / timing

Not blocking the foundation-hardening epic (the one known instance is tracked by `story-fix-failurecode-execution-outcome-unknown`). Worth picking up alongside or shortly after that fix, while the registry set is still small and the mapping is cheap to encode. Defer if a central registry generator (the Q4=C alternative from `feature-verification-contract-authority`) is later promoted, which would subsume this check.
