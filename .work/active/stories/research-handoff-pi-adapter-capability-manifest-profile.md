---
id: research-handoff-pi-adapter-capability-manifest-profile
kind: story
stage: implementing
tags: [adapter, protocol]
parent: research-handoff-pi-adapter-capability
depends_on: [capability-manifest-durability-and-reconciliation-depth, research-handoff-spawn-continuation-payload-authority-contract]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
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

- [ ] No core message contains mandatory `cwd`, `project_trust`, `extensions`, `skills`, `prompts`, `themes`, or `context_files` fields.
- [ ] Generated Pi profile owns those values and is carried opaquely; core attach/replay validation is identical for Pi and another adapter profile envelope.
- [ ] Core permits a generically absent optional profile and rejects an oversized/malformed present envelope without parsing Pi semantics; the Pi adapter refuses to advertise/attach its managed shape when its required generated Pi profile is absent.
- [ ] The sibling's dedup, continuation-proof, cursor, generation-fence, and reconciliation-strength fields are imported once; every Pi value is explicit and uncertainty is false/unknown.
- [ ] Diagnostics/snapshots/audits do not expose raw profile bytes, cwd, session paths, or launch/deployment handles.
- [ ] A profile schema alone advertises no mechanism. Full Pi declaration activates only after lifecycle conformance evidence names the exact implementation version.
- [ ] Capability remains advisory: grants and adapter delivery still decide authority/outcome.

## Ordering constraint

Depends on `capability-manifest-durability-and-reconciliation-depth` so the Pi feature cannot recreate or overclaim its deferred assurance dimensions, and on the spawn continuation payload leaf before defining the generated Pi target-spec payload/profile declaration. Runtime mechanisms may be implemented in parallel, but final activation waits on all Pi children.
