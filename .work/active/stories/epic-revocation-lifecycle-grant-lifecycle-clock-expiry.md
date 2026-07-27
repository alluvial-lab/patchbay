---
id: epic-revocation-lifecycle-grant-lifecycle-clock-expiry
kind: story
stage: implementing
tags: [security, protocol]
parent: epic-revocation-lifecycle-grant-lifecycle
depends_on: []
release_binding: null
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
