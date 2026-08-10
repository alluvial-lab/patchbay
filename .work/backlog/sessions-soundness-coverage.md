---
id: sessions-soundness-coverage
kind: feature
stage: backlog
tags: [protocol, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-08
updated: 2026-08-09
---

# Sessions soundness and coverage

> **Superseded 2026-08-09 (split, option A).** This consolidation mixed three ownership boundaries (registry/replay/domain soundness; adapter source-ordering contract; production composition-root serialization). Split into standalone features for separate review boundaries:
> - `session-registry-replay-domain-soundness` (authority-domain-isolation + idempotency replay-equality + test-coverage)
> - `adapter-report-source-ordering` (report-source-ordering — a wire contract change, distinct from LSN ordering)
>
> Absorbed-findings detail + currency retained below as the analysis record.

## Brief
Consolidate parked session-registry soundness and evidence gaps into one currency-checked feature. Absorbed findings:

- **`backlog-sessions-authority-domain-isolation`** — bind session registries + target resolution to their authority domain; reject cross-domain access. *Src:* deep review of `feature-v0-core-sessions` Phase 1+2. *Currency (2026-08-09 review):* **OPEN** — `SessionRegistry` has no owning domain (`core/src/session/registry.rs:50-55`); `TargetResolver::resolve` names the param `_authority_domain_id` and ignores it (`resolver.rs:15-24`); `SessionLookup::current_session` takes no domain arg (`ingest.rs:88-95`). *Direction:* bind each registry to an `AuthorityDomainId` at construction, validate on lookup/ingest, reject others; forward-compat for the `(authority_domain_id, LSN)` federation seam. *Disposition:* **keep** in a registry/domain-soundness feature.
- **`backlog-sessions-idempotency-and-concurrency`** — redelivery should compare event identity/payload, not just key+LSN; the unlocked warm read-decide-append can create unreplayable logs under concurrency. *Src:* deep review Phase 2. *Currency:* **PARTIAL** — production adapter ingress now serializes through the shared decision gate + rebuilds before/after report ingest (`server/src/adapter_service.rs:753-829`); but registry redelivery is content-blind (duplicate registration returns `Ok` by key alone, `registry.rs:329-334`), state mutations no-op on any `event_lsn <= last_lsn` (`registry.rs:482-488…`), and the writer still documents caller-managed single-delta warming (`ingest.rs:145-149`). *Direction:* distinguish production serialization (done) from unresolved replay equality; payload-equality on redelivery; serialization token or append-then-replay on the warm path (same shape exists in acceptance `ingest_observation` — evaluate there too). *Disposition:* **keep**, split production-serialization from replay-equality.
- **`backlog-sessions-test-coverage-gaps`** — replay-corruption, acceptance-integration, malformed-event, resolver-boundary, multi-identity coverage. *Src:* deep review Phase 1. *Currency:* **PARTIAL** — resolver now enforces `RuntimeSession` kind (`resolver.rs:21-24`) and some malformed cases exist (`sessions_registry.rs:220-240`); but replay tests stay happy-path (`sessions_replay_resolver.rs:79-229`), acceptance still uses `TestTargetResolver` (`acceptance_pipeline.rs:69-91`), proptest fixes all reports to one adapter/session (`sessions_proptest.rs:97-104`). *Direction:* highest-value is the acceptance↔sessions integration test; plus table-driven malformed-event tests + multi-identity proptest. *Disposition:* **split** as acceptance criteria attached to the corresponding fixes, not a generic coverage bucket.
- **`backlog-session-report-source-ordering`** — adapter reports carry no source revision, so a delayed stale report can roll mutable fields backward (arrival order is treated as source order). *Src:* `feature-session-model-field` review (2026-07-24). *Currency:* **OPEN** — `SessionReport` ends at `model = 11`, no source revision (`contracts/proto/patchbay/adapter_control.proto:35-47`); core ingest interprets arrival order as source order; production serialization can't distinguish a delayed stale report. *Direction:* add a monotonic generation-scoped report revision to `SessionReport`, reject stale revisions before append; contract change → conformance vector + extension-seams classification. *Disposition:* **split** into adapter report-contract/source-ordering work (separate from the registry/replay feature — LSN ordering ≠ source ordering).

*Currency verified 2026-08-09. Per the review this feature should **split into 2**: (1) session registry/replay/domain soundness [authority-domain-isolation, idempotency replay-equality, test-coverage]; (2) adapter report source-ordering contract [report-source-ordering]. Production decision-gate serialization is a composition-root invariant tested independently, NOT advertised as core writer safety (Fail Fast: a future composition root can bypass the server gate).*

## Simplification opportunity
Prefer one domain-scoped session/replay validation path and shared test fixtures over separate defensive checks for each report type; retain only coverage that exercises stable session and acceptance contracts.
