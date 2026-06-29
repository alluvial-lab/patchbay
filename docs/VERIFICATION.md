# Patchbay Verification

Patchbay treats coordination semantics as specification-first. Formal models define the behavior the implementation must preserve.

## TLA+ and Quint position

TLA+ and Quint are compatible at the architecture level because both model state machines in the TLA tradition. Patchbay does not need to choose a permanent winner before design starts.

Patchbay uses this policy:

- **TLA+ is the semantic baseline** for durable, long-lived protocol models because it is established and has mature TLC tooling.
- **Quint is the ergonomic authoring candidate** for models where readability, type checking, and developer/agent editing speed matter.
- Models may begin in Quint and be checked through available backends, including TLC where appropriate.
- A model promoted to normative status must have a stable checked artifact and documented tool invocation, regardless of whether it is authored in TLA+ or Quint.
- The repository keeps model intent portable: no product decision depends on a tool-specific trick when the underlying property can be stated plainly.

This means Patchbay can start with Quint for approachability and keep TLA+ as the reference-compatible foundation.

## Alloy position

Alloy is complementary rather than competing with TLA+/Quint.

Patchbay uses Alloy for bounded relational invariants:

- actor identity uniqueness;
- endpoint/address ambiguity;
- authority graph constraints;
- revocation relationships;
- lease exclusivity;
- routing legality;
- anti-spoofing relationships.

TLA+/Quint models dynamic histories. Alloy models relational shapes and small counterexamples.

## Required model areas

### Operator intent delivery

Properties:

- An accepted command is durably recorded before delivery.
- An accepted command cannot vanish silently.
- Every accepted command remains observable in exactly one canonical `CommandState` from `docs/PROTOCOL.md` until and after it reaches a terminal state.
- Timeout does not imply success or denial.

### Wrong-session prevention

Properties:

- Commands bind to target session identity and generation.
- Late replies for old generations cannot mutate a new session generation.
- Human-readable labels cannot override verified target identity.

### Reply correlation

Properties:

- A reply references a known prior message or command.
- A reply cannot forge correlation to an unrelated session or authority context.
- Duplicate replies are either idempotent or visibly rejected.

### Idempotent retry

Properties:

- Retrying the same idempotency key cannot double-apply a command at the Patchbay boundary.
- Duplicate submission returns existing command state.
- Explicit duplicate action requires a new command id/key.

### Snapshot convergence

Properties:

- A reconnecting control surface can recover authoritative state from snapshots.
- Stale cached live/working state is corrected by a newer authoritative snapshot.
- Event streams are not required for correctness when snapshots exist.

### Authority safety

Properties:

- Commands without grants are rejected before delivery.
- Revocation prevents future command acceptance under the revoked grant.
- Delegation cannot create authority beyond its parent grant.

### Lease safety

Lease safety remains a required model area before any lease-backed product behavior ships. It is not part of the v0 executable walking skeleton unless later foundation work explicitly promotes a specific lease-backed workflow.

Properties:

- Two actors cannot simultaneously hold the same exclusive live lease in one authority domain.
- Expired leases do not authorize new exclusive action.
- Lease renewal respects holder identity and scope.

### Adapter failure visibility

Properties:

- Adapter disconnect, crash, rejection, unsupported command, target offline, timeout, expiration, cancellation, and supersession remain distinguishable using the failure/outcome vocabulary in `docs/PROTOCOL.md`.
- Adapter failure cannot appear as command completion.

## Out of formal scope

Patchbay formal models do not prove:

- LLM output quality;
- correctness of cryptographic primitives;
- operating-system scheduling or mobile background behavior;
- UI rendering correctness;
- third-party harness internals;
- real-world network latency bounds;
- adapter-specific behavior beyond declared adapter contracts.

Those areas require tests, monitoring, adapter documentation, and operational discipline.

## Conformance testing

Formal models produce implementation obligations. The implementation uses:

- protocol golden vectors shared across languages and derived from the canonical state/failure registries in `docs/PROTOCOL.md`;
- property tests for Rust core behavior;
- property tests for TypeScript operator-domain behavior;
- adapter conformance tests for declared capabilities;
- replay tests for event logs and snapshots;
- reconnect tests for stale control surfaces.

A protocol semantic change updates `docs/PROTOCOL.md`, the model, generated contract, conformance vectors, and implementation together.

## Model promotion rule

A model becomes normative only when it includes:

- the property being checked;
- finite bounds or constants used for checking;
- command/tool invocation;
- expected pass/fail status;
- a short explanation connecting the model to product semantics.

Draft models may explore ideas without becoming product commitments.
