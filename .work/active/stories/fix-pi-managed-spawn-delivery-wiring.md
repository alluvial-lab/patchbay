---
id: fix-pi-managed-spawn-delivery-wiring
kind: story
stage: implementing
tags: [adapter, verification]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-16
updated: 2026-08-16
---

# Fix: managed spawn claims never reach the Pi supervisor from the delivery loop

## Reproduction (live UAT, 2026-08-16)

Operator clicked cockpit "+" ten times. Core accepted ALL TEN spawn submits
(ten durable `SPAWN_CLAIM` events, commands `command-93d8ca1b…` etc.). Zero
command transitions, zero effect-journal entries, no `pi --mode rpc` child,
empty adapter log. The claims sit undelivered; the runtime never launches.

## Root cause (diagnosed to the line)

`pi-adapter/src/delivery.ts:65` and `:98` still throw
`UnsupportedCommandError("Pi spawn is unsupported in v0.1.0")` — and the
delivery loop never offers managed-target spawn claims to the
`SpawnSupervisor` at all (the Unit-3 supervisor + journal + handshake +
materialization machinery is fully landed and reviewed, but unreachable from
the production delivery path; its tests drive it directly). The spawn-feature
review flagged `delivery.ts`'s stale rejection as deferred-by-scope; the Pi
Unit-3 rework was supposed to replace this path and the wiring was missed.

Secondary (same story): the supervisor's claim delivery must consume
`Delivery.accepted_spawn` (Unit 2's exact-envelope carriage) rather than a
bare Operation.

## Fix

- Wire the delivery loop: managed-target spawn deliveries (the
  `accepted_spawn` envelope) route to the `SpawnSupervisor` (launch/continuate
  per the fixed 10-step order); remove/replace both stale
  "unsupported in v0.1.0" throws.
- Non-managed/session targets keep existing behavior.
- Add an integration test that drives a spawn claim through the REAL delivery
  loop (not the supervisor directly) against the offline fixture runtime, and
  extend the real-process e2e to assert a child launches + the journal writes.

## Acceptance

- [ ] A spawn claim offered through the production delivery loop launches the
      supervised runtime (journal written, claim transitions observed).
- [ ] Live-stack retest: cockpit "+" produces a live managed session.
- [ ] Full four verification groups + pi-adapter suites (incl. mutations)
      green.
