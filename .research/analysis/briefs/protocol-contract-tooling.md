---
provenance: agent-synthesis
updated: 2026-06-28
research_item: feature-research-contract-tooling
intent: inform-architecture-decision
output_kind: synthesis-brief
---

# Protocol contract and schema source of truth for Patchbay

## Recommendation

Use **Protobuf schemas managed by Buf** as Patchbay's first boundary-contract source for durable protocol messages, command/event payloads, and shared enum vocabularies across the Rust coordination core and TypeScript operator domain.

The recommended shape is:

1. Keep `.proto` files as the source for **wire contracts and boundary DTOs**, not as the full internal domain model.
2. Generate TypeScript with Protobuf-ES and Rust with a prost-based generator path.
3. Run Buf generation and breaking-change checks in local development and CI.
4. Keep formal state-machine semantics and conformance vectors beside the schema so `.proto` defines shape and vocabulary while models/tests define allowed behavior.
5. Use JSON Schema / TypeBox / Zod for JSON-native local validation surfaces when needed, but not as the first cross-language protocol source.
6. Revisit TypeSpec if Patchbay later needs OpenAPI, JSON Schema, and Protobuf emitted as peer outputs from one authoring language.

This gives Patchbay a pragmatic generated-contract foundation without hand-copied DTOs: Buf runs language plugins from checked-in generation configuration [buf-generate]{1} [buf-generate]{2}, detects compatibility breaks against prior schemas [buf-breaking]{1}, Protobuf-ES emits plain TypeScript build artifacts [protobuf-es]{1}, and prost generates idiomatic Rust from `.proto` files [prost]{1}.

## Why Protobuf + Buf fits Patchbay's first contract decision

Patchbay's initial boundary needs are cross-language and lifecycle-heavy: Rust core, TypeScript operator domain, future client surfaces, adapter-neutral command/event contracts, and compatibility checks. Buf directly addresses the code-generation and lifecycle parts: `buf generate` runs Protobuf plugins for target languages including TypeScript and Rust [buf-generate]{1}, and `buf breaking` compares current schemas to a past input to catch client/server/generated-code breaks [buf-breaking]{1}. Buf's rule categories also distinguish source-level compatibility from binary and JSON wire compatibility [buf-breaking]{4}, which maps well to Patchbay's need to separate internal package consumers from wire consumers.

For TypeScript, Protobuf-ES is a good fit for the operator-domain/client side because the generated files are plain TypeScript message types plus schema exports and are meant to be regenerated rather than edited [protobuf-es]{1}. It also provides runtime helpers for binary and JSON conversion from the schema export [protobuf-es]{3}. For Rust, prost documents generation of simple Rust code from proto2/proto3 files and recommends `prost-build` for Cargo build-time `.proto` compilation [prost]{1} [prost]{3}.

Buf managed mode is also aligned with Patchbay's adapter-neutral posture: language-specific options can live in generation configuration instead of the `.proto` source, keeping schema files less tied to one consumer language [buf-generate]{4}.

## What should be source of truth for what

### Semantic model

Patchbay should keep the **semantic protocol model** explicit: command lifecycle, submission outcomes, session axes, failure vocabulary, authority checks, snapshots, idempotency, and race rules remain protocol semantics, not mere serialization details.

The recommended split:

- `.proto` owns boundary **shape**, shared identifiers, payload envelopes, and generated enum vocabularies.
- Formal models own dynamic invariants such as terminal-state finality, idempotent retry, stale snapshot reconciliation, and authority rules.
- Conformance vectors own executable examples that both Rust and TypeScript must pass.
- Rust and TypeScript internal domain types may wrap generated DTOs, but boundary conversion should be generated or centrally implemented from the schema rather than hand-copied.

This avoids two common failures: treating `.proto` as a complete product semantics model, or letting Rust and TypeScript drift into parallel DTO definitions.

### Wire contracts

Use Protobuf messages for:

- command envelopes;
- command payload variants;
- session snapshots;
- reply/correlation envelopes;
- event-log records;
- adapter capability descriptors;
- explicit enum vocabularies shared between Rust and TypeScript.

