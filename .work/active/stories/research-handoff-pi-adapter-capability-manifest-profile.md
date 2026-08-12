---
id: research-handoff-pi-adapter-capability-manifest-profile
kind: story
stage: implementing
tags: [adapter, protocol]
parent: research-handoff-pi-adapter-capability
depends_on: [research-handoff-spawn-restart-continuation-orchestration]
release_binding: null
gate_origin: null
research_origin: v1-control-plane-and-spawn
created: 2026-08-12
updated: 2026-08-12
---

# Generated Pi runtime capability profile

## Checkpoint

Extend the generated advisory adapter manifest with the minimum runtime-session profile and make the Pi adapter declare its actual v1 substrate and limits: RPC JSONL subprocess transport; prompt/steer/follow-up; partially ordered live events with parallel-tool interleaving; persisted-entry cursor replay; JSONL session persistence; supervised process continuation; extension-resource-only reload; cwd/trust/resource scope; and persisted-only state rehydration.

The profile remains advisory. It does not replace grants, core delivery, authenticated reports, or adapter delivery outcomes. Fresh runtime-session attachment must be structurally complete; replay of pre-profile durable registrations normalizes missing fields to conservative unknown/false values only. The sibling `capability-manifest-durability-and-reconciliation-depth` owns later cross-adapter assurance-strength declarations and must extend this profile rather than duplicate its mechanism fields.

## Design

**Files**
- `contracts/proto/patchbay/adapter.proto` and `contracts/proto/patchbay/diagnostics.proto` — generated runtime-session capability messages/enums and diagnostic projection.
- `core/src/adapter/capability.rs` and `core/src/adapter/mod.rs` — fail-fast fresh-attach validation and replay-only conservative legacy normalization.
- `core/src/diagnostics/mod.rs`, `cli/src/output/diagnostics.ts`, and cockpit capability consumers — preserve the generated advisory profile without turning it into authority.
- `pi-adapter/src/core_client.ts` — one `piCapabilityManifest()` declaration including `spawn` and the managed target-spec shape only when supervision is configured.
- `docs/PROTOCOL.md`, `docs/ARCHITECTURE.md`, `docs/ADAPTER-PI.md`, and `docs/GLOSSARY.md` — roll the v1 declaration and capability-not-authority rule forward.

The profile uses bounded adapter-owned mechanism ids plus typed guarantees. The Pi declaration selects `pi-rpc-jsonl`, partial event ordering, explicit unknown-cursor rejection plus full replay, supervised process continuation, extension-resource reload, and persisted-only rehydration. Its session snapshot tier remains `partial`: persisted transcript recovery does not prove process liveness or arbitrary in-memory state.

## Acceptance evidence

- [ ] Rust and TypeScript generated contracts and drift checks contain every minimum field exactly once.
- [ ] Fresh runtime-session registration rejects missing/unspecified required profile members before durable registration; historical replay conservatively normalizes a pre-profile record.
- [ ] Pi declares `spawn`, shape `patchbay.pi.managed-rpc.v1`, process isolation, partial event order, cursor replay/full-resync behavior, resource-only reload, and persisted-only rehydration only when the configured implementation provides them.
- [ ] `session_snapshot_support=partial` and `idempotency_strength=at_patchbay_boundary` remain honest.
- [ ] A control surface may hide an unavailable action from the profile, but core delivery still relies on grants and the adapter's current delivery result.
- [ ] The sibling durability/reconciliation-depth feature has a named additive seam and no duplicate cursor/continuation mechanism registry.

## Ordering constraint

Consumes the spawn continuation contract before declaring replacement semantics.
