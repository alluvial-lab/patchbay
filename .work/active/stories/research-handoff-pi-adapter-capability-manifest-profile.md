---
id: research-handoff-pi-adapter-capability-manifest-profile
kind: story
stage: review
tags: [adapter, protocol]
parent: research-handoff-pi-adapter-capability
depends_on: [capability-manifest-durability-and-reconciliation-depth, research-handoff-spawn-continuation-payload-authority-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-16
---

# Generated opaque Pi runtime profile and declaration gate

## Redesign disposition

Rewritten after the consolidated spawn-stride review. Pi resource vocabulary is no longer a mandatory core runtime-profile shape, and this checkpoint no longer claims a complete assurance manifest before its drafting sibling lands.

## Checkpoint

Keep the generic adapter manifest adapter-neutral while carrying one bounded generated adapter-specific profile opaquely. Consume the generated durability/reconciliation dimensions from `capability-manifest-durability-and-reconciliation-depth`; do not re-enumerate them. Define the Pi profile schema, but leave support activation false/absent until the final Pi lifecycle conformance checkpoint proves each mechanism.

The core may validate envelope framing, content type, schema-ref bounds, and payload size and may store the bytes as part of registration. It does not interpret Pi cwd, trust, loader, resource, cursor, or reload fields and does not expose raw profile bytes in diagnostics.

## Design

**Files**
- `contracts/proto/patchbay/adapter.proto` — one optional generic `PayloadEnvelope adapter_profile` (or equivalent generated opaque carrier) on `AdapterCapability`; import the sibling's generated assurance fields rather than defining a parallel registry.
- New `contracts/proto/patchbay/pi_adapter.proto` — `PiRuntimeProfile`, Pi transport/event/session-materialization/cursor/control-extension/reload/resource enums, typed Pi spawn target spec, and reconfigure payloads.
- `contracts/proto/patchbay/diagnostics.proto` and `core/src/diagnostics/mod.rs` — expose only safe profile schema/version/presence and generic assurance summaries, never raw bytes or paths.
- `core/src/adapter/{capability,mod}.rs` — bounded opaque-envelope validation plus sibling assurance validation; no Pi decode or Pi-required field branch.
- `pi-adapter/src/core_client.ts` — construct `piCapabilityManifest()` from generated types with all uncertain assurance fields false/unknown.
- Generated Rust/TypeScript output, drift checks, and rolling `docs/{PROTOCOL,ARCHITECTURE,ADAPTER-PI,GLOSSARY}.md` updates during implementation.

```proto
message PiRuntimeProfile {
  PiTransportMechanism transport = 1;
  PiEventSemantics events = 2;
  PiSessionDurability session_durability = 3;
  PiCursorSemantics cursor = 4;
  PiControlProof control_proof = 5;
  PiReloadBoundary reload = 6;
  repeated PiDiscoveredResourceKind enumerated_resources = 7;
  PiProjectContextSemantics project_context = 8;
}
```

The v1 Pi profile uses these generated, Pi-local values:

- `PiTransportMechanism`: `RPC_JSONL_SUBPROCESS`;
- `PiSessionMaterializationPolicy`: `AFTER_FIRST_ASSISTANT_MESSAGE` with `MEMORY_ONLY_NOT_RESUMABLE` before that boundary;
- `PiControlProofKind`: `CHALLENGED_EXTENSION_CUSTOM_ENTRY`;
- `PiDiscoveredResourceKind`: `EXTENSION_ENTRYPOINT`, `SKILL`, `PROMPT`, `THEME`, `CONTEXT_FILE`;
- `PiProcessReplacementOnlyKind`: `ARBITRARY_EXTENSION_DEPENDENCY_GRAPH`, `PI_RUNTIME_PACKAGE_DIST`, `NATIVE_DEPENDENCY`, `EXECUTABLE`, `UNKNOWN_SCOPE`;
- `PiProjectContextSemantics`: adapter-resolved cwd + project-trust/resource roots, with cwd proof supplied by the control extension rather than generic RPC.

All values are `UNSPECIFIED`-rejecting at the Pi profile decoder. They remain inside this opaque profile. Generic core fields remain the existing behavior branches plus the sibling assurance registry.

The Pi declaration is conditional:

```ts
export interface PiCapabilityEvidence {
  readonly supervisor: boolean;
  readonly controlHandshake: boolean;
  readonly strictSessionTreeValidation: boolean;
  readonly authoritativeCursorReplacement: boolean;
  readonly idleMaterializedReload: boolean;
  readonly conformanceVersion?: string;
}
```

Without the final evidence record, `spawn` and managed target shape are not advertised, resume/cursor/reload guarantees stay false/unknown, and unsupported delivery remains the honest behavior. `session_replacement_support=true` may be emitted only with the conditional continuation proof from the sibling assurance contract; it never means every fresh Pi session is already resumable.

## Acceptance evidence

- [x] No core message contains mandatory `cwd`, `project_trust`, `extensions`, `skills`, `prompts`, `themes`, or `context_files` fields.
- [x] Generated Pi profile owns those values and is carried opaquely; core attach/replay validation is identical for Pi and another adapter profile envelope.
- [x] Core permits a generically absent optional profile and rejects an oversized/malformed present envelope without parsing Pi semantics; the Pi adapter refuses to advertise/attach its managed shape when its required generated Pi profile is absent.
- [x] The sibling's dedup, continuation-proof, cursor, generation-fence, and reconciliation-strength fields are imported once; every Pi value is explicit and uncertainty is false/unknown.
- [x] Diagnostics/snapshots/audits do not expose raw profile bytes, cwd, session paths, or launch/deployment handles.
- [x] A profile schema alone advertises no mechanism. Full Pi declaration activates only after lifecycle conformance evidence names the exact implementation version.
- [x] Capability remains advisory: grants and adapter delivery still decide authority/outcome.

## Ordering constraint

Depends on `capability-manifest-durability-and-reconciliation-depth` so the Pi feature cannot recreate or overclaim its deferred assurance dimensions, and on the spawn continuation payload leaf before defining the generated Pi target-spec payload/profile declaration. Runtime mechanisms may be implemented in parallel, but final activation waits on all Pi children.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`; selected by the delegating autopilot for this cross-language public-contract and conservative declaration-gate unit.
- Review weight: `thorough` (caller override); implementation is left at `stage: review` for the independent convergence review requested by the delegating goal.
- Dispatch rationale: direct-read implementation. This worker was already a delegated child and did not attempt forbidden nested orchestration.
- Files changed: added the optional `AdapterCapability.adapter_profile` and safe `AdapterProfileSummary`; extended the landed `pi_adapter.proto` with generated runtime-profile, target-spec, and reconfigure vocabulary; regenerated committed Rust/TypeScript artifacts; added generic Rust validation and diagnostics projection; constructed and adapter-validated the Pi profile in `pi-adapter/src/core_client.ts`; added focused Rust/TypeScript tests; rolled `docs/{PROTOCOL,ARCHITECTURE,ADAPTER-PI,GLOSSARY}.md` forward.
- Generic profile boundary: absence is valid; a present envelope requires non-empty bytes up to 64 KiB, a bounded non-whitespace/control schema reference, and a known non-sentinel content type. Attach and replay use the same validator. `ValidatedAdapterProfile` retains only safe schema/content-type metadata, so core capability logic has no Pi decoder or Pi-specific branch.
- Pi profile: `patchbay.PiRuntimeProfile.v1` carries the generated JSONL RPC mechanism, partial-live-event caveats, first-assistant materialization boundary and memory-only pre-state, persisted-entry/exact-set cursor design, challenged control proof, idle/materialized reload boundary, existing generated reloadable resource set, process-replacement exclusions, and adapter-resolved cwd/project-trust/resource-root semantics. The Pi-side decoder requires the exact complete non-`UNSPECIFIED` profile before attach.
- Declaration gate: current Pi still omits `spawn` and every managed target shape, emits `session_replacement_support=false`, retains all three uncertain assurance booleans as `false`, reconciliation as `NONE`, and does not accept a conformance evidence record. The profile contract is therefore descriptive carriage only; the lifecycle-conformance child remains the sole positive activation owner.
- Tests added/updated: Rust boundary coverage proves absent/generic opaque profiles work identically at attach/replay, deliberately invalid Pi bytes are not semantically decoded, and empty/oversized/malformed envelopes fail; diagnostics coverage proves only profile presence/schema/content-type survive and raw bytes do not; Pi tests prove the exact generated profile, conservative generic declaration, missing/malformed profile rejection, and non-activation of spawn/managed shape/replacement/assurance claims.
- Simplification: reused Unit 2's landed `PiReloadableResourceKind` as the single generated entrypoint/skill/prompt/theme/context-file registry rather than adding the design sketch's duplicate `PiDiscoveredResourceKind`; diagnostics reuse one profile-summary message rather than copying adapter-local fields.
- Discrepancies from design: the generic 64 KiB profile bound was not numerically pinned by the design; it was chosen as a conservative static-registration limit and recorded in protocol/glossary. `schema_ref` carries the profile version (`patchbay.PiRuntimeProfile.v1`) so the core can project safe version identity without parsing bytes. The generated target spec uses an adapter-local `project_context_ref` rather than raw cwd/path, preserving the canonical no-log boundary.
- Mutation evidence: every required mutant was applied independently on the main tree and restored with `git restore`. The focused oracles killed (1) core semantic decoding of `patchbay.PiRuntimeProfile.v1`, (2) oversized-profile acceptance, (3) malformed schema framing acceptance, (4) Pi cwd/resource values copied into generic target-shape fields, (5) uncertain cursor assurance promoted to `true`, and (6) premature `spawn`/managed-shape/session-replacement activation. No mutant was committed.
- Full verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- Full verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (59 vectors, 19 promoted, 29 implementation checks, 38 mutation witnesses).
- Full verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS** (28/28).
- Full verification group 4 — `cd pi-adapter && npm test`: **PASS** (62/62, including the real-core loop).
- Diagnostics consumers — `cd web-cockpit && npm test`: **PASS** (144/144); `cd cli && npm test`: **PASS** (53/53 plus real-core resource projection); `cd token-commune-adapter && npm test`: **PASS** (63/63, including both real-core flows).
- Hygiene: `cargo fmt --all -- --check`, `git diff --check`, generated-contract drift, and post-mutation clean-tree checks passed. Adjacent issues parked: none.

### Fix round — semantic-invalid profile oracle (Pi Unit 1 r1)

- Fixed the thorough-review MATERIAL by separating malformed envelope/wire coverage from semantic-profile coverage and round-tripping generated `PiRuntimeProfile` fixtures through `fromBinary`/`toBinary`. The semantic cases are protobuf-decodable but violate adapter-owned validity rules: an `UNSPECIFIED` transport, an absent required nested durability message, a duplicate exact-set resource member, and an omitted required exact-set resource member.
- Rationale: mutating the decoded generated type proves the fixtures reach `validatePiRuntimeProfile`; raw byte substitutions only prove envelope or protobuf decoding. The four representative cases cover scalar equality, required-message presence, exact-set uniqueness, and exact-set completeness without duplicating the validator in the test.
- Mutation evidence: removing the complete `validatePiRuntimeProfile(profile)` call made the focused semantic test fail on the first `UNSPECIFIED` scalar witness (1 failed, 1 passed), killing the reviewer's bypass mutant. A fresh canonical-profile mutant setting `project_context.cwd_proof=UNSPECIFIED` also failed the focused manifest test with `invalid project context semantics`. Each production mutant was restored with `git restore`; neither was committed.
- Focused clean-tree oracle — `cd pi-adapter && npm run build && node --test --test-name-pattern='Pi manifest|Pi profile|semantically decodable invalid Pi profiles' dist/tests/core_client.test.js`: **PASS** (3/3).
- Full verification group 1 — `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`: **PASS**.
- Full verification group 2 — `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build`: **PASS** (59 vectors, 19 promoted, 29 implementation checks, 38 mutation witnesses).
- Full verification group 3 — `cd operator-domain && npm run build && npm test`: **PASS** (28/28).
- Full verification group 4 — `cd pi-adapter && npm test`: **PASS** (63/63, including the real-core loop).
- Diagnostics consumers — `cd web-cockpit && npm test`: **PASS** (144/144); `cd cli && npm test`: **PASS** (53/53 plus real-core resource projection); `cd token-commune-adapter && npm test`: **PASS** (63/63, including both real-core flows). The first parallel consumer attempt raced shared `operator-domain/dist` rebuilds and failed with transient missing-module errors; the required commands were rerun sequentially and all passed.
- Hygiene: no proto or generated-contract edits; `git diff --check` passed; only this story and the focused Pi test are committed. Review weight remains `thorough`; stage returns to `review` for convergence.
