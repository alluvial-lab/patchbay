---
source_handle: statig
fetched: 2026-07-07
source_url: https://docs.rs/statig/latest/statig/
provenance: source-direct
---

# Attestation: statig hierarchical state-machine docs

## Summary

The statig docs describe hierarchical state machines for event-driven systems, with optional macros, no-std support, state-local storage, introspection, and async actions/handlers. The FAQ distinguishes statig from typestate by positioning it for dynamic systems where external events arrive in runtime order. The fetched docs.rs latest page identified the documented crate as `statig-0.4.1`.

## Key passages

1. From the crate description:

> Hierarchical state machines for designing event-driven systems.

2. From "Features":

> Hierarchical state machines; State-local storage; Compatible with #![no_std], state machines are defined in ROM and no heap memory allocations.

3. From "Features":

> (Optional) macro’s for reducing boilerplate.

4. From "Features":

> Support for async actions and handlers.

5. From "What advantage does this have over using the typestate pattern?":

> The typestate pattern is very useful for designing an API as it is able to enforce the validity of operations at compile time by making each state a unique type.

6. From the same FAQ entry:

> But statig is designed to model a dynamic system where events originate externally and the order of operations is determined at run time.

7. From the same FAQ entry:

> More concretely, this means that the state machine is going to sit in a loop where events are read from a queue and submitted to the state machine using the handle() method.

8. From "Implementation":

> the generated code is actually pretty straight-forward and could easily be written by hand, so if you prefer to avoid using macro’s this is totally feasible.

9. From the fetched docs.rs page metadata:

> statig-0.4.1
