---
id: release-v0.2.0
kind: release
stage: quality-gate
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

Pending Phase 4 gate execution.
