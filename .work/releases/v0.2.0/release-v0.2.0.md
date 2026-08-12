---
id: release-v0.2.0
kind: release
stage: released
tags: []
parent: null
depends_on: []
release_binding: v0.2.0
gate_origin: null
created: 2026-08-11
updated: 2026-08-11
---

# Release v0.2.0

## Bound items

136 active done items bound by the Phase 3 straggler sweep; no eligible archived stubs were found. Six `[research]`-tagged done engagements were excluded by convention. The lone archive husk (`backlog-session-record-fields-gap`) has neither `stage: done` nor release frontmatter and remains excluded.

- epic-agent-operations-resource-plane (epic) — Agent-operations resource plane
- epic-observability-dogfooding (epic) — Epic: Observability for dogfooding
- epic-revocation-lifecycle (epic) — Epic: Revocation and lockdown lifecycle
- epic-token-commune-observer (epic) — token-commune observer adapter
- adapter-report-source-ordering (feature) — Adapter report source-ordering (stale-report rollback prevention)
- authority-descendant-grant-completion (feature) — Descendant-grant live completion (audit producer + composition root)
- authority-grant-selection-determinism (feature) — Authority grant-selection determinism (stable rule + regression)
- authority-writer-correctness (feature) — Authority writer correctness (pre-append conflict check + durable idempotency)
- cockpit-ux-polish (feature) — Cockpit UX polish
- elicitation-responder-validation (feature) — Elicitation responder authority validation
- epic-agent-operations-resource-plane-capability-manifest (feature) — Resource capability manifest & projection contract
- epic-agent-operations-resource-plane-cockpit-composition (feature) — Cockpit resource composition
- epic-agent-operations-resource-plane-conformance (feature) — Resource-plane conformance evidence
- epic-agent-operations-resource-plane-resource-identity (feature) — Resource identity, resolution & authority
- epic-agent-operations-resource-plane-resource-state (feature) — Resource snapshot, revision & ingestion
- epic-observability-dogfooding-adapter-log-sink (feature) — Adapter durable diagnostics log sink
- epic-observability-dogfooding-cli-diagnostics (feature) — CLI diagnostics commands
- epic-observability-dogfooding-cockpit-diagnostics (feature) — Adapter diagnostics forwarding + cockpit surfacing
- epic-observability-dogfooding-core-diagnostics (feature) — Core-diagnostics query capability
- epic-revocation-lifecycle-grant-lifecycle (feature) — Grant lifecycle: revocation, expiry enforcement, Subscribe check
- epic-revocation-lifecycle-lockdown (feature) — Security lockdown & bootstrap-channel exit
- epic-revocation-lifecycle-session-principal-revocation (feature) — Session & principal revocation
- epic-token-commune-observer-adapter-foundation (feature) — token-commune adapter foundation
- epic-token-commune-observer-cockpit-panel (feature) — token-commune cockpit resource panel and CLI projection
- epic-token-commune-observer-conformance (feature) — token-commune observer conformance and end-to-end evidence
- epic-token-commune-observer-polling-ingestion (feature) — token-commune polling ingestion and observations
- epic-token-commune-observer-snapshot-mapping (feature) — token-commune resource snapshot mapping
- recovery-checkpoint-writer (feature) — Recovery checkpoint writer + scheduling policy
- replay-integrity-prefix-discipline (feature) — Replay integrity: gap-free LSN + reject Unspecified (cross-projection)
- resource-reconciliation-followups (feature) — Resource reconciliation follow-ups
- session-registry-replay-domain-soundness (feature) — Session registry/replay/domain soundness
- snapshot-core-generation-semantics (feature) — Snapshot / core-generation semantics
- adapter-report-source-ordering-conformance (story) — Promote source-ordering model and executable vector evidence
- adapter-report-source-ordering-contract-foundation (story) — Define the session-report source cursor and atomic wire event
- adapter-report-source-ordering-core-fence (story) — Fence session ingestion by durable source order
- adapter-report-source-ordering-pi-sequencer (story) — Emit ordered Pi session-report cursors
- authority-descendant-grant-completion-contract-fold (story) — Complete the descendant-completion contract and durable fold
- authority-descendant-grant-completion-crash-safe-writer (story) — Defer spawn terminalization and execute the crash-safe writer
- authority-descendant-grant-completion-live-composition (story) — Wire startup repair and continuous descendant completion
- authority-writer-correctness-atomic-storage (story) — Atomic grant-identity storage and audit transaction
- authority-writer-correctness-ingest-contract (story) — Normal and descendant authority writer contract
- authority-writer-correctness-retry-evidence (story) — Ambiguous-response, concurrency, audit, and driver evidence
- cockpit-ux-polish-delivery-cards (story) — Cockpit instruction-card delivery stability
- cockpit-ux-polish-session-rows (story) — Cockpit session-row hierarchy
- cockpit-ux-polish-settings (story) — Cockpit settings visibility preference
- cockpit-ux-polish-visual-contract (story) — Cockpit polish visual contract
- epic-agent-operations-resource-plane-capability-manifest-contract-registry (story) — Generate the target-category and resource projection contract
- epic-agent-operations-resource-plane-capability-manifest-core-admission (story) — Validate manifests and expose one resource admission boundary
- epic-agent-operations-resource-plane-capability-manifest-integration-foundation (story) — Integrate capability diagnostics, Pi declaration, and foundation contract
- epic-agent-operations-resource-plane-cockpit-composition-resource-projection-domain (story) — Resource projection domain and local decoders
- epic-agent-operations-resource-plane-cockpit-composition-resource-reconciliation (story) — Resource event and snapshot reconciliation
- epic-agent-operations-resource-plane-cockpit-composition-session-resource-linkage (story) — Resources destination and session resource linkage
- epic-agent-operations-resource-plane-cockpit-composition-shared-resource-rendering (story) — Shared resource target and Operation rendering
- epic-agent-operations-resource-plane-conformance-authority-source-isolation (story) — Prove resource authority and authenticated-source isolation
- epic-agent-operations-resource-plane-conformance-durability-reconnect-honesty (story) — Prove durable resource reconnect and completeness honesty
- epic-agent-operations-resource-plane-conformance-promotion-traceability-closeout (story) — Promote and close resource-plane conformance evidence
- epic-agent-operations-resource-plane-conformance-stale-presentation-dominance (story) — Prove stale resource presentation dominance
- epic-agent-operations-resource-plane-conformance-vector-execution-bridge (story) — Make the shared conformance corpus executable
- epic-agent-operations-resource-plane-resource-identity-integration-conformance (story) — Close resource identity acceptance and compatibility evidence
- epic-agent-operations-resource-plane-resource-identity-polymorphic-target-resolution (story) — Make target resolution target-kind-polymorphic
- epic-agent-operations-resource-plane-resource-identity-resource-authority-containment (story) — Fence resource grant containment by full identity
- epic-agent-operations-resource-plane-resource-identity-typed-resource-identity (story) — Define typed operational-resource identity
- epic-agent-operations-resource-plane-resource-state-contract (story) — Define the resource-state and snapshot contracts
- epic-agent-operations-resource-plane-resource-state-integration-foundation (story) — Close resource-state integration and foundation assertions
- epic-agent-operations-resource-plane-resource-state-projection-replay (story) — Fold and replay durable resource state
- epic-agent-operations-resource-plane-resource-state-report-ingress-reconciliation (story) — Ingest authenticated resource reports and reconcile reconnects
- epic-agent-operations-resource-plane-resource-state-snapshot-load (story) — Materialize and load resource snapshots
- epic-observability-dogfooding-cockpit-diagnostics-adapter-forwarding (story) — Failure-isolated Pi adapter diagnostics forwarding
- epic-observability-dogfooding-cockpit-diagnostics-cockpit-composition (story) — Cockpit adapter-status composition
- epic-observability-dogfooding-cockpit-diagnostics-contract-ingestion (story) — Adapter diagnostic contract and audited core ingestion
- epic-observability-dogfooding-core-diagnostics-audit-records (story) — Durable canonical audit records
- epic-observability-dogfooding-core-diagnostics-query-surface (story) — Typed core-diagnostics query surface
- epic-revocation-lifecycle-grant-lifecycle-cli-conformance (story) — Expose grant revocation in CLI and lock executable evidence
- epic-revocation-lifecycle-grant-lifecycle-clock-expiry (story) — Inject the core clock and enforce grant expiry
- epic-revocation-lifecycle-grant-lifecycle-revocation-decision (story) — Make grant revocation a durable policy decision
- epic-revocation-lifecycle-grant-lifecycle-subscribe-authorization (story) — Grant-check and audit Subscribe establishment
- epic-revocation-lifecycle-lockdown-cli-conformance (story) — Ship CLI recovery, integrated conformance, and rolling foundation
- epic-revocation-lifecycle-lockdown-cockpit-shell-ui (story) — Realize the signed-off cockpit shell and lockdown Security view
- epic-revocation-lifecycle-lockdown-core-posture (story) — Build the durable lockdown posture and acceptance fence
- epic-revocation-lifecycle-lockdown-trigger-exit-rpcs (story) — Expose authorized lockdown entry and bootstrap-only exit
- epic-revocation-lifecycle-session-principal-revocation-cli-controls (story) — CLI revocation and recovery controls
- epic-revocation-lifecycle-session-principal-revocation-conformance-foundation (story) — Integrated revocation conformance and foundation
- epic-revocation-lifecycle-session-principal-revocation-contract-model (story) — Generated revocation contract and model
- epic-revocation-lifecycle-session-principal-revocation-core-state (story) — Replayable core session and principal revocation
- epic-revocation-lifecycle-session-principal-revocation-web-session-plane (story) — Web browser-session revocation projection
- epic-token-commune-observer-adapter-foundation-attachment-lifecycle (story) — Attach the adapter and compose its long-lived process
- epic-token-commune-observer-adapter-foundation-contract-foundation (story) — Establish the token-commune package and stable resource contract
- epic-token-commune-observer-adapter-foundation-credential-diagnostics (story) — Load the gateway credential and enforce diagnostic redaction
- epic-token-commune-observer-adapter-foundation-gateway-client (story) — Implement the consumer-owned token-commune gateway client
- epic-token-commune-observer-adapter-foundation-unsupported-delivery-loop (story) — Keep delivery liveness open and reject all Operations honestly
- epic-token-commune-observer-cockpit-panel-cli-projection (story) — CLI resource query and inspect projections
- epic-token-commune-observer-cockpit-panel-cockpit-integration (story) — Cockpit data-layer and grant integration
- epic-token-commune-observer-cockpit-panel-honesty-evidence (story) — Cross-surface honesty and mutation evidence
- epic-token-commune-observer-cockpit-panel-panel-component (story) — Option-7 token-commune panel component
- epic-token-commune-observer-cockpit-panel-pool-compositor (story) — Per-pool signal compositor
- epic-token-commune-observer-cockpit-panel-projection-decoder (story) — Shared manifest-bound token-commune decoder
- epic-token-commune-observer-cockpit-panel-verdict-synthesis (story) — Patchbay-owned verdict synthesis
- epic-token-commune-observer-conformance-harness-registry-guards (story) — Extend the shared conformance profile with exact mutation accounting
- epic-token-commune-observer-conformance-phase-1-completeness-vectors (story) — Phase 1: completeness vectors for honest adapter behavior
- epic-token-commune-observer-conformance-phase-2-failure-presentation-adversaries (story) — Phase 2: failure-terminalization and presentation adversaries
- epic-token-commune-observer-conformance-phase-2-security-adversaries (story) — Phase 2: source-authentication and gateway-key adversaries
- epic-token-commune-observer-conformance-promotion-closeout (story) — Promote exact evidence and close through the verification deep lane
- epic-token-commune-observer-conformance-real-core-e2e (story) — Bind completeness evidence to the real gateway, adapter, and core process
- epic-token-commune-observer-polling-ingestion-dedup-gap (story) — token-commune latest-50 dedup and gap reconciliation
- epic-token-commune-observer-polling-ingestion-disconnect-reconnect (story) — token-commune disconnect, stale, and reconnect composition
- epic-token-commune-observer-polling-ingestion-event-observation-map (story) — token-commune pool-event status Observation mapping
- epic-token-commune-observer-polling-ingestion-honesty-mutation-evidence (story) — token-commune polling honesty mutation evidence
- epic-token-commune-observer-polling-ingestion-poll-runtime (story) — token-commune non-overlapping poll runtime
- epic-token-commune-observer-polling-ingestion-report-emission (story) — token-commune projected report emission
- epic-token-commune-observer-snapshot-mapping-completeness-mutation-evidence (story) — Prove PARTIAL omission and null-state honesty with fixtures
- epic-token-commune-observer-snapshot-mapping-envelope-construction (story) — Construct manifest-bound JSON and ResourceReport envelopes
- epic-token-commune-observer-snapshot-mapping-member-draw-projection (story) — Project per-provider member draw without aggregation
- epic-token-commune-observer-snapshot-mapping-projection-contract (story) — Preserve the projection input and schema honesty contract
- epic-token-commune-observer-snapshot-mapping-provider-pool-projection (story) — Project honest per-provider pool snapshots
- recovery-checkpoint-writer-bounded-recovery-evidence (story) — Prove the narrow recovery bound honestly
- recovery-checkpoint-writer-scheduling-runtime (story) — Schedule and persist session checkpoints
- recovery-checkpoint-writer-session-recovery-state (story) — Complete session recovery checkpoint
- replay-integrity-prefix-discipline-cross-projection-evidence (story) — Cross-projection replay-integrity evidence
- replay-integrity-prefix-discipline-shared-replay-boundary (story) — Shared contiguous-prefix replay boundary
- resource-reconciliation-followups-applied-prefix-semantics (story) — Apply resource events against one validated authority-domain prefix
- resource-reconciliation-followups-cross-dimensional-evidence (story) — Generate cross-dimensional resource reconciliation evidence
- session-registry-replay-domain-soundness-bound-registry-contract (story) — Bound session-registry contract
- session-registry-replay-domain-soundness-integration-evidence (story) — Session integration and property evidence
- snapshot-core-generation-semantics-continuity-evidence (story) — Verify continuity semantics and roll the foundation forward
- snapshot-core-generation-semantics-durable-epoch (story) — Persist the authority-domain continuity epoch
- snapshot-core-generation-semantics-snapshot-compatibility (story) — Carry and validate the snapshot continuity anchor
- story-fix-chat-activity-indicator (story) — Chat view has no agent-activity indicator
- story-fix-cli-resource-projection-exact-grant (story) — Let exact-resource operators read the CLI resource projection
- story-fix-cockpit-render-amplification (story) — Cockpit renders once per subscription event — text turns freeze the tab
- story-fix-cockpit-scroll-anchor (story) — Cockpit transcript scroll resets to top when a tool call finishes
- story-fix-expired-session-startup-crash (story) — Expired operator session crashes cockpit startup instead of showing login
- story-fix-grant-identity-index-bootstrap (story) — Bug: resource-projection seed rewrite leaves the grant identity index stale
- story-fix-tool-call-args-preview (story) — Tool rows discard call arguments — no "what is it doing" preview
- story-generated-contract-drift-ci-gap (story) — Generated-contract drift is real and CI doesn't check it
- test-tempfile-hygiene (story) — Backlog: test-suite tempfile hygiene (201K leaked SQLite temp files filled /tmp)
- test-tempfile-root-cause-scoping (story) — Test tempfile root-cause scoping (the opt-in wrapper is not a root fix)

