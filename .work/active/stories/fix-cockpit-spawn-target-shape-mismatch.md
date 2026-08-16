---
id: fix-cockpit-spawn-target-shape-mismatch
kind: story
stage: review
tags: [verification, adapter]
parent: null
depends_on: [fix-pi-managed-spawn-delivery-wiring]
release_binding: null
gate_origin: null
created: 2026-08-16
updated: 2026-08-16
---

# Fix: cockpit spawn payloads use a target shape the Pi supervisor rejects

## Reproduction (live UAT, 2026-08-16)

Delivery wiring fixed (fix-pi-managed-spawn-delivery-wiring): a spawn claim now
flows accepted→delivered→running. But every cockpit "+"/restart spawn still
ends unsupported: the supervisor rejects the payload at
`pi-adapter/src/spawn_supervisor.ts:974` — it requires target-spec shape
`"pi-rpc"` (declared in the adapter capability as `supportedTargetSpecShapes`)
plus an adapter payload (`PiSpawnTargetSpec` with `projectContextRef` matching
a configured managed target), while the cockpit builds payloads with
`shape:"session"` and no adapter payload
(`web-cockpit/src/main.ts` `buildFreshSpawnOperation`/`buildRestartOperation`
→ `operator-domain/src/spawn.ts` `freshSpawnPayload({shape:"session"})`).

The ten original UAT "+" spawns were delivered to the OLD adapter (pre-wiring),
rejected without no-effect proof, and are correctly
`poisoned_pending_reconciliation` — they demo the poison path but can never
produce sessions.

Component tests passed because the e2e/tests construct `pi-rpc`-shaped
payloads directly; the cockpit shape was never exercised against the real
supervisor.

## Fix (bounded, adapter-neutral)

The cockpit must build spawn payloads from the TARGET ADAPTER'S DECLARED
capability, not a hardcoded shape:

1. The cockpit's adapter/session projections already carry the adapter
   capability summary (see `adapter-status` output); extend it in the browser
   model with `supportedTargetSpecShapes` (+ any per-shape requirements the
   manifest already declares).
2. `buildFreshSpawnOperation`/`buildRestartOperation` take the target
   adapter's declared shape; if the adapter declares exactly one shape, use
   it; zero or multiple shapes → the spawn action is disabled with a canonical
   reason (no silent fallback). For adapters declaring `pi-rpc`, the payload
   carries the `PiSpawnTargetSpec` envelope (projectContextRef from the
   selected managed target surfaced through the capability/profile; for the
   single-managed-target UAT shape this is the configured ref).
3. CLI `spawn` gains the same declared-shape derivation (its current bare
   no-payload spawn is diagnostic-only and should carry the real payload).
4. Regression: a cockpit-built spawn against the real supervisor path (offline
   fixture runtime) launches; a shape-mismatch mutation fails; a
   zero/multi-shape adapter disables the action.

If the capability surface does not yet expose what the cockpit needs (shape
list / project context refs), extend the generated capability summary — proto
window coordinated with the orchestrator.

## Acceptance

- [ ] Cockpit "+" produces a spawn the Pi supervisor accepts (delivery →
      launch → journal → staged → promoted → live) on the live stack.
- [ ] Restart (continuation) same.
- [x] Adapter-neutral: no Pi vocabulary hardcoded in cockpit; undeclared
      adapters disable the action canonically.
- [x] Full four verification groups + web-cockpit/cli suites green.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` (caller-selected for the
  generated-contract, adapter-boundary, and multi-surface integration). Direct
  implementation was used because this delegated worker cannot fan out under
  the harness recursion guard.
- Review weight: `standard` (project default). This standalone fix is left at
  `stage: review` for the orchestrator's bounded review and live UAT checkpoint.
- Contract and validation: `AdapterCapability` and canonical diagnostics now
  carry bounded `managed_spawn_targets`, binding a logical-target id and a
  declared shape to fresh and optional continuation opaque payload templates.
  Core registration validates shape/id uniqueness, declared-shape membership,
  operation/category coherence, envelope framing, and bounds without decoding
  adapter-specific bytes.
- Pi declaration: each configured managed target now emits exact `pi-rpc`
  `PiSpawnTargetSpec` templates: unspecified continuation mode for fresh spawn
  and `require_resume` for continuation. The bounded project-context reference
  is the only Pi construction value surfaced; paths, labels, and credentials
  remain excluded.
- Shared consumers: `operator-domain` owns fail-closed managed-target selection
  and canonical reasons. Cockpit fresh/restart and CLI spawn/restart both reuse
  it, pass through the exact declared opaque payload, and reject zero/multiple
  shapes, unavailable capability, undeclared targets, shape mismatch, and
  absent intent payloads before submission. Fresh managed command identity is
  the declared logical-target id; restart preserves the exact prior runtime
  generation.
- Regression coverage: core malformed-manifest tests, Pi manifest and real-core
  diagnostics assertions, production delivery-loop construction through the
  shared helper, cockpit/CLI exact protobuf assertions, and zero-shape UI/CLI
  fail-closed tests. The real Pi lifecycle E2E exercises fresh launch,
  journal/stage/promotion/live publication and exact continuation with these
  shared templates.
- Mutation evidence: the existing Pi mutation suite killed **31/31**. Four
  focused new manual probes also died: hardcoding `session` instead of the
  declared shape, keeping zero-shape spawn enabled, swapping fresh to the
  continuation payload, and omitting the configured Pi project-context
  reference (**4/4**).
- Full verification (2026-08-16):
  1. Rust workspace build/tests, `cargo fmt --all --check`, workspace clippy
     with `-D warnings`, and `cargo check -p patchbay-core` — **PASS**.
  2. Contract generated drift, vectors, model promotion, and TypeScript build —
     **PASS** (60 vectors, 19 promoted vectors, 33 implementation checks, 38
     contract mutation witnesses).
  3. `operator-domain` build/tests — **PASS** (34/34).
  4. `pi-adapter` build/tests plus mutations — **PASS** (129/129; 31/31).
  Consumer suites: `web-cockpit` **150/150**, CLI **54/54** plus real-core
  resource projection, and token-commune adapter **63/63** including both
  real-core flows.
- The live UAT stack was not restarted or otherwise touched, per operator
  instruction. The fresh/restart live-UAT acceptance checkboxes remain open for
  operator confirmation; automated real-core/real-Pi coverage is green.
- Pre-existing untracked `cli/rt-probe.tmp.mjs` was preserved and excluded from
  this change. No worktree was created and nothing was pushed.
