---
id: gate-cruft-stale-e2e-poll-fixture
kind: story
stage: done
tags: [cleanup]
parent: null
depends_on: []
release_binding: v0.1.0
gate_origin: cruft
created: 2026-07-24
updated: 2026-07-24
---

# Replace the stale polling E2E adapter fixture

## Confidence
High

## Category
stale test fixture

## Location
`e2e/pi-adapter-fixture.mjs:98`

## Evidence
```js
await adapter.start();
console.log(`PI_ADAPTER_READY ${runtimeSessionId}`);
while (!stopped) {
  const delivered = await adapter.pollOnce();
  if (delivered > 0) console.log(`PI_ADAPTER_PROCESSED ${delivered}`);
  if (delivered === 0) await delay(50);
}
```

`AdapterProcess` now exposes only `start`, `run`, and `dispose` (`pi-adapter/src/main.ts:64,135,168`); `git grep pollOnce` finds this as its sole source caller. The delivery-loop replacement commit `a7d3058` changed `pi-adapter/src/main.ts` but not this fixture. Consequently `cd e2e && npm test` exits 1 after the submitted command never reaches `working`.

## Removal
Replace the obsolete `pollOnce()`/50ms polling loop with cancellation-aware `adapter.run(signal)` lifecycle management. Update the walking-skeleton harness's `PI_ADAPTER_PROCESSED` synchronization to observe durable command/session completion instead of a removed batch-count side channel, then verify `cd e2e && npm test`.

## Implementation

- Replaced the removed `pollOnce()` loop and artificial 50ms delay with `adapter.run(controller.signal)` and abort-driven SIGINT/SIGTERM handling.
- Removed the `PI_ADAPTER_PROCESSED` batch-count side channel. The harness now resumes the authenticated core event stream from the accepted LSN, decodes command transitions, requires the submitted command to reach durable `completed`, fails on any other terminal state, and then verifies the session is live/idle.
- Updated setup/login fixtures to pass secrets through environment variables because the hardened CLI rejects secret-bearing argv flags.
- Documented the separate-process suite and its relationship to package integration tests in `docs/RUNBOOK.md`.

Execution capability: direct host ownership of the cohesive E2E fixture and synchronization repair.

## Verification

- `cd e2e && npm test` — passed: `Walking skeleton: core → Pi adapter/AgentSession → CLI login/instruct → durable completed/idle passed`.
- `git diff --check` — passed.

## Bounded review

The interrupted worker's initial repair correctly migrated to `adapter.run(signal)`, but retaining a poll for transient session `working` was flaky: the first verification run observed the final durable LSN and idle state without catching that intermediate projection. Replaced that synchronization with the command's durable transition stream, which directly proves delivery/execution completion and avoids timing dependence. No material blocker remains.