If the web cockpit needs HTTP/JSON ergonomics, prefer a Protobuf-compatible transport such as Connect or ProtoJSON at the boundary rather than a second independent JSON schema. Connect's public docs describe plain HTTP/curl support, browser use, generated clients, and Protocol Buffers as the API definition source [connectrpc]{1} [connectrpc]{3} [connectrpc]{4}. ProtoJSON is available as a canonical JSON encoding [protojson]{1}, but it has important limits: it is not meant to represent arbitrary JSON schemas [protojson]{2}, and it has weaker schema-evolution properties than the binary wire format because it lacks unknown-field support and encodes field/enum names [protojson]{4}.

### Generated Rust and TypeScript types

Generate and treat outputs as artifacts:

- TypeScript: Protobuf-ES generated `_pb.ts` files, exported schema descriptors, and runtime helpers [protobuf-es]{1} [protobuf-es]{3}.
- Rust: prost/prost-build generated modules [prost]{1} [prost]{3}.
- Generation config: checked-in `buf.gen.yaml`, with pinned plugins where feasible [buf-generate]{2} [buf-generate]{3}.

Do not edit generated outputs by hand; regenerate them. Protobuf-ES explicitly states this for its generated files [protobuf-es]{1}.

### Conformance vectors

Put protocol conformance vectors in a transport-neutral corpus, for example:

```text
contracts/
  proto/patchbay/v1/*.proto
  buf.yaml
  buf.gen.yaml
  vectors/
    command_lifecycle/*.json
    snapshots/*.json
    authority/*.json
```

Each vector should name:

- schema version or module digest;
- initial state;
- input event/command;
- expected accepted/rejected result;
- expected durable state transition or snapshot;
- expected failure vocabulary term when applicable.

The key rule is that both generated Rust and generated TypeScript paths consume the same corpus. Schema generation proves shape compatibility; vectors prove semantic compatibility.

## Alternatives assessed

### JSON Schema as the primary source

JSON Schema is strong when the boundary is JSON-first. Its specification defines a JSON media type for describing JSON data structure and asserts what a JSON document must look like [json-schema-core]{1}. That is a useful fit for configuration files, public REST shapes, admin import/export, or validation of arbitrary JSON payloads.

It is a weaker first choice for Patchbay's central durable protocol because Patchbay is not just validating JSON documents; it needs cross-language generated types, enum/variant registries, wire compatibility checks, and durable event semantics shared by Rust and TypeScript. The attested JSON Schema core source establishes structure/validation, not Rust/TypeScript generation lifecycle or protocol compatibility governance [json-schema-core]{1}.

### TypeBox or Zod as the primary source

The TypeBox docs describe a TypeScript-side validation tool that creates JSON Schema objects, infers TypeScript types, and can be runtime-checked with JSON Schema validation [typebox]{1} [typebox]{2}. The Zod docs describe JSON Schema conversion and conversion from JSON Schema, but mark JSON import as experimental and state that some Zod types cannot be reasonably represented as JSON Schema [zod-json-schema]{1} [zod-json-schema]{2} [zod-json-schema]{4}.

For Patchbay, these tools fit local operator-domain validation and UI form/config validation better than the core Rust/TypeScript wire source. A TypeScript-first schema risks making Rust a downstream consumer of TypeScript choices, which conflicts with the Rust core's role as the authoritative coordination boundary.

### TypeSpec as the primary source

TypeSpec is the candidate to revisit if Patchbay later wants one authoring language emitting OpenAPI, JSON Schema, and Protobuf. Its docs say the standard library includes emitters for OpenAPI, JSON Schema, and Protobuf, and frame TypeSpec as a single source for data shapes [typespec]{1} [typespec]{2}. Its Protobuf emitter can generate `.proto` files [typespec-protobuf]{1}.

The caution is that TypeSpec would add an IDL layer above Protobuf before Patchbay has a stable first protocol. The TypeSpec Protobuf guide says models must comply with Protobuf-specific rules and limitations [typespec-protobuf]{3}, including explicit field decorators for every Protobuf field [typespec-protobuf]{4}. That means TypeSpec would not remove Protobuf's evolution discipline; it would move it into a higher-level authoring tool. For the first executable slice, that extra layer is not necessary.

### OpenAPI as the primary source

