---
id: backlog-authority-failed-authorization-audit
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

# Backlog: Distinct failed-authorization audit records

## Source
Authority design review (R4 decision, revision 2). The `feature-v0-core-authority` design delivers grant-lifecycle provenance (durable Grant/DescendantGrant/Revocation events with `GrantProvenance`) but defers the distinct failed-authorization audit record.

## Finding
`docs/SECURITY.md` treats security audit records as distinct records capable of representing denied attempts and decisions that do not create command state ("audit records are distinct from durable command/session state-transition events: they may record rejected attempts and failed checks that do not create command records"). The authority feature's grant/revocation events satisfy grant-lifecycle audit but NOT the failed-authorization-audit requirement (a denied `GrantCheck` currently produces a `SubmissionResult` rejection with no durable audit record of the denial).

## Direction
Design a distinct audit event/record for failed authorizations (and other security-relevant decisions: grant-created/changed/expired/revoked, failed-authorization, stale-event rejection). This is cross-cutting — it touches acceptance's rejection path, not just authority. Scope as a feature when prioritized. Until then, the authority feature does NOT claim to deliver full audit; it delivers grant-lifecycle provenance only.

## Priority
Not blocking for v0.1.0 (grant-lifecycle provenance is delivered). Becomes important for compliance/security-incident forensics.