## Gate runs

- **gate-security** (2026-08-11) — 3 findings (2 high, 1 medium); scanner-agent path unavailable in this sub-agent harness, so the documented inline fallback was used. High findings were fixed and verified; medium finding `gate-security-upgrade-dompurify` is parked unbound per operator policy.
- **gate-tests** (2026-08-11) — 0 findings; inline fallback used because scanner agents were unavailable. CI's contract/conformance/Rust/TypeScript suites were green; targeted integrity scans found no skipped/tautological tests or deleted coverage, and post-security-fix Pi/web suites passed.
- **gate-cruft** (2026-08-11) — 0 findings; inline fallback used because scanner agents were unavailable. Rust clippy with warnings denied, all TypeScript builds, stale-marker searches, and tracked test-deletion checks were clean.
- **gate-docs** (2026-08-11) — 2 low-risk drift findings; inline fallback used because scanner agents were unavailable. `gate-docs-readme-v0.2-current-status` and `gate-docs-runbook-release-framing` are parked unbound per operator policy.
- **gate-patterns** (2026-08-11) — 0 new patterns, 0 inconsistencies; inline fallback used because scanner agents were unavailable. Existing canonical index and generated digest were verified consistent; tracking item `gate-patterns-v0.2.0` is done and bound.

### Binding-consistency guard