OpenAPI is a better fit for HTTP endpoint documentation and REST clients than for Patchbay's core command/event protocol. OpenAPI Generator has Rust and TypeScript client generators, but the Rust generator metadata is client-oriented and its help text says the Rust client library is beta [openapi-generator-rust]{3} [openapi-generator-rust]{4}; the generic TypeScript generator is marked experimental and beta in its own docs [openapi-generator-typescript]{2} [openapi-generator-typescript]{4}.

Use OpenAPI later for public HTTP/admin surfaces if needed. Do not make it the first source for command lifecycle, event-log, snapshot, and adapter-capability contracts.

## Disconfirming analysis

The main evidence against Protobuf as the universal answer is JSON expressiveness and evolution tradeoff. ProtoJSON is not designed to represent arbitrary JSON schemas [protojson]{2} and lacks the binary wire format's unknown-field evolution behavior [protojson]{4}. Therefore, if Patchbay has genuinely JSON-native extension payloads, those should use JSON Schema or opaque JSON fields with explicit validation boundaries rather than forcing every shape into Protobuf.

A reason not to use JSON Schema-family tools as the first protocol source is cross-language governance. The JSON Schema attestation records structure and validation claims [json-schema-core]{1}; the TypeBox and Zod attestations record TypeScript-oriented validation and JSON Schema conversion claims [typebox]{1} [zod-json-schema]{1}. These sources do not establish the same integrated Rust/TypeScript code-generation and breaking-change lifecycle available from Buf + Protobuf [buf-generate]{1} [buf-breaking]{1}.

The main evidence against TypeSpec is timing. It can emit multiple artifact formats [typespec]{1}, including Protobuf [typespec-protobuf]{1}, but Protobuf-specific field numbering and limitations still surface in the TypeSpec authoring model [typespec-protobuf]{3} [typespec-protobuf]{4}. Adopt TypeSpec when multi-protocol emission becomes a real requirement, not as a preemptive abstraction.

## Contradictions and tensions

No direct source contradiction surfaced. The important tension is architectural:

- JSON Schema and TypeScript-first schema tools are better aligned with arbitrary JSON validation and UI/runtime validation [json-schema-core]{1} [typebox]{2}.
- Protobuf + Buf is better aligned with cross-language generated DTOs and schema lifecycle checks for a Rust/TypeScript protocol boundary [buf-generate]{1} [buf-breaking]{1}.
- ProtoJSON bridges Protobuf into JSON, but the Protobuf docs explicitly warn that ProtoJSON is not an arbitrary JSON Schema representation and has weaker schema-evolution properties than binary Protobuf [protojson]{2} [protojson]{4}.

Patchbay should accept that tension by using Protobuf for durable protocol contracts and JSON Schema-family tools for JSON-native edges.

## Implementation notes for Patchbay

1. Add `contracts/proto/patchbay/v1/` with initial envelopes and registries:
   - `command.proto`
   - `session.proto`
   - `event.proto`
   - `adapter.proto`
   - `authority.proto`
2. Add `contracts/buf.yaml` and `contracts/buf.gen.yaml`.
3. Generate:
   - `crates/patchbay-protocol/src/generated/` for Rust;
   - `packages/protocol/src/generated/` for TypeScript.
4. Add CI checks:
   - `buf lint`;
   - `buf breaking --against '.git#branch=main'` once a baseline exists;
   - generation drift check;
   - Rust and TypeScript conformance-vector tests.
5. Set the Buf breaking policy deliberately:
   - start with `WIRE_JSON` if web/JSON transport compatibility matters immediately;
   - use a stricter source-code category if generated language packages become external consumer APIs.
6. Keep `docs/PROTOCOL.md` as explanatory semantics, but derive or check enum tables against the schema once `.proto` becomes authoritative.
7. Do not place Pi-specific concepts into the shared protocol; expose them through adapter capability messages.

## Revisit if

- Patchbay chooses a REST/OpenAPI-first public API surface before durable event/command contracts stabilize.
- External consumers need OpenAPI and Protobuf as peer artifacts from the same authoring source.
- The TypeScript operator domain becomes the true authority boundary instead of the Rust core.
- ProtoJSON limitations block a required JSON-native extension shape.
- A future adapter/client ecosystem requires generated SDK distribution rather than repo-local generation.

## Acquisition candidates

None. No load-bearing source was blocked during this engagement.
