---
source_handle: tlc-liveness-v174
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/tlaplus/tlaplus/v1.7.4/tlatools/org.lamport.tlatools/src/tlc2/tool/liveness/LiveWorker.java
provenance: source-direct
---

# Per-source attestation: tlc-liveness-v174

## Paraphrased summary

The TLC v1.7.4 liveness-worker source explains how TLC detects and prints liveness counterexamples: it searches strongly connected components against possible error-model conditions, prints temporal-property violation headers, constructs a prefix and a cycle, prints states, and ends with a stuttering or back-to-state marker.

## Key passages

[tlc-liveness-v174]{1} The source describes checking a component and says a counterexample is found when all required possible-error-model conditions are satisfied; if this thread is first to find it, it calls `printTrace`.

[tlc-liveness-v174]{2} The `printTrace` method first prints `TLC_TEMPORAL_PROPERTY_VIOLATED` and then `TLC_COUNTER_EXAMPLE`.

[tlc-liveness-v174]{3} The source says `printTrace` constructs a prefix path from an initial node to the state in the strongly connected component and says the prefix and cycle together form the complete counterexample.

[tlc-liveness-v174]{4} The source prints recovered prefix states with `StatePrinter.printState`, then prints the cycle/postfix states, and finally prints either a stuttering marker or a back-to-state marker.

[tlc-liveness-v174]{5} A source comment gives an example liveness-violation trace shape with `Temporal properties were violated.`, `The following behavior constitutes a counter-example:`, numbered states such as `1: <Initial predicate>`, variable assignments such as `x = 0`, and a `Back to state` marker.

## Structural metadata

- Source type: Java source file.
- Repository tag fetched: `v1.7.4`.
- Class: `tlc2.tool.liveness.LiveWorker`.