- Mode: `warn` (default)
- Epic cohesion: `phased` (default)
- Result: 0 conflicts, 0 incomplete parent/child bindings.

## Shipped items

Bodies live in git history (delete-refs) — `git show ffd225b:<former active path>` recovers any pruned body.

| id | title | kind | archived_atop | git ref |
|----|-------|------|---------------|---------|
| epic-agent-operations-resource-plane | Agent-operations resource plane | epic | — | ffd225b |
| epic-observability-dogfooding | Epic: Observability for dogfooding | epic | — | ffd225b |
| epic-revocation-lifecycle | Epic: Revocation and lockdown lifecycle | epic | — | ffd225b |
| epic-token-commune-observer | token-commune observer adapter | epic | — | ffd225b |
| adapter-report-source-ordering | Adapter report source-ordering (stale-report rollback prevention) | feature | — | ffd225b |
| authority-descendant-grant-completion | Descendant-grant live completion (audit producer + composition root) | feature | — | ffd225b |
| authority-grant-selection-determinism | Authority grant-selection determinism (stable rule + regression) | feature | — | ffd225b |
| authority-writer-correctness | Authority writer correctness (pre-append conflict check + durable idempotency) | feature | — | ffd225b |
| cockpit-ux-polish | Cockpit UX polish | feature | — | ffd225b |
| elicitation-responder-validation | Elicitation responder authority validation | feature | — | ffd225b |
| epic-agent-operations-resource-plane-capability-manifest | Resource capability manifest & projection contract | feature | — | ffd225b |
| epic-agent-operations-resource-plane-cockpit-composition | Cockpit resource composition | feature | — | ffd225b |
| epic-agent-operations-resource-plane-conformance | Resource-plane conformance evidence | feature | — | ffd225b |
| epic-agent-operations-resource-plane-resource-identity | Resource identity, resolution & authority | feature | — | ffd225b |
| epic-agent-operations-resource-plane-resource-state | Resource snapshot, revision & ingestion | feature | — | ffd225b |
| epic-observability-dogfooding-adapter-log-sink | Adapter durable diagnostics log sink | feature | — | ffd225b |
| epic-observability-dogfooding-cli-diagnostics | CLI diagnostics commands | feature | — | ffd225b |
| epic-observability-dogfooding-cockpit-diagnostics | Adapter diagnostics forwarding + cockpit surfacing | feature | — | ffd225b |
| epic-observability-dogfooding-core-diagnostics | Core-diagnostics query capability | feature | — | ffd225b |
| epic-revocation-lifecycle-grant-lifecycle | Grant lifecycle: revocation, expiry enforcement, Subscribe check | feature | — | ffd225b |
| epic-revocation-lifecycle-lockdown | Security lockdown & bootstrap-channel exit | feature | — | ffd225b |
| epic-revocation-lifecycle-session-principal-revocation | Session & principal revocation | feature | — | ffd225b |
| epic-token-commune-observer-adapter-foundation | token-commune adapter foundation | feature | — | ffd225b |
| epic-token-commune-observer-cockpit-panel | token-commune cockpit resource panel and CLI projection | feature | — | ffd225b |
| epic-token-commune-observer-conformance | token-commune observer conformance and end-to-end evidence | feature | — | ffd225b |
| epic-token-commune-observer-polling-ingestion | token-commune polling ingestion and observations | feature | — | ffd225b |
| epic-token-commune-observer-snapshot-mapping | token-commune resource snapshot mapping | feature | — | ffd225b |
| recovery-checkpoint-writer | Recovery checkpoint writer + scheduling policy | feature | — | ffd225b |
| replay-integrity-prefix-discipline | Replay integrity: gap-free LSN + reject Unspecified (cross-projection) | feature | — | ffd225b |
| resource-reconciliation-followups | Resource reconciliation follow-ups | feature | — | ffd225b |
| session-registry-replay-domain-soundness | Session registry/replay/domain soundness | feature | — | ffd225b |
| snapshot-core-generation-semantics | Snapshot / core-generation semantics | feature | — | ffd225b |
| adapter-report-source-ordering-conformance | Promote source-ordering model and executable vector evidence | story | — | ffd225b |
| adapter-report-source-ordering-contract-foundation | Define the session-report source cursor and atomic wire event | story | — | ffd225b |
| adapter-report-source-ordering-core-fence | Fence session ingestion by durable source order | story | — | ffd225b |
| adapter-report-source-ordering-pi-sequencer | Emit ordered Pi session-report cursors | story | — | ffd225b |
| authority-descendant-grant-completion-contract-fold | Complete the descendant-completion contract and durable fold | story | — | ffd225b |
| authority-descendant-grant-completion-crash-safe-writer | Defer spawn terminalization and execute the crash-safe writer | story | — | ffd225b |
| authority-descendant-grant-completion-live-composition | Wire startup repair and continuous descendant completion | story | — | ffd225b |
| authority-writer-correctness-atomic-storage | Atomic grant-identity storage and audit transaction | story | — | ffd225b |
| authority-writer-correctness-ingest-contract | Normal and descendant authority writer contract | story | — | ffd225b |
| authority-writer-correctness-retry-evidence | Ambiguous-response, concurrency, audit, and driver evidence | story | — | ffd225b |
| cockpit-ux-polish-delivery-cards | Cockpit instruction-card delivery stability | story | — | ffd225b |
| cockpit-ux-polish-session-rows | Cockpit session-row hierarchy | story | — | ffd225b |
| cockpit-ux-polish-settings | Cockpit settings visibility preference | story | — | ffd225b |
| cockpit-ux-polish-visual-contract | Cockpit polish visual contract | story | — | ffd225b |
| epic-agent-operations-resource-plane-capability-manifest-contract-registry | Generate the target-category and resource projection contract | story | — | ffd225b |
| epic-agent-operations-resource-plane-capability-manifest-core-admission | Validate manifests and expose one resource admission boundary | story | — | ffd225b |
| epic-agent-operations-resource-plane-capability-manifest-integration-foundation | Integrate capability diagnostics, Pi declaration, and foundation contract | story | — | ffd225b |
| epic-agent-operations-resource-plane-cockpit-composition-resource-projection-domain | Resource projection domain and local decoders | story | — | ffd225b |
| epic-agent-operations-resource-plane-cockpit-composition-resource-reconciliation | Resource event and snapshot reconciliation | story | — | ffd225b |
| epic-agent-operations-resource-plane-cockpit-composition-session-resource-linkage | Resources destination and session resource linkage | story | — | ffd225b |
| epic-agent-operations-resource-plane-cockpit-composition-shared-resource-rendering | Shared resource target and Operation rendering | story | — | ffd225b |
| epic-agent-operations-resource-plane-conformance-authority-source-isolation | Prove resource authority and authenticated-source isolation | story | — | ffd225b |
| epic-agent-operations-resource-plane-conformance-durability-reconnect-honesty | Prove durable resource reconnect and completeness honesty | story | — | ffd225b |
| epic-agent-operations-resource-plane-conformance-promotion-traceability-closeout | Promote and close resource-plane conformance evidence | story | — | ffd225b |
| epic-agent-operations-resource-plane-conformance-stale-presentation-dominance | Prove stale resource presentation dominance | story | — | ffd225b |
| epic-agent-operations-resource-plane-conformance-vector-execution-bridge | Make the shared conformance corpus executable | story | — | ffd225b |
| epic-agent-operations-resource-plane-resource-identity-integration-conformance | Close resource identity acceptance and compatibility evidence | story | — | ffd225b |
| epic-agent-operations-resource-plane-resource-identity-polymorphic-target-resolution | Make target resolution target-kind-polymorphic | story | — | ffd225b |
| epic-agent-operations-resource-plane-resource-identity-resource-authority-containment | Fence resource grant containment by full identity | story | — | ffd225b |
| epic-agent-operations-resource-plane-resource-identity-typed-resource-identity | Define typed operational-resource identity | story | — | ffd225b |
| epic-agent-operations-resource-plane-resource-state-contract | Define the resource-state and snapshot contracts | story | — | ffd225b |
| epic-agent-operations-resource-plane-resource-state-integration-foundation | Close resource-state integration and foundation assertions | story | — | ffd225b |
| epic-agent-operations-resource-plane-resource-state-projection-replay | Fold and replay durable resource state | story | — | ffd225b |
| epic-agent-operations-resource-plane-resource-state-report-ingress-reconciliation | Ingest authenticated resource reports and reconcile reconnects | story | — | ffd225b |
| epic-agent-operations-resource-plane-resource-state-snapshot-load | Materialize and load resource snapshots | story | — | ffd225b |
| epic-observability-dogfooding-cockpit-diagnostics-adapter-forwarding | Failure-isolated Pi adapter diagnostics forwarding | story | — | ffd225b |
| epic-observability-dogfooding-cockpit-diagnostics-cockpit-composition | Cockpit adapter-status composition | story | — | ffd225b |
| epic-observability-dogfooding-cockpit-diagnostics-contract-ingestion | Adapter diagnostic contract and audited core ingestion | story | — | ffd225b |
| epic-observability-dogfooding-core-diagnostics-audit-records | Durable canonical audit records | story | — | ffd225b |
| epic-observability-dogfooding-core-diagnostics-query-surface | Typed core-diagnostics query surface | story | — | ffd225b |
| epic-revocation-lifecycle-grant-lifecycle-cli-conformance | Expose grant revocation in CLI and lock executable evidence | story | — | ffd225b |
| epic-revocation-lifecycle-grant-lifecycle-clock-expiry | Inject the core clock and enforce grant expiry | story | — | ffd225b |
| epic-revocation-lifecycle-grant-lifecycle-revocation-decision | Make grant revocation a durable policy decision | story | — | ffd225b |
| epic-revocation-lifecycle-grant-lifecycle-subscribe-authorization | Grant-check and audit Subscribe establishment | story | — | ffd225b |
| epic-revocation-lifecycle-lockdown-cli-conformance | Ship CLI recovery, integrated conformance, and rolling foundation | story | — | ffd225b |
| epic-revocation-lifecycle-lockdown-cockpit-shell-ui | Realize the signed-off cockpit shell and lockdown Security view | story | — | ffd225b |
| epic-revocation-lifecycle-lockdown-core-posture | Build the durable lockdown posture and acceptance fence | story | — | ffd225b |
| epic-revocation-lifecycle-lockdown-trigger-exit-rpcs | Expose authorized lockdown entry and bootstrap-only exit | story | — | ffd225b |
| epic-revocation-lifecycle-session-principal-revocation-cli-controls | CLI revocation and recovery controls | story | — | ffd225b |
| epic-revocation-lifecycle-session-principal-revocation-conformance-foundation | Integrated revocation conformance and foundation | story | — | ffd225b |
| epic-revocation-lifecycle-session-principal-revocation-contract-model | Generated revocation contract and model | story | — | ffd225b |
| epic-revocation-lifecycle-session-principal-revocation-core-state | Replayable core session and principal revocation | story | — | ffd225b |
| epic-revocation-lifecycle-session-principal-revocation-web-session-plane | Web browser-session revocation projection | story | — | ffd225b |
| epic-token-commune-observer-adapter-foundation-attachment-lifecycle | Attach the adapter and compose its long-lived process | story | — | ffd225b |
| epic-token-commune-observer-adapter-foundation-contract-foundation | Establish the token-commune package and stable resource contract | story | — | ffd225b |
| epic-token-commune-observer-adapter-foundation-credential-diagnostics | Load the gateway credential and enforce diagnostic redaction | story | — | ffd225b |
| epic-token-commune-observer-adapter-foundation-gateway-client | Implement the consumer-owned token-commune gateway client | story | — | ffd225b |
| epic-token-commune-observer-adapter-foundation-unsupported-delivery-loop | Keep delivery liveness open and reject all Operations honestly | story | — | ffd225b |
| epic-token-commune-observer-cockpit-panel-cli-projection | CLI resource query and inspect projections | story | — | ffd225b |
| epic-token-commune-observer-cockpit-panel-cockpit-integration | Cockpit data-layer and grant integration | story | — | ffd225b |
| epic-token-commune-observer-cockpit-panel-honesty-evidence | Cross-surface honesty and mutation evidence | story | — | ffd225b |
| epic-token-commune-observer-cockpit-panel-panel-component | Option-7 token-commune panel component | story | — | ffd225b |
| epic-token-commune-observer-cockpit-panel-pool-compositor | Per-pool signal compositor | story | — | ffd225b |
| epic-token-commune-observer-cockpit-panel-projection-decoder | Shared manifest-bound token-commune decoder | story | — | ffd225b |
| epic-token-commune-observer-cockpit-panel-verdict-synthesis | Patchbay-owned verdict synthesis | story | — | ffd225b |
| epic-token-commune-observer-conformance-harness-registry-guards | Extend the shared conformance profile with exact mutation accounting | story | — | ffd225b |
| epic-token-commune-observer-conformance-phase-1-completeness-vectors | Phase 1: completeness vectors for honest adapter behavior | story | — | ffd225b |
| epic-token-commune-observer-conformance-phase-2-failure-presentation-adversaries | Phase 2: failure-terminalization and presentation adversaries | story | — | ffd225b |
| epic-token-commune-observer-conformance-phase-2-security-adversaries | Phase 2: source-authentication and gateway-key adversaries | story | — | ffd225b |
| epic-token-commune-observer-conformance-promotion-closeout | Promote exact evidence and close through the verification deep lane | story | — | ffd225b |
| epic-token-commune-observer-conformance-real-core-e2e | Bind completeness evidence to the real gateway, adapter, and core process | story | — | ffd225b |
| epic-token-commune-observer-polling-ingestion-dedup-gap | token-commune latest-50 dedup and gap reconciliation | story | — | ffd225b |
| epic-token-commune-observer-polling-ingestion-disconnect-reconnect | token-commune disconnect, stale, and reconnect composition | story | — | ffd225b |
| epic-token-commune-observer-polling-ingestion-event-observation-map | token-commune pool-event status Observation mapping | story | — | ffd225b |
| epic-token-commune-observer-polling-ingestion-honesty-mutation-evidence | token-commune polling honesty mutation evidence | story | — | ffd225b |
| epic-token-commune-observer-polling-ingestion-poll-runtime | token-commune non-overlapping poll runtime | story | — | ffd225b |
| epic-token-commune-observer-polling-ingestion-report-emission | token-commune projected report emission | story | — | ffd225b |
| epic-token-commune-observer-snapshot-mapping-completeness-mutation-evidence | Prove PARTIAL omission and null-state honesty with fixtures | story | — | ffd225b |
| epic-token-commune-observer-snapshot-mapping-envelope-construction | Construct manifest-bound JSON and ResourceReport envelopes | story | — | ffd225b |
| epic-token-commune-observer-snapshot-mapping-member-draw-projection | Project per-provider member draw without aggregation | story | — | ffd225b |
| epic-token-commune-observer-snapshot-mapping-projection-contract | Preserve the projection input and schema honesty contract | story | — | ffd225b |
| epic-token-commune-observer-snapshot-mapping-provider-pool-projection | Project honest per-provider pool snapshots | story | — | ffd225b |
| gate-patterns-v0.2.0 | Patterns extracted for v0.2.0 | story | — | ffd225b |
| gate-security-upgrade-pi-sdk-vulnerable-transitives | Upgrade the Pi SDK dependency chain past high-severity advisories | story | — | ffd225b |
| gate-security-upgrade-web-server-router-deps | Upgrade vulnerable web-server routing dependencies | story | — | ffd225b |
| recovery-checkpoint-writer-bounded-recovery-evidence | Prove the narrow recovery bound honestly | story | — | ffd225b |
| recovery-checkpoint-writer-scheduling-runtime | Schedule and persist session checkpoints | story | — | ffd225b |
| recovery-checkpoint-writer-session-recovery-state | Complete session recovery checkpoint | story | — | ffd225b |
| replay-integrity-prefix-discipline-cross-projection-evidence | Cross-projection replay-integrity evidence | story | — | ffd225b |
| replay-integrity-prefix-discipline-shared-replay-boundary | Shared contiguous-prefix replay boundary | story | — | ffd225b |
| resource-reconciliation-followups-applied-prefix-semantics | Apply resource events against one validated authority-domain prefix | story | — | ffd225b |
| resource-reconciliation-followups-cross-dimensional-evidence | Generate cross-dimensional resource reconciliation evidence | story | — | ffd225b |
| session-registry-replay-domain-soundness-bound-registry-contract | Bound session-registry contract | story | — | ffd225b |
| session-registry-replay-domain-soundness-integration-evidence | Session integration and property evidence | story | — | ffd225b |
| snapshot-core-generation-semantics-continuity-evidence | Verify continuity semantics and roll the foundation forward | story | — | ffd225b |
| snapshot-core-generation-semantics-durable-epoch | Persist the authority-domain continuity epoch | story | — | ffd225b |
| snapshot-core-generation-semantics-snapshot-compatibility | Carry and validate the snapshot continuity anchor | story | — | ffd225b |
| story-fix-chat-activity-indicator | Chat view has no agent-activity indicator | story | — | ffd225b |
| story-fix-cli-resource-projection-exact-grant | Let exact-resource operators read the CLI resource projection | story | — | ffd225b |
| story-fix-cockpit-render-amplification | Cockpit renders once per subscription event — text turns freeze the tab | story | — | ffd225b |
| story-fix-cockpit-scroll-anchor | Cockpit transcript scroll resets to top when a tool call finishes | story | — | ffd225b |
| story-fix-expired-session-startup-crash | Expired operator session crashes cockpit startup instead of showing login | story | — | ffd225b |
| story-fix-grant-identity-index-bootstrap | Bug: resource-projection seed rewrite leaves the grant identity index stale | story | — | ffd225b |
| story-fix-tool-call-args-preview | Tool rows discard call arguments — no "what is it doing" preview | story | — | ffd225b |
| story-generated-contract-drift-ci-gap | Generated-contract drift is real and CI doesn't check it | story | — | ffd225b |
| test-tempfile-hygiene | Backlog: test-suite tempfile hygiene (201K leaked SQLite temp files filled /tmp) | story | — | ffd225b |
| test-tempfile-root-cause-scoping | Test tempfile root-cause scoping (the opt-in wrapper is not a root fix) | story | — | ffd225b |

## Ship record

- **Shipped:** 2026-08-11
- **Mapping:** tag-based — annotated `v0.2.0` tag and `main` pushed to `origin` (`github.com/alluvial-lab/patchbay`)
- **Items shipped:** 139
- **Gate findings:** security 3 (2 high fixed, 1 medium parked); tests 0; cruft 0; docs 2 low-risk findings parked; patterns 0 new patterns and 0 inconsistencies (tracking item completed)
- **Terminal retention:** delete-refs; bound item bodies pruned to git history
- **Verification:** Rust workspace tests and doctests; contract drift/vector/model/presentation checks; TypeScript package suites; Pi/web security audits; and the separate-process walking-skeleton E2E passed.
