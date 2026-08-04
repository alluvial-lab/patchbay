---
id: epic-agent-operations-resource-plane-capability-manifest-integration-foundation
kind: story
stage: implementing
tags: [foundation, protocol, adapter]
parent: epic-agent-operations-resource-plane-capability-manifest
depends_on: [epic-agent-operations-resource-plane-capability-manifest-core-admission]
release_binding: null
gate_origin: null
created: 2026-08-04
updated: 2026-08-04
---

# Integrate capability diagnostics, Pi declaration, and foundation contract

## Design checkpoint

Land Unit 3 from the parent design. Extend the existing redacted
`AdapterCapabilitySummary` and its one core mapping with target categories,
explicit session snapshot support, and per-resource declarations; update CLI
adapter-status output without exposing attachment descriptors. Migrate every
repository-owned manifest fixture. Pi must declare only `RUNTIME_SESSION`, its
partial session tier, and no resource kinds. Server evidence must accept a valid
two-kind operational-resource manifest and reject the reserved category without
a durable registration event.

Roll SPEC/ARCHITECTURE/PROTOCOL/VERIFICATION/UX/GLOSSARY assertions forward in
place. PROTOCOL owns the registry and admission rules; UX describes canonical
wrapper plus nested adapter domain projection; VERIFICATION labels current
checks implementation evidence and leaves promoted vectors to the conformance
sibling. Do not claim schema-ref matching semantically validates arbitrary
payload bytes, and do not absorb resource snapshot state or cockpit rendering.

## Acceptance evidence

- Pi attach/e2e remains green with an honest session-only capability manifest.
- Durable registration replay and `adapter-status` preserve exact categories,
  resource kinds, tiers, and schema descriptors while attachment material stays
  redacted.
- CLI JSON and human output distinguish the session tier from every per-resource
  tier; capability output remains diagnostic/advisory rather than authoritative.
- Integration tests prove valid `provider_pool` + `usage_window` declarations
  and fail-closed knowledge-bundle/OKF admission.
- Foundation docs classify operational resources as committed post-v0.1,
  knowledge bundles/OKF v0.2 as reserved, and dynamic adapter UI loading as
  rejected for this arc, with no overstated formal/conformance claim.
- Rust workspace, generated contract, Pi, CLI, vector/model, presentation, and
  drift checks are green.

## Ordering constraint

Depends on the validated manifest/admission API. Resource report/revision state
and cockpit renderer/decoder composition remain sibling feature scope; this
checkpoint integrates only the manifest contract and its existing diagnostic
and producer consumers.
