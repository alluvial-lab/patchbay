---
id: fix-cockpit-spawn-target-shape-mismatch
kind: story
stage: implementing
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
- [ ] Adapter-neutral: no Pi vocabulary hardcoded in cockpit; undeclared
      adapters disable the action canonically.
- [ ] Full four verification groups + web-cockpit/cli suites green.
