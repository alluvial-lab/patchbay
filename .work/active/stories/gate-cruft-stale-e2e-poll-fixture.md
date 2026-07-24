---
id: gate-cruft-stale-e2e-poll-fixture
kind: story
stage: implementing
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
