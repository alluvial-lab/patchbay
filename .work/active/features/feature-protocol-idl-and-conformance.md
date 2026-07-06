---
id: feature-protocol-idl-and-conformance
kind: feature
stage: review
tags: [protocol, verification, foundation]
parent: epic-foundation-hardening
depends_on: [feature-verification-contract-authority, feature-session-identity-adapter-contract, feature-operator-presence-and-action-inventory]
created: 2026-06-28
updated: 2026-07-06
gate_origin: null
release_binding: null
---

# Feature: Author v0 protocol IDL and conformance vectors

Patchbay's generated-contract posture requires actual schema/IDL artifacts, generated boundary types, and conformance vectors before Rust core, TypeScript operator domain, or adapters implement durable protocol behavior.

## Scope

- Create the v0 protocol IDL/schema using the contract source selected by `feature-verification-contract-authority`.
- Define initial wire contracts for actors/endpoints, sessions, commands, replies, events, snapshots, grants, and adapter capabilities that are in v0 scope.
- Establish generation targets for Rust core types and TypeScript client/operator-domain types.
- Produce golden conformance vectors for command acceptance, reply correlation, snapshot reconciliation, terminal-commit race resolution, and failure/outcome mapping.
- Document how generated contracts relate to prose semantics and formal models.

### Normative registry inheritance

This feature **inherits the normative action registry** from `feature-operator-presence-and-action-inventory`. It does **not** invent a separate command/action-kind list. The product-vocabulary registry is authored in `docs/PROTOCOL.md` (Operation, Observation, Elicitation, Payload, `OperationKind` registry, `ElicitationState` lifecycle, `response_contract` registry, the five id spaces, and the Presence/Subscription axes). This feature's `.proto` enum/wire representation derives from that registry: if `.proto` needs a new action kind, the product-vocabulary registry in `docs/PROTOCOL.md` changes first, then `.proto`, models, vectors, and implementation follow.

The original Q4 ("what are the command/action kinds?") is **dissolved**: it is answered by consuming `OperationKind`, `ElicitationState`, `response_contract.contract_kind`, and Presence/Subscription registries from the foundation work rather than by introducing a parallel enum here.

## Acceptance criteria

- `contracts/` contains the v0 IDL/schema and generation instructions.
- Rust and TypeScript generation targets are documented, even if generated code packages are created in later implementation work.
- Conformance vectors exist in a stable location and are referenced from `docs/VERIFICATION.md`.
- Terminal-commit race vectors cover completion before cancellation, cancellation before completion, expiration before late completion, retry after terminal, late terminal candidate as audit/reconciliation only, and replay of the same committed prefix.
- No hand-written DTO set is introduced as the durable source of truth.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.

## Design decisions

Captured from the design-pass Q&A (2026-07-06). The contract source (Protobuf+Buf, prost for Rust, Protobuf-ES for TypeScript), the authority order, and the traceability mechanism are settled by `feature-verification-contract-authority` (Q1–Q4 there) and `feature-research-contract-tooling`; this feature applies them, it does not re-ask them.

