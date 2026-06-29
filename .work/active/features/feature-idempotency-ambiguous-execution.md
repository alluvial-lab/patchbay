---
id: feature-idempotency-ambiguous-execution
kind: feature
stage: drafting
tags: [protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-command-state-ssot, feature-session-identity-adapter-contract]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Feature: Refine idempotency and ambiguous execution semantics

Patchbay can deduplicate accepted commands at the coordination boundary, but adapters may not guarantee exactly-once external execution. The docs need to distinguish safe retry from maybe-executed ambiguity.

## Retag note (2026-06-28)

Retagged from `[prose]` to a design feature. The `prose` tag was removed because the scope includes genuine semantic design choices: a new `maybe_executed` / ambiguous execution state (or equivalent), idempotency-key scope and lifetime rules, and payload-equivalence rules. These are protocol semantic decisions with real alternatives, not prose consolidation. The prose-author black-box test should have caught this originally.

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
