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
- A snapshot with a log sequence number strictly less than the core's current revision for that view is rejected as an authority source and replaced by the current view.
- A snapshot from a different authority domain or core generation is rejected outright.
- A late event whose log sequence number is older than the view it would mutate is recorded as an audit/reconciliation event and does not rewrite the current view.
- Snapshot materialization reads a consistent log prefix: it reflects every event with `LSN <= snapshot_LSN` and no event with `LSN > snapshot_LSN`.

Normative model variables should include at least `LSN`, `Cursor`, `SnapshotRevision`, `AuthorityDomain`, `CoreGeneration`, and the view variables (`CommandId`, `SessionId`, `ActorId`) the snapshot reconciles.

### Crash recovery

Properties:

- After an ungraceful core restart, replay of the durable log reconstructs in-memory state up to the last committed `LSN`.
- Accepted commands are restored as `accepted` (or a later committed state) and continue through their lifecycle; no accepted command disappears silently.
- Log replay is idempotent: replaying the same committed prefix produces identical state.
- Snapshot checkpointing bounds recovery replay cost without becoming an alternate ordering authority.

V0 models do not need to prove remote replication, HA failover, or split-brain resolution. Those are out of formal scope.

### Authority safety

Properties:

- Commands without grants are rejected before durable acceptance and before delivery.
- Grant matching checks issuer actor, optional endpoint, target scope, command kind, expiration, and revocation generation.
- Revocation prevents future command acceptance under the revoked grant.
- Already accepted commands follow the grant's revocation policy: continue, cancel where supported, or require reauthorization.
- Lockdown rejects new commands and marks affected runtime sessions stale until fresh authentication or operator action clears the condition.
- Delegation cannot create authority beyond its parent grant.

Normative model variables should include at least `Actor`, `Device`, `Endpoint`, `OperatorSession`, `Grant`, `GrantScope`, `CommandKind`, `Target`, `TargetGeneration`, `RevocationGeneration`, `CommandIssuer`, and `AuthorityDomain`.

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
- A `partial` or `no snapshot` adapter cannot cause the core to fabricate a live snapshot from cached or optimistic state; affected session axes move to `stale` or `unknown`.

### Browser session and CSRF boundary

Properties:

- A state-changing browser request without an authenticated operator session is rejected before command acceptance.
- A state-changing browser request without a valid session-bound CSRF proof is rejected before command acceptance.
- Revoked or expired operator sessions cannot issue new commands.
- Browser-local state cannot grant authority or override core grant checks.

Formal models do not prove browser cookie mechanics or cryptographic token strength; they model the server-side effects of valid, missing, expired, and revoked session/CSRF evidence.

### Audit integrity

Properties:

- Security-relevant decisions produce audit records: authentication success/failure, session revocation, failed authorization, command acceptance/rejection, grant changes, lockdown, adapter failure, and stale-event rejection.
- Audit records correlate to actor, device, endpoint/session when known, target, command, outcome, and reason without requiring secret material in the model.
- Rejected attempts and failed checks can produce audit records without creating command records.
- Revocation and terminal command outcomes remain visible in audit history; they are not deleted by later state changes.

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
