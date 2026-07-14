---
id: backlog-sessions-authority-domain-isolation
kind: feature
stage: backlog
tags: [protocol, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-13
---

# Backlog: Session registry authority-domain isolation

## Source
Found during deep review of `feature-v0-core-sessions` (Phase 1 + Phase 2, cross-model openai-codex/gpt-5.6-sol). Parked because v0.1.0 is single-authority-domain, so exposure is limited, but the API contract is unsound.

## Finding
`SessionRegistry` records no owning `AuthorityDomainId`, `SessionLookup::current_session` takes no domain argument, and `TargetResolver::resolve` ignores its `authority_domain_id` parameter (`_authority_domain_id`). A registry rebuilt for domain A can resolve an operation submitted in domain B, or derive a domain-B delta using domain-A projected state.

The `TargetResolver` trait is explicitly domain-scoped (`TargetNotFound` describes failure "in the requested authority domain"). v0.1.0 has one authority domain, so this is latent, but the API accepts arbitrary domain IDs and fails to enforce its contract.

## Direction
Bind each `SessionRegistry` to an `AuthorityDomainId` at construction. Include the domain in lookup validation. Reject resolver/ingestion calls for any other domain. Add cross-domain ingestion and resolution tests. This is forward-compatibility hygiene for the `(authority_domain_id, LSN)` key shape — the federation seam (PROTOCOL "Extension seams registry").

## Priority
Not blocking for v0.1.0 (single domain). Should land before any multi-domain/federation work.
