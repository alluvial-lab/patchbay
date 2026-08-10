---
id: replay-integrity-prefix-discipline-shared-replay-boundary
kind: story
stage: done
tags: [protocol, storage]
parent: replay-integrity-prefix-discipline
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-10
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

## Implementation notes

- Execution capability: `openai-codex/gpt-5.6-sol`, xhigh; explicit caller selection for normative cross-projection replay integrity.
- Review weight: `thorough` (explicit caller selection), retained for the feature review boundary; this child advances directly to done on verification.
- Files changed: `core/src/storage/{prefix,mod,port,recovery}.rs`; complete-log replay/dispatch paths under `core/src/{acceptance,adapter,authority,diagnostics,resource,security,session}`; `server/src/{adapter_service,operator_session,spawn_completion,state}.rs`.
- Tests added/removed: none owned by this checkpoint; the dependent evidence checkpoint owns the scripted-storage matrix and mutation witnesses. Focused existing acceptance, Elicitation, adapter, authority, diagnostics, resource, session, recovery, and server-state suites remained green against this implementation.
- Simplification: moved the partial validator out of snapshot recovery into one storage-owned complete-prefix boundary; removed Elicitation, resource, security, diagnostics, and server-local monotonic/order gates; all complete consumers now validate domain, concrete generated kind, and exact successor before folding, then advance only after successful application.
- Discrepancies from design: current `main` already contained a narrow exact-LSN `validate_next_replay_event` added by the overlapping descendant-completion work. This did not invalidate the design: it lacked kind validation, typed record/log classification, recovery-tail validation, and most call sites. Implementation consolidated and extended it rather than creating a second validator.
- Adjacent issues parked: none (operator forbade backlog/exclusion expansion).

## Verification evidence

- `cargo check --workspace`
- Focused core suites: `acceptance_replay`, `acceptance_elicitation`, `authority_replay`, `sessions_replay_resolver`, `resource_replay`, `adapter_capability`, `diagnostics_projection`.
- Focused server state suite, including aggregate startup/catch-up/as-of behavior, passed before transition.
- `git diff --check`
