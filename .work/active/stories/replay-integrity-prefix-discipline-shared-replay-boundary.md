---
id: replay-integrity-prefix-discipline-shared-replay-boundary
kind: story
stage: implementing
tags: [protocol, storage]
parent: replay-integrity-prefix-discipline
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Shared contiguous-prefix replay boundary

## Checkpoint

Introduce the one complete-log replay validator and route every cold rebuild,
snapshot tail, aggregate startup/catch-up, and complete as-of diagnostics fold
through it. A returned event must belong to the requested authority domain,
carry a concrete generated `StoredEventKind`, and have exactly the next LSN
before any projection mutates.

## Acceptance evidence

- Full replay from cursor 0 accepts only `1..=N`; a snapshot tail from cursor
  `K` accepts only `K+1..=N`. Empty prefixes remain valid.
- Initial/interior gaps, duplicates, reversals, LSN 0, wrong domains, and
  successor overflow fail closed; `Unspecified` is corrupt log history and an
  unknown numeric kind is a corrupt record.
- Command, Elicitation, authority, operator, session, resource, security, and
  adapter standalone rebuilds use the shared rule. Server rebuild, catch-up,
  and complete as-of diagnostics validate before fold and advance their cursor
  only after successful application.
- Direct projection dispatch rejects `StoredEventKind::Unspecified` without
  mutation while still ignoring known concrete sibling kinds.
- Duplicate local replay identity/order helpers are removed; no filtered
  subscription or audit-page stream is incorrectly required to be contiguous.

## Ordering constraints

This is the first checkpoint. It establishes the stable boundary consumed by
the cross-projection evidence checkpoint. It does not depend on the session
replay-equality or resource prefix-covered-redelivery features: those own
content equality and duplicate catch-up semantics after this strict new-prefix
validation.
