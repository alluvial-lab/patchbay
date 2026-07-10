---
id: epic-public-product-contract-verification-claim-correction
kind: feature
stage: drafting
tags: [verification, protocol, foundation]
parent: epic-public-product-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Verification claim correction

## Brief

Make every verification artifact claim only what its formula, modeled failure boundary, and independent evidence support. Re-inventory current HEAD using the epic's three-way work classification: remove or correct artifacts valueless at every scale, preserve useful seams while deferring their implementation, and keep machinery that serves the committed product. Initial review candidates are evidence to investigate, not a stale deletion checklist, because completed foundation work has already corrected some findings.

The feature is the home for rewriting, renaming, demoting, relocating, or removing remaining overclaims: lifecycle properties that do not represent the durability or race boundary named; weak draft crash/replay/snapshot formulas; fact-consequence Alloy checks; generated TLA+ presented anywhere as independent evidence; toy models appearing in the product inventory; stale semantic traceability; and metadata/process machinery described as behavioral assurance. It preserves the property-graded program, genuine-checking mutation discipline, independently useful Alloy/TLA+/Quint roles, and future-useful authority and protocol seams. It creates honest inputs for executable release assurance; it does not substitute metadata validation for running implementation evidence.

## Epic context

- Parent epic: `epic-public-product-contract`
- Position in epic: independent correction arc; executable release assurance depends on its reconciled property identities and claims.
- Reuse completed `feature-formal-model-realignment` work rather than replaying it.

## Foundation references

- `docs/VERIFICATION.md` — v1 release assurance policy; promotion and genuine-checking rules
- `docs/PROTOCOL.md` — canonical semantics the models claim to represent
- `specs/seed/` — current model inventory
- `contracts/scripts/check-models.mjs` — metadata/traceability check, not a model runner
- `contracts/scripts/check-vectors.mjs` — vector metadata check, not an implementation executor
