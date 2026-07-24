---
id: release-v0.1.0
kind: release
stage: quality-gate
tags: []
parent: null
depends_on: []
release_binding: v0.1.0
gate_origin: null
created: 2026-07-24
updated: 2026-07-24
---

# Release v0.1.0

Initial-operator walking skeleton: one operator controls Pi-backed sessions
through the responsive web cockpit and diagnostic CLI. Personal/internal
milestone — not a public distribution milestone (per epic-public-product-contract).

Release mapping: tag-based (git tag v0.1.0, push to origin).
Gates: security, tests, cruft, docs, patterns (default set).

## Bound items
131 items bound at release cut (active done straggler sweep; no archived stubs existed).
5 [research]-tagged done items excluded per convention (research inputs, not release members).

- epic-foundation-hardening (epic)
- epic-retroactive-design-gate-audit (epic)
- epic-v0-1-0-implementation (epic)
- epic-v0-core (epic)
- epic-public-product-contract-verification-claim-correction (feature)
- feature-adapter-staleness-liveness (feature)
- feature-audit-command-state-ssot (feature)
- feature-audit-persistence-snapshot-model (feature)
- feature-audit-security-threat-model (feature)
- feature-audit-v0-walking-skeleton (feature)
- feature-bank-formal-methods-skills (feature)
- feature-cockpit-icon-set (feature)
- feature-command-state-ssot (feature)
- feature-design-grant-shape (feature)
- feature-design-terminal-commit-race (feature)
- feature-extension-seams-non-foreclosure (feature)
- feature-formal-model-realignment (feature)
- feature-formal-model-seed (feature)
- feature-foundation-doc-completeness-gaps (feature)
- feature-idempotency-ambiguous-execution (feature)
- feature-lease-scope-decision (feature)
- feature-observability-operator-admin (feature)
- feature-operator-presence-and-action-inventory (feature)
- feature-persistence-snapshot-model (feature)
- feature-pi-parity-checklist (feature)
- feature-protocol-idl-and-conformance (feature)
- feature-security-threat-model (feature)
- feature-session-identity-adapter-contract (feature)
- feature-session-model-field (feature)
- feature-ux-v0-acceptance (feature)
- feature-v0-approval-response-contract (feature)
- feature-v0-cli (feature)
- feature-v0-control-surface-trust-boundary (feature)
- feature-v0-core-acceptance (feature)
- feature-v0-core-authority (feature)
- feature-v0-core-persistence (feature)
- feature-v0-core-sessions (feature)
- feature-v0-elicitation-response-contract (feature)
- feature-v0-pi-adapter (feature)
- feature-v0-presentation-component-layer (feature)
- feature-v0-protocol-seam (feature)
- feature-v0-walking-skeleton (feature)
- feature-v0-web-cockpit (feature)
- feature-v0-web-server (feature)
- feature-verification-contract-authority (feature)
- feature-adapter-staleness-liveness-core-delivery-subscription (story)
- feature-adapter-staleness-liveness-pi-delivery-loop (story)
- feature-cockpit-icon-set-cockpit-chrome (story)
- feature-cockpit-icon-set-design-system-conformance (story)
- feature-session-model-field-core-registry (story)
- feature-session-model-field-pi-adapter (story)
- feature-session-model-field-proto-contract (story)
- feature-session-model-field-surfaces (story)
- story-acceptance-issuer-context (story)
- story-approval-response-adapter-delivery (story)
- story-approval-response-conformance-vectors (story)
- story-approval-response-core-validation (story)
- story-approval-response-foundation-doc (story)
- story-approval-response-proto-message (story)
- story-bootstrap-substrates (story)
- story-connect-node-tonic-interop-spike (story)
- story-elicitation-response-conformance-vectors (story)
- story-elicitation-response-core-validation (story)
- story-elicitation-response-projection-wiring (story)
- story-elicitation-response-proto-messages (story)
- story-fix-alloy-relational-assertions (story)
- story-fix-authority-compound-issuer-integration-test (story)
- story-fix-authority-conflicting-revocation-detection (story)
- story-fix-authority-runtime-session-deployment-scope (story)
- story-fix-csrf-trace-and-ssot-drift (story)
- story-fix-failurecode-execution-outcome-unknown (story)
- story-fix-formal-model-disclosure-drift (story)
- story-fix-formal-model-genuine-checks (story)
- story-fix-sessions-ingest-correctness (story)
- story-fix-sessions-multi-delta-atomicity (story)
- story-fix-sessions-tombstone-key (story)
- story-formal-model-command-lifecycle (story)
- story-formal-model-realignment-adjacency (story)
- story-formal-model-realignment-elicitation (story)
- story-formal-model-realignment-spawn (story)
- story-formal-model-realignment-subscription (story)
- story-formal-model-realignment-traceability (story)
- story-formal-model-realignment-typed-correlation (story)
- story-protocol-idl-conformance-vectors (story)
- story-protocol-idl-generation-wiring (story)
- story-protocol-idl-proto-package (story)
- story-protocol-idl-traceability-script (story)
- story-review-provisional-semantics (story)
- story-sessions-spawn-origin-field (story)
- story-v0-core-acceptance-elicitation-slot (story)
- story-v0-core-acceptance-observation-ingestion (story)
- story-v0-core-acceptance-pipeline (story)
- story-v0-core-acceptance-proptests (story)
- story-v0-core-acceptance-replay (story)
- story-v0-core-acceptance-state-machine (story)
- story-v0-core-authority-grant-check (story)
- story-v0-core-authority-ingest (story)
- story-v0-core-authority-proptests (story)
- story-v0-core-authority-registry (story)
- story-v0-core-authority-replay (story)
- story-v0-core-authority-spawn-tail (story)
- story-v0-core-persistence-proptests (story)
- story-v0-core-persistence-recovery (story)
- story-v0-core-persistence-rusqlite-impl (story)
- story-v0-core-persistence-workspace-and-port (story)
- story-v0-core-sessions-ingest (story)
- story-v0-core-sessions-proptests (story)
- story-v0-core-sessions-registry (story)
- story-v0-core-sessions-replay-resolver (story)
- story-v0-core-sessions-state-machine (story)
- story-v0-pi-adapter-core-surface (story)
- story-v0-pi-adapter-pi-rpc-client (story)
- story-v0-pi-adapter-translation (story)
- story-v0-protocol-seam-grpc-server (story)
- story-v0-protocol-seam-proto-services (story)
- story-v0-web-cockpit-elicitation-handling (story)
- story-v0-web-cockpit-markdown-rendering (story)
- story-v0-web-cockpit-presentation-model-fold (story)
- story-v0-web-cockpit-protocol-client-reconcile (story)
- story-v0-web-cockpit-shell-session-list-detail (story)
- story-v0-web-server-rpc-bridge (story)
- story-v0-web-server-scaffold (story)
- story-v0-web-server-sessions (story)
- story-verification-correction-alloy-and-toys (story)
- story-verification-correction-command-lifecycle (story)
- story-verification-correction-draft-formulas (story)
- story-verification-correction-mutation-fragility-demotion (story)
- story-verification-correction-prose (story)
- story-verification-correction-retained-semantics (story)
- story-verification-correction-session-elicitation (story)
- story-verification-correction-trace-fidelity-demotion (story)


## Gate runs
- **gate-security** (2026-07-24) — 5 findings (1 critical, 4 medium); commit 1f1a143
- **gate-tests** (2026-07-24) — 1 finding (1 high); commit a666a40
- **gate-cruft** (2026-07-24) — 1 finding; commit 31045b5
- **gate-docs** (2026-07-24) — 6 findings; commit 2a3142b
- **gate-patterns** — pending (runs last)

### Binding-consistency warnings

BINDING CONSISTENCY — release v0.1.0 (epic_cohesion: phased): 0 CONFLICTs.
5 INCOMPLETEs, all informational under phased: the five [research]-tagged
children of epic-foundation-hardening (feature-research-contract-tooling,
feature-research-formal-methods-tooling, feature-research-harness-action-surfaces,
feature-research-v0-stack-tooling, feature-research-web-control-security) are
unbound by design — research engagements are inputs, not release members.