- **Q1 — v0 IDL scope: spine + envelopes, opaque payloads.** Define wire envelopes for all committed primitives (Operation, Observation, Elicitation, response Operations) and all committed registries (OperationKind enum, OperationState/ElicitationState/SessionState/SessionConnectivityState/SessionActivityState enums, SubmissionOutcome, response_contract.contract_kind, the 5 id spaces, Presence/Subscription, grant shape, adapter capability shape, failure vocabulary). Payloads are `bytes` (or `google.protobuf.Struct` for structured-but-adapter-defined content) — NOT contract_kind-specific payload schemas. This preserves the "Payload is content, not protocol primitive" principle: the protocol carries envelopes; adapters/harnesses interpret payload. Reserved OperationKinds (`agent-send`, `adapter-utility-exec`) and reserved response_contract kinds (`freeform`, `secret`, `function_result`, `file_attachment`, `structured_schema`, `service_request`) are enum values marked reserved (not validatable in v0; wire-present for forward compatibility). Rationale: full envelope coverage lets the core implement routing/lifecycle/authority; opaque payloads defer contract_kind-specific schemas until a real consumer needs them (Late-Binding).
- **Q2 — `.proto` organization: package split by concern.** Multiple files under `contracts/proto/patchbay/`, mirroring `docs/PROTOCOL.md` section structure: `operations.proto`, `observations.proto`, `elicitations.proto`, `sessions.proto`, `authority.proto`, `adapter.proto`, `common.proto` (id spaces, shared types). One buf package `patchbay`. Rationale: mirrors the prose SSOT structure; generated Rust modules + TS modules organize naturally; buf's package system is designed for this; minimal cost.
- **Q3 — Conformance vector format: `contracts/vectors/*.json`.** One JSON file per vector, with a structured envelope (model property id, promotion status, `.proto` fields constrained, input, expected outcome) + the input/expected-output pair. Machine-readable for the CI script (Q4 from verification-contract-authority). Rationale: JSON is the natural format for executable examples CI consumes; matches the "machine-readable per-vector metadata" decision; no markdown parsing in CI.
- **Q4 — Generation: document + wire up.** Write `.proto`, write `buf.yaml` + `buf.gen.yaml`, AND run `buf generate` to produce actual Rust (prost) + TypeScript (Protobuf-ES) generated code in `contracts/rust/` + `contracts/ts/`. Create the Rust crate skeleton (`Cargo.toml` consuming the generated code) and the TS package skeleton (`package.json` consuming the generated code). Verify both compile. This tests that the `.proto` is actually generatable (not just documented) and unblocks downstream Rust core / TS domain implementation. NOT in scope: wiring the generated types into an end-to-end Operation submission round-trip (that's walking-skeleton implementation, a separate feature).

## Architectural choice

**Greenfield contract package: `contracts/` as the root, with `proto/`, `rust/`, `ts/`, `vectors/` subdirectories.** The Rust crate and TS package live under `contracts/` (not at repo root) because they are *contract* artifacts — generated boundary types — not the eventual Rust core or TS operator domain. The eventual `core/` Rust workspace and `operator/` TS app will depend on `contracts/rust` and `contracts/ts` as path/git dependencies. This keeps the contract surface as a discrete, reviewable unit and matches the generated-contracts principle (boundary types come from schema inference/generation, not hand copies).

Alternatives considered:
- Rust crate at repo root: rejected — conflates the contract crate with the future core crate; the contract surface should be a dependency of core, not core itself.
- Generated code committed vs. generated-on-build: generated code committed to `contracts/rust/` and `contracts/ts/` so reviewers and downstream consumers can read the wire types without running buf; `buf generate` is re-runnable and CI verifies the committed generated code matches a fresh generation (drift check).

## Implementation Units

### Unit 1: `.proto` package (Q1, Q2)
**Files**: `contracts/proto/patchbay/{common,operations,observations,elicitations,sessions,authority,adapter}.proto`, `contracts/proto/buf.yaml`
**Story**: `story-protocol-idl-proto-package`

Author the v0 `.proto` package. Map every committed registry in `docs/PROTOCOL.md` to Protobuf enums and messages:
- `common.proto` — the 5 id spaces (`CommandId`, `MessageId`, `ReplyId`, `EventId`, `ElicitationId` as distinct message wrappers over `string`), `LSN`, `ActorId`, `EndpointId`, `AuthorityDomainId`, `Generation`, timestamps.
- `operations.proto` — `Operation` message (sender, recipient, `OperationKind`, target scope, idempotency key, payload as `bytes` + `PayloadContentType` enum), `OperationKind` enum (committed + reserved values), `OperationState` enum, `SubmissionOutcome` enum, `SubmissionResult` message.
- `observations.proto` — `Observation` message (sender, recipient, correlation to Operation/Elicitation, payload as `bytes`), `ObservationKind` (event/status/delta/result).
- `elicitations.proto` — `Elicitation` message (opener, `expected_responder_actor`, `response_contract`, target context, timeout/cancellation policy, correlation), `ElicitationState` enum, `ResponseContract` message (`contract_kind` enum + optional `ui_hints`), `response_contract.contract_kind` enum (committed: `approval`, `question`; reserved: `freeform`, `secret`, `function_result`, `file_attachment`, `structured_schema`, `service_request`).
- `sessions.proto` — `Session` message, `SessionState` axes (`SessionConnectivityState`, `SessionActivityState` enums), `SessionSnapshot` message.
- `authority.proto` — `Grant` message (the full v0 grant shape from `docs/SECURITY.md` + `feature-design-grant-shape`), `DescendantGrant` (the spawn descendant grant field list from `feature-foundation-doc-completeness-gaps`), `Revocation` message.
- `adapter.proto` — `AdapterCapability` message (OperationKinds, `target_spec.shape` support, idempotency strength), `AdapterRegistration` message.

Reserved enum values use Protobuf `reserved` + a naming convention (e.g. `OPERATION_KIND_RESERVED_AGENT_SEND = 99` with a comment "reserved, not validatable in v0"). This makes reserved seams wire-present for forward compatibility without making them validatable.

`buf.yaml` configures the `patchbay` package with `lint` + `breaking` rules per `feature-research-contract-tooling`.

**Acceptance Criteria**:
- [ ] Every committed registry in `docs/PROTOCOL.md` has a corresponding Protobuf enum/message.
- [ ] Reserved kinds/contracts are enum values marked reserved, not omitted.
- [ ] Payloads are `bytes` + `PayloadContentType`, not contract_kind-specific schemas.
- [ ] `buf lint` passes on the package.
- [ ] `.proto` derives names from `docs/PROTOCOL.md` registries (no parallel vocabulary).

### Unit 2: Rust + TS generation wiring (Q4)
**Files**: `contracts/buf.gen.yaml`, `contracts/rust/Cargo.toml`, `contracts/rust/src/lib.rs`, `contracts/rust/build.rs`, `contracts/ts/package.json`, `contracts/ts/src/index.ts`, `contracts/ts/tsconfig.json`
**Story**: `story-protocol-idl-generation-wiring`
**Depends on**: `story-protocol-idl-proto-package`

Wire up `buf generate` to produce Rust (prost) and TypeScript (Protobuf-ES) code. Install `buf` (and `protoc` if buf requires it) — this may require a tooling install step; document the install in `contracts/README.md`. Run `buf generate` and commit the generated code to `contracts/rust/src/gen/` and `contracts/ts/src/gen/`. Create the Rust crate skeleton (`Cargo.toml` with prost/prost-build deps, `build.rs` running prost-build over the `.proto`, `lib.rs` re-exporting the generated modules) and the TS package skeleton (`package.json` with `@bufbuild/protobuf` + `@bufbuild/protoc-gen-es` deps, `tsconfig.json`, `src/index.ts` re-exporting the generated modules). Verify `cargo build` and `npm run build` both succeed.

**Acceptance Criteria**:
- [ ] `buf generate` runs and produces Rust + TS code.
- [ ] `cargo build` succeeds in `contracts/rust/`.
- [ ] `npm run build` succeeds in `contracts/ts/`.
- [ ] Generated code is committed (not gitignored).
- [ ] `contracts/README.md` documents how to install buf + regenerate.

### Unit 3: Conformance vectors (Q3)
**Files**: `contracts/vectors/*.json`, `contracts/vectors/README.md`
**Story**: `story-protocol-idl-conformance-vectors`
**Depends on**: `story-protocol-idl-proto-package`

Author the golden conformance vectors per the feature's acceptance criteria. Each vector is a JSON file with the structured envelope (model property id, promotion status, `.proto` fields constrained, input, expected outcome). Required vectors (from the brief's acceptance criteria):
- Command acceptance (valid submission → `accepted`).
- Reply correlation (response Operation → command/message id via typed correlation).
- Snapshot reconciliation (stale snapshot rejected; cursor replay returns only `LSN > cursor`).
- Terminal-commit race: completion before cancellation; cancellation before completion; expiration before late completion; retry after terminal returns existing; late terminal candidate as audit/reconciliation only; replay of the same committed prefix is idempotent.
- Failure/outcome mapping: unknown OperationKind → `validation_failed`; missing grant → `authorization_denied`; missing target → `target_not_found`.

