---
id: capability-manifest-durability-and-reconciliation-depth-consumer-wiring
kind: story
stage: review
tags: [adapter, protocol, verification]
parent: capability-manifest-durability-and-reconciliation-depth
depends_on: [capability-manifest-durability-and-reconciliation-depth-contract-validation]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
research_refs: [v1-control-plane-and-spawn]
created: 2026-08-15
updated: 2026-08-15
---

# Canonical assurance consumers, attach diagnostics, and Pi-profile seam

## Checkpoint

Wire the validated generated assurance registry through current adapter attach/redeclaration, canonical diagnostics, retry/outcome presentation, and the two existing adapter manifest constructors. Consumers must not flatten or re-enumerate the registry. Runtime installed/reachable state remains separate, and capability remains advisory rather than grant or delivery authority.

The Pi manifest/profile child consumes this seam. Until its supervisor, continuation proof, cursor replacement, generation fence, and lifecycle conformance checkpoints pass, Pi emits only evidence-backed values and leaves uncertain dimensions false/none.

## Design

### Files

- `contracts/proto/patchbay/diagnostics.proto` — add the generated `AdapterAssuranceManifest` to `AdapterCapabilitySummary`; do not copy its six V1 fields or include future raw adapter-profile bytes.
- Generated Rust/TypeScript artifacts — regenerate through Buf.
- `core/src/diagnostics/mod.rs` — construct diagnostics from canonical `ValidatedAdapterCapability` assurance rather than raw replay-only tag 7; keep attachment descriptors and profile bytes redacted.
- `server/src/adapter_service/{mod.rs,tests.rs}` and diagnostics/server tests — attach, replay, same-generation redeclaration, newer-generation replacement, and no-append/no-token failure paths.
- `pi-adapter/src/core_client.ts` and focused manifest tests — emit complete V1 with current evidence-backed values; import the same generated registry later used by `research-handoff-pi-adapter-capability-manifest-profile`.
- `token-commune-adapter/src/manifest.ts` and `token-commune-adapter/tests/resource_contract.test.ts` — emit complete conservative V1 explicitly; latest-50 visibility or partial snapshots must not be mislabeled as an authoritative cursor/outcome guarantee.
- `cli/src/commands/diagnostics.ts` and CLI diagnostics tests — safe JSON/human presentation of generated assurance dimensions and the closed `unknown` / `manual-required` qualifier.
- Existing web retry/adapter-diagnostics consumers and tests — read `assurance.v1.deduplication_strength`; keep retry safety based on the canonical failure plus declared dedup, never capability alone.
- Rolling current-state assertions in `docs/{ARCHITECTURE,PROTOCOL,VERIFICATION,UX,GLOSSARY}.md`; Pi-specific profile/detail remains owned by the dependent Pi feature.

### Canonical consumer contract

```rust
pub struct AdapterCapabilitySummary {
    // Existing safe capability summary fields.
    pub assurance: Option<AdapterAssuranceManifest>,
}
```

The exact generated shape will come from `diagnostics.proto`; no handwritten Rust DTO is introduced. The core obtains it from:

```rust
record.validated_capability.assurance().to_wire_v1()
```

not from the deprecated raw `registration.capability.idempotency_strength`. Consequently, historical replay and fresh V1 declarations produce the same complete diagnostics shape.

TypeScript consumers narrow the generated oneof once and fail closed if it is not V1. They do not create local string unions for strength/action names.

### Current conservative declarations

- **Pi before final lifecycle activation:** `deduplication_strength = AT_PATCHBAY_BOUNDARY`; `continuation_proof_support = false`; `cursor_support = false`; `generation_fence_support = false`; `reconciliation_strength = NONE`; `unproven_outcome_action = MANUAL_REQUIRED` for accepted external execution ambiguity. Existing `session_replacement_support` remains a separate capability and proves none of those fields.
- **token-commune observer:** `deduplication_strength = NONE`; continuation/cursor/generation-fence fields `false`; reconciliation `NONE`; unproven action `NONE`. Its partial, latest-visible-window resource reconciliation stays expressed by existing snapshot/report contracts, not inflated into an Operation-outcome assurance.
- **Pi after its dependent conformance checkpoint:** that feature may promote individual values (expected bounded reconciliation plus proven continuation/cursor/fence) only through the same V1 constructor and exact promoted vectors. It may not mutate the generic enum set or add Pi fields here.

### Outcome and presentation guard

`SubmissionOutcome.UNKNOWN` remains the pre-acceptance transport terminal. Accepted external ambiguity remains `FAILED / EXECUTION_OUTCOME_UNKNOWN`. Diagnostics/presentation combines that canonical unknown with `ReconciliationAction`: `NONE` renders `unknown`; `MANUAL_REQUIRED` renders `manual-required`. The qualifier never rewrites the Operation outcome.

