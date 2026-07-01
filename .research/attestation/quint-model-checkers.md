---
source_handle: quint-model-checkers
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/informalsystems/quint/main/docs/content/docs/model-checkers.mdx
provenance: source-direct
---

# Per-source attestation: quint-model-checkers

## Paraphrased summary

The Quint model-checkers documentation explains TLC and Apalache as model-checking backends for Quint, describes TLC as an explicit-state checker reached through TLA+ transpilation, and states TLC-specific constraints and supported property classes.

## Key passages

[quint-model-checkers]{1} The document says Quint specifications can be verified by two model checkers: TLC and Apalache.

[quint-model-checkers]{2} The TLC section says TLC is the first model checker for TLA+ and can be used for Quint by transpiling Quint specs into TLA+.

[quint-model-checkers]{3} The TLC section says TLC is an explicit-state model checker that enumerates possible states, keeping a reachability graph and a queue of states to check next.

[quint-model-checkers]{4} The TLC section says TLC cannot pick a number from the set of all integers because the set is infinite, so a constrained set must be defined.

[quint-model-checkers]{5} The TLC outline says TLC is not integrated with Quint and requires transpilation into TLA+.

[quint-model-checkers]{6} The TLC outline says TLC enumerates all possible states, requires the state space to be small enough, checks executions of any length, and checks invariants and temporal properties.

[quint-model-checkers]{7} The Apalache section contrasts bounded checking by saying Apalache needs a `--max-steps` argument and verification is for a specified bound.

[quint-model-checkers]{8} The Apalache section says Apalache checks invariants and that temporal properties have partial support (source line 56: "Checks invariants. Temporal properties have partial support").

## Structural metadata

- Source type: repository documentation file.
- Repository branch fetched: `main`.
- Document: Quint model checkers documentation.
