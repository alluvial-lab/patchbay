---
source_handle: tla-examples-hourclock
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/tlaplus/Examples/master/specifications/SpecifyingSystems/RealTime/MCRealTimeHourClock.tla
provenance: source-direct
---

# Per-source attestation: tla-examples-hourclock

## Paraphrased summary

The TLA+ Examples real-time hour-clock model shows a model-checking module with `INIT`, `NEXT`, fairness, and temporal formulas including eventuality patterns.

## Key passages

[tla-examples-hourclock]{1} The module extends `RealTimeHourClock` and defines `Init == Hini /\\ now \in Real`.

[tla-examples-hourclock]{2} The module defines `NowNext == /\\ now' \in {r \in Real : r > now} /\\ UNCHANGED hr`.

[tla-examples-hourclock]{3} The module defines `BigNext == /\\ [NowNext]_now /\\ [Hnext]_hr`.

[tla-examples-hourclock]{4} The module defines `Fairness == \A r \in Real : WF_now(NowNext /\\ (now'>r))`.

[tla-examples-hourclock]{5} The module defines `NonZeno == \A r \in Real : <>(now \geq r)`.

[tla-examples-hourclock]{6} The module defines `ImpliedTemporal == \A h \in 1..12 : []<>(hr = h)`.

[tla-examples-hourclock]{7} The module defines `RT == /\\ now \in Real /\\ [][NowNext]_now /\\ \A r \in Real : WF_now(NowNext /\\ (now'>r))`.

[tla-examples-hourclock]{8} The module defines `ErrorTemporal == []((now # 4) => <>[](now # 4))`.

## Structural metadata

- Source type: TLA+ source file in examples repository.
- Repository branch fetched: `master`.
- Module: `MCRealTimeHourClock`.
