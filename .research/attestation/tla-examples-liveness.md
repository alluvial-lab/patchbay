---
source_handle: tla-examples-liveness
fetched: 2026-07-01
source_url: https://raw.githubusercontent.com/tlaplus/Examples/master/specifications/SpecifyingSystems/Liveness/LiveWriteThroughCache.tla
provenance: source-direct
---

# Per-source attestation: tla-examples-liveness

## Paraphrased summary

The TLA+ Examples liveness module extends a cache example with weak and strong fairness formulas and composes those formulas into a liveness-enriched specification.

## Key passages

[tla-examples-liveness]{1} The source defines `vars == <<memInt, wmem, buf, ctl, cache, memQ>>`.

[tla-examples-liveness]{2} The source defines `Liveness` using weak fairness formulas such as `WF_vars(Rsp(p) \/ DoRd(p))` and `WF_vars((QCond /\\ MemQWr) \/ MemQRd)`.

[tla-examples-liveness]{3} The source defines `Liveness` using a strong fairness formula `SF_vars(RdMiss(p) \/ DoWr(p))`.

[tla-examples-liveness]{4} The source defines `LSpec == Spec /\\ Liveness`.

## Structural metadata

- Source type: TLA+ source file in examples repository.
- Repository branch fetched: `master`.
- Module: `LiveWriteThroughCache`.
