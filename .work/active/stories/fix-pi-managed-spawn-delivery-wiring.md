---
id: fix-pi-managed-spawn-delivery-wiring
kind: story
stage: review
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

- [x] A spawn claim offered through the production delivery loop launches the
      supervised runtime (journal written, claim transitions observed).
- [ ] Live-stack retest: cockpit "+" produces a live managed session.
- [x] Full four verification groups + pi-adapter suites (incl. mutations)
      green.

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol` (caller-selected for the
  exact-envelope, external-effect, and process-supervision boundary). Direct
  read/implementation was used because the integration points were explicit and
  this delegated worker cannot safely fan out under the harness recursion guard.
- Review weight: `standard` (project default). This standalone fix is left at
  `stage: review` for the orchestrator's bounded review and operator UAT retest.
- Delivery wiring: the production loop now treats `Delivery.accepted_spawn` as
  the authoritative discriminator and derives the Operation passed to delivery
  acknowledgement, running transition, and the `SpawnSupervisor` from that exact
  durable envelope. The compatibility `Delivery.operation` is used only for
  ordinary deliveries. A bare/unmanaged spawn still rejects before session
  execution, but the stale "unsupported in v0.1.0" wording was replaced with the
  actual missing-envelope precondition.
- Design/repository discrepancy and rationale: Unit 3 had already landed a
  `SpawnSupervisor` branch in `main.ts`, contrary to the story's statement that
  no branch existed, but the loop selected `Delivery.operation` before inspecting
  `accepted_spawn` and had no integration oracle proving the exact envelope owned
  routing. The bounded fix keeps the reviewed supervisor intact and closes that
  compatibility-view bypass rather than redesigning spawn mechanics.
- Files changed: `pi-adapter/src/{main,delivery}.ts`;
  `pi-adapter/tests/{delivery,e2e}.test.ts`;
  `pi-adapter/scripts/mutation-cycle.mjs`; this story. No Protobuf or generated
  contract file changed.
- Tests added/extended: `delivery.test.ts` now drives a fresh accepted claim
  through `AdapterProcess.run` with a fully injected offline RPC runtime and a
  deliberately non-authoritative compatibility Operation. It proves
  delivered/running and all supervisor evidence phases, journal-before-launch,
  staging/promotion/publication, exact claim generation, one launch, and child
  cleanup. The real-process E2E now spies the production runtime port and proves
  the delivery loop launches a live `pi --mode rpc` child, writes the exact-claim
  journal, preserves delivery/running before atomic promotion, launches one
  continuation replacement, and reaps both process groups.
- Mutation evidence: `npm run test:mutations` killed **31/31**. The three new
  witnesses fail when managed spawn falls through to the stale unsupported
  translator, when the supervisor call is skipped, or when routing is rebuilt
  from the bare compatibility Operation instead of `accepted_spawn`.
- Four-step confirmation: the new focused delivery-loop regression passed; the
  complete Pi suite passed **128/128** including both real-core/real-process
  E2Es; the injected production-loop reproduction observed one launch and a
  committed journal receipt; the live UAT process was deliberately not restarted
  or touched, so cockpit retest remains the orchestrator/operator checkpoint.
- Full verification (2026-08-16):
  1. `cargo build --workspace --all-targets && cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings` — **PASS**. An initial run encountered a transient rustdoc `E0463` while shared Cargo artifacts were concurrently rebuilding; the focused doctest and the complete quoted group both passed on the clean rerun.
  2. `cd contracts/ts && npm run check:drift && npm run check:vectors && npm run check:models && npm run build` — **PASS** (60 vectors, 19 promoted, 33 implementation checks, 38 mutation witnesses).
  3. `cd operator-domain && npm run build && npm test` — **PASS** (32/32).
  4. `cd pi-adapter && npm test && npm run test:mutations` — **PASS** (128/128; 31/31 mutations killed).
  Consumer suites: `cd web-cockpit && npm test` — **PASS** (149/149);
  `cd cli && npm test` — **PASS** (53/53 plus real-core resource projection);
  `cd token-commune-adapter && npm test` — **PASS** (63/63, including both
  real-core flows).
- Simplification: one accepted-envelope selection now supplies both dispatch and
  supervisor input; no parallel spawn parser, generation allocator, or alternate
  journal path was added.
- Adjacent issues parked: none.
