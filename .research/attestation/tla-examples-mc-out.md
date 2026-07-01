---
source_handle: tla-examples-mc-out
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/tlaplus/Examples/master/specifications/dijkstra-mutex/DijkstraMutex.toolbox/LSpec-model/MC.out
provenance: source-direct
---

# Per-source attestation: tla-examples-mc-out

## Paraphrased summary

The TLA+ Examples repository includes a Toolbox-generated `MC.out` file showing TLC output captured in a model directory. The file contains message framing, parser and semantic-processing lines, model-checking progress, a liveness-checking phase, success text, statistics, and completion text.

## Key passages

[tla-examples-mc-out]{1} The file begins with Toolbox message markers and `TLC2 Version 2.03 of 26 May 2010`.

[tla-examples-mc-out]{2} The file says `Running in Model-Checking mode.`, then `Starting SANY...`, then lists parsed TLA files.

[tla-examples-mc-out]{3} The file reports `Computing initial states...` and then `Finished computing initial states: 3 distinct states generated.`

[tla-examples-mc-out]{4} The file includes progress lines with counts of generated states, distinct states, and queue size.

[tla-examples-mc-out]{5} The file says `Checking temporal properties for the complete state space...`.

[tla-examples-mc-out]{6} The file says `Model checking completed. No error has been found.` and includes fingerprint-collision probability estimates.

[tla-examples-mc-out]{7} The file includes coverage statistics and ends with final state-count/depth reporting and `Finished.`.

## Structural metadata

- Source type: generated TLC output file in examples repository.
- Repository branch fetched: `master`.
- Path suffix: `DijkstraMutex.toolbox/LSpec-model/MC.out`.
