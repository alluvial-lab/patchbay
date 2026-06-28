---
id: feature-idempotency-ambiguous-execution
kind: feature
stage: drafting
tags: [prose, protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-command-state-ssot, feature-session-identity-adapter-contract]
---

# Feature: Refine idempotency and ambiguous execution semantics

Patchbay can deduplicate accepted commands at the coordination boundary, but adapters may not guarantee exactly-once external execution. The docs need to distinguish safe retry from maybe-executed ambiguity.

## Scope

- Command id vs idempotency key.
- Idempotency key scope, payload equivalence, and lifetime.
- Acceptance deduplication vs adapter/target execution deduplication.
- Adapter crash-after-execute-before-ack scenario.
- `maybe_executed` / ambiguous state or equivalent.
- UI language for safe retry, unsafe retry, and intentional duplicate.

## Acceptance criteria

- `docs/PROTOCOL.md` no longer overclaims end-to-end idempotency.
- `docs/UX.md` explains retry affordances using precise execution state.
- `docs/VERIFICATION.md` scopes the formal guarantee to Patchbay acceptance unless adapter capability declares stronger semantics.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.
