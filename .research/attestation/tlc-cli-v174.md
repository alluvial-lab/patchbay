---
source_handle: tlc-cli-v174
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/tlaplus/tlaplus/v1.7.4/tlatools/org.lamport.tlatools/src/tlc2/TLC.java
provenance: source-direct
---

# Per-source attestation: tlc-cli-v174

## Paraphrased summary

The TLC v1.7.4 source file documents and implements the command-line arguments accepted by the TLC model checker. It identifies model-checking mode, configuration-file handling, deadlock checking behavior, liveness-check scheduling, worker count parsing, and input-module handling.

## Key passages

[tlc-cli-v174]{1} The source comment lists model checking as `java tlc2.TLC [-modelcheck] spec[.tla]` and simulation as `java tlc2.TLC -simulate spec[.tla]`.

[tlc-cli-v174]{2} The source comment describes `-config file` as providing the config file and says TLC defaults to `spec.cfg` when `-config` is not provided.

[tlc-cli-v174]{3} The source comment describes `-deadlock` as “do not check for deadlock” and says the default is to check deadlock when that option is not specified.

[tlc-cli-v174]{4} The source comment describes `-workers num` as the number of TLC worker threads and says the default is 1.

[tlc-cli-v174]{5} The parameter parser for `-config` records the following argument as the config file and strips a trailing `.cfg` extension before storing it.

[tlc-cli-v174]{6} The parameter parser for `-deadlock` sets the internal `deadlock` flag to `false`.

[tlc-cli-v174]{7} The parameter parser for `-workers` accepts either an integer or the string `auto`; when `auto` is used, it selects `Runtime.getRuntime().availableProcessors()`.

[tlc-cli-v174]{8} The parameter parser accepts `-modelcheck` as an option and otherwise treats the non-option argument as the main input module, rejecting more than one input file.

[tlc-cli-v174]{9} The parameter parser accepts `-lncheck` with a following strategy string and reports an error if no strategy is supplied.

[tlc-cli-v174]{10} The source comment describes `-continue` as continuing to run even when an invariant is violated; the default is to stop at the first violation.

## Structural metadata

- Source type: Java source file.
- Repository tag fetched: `v1.7.4`.
- Class: `tlc2.TLC`.
