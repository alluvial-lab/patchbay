---
id: story-verification-correction-prose-and-toys
kind: story
stage: implementing
tags: [verification, foundation]
parent: epic-public-product-contract-verification-claim-correction
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Fix stale PROTOCOL.md prose and relocate toy examples

## Scope

Fix two stale PROTOCOL.md assertions that contradict current HEAD, relocate hello-world toy examples out of the product seed directory, and audit emitted TLA+ files for any prose presenting them as independent evidence.

## Unit

`Unit 4` from `epic-public-product-contract-verification-claim-correction` design.

## Files

- `docs/PROTOCOL.md` — two stale assertions
- `specs/seed/Counter.qnt`, `specs/seed/Counter.tla`, `specs/seed/Counter.cfg` — toy examples
- `specs/seed/*.emitted.tla` — generated inspection artifacts (audit only)

## Implementation

### PROTOCOL.md fixes

1. **Line 94** — currently says: "The existing `reply_correlation.qnt` does **not** cover response Operation → Elicitation. Extending typed correlation is a new verification obligation."

   Current HEAD: `TypedCorrelation` in `reply_correlation.qnt` now covers both Reply → Command/Message AND response Operation (`approval-response`/`elicitation-response`) → Elicitation typed references. Update to reflect that the coverage exists, while noting it is checked-model (not checked-normative until vectors are promoted).

2. **Line 142** — currently says: "the current checked model permits any non-terminal state to commit any terminal candidate, so adjacency rules such as no `accepted → completed` require a strengthened lifecycle model..."

   Current HEAD: `NoAcceptedToCompleted` is now a checked-model property, and `allowedTransition` enforces the exact PROTOCOL transition table. Update to reflect that the no-`accepted → completed` adjacency is now checked, while the full transition graph and read/query fast-path rule remain stated-normative.

### Toy example relocation

`Counter.qnt`, `Counter.tla`, and `Counter.cfg` are hello-world tooling examples, not product verification. No references to them exist outside the files themselves (verified via `rg Counter`). Remove them from `specs/seed/`. If the `.agents/skills/quint/` or `.agents/skills/tla-plus/` skill directories would benefit from a tooling example, relocate there; otherwise delete.

### Emitted TLA+ audit

VERIFICATION.md already states that `*.emitted.tla` files are generated inspection artifacts, not an independent verification lane. Audit all prose (docs, README, work items) for any claim that presents emitted TLA+ as independent evidence. If found, correct to "generated inspection artifact, not independently checked." Expected outcome: no corrections needed (the discipline is already honest), but verify.

## Acceptance criteria

- [ ] PROTOCOL.md line 94 corrected: `TypedCorrelation` now covers response Operation → Elicitation.
- [ ] PROTOCOL.md line 142 corrected: `NoAcceptedToCompleted` is checked-model; `allowedTransition` enforces the exact table; full adjacency graph remains stated-normative.
- [ ] `Counter.qnt`, `Counter.tla`, `Counter.cfg` removed from `specs/seed/` (relocated or deleted).
- [ ] `*.emitted.tla` files audited: no prose presents them as independent evidence.
- [ ] `node contracts/scripts/check-models.mjs` exits 0 (Counter files are not `@promotion`-bearing, so removal doesn't affect traceability).
