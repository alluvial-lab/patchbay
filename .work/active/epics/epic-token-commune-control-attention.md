---
id: epic-token-commune-control-attention
kind: epic
stage: drafting
tags: [adapter, protocol, ux, security, integration]
parent: null
depends_on: [epic-token-commune-observer]
release_binding: null
gate_origin: null
created: 2026-07-26
updated: 2026-07-26
---

# token-commune control and attention plane

## Brief

Add durable, grant-gated token-commune control and human-attention workflows after the observer adapter is trustworthy in daily use. Patchbay Operations manage the pool's operator-facing administrative actions through explicit adapter payload contracts and visible lifecycle state; token-commune remains the semantic authority for whether an upstream change applied. Patchbay boundary deduplication is not presented as end-to-end idempotency unless the gateway mutation contract supplies the corresponding guarantee.

The same arc turns resource events into calm, actionable attention: fingerprint drift can request admin review, pending contributions can open approval workflows once token-commune has a durable pending-approval object, and broken authentication can direct the affected member into token-commune's external re-onboarding flow. Secrets and OAuth grants stay outside Patchbay's durable payload history.

## Strategic decisions

- **Which controls belong here?** Only actions that keep agent-operational resources usable or govern their safe availability, beginning with fingerprint probe/accept, contribution approval/removal, and draw/decree administration as the upstream API supports them.
- **Who is authoritative?** Patchbay owns durable operator intent, local grants, delivery visibility, retry presentation, and audit; token-commune owns member/admin policy and the semantic result of pool mutations.
- **What does idempotent mean?** Patchbay guarantees boundary deduplication. End-to-end idempotency is claimed only per upstream operation when token-commune accepts a stable idempotency key or the operation is proven inherently idempotent.
- **How does re-onboarding work?** `auth_broken` raises attention with a safe handoff to token-commune's local onboarding flow. Access/refresh tokens and other secret material never pass through ordinary Patchbay Operations, Elicitations, Observations, audit, or snapshots.
- **Does this promote shared-human coordination?** No. An admin's personal Patchbay handles admin attention and a member's personal Patchbay handles member attention; quorum and cross-human delegation remain later coordination work.

## Arc position

Depends on `epic-token-commune-observer`. It completes the token-commune reference integration by exercising durable mutations, asynchronous result correlation, attention, approvals, upstream authorization, and retry-safety presentation. The existing `epic-public-product-contract-adapter-portability-proof` consumes this completed arc for v1 conformance evidence.

## Capability outline

- typed payload contracts and capability declarations for supported token-commune administrative Operations;
- upstream error-to-Patchbay failure mapping and known failure modes;
- correlation of asynchronous gateway acceptance with semantic completion, failure, or unknown outcome;
- per-operation idempotency classification and safe retry presentation;
- local Patchbay grants layered over upstream member/admin credential scope;
- resource attention derived from authenticated Observations without inventing lifecycle states;
- durable approval Elicitations only where token-commune exposes a stable pending object and response contract;
- external re-onboarding handoffs that keep secrets out of Patchbay persistence;
- cockpit and CLI mutation/attention flows with audit and reconnect behavior;
- adversarial tests for duplicate mutation, ambiguous transport outcomes, revoked grants, stale resource identity, and upstream-role mismatch.

## External collaboration boundary

Mutation endpoints, pending-approval identity, event/result correlation, and any end-to-end idempotency mechanism are token-commune contract work and must be tracked in token-commune's repository. Patchbay does not infer these guarantees from HTTP method choice or current implementation behavior.

## Scope boundaries

- No multi-human Patchbay authority domain, delegation, quorum, agent mesh, or shared-work routing.
- No credential onboarding or secret response contract promotion merely to fit this adapter.
- No direct database access to token-commune and no bypass of its gateway policy.
- No generic automation engine that reacts to arbitrary resource events without the operator.
- No promise that every administrative action maps to `reconfigure`; epic design must validate OperationKind semantics and promote a new registry entry if the existing vocabulary would be dishonest.

## Simplification opportunity

Reuse the canonical Operation lifecycle, failure vocabulary, grants, audit, AttentionRequired projection, and Elicitation machinery where their semantics genuinely fit. Avoid a parallel token-commune command state machine, UI-only authorization, or adapter-local durable queue. Keep OAuth/device flows in token-commune instead of adding secret persistence and redaction complexity to Patchbay.

## Mockups

Epic design must mock the admin mutation lifecycle, fingerprint review, pending-contribution approval, member `auth_broken` attention, ambiguous-retry warning, and cross-device clearing behavior. The observer epic's selected resource surface and shared design-system artifacts are inherited.

## Extension pressure classification

- **Committed post-v0.1.0 direction:** durable token-commune control, attention, and approval where upstream contracts support them; layered local and upstream authority; honest retry semantics.
- **Reserved seams:** multi-human delegation/quorum, secret-response handling, automatic remediation, agent-to-agent coordination, and cross-Patchbay attention routing.
- **Explicitly rejected for this arc:** persisting OAuth credentials in Patchbay, equating boundary dedup with upstream exactly-once execution, bypassing token-commune authorization, or turning resource alerts into a generic automation system.
