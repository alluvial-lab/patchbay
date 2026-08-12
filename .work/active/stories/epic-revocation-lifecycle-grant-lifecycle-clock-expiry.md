---
id: epic-revocation-lifecycle-grant-lifecycle-clock-expiry
kind: story
stage: done
tags: [security, protocol]
parent: epic-revocation-lifecycle-grant-lifecycle
depends_on: []
release_binding: v0.2.0
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Inject the core clock and enforce grant expiry

## Checkpoint

Create the shared core `Clock` port with production `SystemClock` and deterministic `TestClock` adapters, remove authority-domain wall-clock reads, and make one sampled instant drive both Operation-window validation and grant liveness. Preserve typed denial detail so an otherwise-matching expired grant returns `SubmissionOutcome = rejected`, `FailureCode::Expired`, reason `grant_expired`, and the existing `GrantExpired` audit kind without a post-hoc second clock read.

## Acceptance evidence

- Boundary tests freeze and advance `TestClock` across just-before and exact-expiry instants; `[starts_at, expires_at)` remains half-open.
- An expired otherwise-matching grant cannot append an Operation and produces the typed expired rejection/audit; revoked or absent grants remain `AuthorizationDenied`.
- A live grant still accepts, and the durable Operation plus `SubmissionResult` identify the authorizing grant.
- No `SystemTime::now()` remains in authority matching, and the stale expiry comments are deleted.

## Ordering

Foundation checkpoint for the revocation and Subscribe paths; both consume the same sampled-time grant decision.

## Implementation notes

- Added the core-owned `Clock`, `SystemClock`, and deterministic `TestClock` in `core/src/time.rs`; acceptance samples one timestamp and passes it through validity-window and grant checks.
- Added `GrantLiveness::{Live, Expired, Revoked}` and timestamp-based grant predicates. Revocation takes precedence and matching denial candidates are selected deterministically by grant id.
- Accepted operations now persist as generated `AcceptedOperation` envelopes with verified grant provenance. Retry equivalence uses caller-supplied logical operation bytes, excluding core acceptance metadata.
- Typed `SubmissionResult` grant/reason fields drive `GrantExpired` audit classification directly; the server no longer performs a second wall-clock authority scan.

## Verification

`cargo test --workspace` passed. Existing durable-operation fixtures were migrated to the generated acceptance envelope; no production compatibility reader was added.