No capability value enters grant matching or core delivery suppression. Maximal declared assurance without a live grant still rejects before acceptance. Conservative assurance with a live grant still delivers and lets the adapter return its authoritative accepted/rejected/failure result.

## Acceptance evidence

- [x] `AdapterCapabilitySummary` carries the generated assurance message once; no flattened six-field copy or local Rust/TypeScript registry exists.
- [x] Canonical diagnostics use the validated normalized assurance view; a historical v0.2 registration displays complete conservative V1 and never leaks deprecated/raw fields.
- [x] Fresh attach, replay, same-generation redeclaration, and newer-generation replacement preserve exact assurance values and existing atomic lifecycle behavior.
- [x] Pi and token-commune constructors explicitly set every V1 field; deleting any field fails their manifest/attach test.
- [x] Current Pi does not advertise continuation proof, cursor support, generation fencing, or stronger reconciliation solely from session replacement, `get_entries`, attachment, or reachability.
- [x] The dependent Pi profile consumes this generated registry and has at most one separate bounded opaque profile for Pi-only facts; generic assurance fields are not redefined there.
- [x] CLI JSON and human output show safe canonical names. Web retry safety reads V1 dedup and continues to distinguish `execution_outcome_unknown` from ordinary pre-execution retry.
- [x] Unknown/manual-required presentation derives from existing unknown outcomes plus generated `ReconciliationAction`; no new command/session/failure state or CSS registry member appears.
- [x] A maximal manifest cannot authorize or complete an Operation, and a conservative manifest cannot suppress otherwise authorized adapter delivery; the exact advisory-only conformance check passes.
- [x] Attach/reachability/runtime diagnostic state changes do not mutate assurance values.
- [x] Raw attachment descriptors, future opaque adapter-profile bytes, paths, credentials, and other canonical redaction-list fields remain absent from diagnostics/audit/snapshots.
- [x] Rolling docs update the canonical adapter-capability registry and extension-seams row without framing the v1 declaration as timeless architecture.

## Implementation evidence

- Execution capability: `openai-codex/gpt-5.6-sol`. The change followed the existing generated-contract, boundary-validation, durable-projection, and presentation-registry patterns without introducing a parallel assurance DTO or state registry.
- Diagnostics retain the replay-validated `AdapterRecord` and derive the wire summary only from `ValidatedAdapterCapability::assurance().to_wire_v1()`. Diagnostics tag 7/name `idempotency_strength` is reserved, generated bindings carry `AdapterAssuranceManifest` once at tag 14, and attachment descriptors remain structurally absent.
- The authenticated server lifecycle regression proves fresh attach, byte-exact same-generation redeclaration, newer-generation replacement, prior-token invalidation, durable replay, exact V1 preservation, and one append per accepted declaration. Existing invalid-assurance regressions prove no registration append and no attachment token.
- Pi declares `AT_PATCHBAY_BOUNDARY`, three false evidence flags, `NONE` reconciliation, and `MANUAL_REQUIRED`; token-commune declares the complete conservative all-false/`NONE` observer profile. Their constructor tests compare every V1 field and reject omission/unspecified values.
- CLI diagnostics narrow generated V1 assurance and render every canonical enum/dimension in JSON and tables. Web diagnostics use one generated V1 narrowing; retry presentation requires a qualifying canonical failure and declared deduplication, while canonical unknown outcomes retain `unknown` or `manual-required` without changing lifecycle state.
- Controlled mutation kills, each applied independently and restored before final verification: raw replay assurance instead of canonical validated assurance; attachment descriptor leakage; capability-only retry; Pi cursor promotion to `true`; and token-commune reconciliation promotion to `AUTHORITATIVE`. Each failed its focused production test, and restored focused tests passed.
- Verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS** (including 84 server unit tests and warnings-denied clippy).
- Verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (59 vectors, 19 promoted vectors, 29 implementation checks, and 38 mutation witnesses).
- Verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS** (27/27).
- Verification group 4 — `cd pi-adapter && npm test`: **PASS** (61/61, including the real-core loop).
- Consumer suites — `cd web-cockpit && npm test`: **PASS** (143/143); `cd cli && npm test`: **PASS** (51/51 plus real-core resource projection); `cd token-commune-adapter && npm test`: **PASS** (63/63, including both real-core flows).
- Hygiene — `cargo fmt --all -- --check` and `git diff --check`: **PASS**. Adjacent issues parked: none.

## Ordering constraint

Depends on `capability-manifest-durability-and-reconciliation-depth-contract-validation`. The generated registry and validator must exist before consumers, diagnostics, or adapter constructors can migrate. This story then provides the exact seam consumed by `research-handoff-pi-adapter-capability-manifest-profile`.