Vectors reference `.proto` message types and enum values by their fully-qualified name. The CI script (Unit 4) will consume these.

**Acceptance Criteria**:
- [ ] All required vector cases above exist as JSON files.
- [ ] Each vector carries the structured envelope (property id, promotion status, `.proto` fields).
- [ ] Vectors reference real `.proto` types (no forward references to types that don't exist).
- [ ] `contracts/vectors/README.md` documents the vector format.

### Unit 4: CI traceability script + docs (Q4 from verification-contract-authority)
**Files**: `contracts/scripts/check-vectors.ts` (or `.rs`/`.py`), `contracts/README.md`, `docs/VERIFICATION.md` (append traceability table reference)
**Story**: `story-protocol-idl-traceability-script`
**Depends on**: `story-protocol-idl-conformance-vectors`, `story-protocol-idl-proto-package`

Author the CI script that reads all `contracts/vectors/*.json` and: (a) fails if a checked-model property lacks a promoted vector; (b) fails if a vector references a missing/misspelled property; (c) fails if a promoted vector's expected outcome contradicts its referenced model property's invariant (surfaced contradiction per Q3 of verification-contract-authority); (d) generates the `docs/VERIFICATION.md` traceability table as a checked-in artifact. Wire it as a CI-runnable script (package.json script or cargo xtask). Update `docs/VERIFICATION.md` to reference the vectors location and the generated traceability table.

**Acceptance Criteria**:
- [ ] Script runs and validates all vectors against the model property list.
- [ ] Script generates a traceability table artifact.
- [ ] `docs/VERIFICATION.md` references `contracts/vectors/` and the traceability table.
- [ ] Script fails on a deliberately-broken vector (negative test).

## Implementation Order

1. `story-protocol-idl-proto-package` (Unit 1) — unblocks 2, 3, 4.
2. `story-protocol-idl-generation-wiring` (Unit 2) and `story-protocol-idl-conformance-vectors` (Unit 3) — parallelizable after 1.
3. `story-protocol-idl-traceability-script` (Unit 4) — after 2 and 3.

## Testing

- `buf lint` on the `.proto` package.
- `cargo build` + `npm run build` on the generated code (Unit 2).
- The CI traceability script validates vectors (Unit 4).
- A negative test: a deliberately-broken vector must fail the script.
- Review (not tests) verifies `.proto` fidelity to `docs/PROTOCOL.md` registries.

## Risks / pre-mortem

- **Risk: `buf`/`protoc` install friction in the sandbox.** The sandbox has cargo + node but not buf/protoc. Mitigation: document the install; if buf can't be installed, fall back to `protoc` + prost-build directly (prost-build can invoke protoc or use its own parser). If neither works, Unit 2 downgrades to "document the generation setup + commit hand-verified generated code" and files a follow-on story for the live generation wiring. Don't block the whole feature on tooling install.
- **Risk: `.proto` drifts from `docs/PROTOCOL.md` registries.** Mitigation: Unit 1's acceptance criterion is registry fidelity; the review checks it. The CI script (Unit 4) catches missing properties but not naming drift — the review must.
- **Risk: opaque payloads (`bytes`) under-specify what adapters need.** Mitigation: `PayloadContentType` enum names the content type; contract_kind-specific schemas are a reserved follow-on (Late-Binding). If a real consumer needs a schema, that's a new feature, not a blocker here.
- **Risk: scope creep into walking-skeleton implementation.** Mitigation: Q4(c) explicitly rejected; this feature produces contracts + generation wiring, not an end-to-end round-trip. The review bounces any scope creep into walking-skeleton territory.
- **Risk: reserved enum values confuse downstream consumers.** Mitigation: reserved values are commented "reserved, not validatable in v0"; the boundary rule (unknown/reserved kind → `validation_failed`) is documented in `contracts/README.md`.

## Extension pressure classification

- **Committed v0:** the `.proto` package, the generation wiring, the conformance vectors, the traceability script.
- **Reserved extension seams:** contract_kind-specific payload schemas (payloads are opaque `bytes` for v0); per-variant spawn OperationKinds; `agent-send`/`adapter-utility-exec` as wire-present-but-rejected enum values; delegation lineage in the grant shape.
- **Rejected direction:** a hand-written DTO set as the durable source of truth (the whole point of this feature); markdown conformance vectors (Q3=b rejected); a single monolithic `.proto` file (Q2=a rejected, though refactor-to-single is not forbidden if the package split proves premature).

## Review (2026-07-06)

**Verdict**: Approve

**Blockers**: none
**Important**: none (3 findings F1-F3 from the initial review were fixed in commit 9a2854f; targeted re-review confirmed READY)
**Nits**: none

**Notes**: Substrate feature review, deep lane, fresh-context gpt-5.5. Reviewed the full deliverable across 4 stories: .proto package (7 files), buf generate wiring (Rust prost crate + TS Protobuf-ES package, both building), 12 conformance vectors (JSON with structured envelope), traceability script (check-vectors.mjs) + generated-code drift check (check-generated-drift.mjs). Initial review returned Request changes with 3 important findings (failure vectors contradicted PROTOCOL.md pre-acceptance-refusal semantics; reply-correlation vector was mis-typed as Elicitation response instead of Reply/Observation; generated-code drift check promised by design was missing). All fixed inline; re-review READY. Registry fidelity verified (16/16 PROTOCOL.md registries mapped to .proto). Builds verified: cargo build, npm run build, check-vectors.mjs (12 vectors pass), check:drift (detects modifications). Reserved seams preserved (agent-send/adapter-utility-exec wire-present-but-reserved; freeform + 5 contract kinds reserved; no parent_grant_id; payloads opaque bytes). Generated Contracts principle honored (generated code committed, drift check enforces it). 4 child stories advanced implementing → review; feature rolled up implementing → review.
