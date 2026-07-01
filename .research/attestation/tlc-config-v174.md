---
source_handle: tlc-config-v174
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/tlaplus/tlaplus/v1.7.4/tlatools/org.lamport.tlatools/src/tlc2/tool/impl/ModelConfig.java
provenance: source-direct
---

# Per-source attestation: tlc-config-v174

## Paraphrased summary

The TLC v1.7.4 `ModelConfig` source defines the configuration-file keywords and parser behavior for `.cfg` files. It includes keywords for specification selection, init/next selection, invariants, temporal properties, constraints, constants, views, symmetry, and deadlock checking.

## Key passages

[tlc-config-v174]{1} The source defines configuration keywords including `CONSTANT`, `CONSTANTS`, `CONSTRAINT`, `CONSTRAINTS`, `ACTION_CONSTRAINT`, `ACTION_CONSTRAINTS`, `INVARIANT`, `INVARIANTS`, `INIT`, `NEXT`, `VIEW`, `SYMMETRY`, `SPECIFICATION`, `PROPERTY`, `PROPERTIES`, `ALIAS`, `POSTCONDITION`, `POSTCONDITIONS`, `_PERIODIC`, `_RL_REWARD`, `_POSSIBLE`, and `CHECK_DEADLOCK`.

[tlc-config-v174]{2} In parsing, `INIT`, `NEXT`, and `SPECIFICATION` each consume one following identifier and reject duplicate occurrences of the same keyword.

[tlc-config-v174]{3} In parsing, `INVARIANT` and `INVARIANTS` consume a sequence of following identifiers until another recognized config keyword or EOF; each identifier is added to the invariants collection.

[tlc-config-v174]{4} In parsing, `PROPERTY` and `PROPERTIES` consume a sequence of following identifiers until another recognized config keyword or EOF; each identifier is added to the temporal-properties collection.

[tlc-config-v174]{5} In parsing, `CONSTRAINT`/`CONSTRAINTS` and `ACTION_CONSTRAINT`/`ACTION_CONSTRAINTS` similarly consume identifier lists until another config keyword or EOF.

[tlc-config-v174]{6} In parsing, `CHECK_DEADLOCK` consumes one following token and accepts only `TRUE` or `FALSE`; a second occurrence is rejected.

[tlc-config-v174]{7} Constant assignments can use `=` to bind values and `<-` to override definitions; values parsed directly include numbers, strings, booleans, set enumerations, and model values.

## Structural metadata

- Source type: Java source file.
- Repository tag fetched: `v1.7.4`.
- Class: `tlc2.tool.impl.ModelConfig`.
