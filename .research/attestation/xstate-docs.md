---
source_handle: xstate-docs
fetched: 2026-07-07
source_url: https://stately.ai/docs/state-machines-and-statecharts
additional_source_urls:
  - https://stately.ai/docs/xstate
  - https://stately.ai/docs/actors
  - https://stately.ai/docs/persistence
provenance: source-direct
---

# Stately XState documentation

## Structural metadata

- Source type: official Stately/XState documentation pages.
- Fetched representation: HTML rendered to text with `lynx`.
- Local fetched copies: `.research/fetched/v0-stack-tooling/ts-web-and-browser/xstate-*.txt`.

## Paraphrased source summary

The XState docs describe state machines and statecharts for deterministic event-driven application/workflow logic, actors as running processes with encapsulated state, emitted snapshots and subscriptions, and persistence/restoration of actor state, with caveats around serialization and incompatible machine changes.

## Key passages

1. The docs say state machines model how a process moves from state to state when an event occurs and help capture states, events, and transitions.

2. The docs say state machines help find impossible states and undesirable transitions.

3. The benefits list says state machines are deterministic and simple to test because all possible states and transitions can be tested.

4. The docs say statecharts extend state machines with hierarchy, concurrency, and communication.

5. The docs define events as causing transitions; transitions are deterministic because each combination of state and event always points to the same next state.

6. The docs say parallel states have multiple active regions simultaneously.

7. The actors page says when a state machine runs, it becomes an actor: a running process that can receive events, send events, and change behavior based on events.

8. The actor model section says actors have encapsulated internal state, communicate asynchronously by events, and process one message at a time through an internal mailbox.

9. The actors page says actors emit snapshots when transitions occur, and snapshots can be read synchronously or observed by subscription.

10. The persistence page says actors can persist internal state and restore it later; `actor.getPersistedSnapshot()` obtains persisted state and `createActor(behavior, { snapshot: restoredState }).start()` restores it.

11. The persistence page says event sourcing can restore state by replaying events and can be more reliable than persisting state because it is less prone to incompatible state and allows replaying actions.

12. Persistence caveats include incompatible state when actor logic changes, actions not re-executing on restoration, and JSON-serializability requirements.
