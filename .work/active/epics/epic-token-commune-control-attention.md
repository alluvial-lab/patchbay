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
updated: 2026-08-10
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

Mutation endpoints, pending-approval identity, event/result correlation, and any end-to-end idempotency mechanism are token-commune contract work and tracked in token-commune's repository. Patchbay does not infer these guarantees from HTTP method choice or current implementation behavior.

**Owned upstream (2026-08-09 coordination):** token-commune now owns the mutation half in its `admin-control-surface` epic (4 child features: `foundation`, `contribution-removal`, `decree-ops`, `approval-gate`), behind a `managed` deployment mode (default `friends-only` unchanged). It depends on the read-side `external-consumer-api-identity` + `external-consumer-api-scoped-credentials` features; point dependency tracking at `admin-control-surface`. The read side (stable opaque ids, cursor/replay event-history, scoped read-only creds, completeness envelope) closes the observer's honest-stopgap compensations — this epic no longer assumes it must compensate for snapshot-local/anonymous ids or a 50-event window.

## Cross-project coordination (2026-08-09)

Mesh exchange with the token-commune agent resolved mutation-half ownership and locked several design positions. token-commune's `admin-control-surface-foundation` design is gated on this exchange (its operation state machine and idempotency model are wire-driven); a co-design reply was sent 2026-08-09.

**Locked positions (Patchbay-side, recorded for feature-design):**
- **Two-record split is the authority model.** Patchbay-owned Operation state is terminal-final and never rewritten; a separately-correlated upstream semantic-result Observation carries token-commune's deterministic `applied`/`not_applied`. Correlation key = token-commune op-id + idempotency-key.
- **`unknown` is Patchbay's, not token-commune's.** token-commune's atomic-commit pattern (op-state + event + local mutation, in-process SQLite) means it always knows `applied`/`not_applied` once durable. Patchbay's `failed(execution_outcome_unknown)` is a Patchbay-Operation-side terminal for when Patchbay cannot confirm delivery or reach token-commune to query — not a value token-commune emits. token-commune's durable queryable-state + idempotency-key is what lets Patchbay resolve ambiguity by re-query.
- **Managed-operation credential (new requirement).** Patchbay needs a managed-op credential distinct from the read-only observer scope (token-commune's scoped-credentials forbid mutation; a full admin key recreates overprivilege). Posture: two credentials (observer + managed-op), mapping to two Patchbay grant scopes. Managed-op cred is admin-member-bound, default-deny, incapable of inference/registration. *Scope dimension (token-commune ping 2026-08-09):* token-commune's scope dimension is `authorizationScopes` — a hard ceiling applied *before* role checks (`null` = full member, backward-compat). Observer cred uses the landed read scopes `pool:read` / `events:read` / `member:self:read`. The managed-op cred maps to a future `managed-op` scope (admin-only, separately guarded) that lands with token-commune's `admin-control-surface-foundation` — **not present yet**, so the managed-op half is gated on that feature. Name intended stable but may refine; token-commune will re-flag at the wire-shape-concrete ping if it changes.
- **Compatibility posture.** token-commune's principle is "schema evolves in place, never versioned, no deprecation contract." Patchbay's v1 SemVer scopes to the Patchbay-side adapter boundary and explicitly disclaims upstream token-commune wire stability (see `epic-public-product-contract-public-compatibility`).
- **Consumption posture on token-commune caveats.** Patchbay treats contribution removal as corrective administration, not a ban (a member may re-register while the approval gate is off); Patchbay's attention/projection must not present it as a durable exclusion. fingerprint-accept disposition (recovery primitive vs durable op) is token-commune's call; Patchbay consumes whichever shape lands and treats a retained recovery primitive as out-of-band, not a durable Operation.

**Decomposition gate: LIFTED (2026-08-10).** Both external lift-conditions are satisfied — token-commune's `admin-control-surface-foundation` is now **implemented + reviewed** (concrete, not just designed) and the managed-operation credential has landed (`managed:operate` scope over `authorizationScopes`). The external mutation contract this epic needs is real and documented (token-commune `docs/EXTERNAL-CONSUMER-API.md` + `ARCHITECTURE.md` + `PRINCIPLES.md`). The epic may now be decomposed; design the adapter client + Operation/Elicitation projection against the concrete wire-shape below. The 2026-08-09 adversarial review's **Patchbay-internal** blockers (absent attention/Elicitation producer machinery; secret-exclusion enforceability; OperationKind/grant matrix) remain and are the actual design work — they proceed independently of the (now-satisfied) external gate.

**Concrete upstream wire-shape (token-commune, implemented 2026-08-10 — design against this; unversioned, change-in-place, re-flagged on evolution):**
- **Operation state machine:** `managed_operations` table; outcomes are deterministic terminal `applied` / `not_applied`. token-commune never emits `unknown` (that stays Patchbay's transport terminal, resolved by re-query).
- **Idempotency:** issuer-scoped table mapping `(issuer, key)` → resolved outcome, with typed conflict detection (same key, changed command). Indefinite retention (recorded risk).
- **Correlation + atomicity:** operation-id + idempotency-key (as agreed); structured operation-lifecycle events committed atomically with the mutation + operation record + idempotency record in one SQLite transaction.
- **Re-query (resolves Patchbay's transport ambiguity):** `GET /commune/operations/:operationId` and `GET /commune/operations/by-idempotency-key` — managed-op-credential gated, read-only, durable across restart.
- **Authorization:** managed-op credential = separately-revocable, admin-bound, default-deny, `managed:operate` scope over `authorizationScopes`; distinct from the read-only observer credential (our two-credential model); incapable of inference/contribution-registration.
- **Mode + surfaces:** entire surface behind a `managed` deployment profile (default `friends-only` unchanged); in-process SQLite only (no queue/worker/background loop). The three mutation surfaces (contribution removal, decree override, approve/reject) are implemented on top of this framework.

Trigger status (2026-08-10): scope-dimension name ✅ received; foundation wire-shape concrete ✅ received + implemented.

**Upstream constraints (token-commune, 2026-08-09 — design inputs for feature-design):**
- **Atomic in-process-SQLite managed mode.** Ops resolve synchronously in one SQLite transaction (op-state + event + local mutation committed atomically); no queue/worker/second service/background loop. Once token-commune returns `applied` it is durably committed (survives restart). *Patchbay implication:* the `unknown` window is purely transport (did the response return), always resolvable by re-query with the idempotency-key — no eventual-consistency lag, no background retry queue to race with. Re-query-after-transport-loss is always safe and deterministic.
- **Pending-exclusion invariant.** A pending contribution is encrypted-but-inactive and never enters router candidates, capacity polling, active pool inventory, or allocation math; approval atomically activates; `approval-gate=true` is rejected outside managed mode at startup. *Patchbay implication:* pending is NOT observable capacity — the observer/projection must never project a pending contribution as pool capacity (no phantom-capacity across reconnect).
- **Removal = durable active→removed, not row delete.** Reconciles candidates, credential-manager maps, scheduler poll targets, sticky sessions (router session-stickiness may keep routing to a removed contribution until TTL expiry; current code ~1h sliding, but the removal feature finalizes the actual drain semantics — treat as a data point, not contract), metering, in-flight (policy TBD), restart. *Patchbay implication:* brief drain window where a removed contribution may still serve in-flight/sticky sessions — project removal as "no new admissions + drain," not instantaneous disappearance; a retry against a just-removed id returns deterministic `not_applied` (treat as terminal, not transient-retry).

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
