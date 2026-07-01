---
source_handle: tlc-output-messages-v174
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/tlaplus/tlaplus/v1.7.4/tlatools/org.lamport.tlatools/src/tlc2/output/MP.java
provenance: source-direct
---

# Per-source attestation: tlc-output-messages-v174

## Paraphrased summary

The TLC v1.7.4 message-printer source defines the human-readable messages used for invariant violations, temporal-property violations, counterexamples, behavior traces, state printing, and error prefixes.

## Key passages

[tlc-output-messages-v174]{1} When printing an error-class message, the message builder prefixes output with `Error: `.

[tlc-output-messages-v174]{2} For `TLC_INVARIANT_VIOLATED_INITIAL`, the message text is `Invariant %1% is violated by the initial state:\n%2%`.

[tlc-output-messages-v174]{3} For `TLC_INVARIANT_VIOLATED_BEHAVIOR`, the message text is `Invariant %1% is violated.`.

[tlc-output-messages-v174]{4} For `TLC_INVARIANT_VIOLATED_LEVEL`, the message says the invariant is not a state predicate, described as one with no primes or temporal operators.

[tlc-output-messages-v174]{5} For `TLC_COUNTER_EXAMPLE`, the message text is `The following behavior constitutes a counter-example:\n`.

[tlc-output-messages-v174]{6} For `TLC_TEMPORAL_PROPERTY_VIOLATED`, the message text is `Temporal properties were violated.\n`.

[tlc-output-messages-v174]{7} For `TLC_BEHAVIOR_UP_TO_THIS_POINT`, the message text is `The behavior up to this point is:`.

[tlc-output-messages-v174]{8} State print message `TLC_STATE_PRINT1` formats as `%1%:\n%2%`; state print message `TLC_STATE_PRINT2` formats as `%1%: %2%\n%3%` when debug mode is not enabled.

[tlc-output-messages-v174]{9} For `TLC_DEADLOCK_REACHED`, the message text is `Deadlock reached.`.

## Structural metadata

- Source type: Java source file.
- Repository tag fetched: `v1.7.4`.
- Class: `tlc2.output.MP`.
