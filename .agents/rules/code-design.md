# Code Design Rules

Patchbay implementation should follow these rules once code exists:

- **Ports & Adapters:** core domain logic does not depend directly on DB/filesystem/HTTP/time/randomness or a specific agent harness.
- **Single Source of Truth:** state machines, command kinds, adapter capabilities, failure vocabularies, and protocol variants must have one registry/source. Derive types, validation, routing, and display from it.
- **Generated Contracts:** boundary types come from schema/router/DB inference or generation instead of hand-copied DTOs.
- **Fail Fast:** validate unknown input at system boundaries and assert internal preconditions early.
- **Snapshot correctness:** UI state is never authoritative. Reconnect paths reconcile against core snapshots.
- **Adapter neutrality:** Pi-specific capabilities are adapter-declared features, not core protocol primitives.
