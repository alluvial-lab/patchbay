# Patchbay v0 conformance vectors

This directory contains draft JSON conformance vectors for the v0 protocol contract package. Each `*.json` file is one executable example that constrains a specific protocol behavior and traces it to a formal-model or stated-normative property.

## Vector envelope

Each vector uses this envelope:

```json
{
  "vector_id": "unique-kebab-case-id",
  "property_id": "PropertyIdFromVerificationOrModel",
  "promotion_status": "draft",
  "implementation_checks": [
    { "runner": "rust-core", "case": "registered-case-name" }
  ],
  "mutation_witnesses": [
    {
      "mutation_id": "claim-breaking-mutant",
      "runner": "token-commune-adapter",
      "invariant": "The independent oracle rejects this broken behavior."
    }
  ],
  "proto_fields_constrained": ["patchbay.Message.field"],
  "description": "human-readable behavior exercised",
  "input": {},
  "expected_outcome": {},
  "invariant_check": "short property/invariant explanation"
}
```

- `vector_id`: Stable unique id, normally matching the filename without `.json`.
- `property_id`: A property id from `docs/VERIFICATION.md` or a seed model under `specs/seed/`. Boundary-only examples with no formal property use a descriptive draft id such as `boundary-validation`.
- `promotion_status`: `draft` or `promoted`. Promotion requires review, a property-specific static expectation checker, and at least one successful implementation check. A promoted vector is authority only for the executable example, not for invariants or wire shape.
- `implementation_checks`: Optional for draft vectors and required/non-empty for promoted vectors. Each `{ runner, case }` binds the JSON example to a registered `rust-core`, `rust-server`, `web-cockpit`, or `token-commune-adapter` product-seam executor. The umbrella checker dispatches each used runner once and requires its exact machine-readable executed-id set; unknown, duplicate, missing, or unreported checks fail closed.
- `mutation_witnesses`: Optional outside a certification profile. Every promoted `TokenCommune*` vector requires a non-empty unique list. Each witness names the claim-breaking mutant, the package runner that executes it, and the independent invariant it must violate. Runners receive exact requested witness ids and must report `PATCHBAY_CONFORMANCE_MUTATION_KILLED=<vector>:<mutation>` for the same set; missing, duplicate, or unexpected kills fail before traceability changes.
- `proto_fields_constrained`: Fully-qualified `.proto` field or enum paths constrained by the example, using the `patchbay` package names from `contracts/proto/patchbay/*.proto` (for example `patchbay.Operation.kind`, `patchbay.SubmissionResult.failure_code`, `patchbay.OperationState`).
- `description`: What scenario the vector exercises.
- `input`: Proto-shaped JSON inputs. Objects name the referenced protobuf message with a companion `*_type` field, then use the proto field names from the `.proto` files.
- `expected_outcome`: Proto-shaped JSON result or observable effect, using the same field names and fully-qualified type references when a concrete message is returned.
- `invariant_check`: A concise statement of how the example exercises the referenced property.

## Promotion status values

- `draft`: Informative example under development. Draft vectors can reference checked-model, stated-normative, or descriptive boundary-validation properties, but they do not make product semantics checked-normative.
- `promoted`: Reviewed vector that traces to a named property, agrees with that property's invariant, and executes every registered implementation check against the vector's input and expected outcome.

## Property mapping

`property_id` should match the property vocabulary in `docs/VERIFICATION.md` and the inline `@promotion` blocks in `specs/seed/*.qnt` / `*.als`. Examples:

- `CommandDurability`, `TerminalFinality`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, `BoundaryDedup`, `RetryReusesIdAndKey`, and `RetryAfterTerminalReturnsExisting` come from `specs/seed/command_lifecycle.qnt`.
- `TypedCorrelation` comes from `specs/seed/reply_correlation.qnt`.
- `SnapshotStaleRejected`, `SnapshotCrossDomainRejected`, `SnapshotConsistentPrefix`, `LateEventNoRewrite`, `CrashNoAcceptedLost`, and `IdempotentLogReplay` are reserved in `specs/seed/snapshot_recovery.qnt` and listed in `docs/VERIFICATION.md` as stated-normative.
- `NoCommandWithoutGrant` is reserved in `specs/seed/authority.qnt` and generalizes by documented refinement to Operation authorization.

If a vector exercises a boundary validation rule without a formal property id, keep `promotion_status: "draft"` and use a descriptive id such as `boundary-validation` until the property is reserved or modeled.

## Proto field references

Field paths use the protobuf package and message names from `contracts/proto/patchbay/*.proto`:

- Message fields: `patchbay.Operation.command_id`, `patchbay.Operation.target_scope`, `patchbay.SubmissionResult.outcome`.
- Nested field references can name the containing message and field, e.g. `patchbay.TargetScope.runtime_session_id`.
- Enum constraints name the field carrying the enum, e.g. `patchbay.Operation.kind`, `patchbay.SubmissionResult.operation_state`, or `patchbay.SubmissionResult.failure_code`.

The vectors intentionally reference the `.proto` contract as the wire-shape authority. They do not introduce hand-written DTOs or new protocol vocabulary.
