---
id: backlog-grant-expiration-enforcement
kind: feature
stage: backlog
tags: [security, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Backlog: Grant expiration enforcement

## Source
Authority design review (blocker #3 partial fix; R4-adjacent). The `Grant` proto carries `expires_at`; `GrantRecord` stores it; but `is_live()` does not enforce it in v0.1.0.

## Finding
`docs/SECURITY.md` commits: "Missing, expired, revoked, target-mismatched, or kind-mismatched grants produce `SubmissionOutcome = rejected`." The authority feature's `GrantRecord.is_live()` checks revocation (durable) but NOT expiration — it ignores `expires_at`. Enforcing expiry requires a clock, which the core does not have yet (same gap as time-driven session staleness — deferred in the sessions feature).

## Direction
Add a clock port (injected, like `Storage`) and have `grant_authorizes`/`is_live()` evaluate `expires_at` against the current time. This is coupled with the sessions feature's time-driven staleness work (both need a clock port) — scope them together. Until then, `expires_at` is stored but not enforced; grants are live until revoked. Documented as a v0.1.0 gap.

## Priority
Not blocking for v0.1.0 (revocation is the primary authority lever; expiry is a secondary control). Becomes important for time-bounded authority. Couple with the sessions staleness/timer work when that's scoped.
