---
source_handle: smlang
fetched: 2026-07-07
source_url: https://docs.rs/smlang/latest/smlang/
provenance: source-direct
---

# Attestation: smlang state-machine macro docs

## Summary

The smlang docs describe a procedural macro library with a state-machine DSL, generated project documentation, generated state/event/state-machine types, guarded transitions, async guards/actions, wildcard and multi-state transition patterns, and state data. The fetched docs.rs latest page identified the documented crate as `smlang-0.8.0`.

## Key passages

1. From the crate description:

> smlang is a procedural macro library creating a state machine language DSL is to facilitate the use of state machines, as they quite fast can become overly complicated to write and get an overview of.

2. From "Project dependent documentation":

> When this crate is used in a project the documentation will be auto generated in the documentation of the project, this comes from the procedural macro also generating documentation.

3. From the transition DSL comments:

> The generated trait and types are `<name>States`, `<name>Events`, and `<name>StateMachine` respectively.

4. From the transition DSL comments:

> Guards and actions can be async functions.

5. From the transition DSL comments:

> Pattern matching can be used to support multiple states with the same transition event.

6. From the transition DSL comments:

> wildcarding can be used to allow all states to share a transition event.

7. From the transition DSL comments:

> States can contain data.

8. From the transition DSL comments:

> Guards can be logically combined using `!`, `||`, and `&&`.

9. From the fetched docs.rs page metadata:

> smlang-0.8.0
