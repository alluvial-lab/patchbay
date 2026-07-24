---
id: release-v0.1.0
kind: release
stage: released
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
- **gate-patterns** (2026-07-24) — 6 patterns promoted (`.agents/skills/patterns/` + rules digest); commit e4051fb

### Binding-consistency warnings

BINDING CONSISTENCY — release v0.1.0 (epic_cohesion: phased): 0 CONFLICTs.
5 INCOMPLETEs, all informational under phased: the five [research]-tagged
children of epic-foundation-hardening (feature-research-contract-tooling,
feature-research-formal-methods-tooling, feature-research-harness-action-surfaces,
feature-research-v0-stack-tooling, feature-research-web-control-security) are
unbound by design — research engagements are inputs, not release members.

## Shipped items

Bodies live in git history (delete-refs) — `git show a752ade:<former active path>` recovers any pruned body.

| id | title | kind | archived_atop | git ref |
|----|-------|------|---------------|---------|
| epic-foundation-hardening | Epic: Foundation hardening after adversarial review | epic | — | a752ade |
| epic-retroactive-design-gate-audit | Epic: Retroactive design-gate audit of foundational decision | epic | — | a752ade |
| epic-v0-1-0-implementation | Epic: v0.1.0 implementation | epic | — | a752ade |
| epic-v0-core | Epic: Rust coordination core | epic | — | a752ade |
| epic-public-product-contract-verification-claim-correction | Verification claim correction | feature | — | a752ade |
| feature-adapter-staleness-liveness | Feature: full-coverage adapter staleness (heartbeat / last-r | feature | — | a752ade |
| feature-audit-command-state-ssot | Feature: Retroactive design-gate audit — command/session/f | feature | — | a752ade |
| feature-audit-persistence-snapshot-model | Feature: Retroactive design-gate audit — persistence, orde | feature | — | a752ade |
| feature-audit-security-threat-model | Feature: Retroactive design-gate audit — v0 security, prin | feature | — | a752ade |
| feature-audit-v0-walking-skeleton | Feature: Retroactive design-gate audit — v0 walking skelet | feature | — | a752ade |
| feature-bank-formal-methods-skills | Bank formal-methods reference skills | feature | — | a752ade |
| feature-cockpit-icon-set | Feature: adopt an icon set for the cockpit chrome | feature | — | a752ade |
| feature-command-state-ssot | Feature: Define canonical command, session, and failure stat | feature | — | a752ade |
| feature-design-grant-shape | Design: v0 grant shape and delegation seam | feature | — | a752ade |
| feature-design-terminal-commit-race | Design: command terminal-commit race resolution | feature | — | a752ade |
| feature-extension-seams-non-foreclosure | Feature: Define extension seams and non-foreclosure rules | feature | — | a752ade |
| feature-formal-model-realignment | Feature: Re-align seed formal models with the rolled-forward | feature | — | a752ade |
| feature-formal-model-seed | Feature: Author seed formal models | feature | — | a752ade |
| feature-foundation-doc-completeness-gaps | Feature: Close foundation-doc completeness gaps from the O/O | feature | — | a752ade |
| feature-idempotency-ambiguous-execution | Feature: Refine idempotency and ambiguous execution semantic | feature | — | a752ade |
| feature-lease-scope-decision | Feature: Decide lease scope for v0 | feature | — | a752ade |
| feature-observability-operator-admin | Feature: Define operator/admin observability | feature | — | a752ade |
| feature-operator-presence-and-action-inventory | Feature: Sharpen operator-presence positioning and derive th | feature | — | a752ade |
| feature-persistence-snapshot-model | Feature: Define persistence, event ordering, and snapshot co | feature | — | a752ade |
| feature-pi-parity-checklist | Feature: Define Pi migration and parity checklist | feature | — | a752ade |
| feature-protocol-idl-and-conformance | Feature: Author v0 protocol IDL and conformance vectors | feature | — | a752ade |
| feature-security-threat-model | Feature: Define v0 security, principal, and threat model | feature | — | a752ade |
| feature-session-identity-adapter-contract | Feature: Define session identity and adapter capability cont | feature | — | a752ade |
| feature-session-model-field | Feature: surface the agent model in session reports | feature | — | a752ade |
| feature-ux-v0-acceptance | Feature: Define v0 web cockpit UX acceptance criteria | feature | — | a752ade |
| feature-v0-approval-response-contract | Feature: Typed approval-response contract (binary Approve/De | feature | — | a752ade |
| feature-v0-cli | Feature: CLI | feature | — | a752ade |
| feature-v0-control-surface-trust-boundary | Feature: v0.1.0 control-surface trust boundary (real transpo | feature | — | a752ade |
| feature-v0-core-acceptance | Feature: Operation acceptance and command lifecycle | feature | — | a752ade |
| feature-v0-core-authority | Feature: Authority, grants, and audit | feature | — | a752ade |
| feature-v0-core-persistence | Feature: Core persistence, event log, and recovery | feature | — | a752ade |
| feature-v0-core-sessions | Feature: Session registry and generation | feature | — | a752ade |
| feature-v0-elicitation-response-contract | Feature: Typed elicitation-response contract (EC1–EC3) | feature | — | a752ade |
| feature-v0-pi-adapter | Feature: Pi adapter | feature | — | a752ade |
| feature-v0-presentation-component-layer | Feature: Shared presentation-component layer (v0) | feature | — | a752ade |
| feature-v0-protocol-seam | Feature: Web↔core internal protocol seam | feature | — | a752ade |
| feature-v0-walking-skeleton | Feature: Define the v0 walking skeleton | feature | — | a752ade |
| feature-v0-web-cockpit | Feature: Responsive web cockpit | feature | — | a752ade |
| feature-v0-web-server | Feature: TypeScript web server | feature | — | a752ade |
| feature-verification-contract-authority | Design: Verification, contract, and authority order | feature | — | a752ade |
| feature-adapter-staleness-liveness-core-delivery-subscription | Story: long-lived core delivery stream and adapter-loss reco | story | — | a752ade |
| feature-adapter-staleness-liveness-pi-delivery-loop | Story: consume the Pi delivery stream continuously | story | — | a752ade |
| feature-cockpit-icon-set-cockpit-chrome | Story: Apply typed Lucide icons across cockpit chrome | story | — | a752ade |
| feature-cockpit-icon-set-design-system-conformance | Story: Icon primitive and presentation conformance | story | — | a752ade |
| feature-session-model-field-core-registry | Story: Fold and materialize mutable session model state | story | — | a752ade |
| feature-session-model-field-pi-adapter | Story: Report Pi session model at registration and on change | story | — | a752ade |
| feature-session-model-field-proto-contract | Story: Add the durable session-model contract | story | — | a752ade |
| feature-session-model-field-surfaces | Story: Present the current session model in cockpit and CLI | story | — | a752ade |
| gate-cruft-stale-e2e-poll-fixture | Replace the stale polling E2E adapter fixture | story | — | a752ade |
| gate-docs-architecture-presentation-seam | Architecture defers an implemented presentation layer | story | — | a752ade |
| gate-docs-architecture-split-topology | Architecture claims v0.1 supports split deployment | story | — | a752ade |
| gate-docs-architecture-web-core-seam | Architecture defers the shipped web-to-core protocol seam | story | — | a752ade |
| gate-docs-readme-layout | README repository layout still calls shipped components futu | story | — | a752ade |
| gate-docs-runbook-web-operator-config | Runbook marks required web operator identity optional | story | — | a752ade |
| gate-docs-security-deployment-topology | Security deployment floor permits unsupported v0 topologies | story | — | a752ade |
| gate-patterns-v0.1.0 | Patterns extracted for v0.1.0 | story | — | a752ade |
| gate-security-enforce-operation-validity-window | Enforce Operation validity windows before durable acceptance | story | — | a752ade |
| gate-security-protect-sqlite-state-files | Protect SQLite state files from permissive process umasks | story | — | a752ade |
| gate-security-remove-cli-secrets-from-argv | Remove operator and setup secrets from CLI arguments | story | — | a752ade |
| gate-security-rotate-committed-live-credentials | Rotate live deployment credentials committed in a tracked se | story | — | a752ade |
| gate-security-verify-proxied-https | Verify browser HTTPS across trusted reverse-proxy hops | story | — | a752ade |
| gate-tests-delivery-reconnect-cursor-terminal-filter | Test reconnect catch-up preserves delivery eligibility | story | — | a752ade |
| story-acceptance-issuer-context | Story: Acceptance submit takes a verified IssuerContext (aut | story | — | a752ade |
| story-approval-response-adapter-delivery | Story: Pi-adapter delivery of the approval decision | story | — | a752ade |
| story-approval-response-conformance-vectors | Story: Conformance vectors for the approval response contrac | story | — | a752ade |
| story-approval-response-core-validation | Story: Core boundary validation + DENIED→Declined terminal | story | — | a752ade |
| story-approval-response-foundation-doc | Story: Foundation-doc roll-forward (PROTOCOL.md) | story | — | a752ade |
| story-approval-response-proto-message | Story: Typed proto message (ApprovalDecision + ApprovalRespo | story | — | a752ade |
| story-bootstrap-substrates | Story: Bootstrap work and research substrates | story | — | a752ade |
| story-connect-node-tonic-interop-spike | Spike: @connectrpc/connect-node ↔ tonic interop validation | story | — | a752ade |
| story-elicitation-response-conformance-vectors | Story: Conformance vectors for the elicitation response cont | story | — | a752ade |
| story-elicitation-response-core-validation | Story: Core boundary validation (Fail-Fast response payload  | story | — | a752ade |
| story-elicitation-response-projection-wiring | Story: ElicitationSlotLayer extension + server projection wi | story | — | a752ade |
| story-elicitation-response-proto-messages | Story: Typed proto messages (question contract + response pa | story | — | a752ade |
| story-fix-alloy-relational-assertions | Story: Fix the failing Alloy relational assertions (B5/B6 re | story | — | a752ade |
| story-fix-authority-compound-issuer-integration-test | Story: CompoundIssuer proptest must drive acceptance::submit | story | — | a752ade |
| story-fix-authority-conflicting-revocation-detection | Story: Conflicting same-generation revocations must be Corru | story | — | a752ade |
| story-fix-authority-runtime-session-deployment-scope | Story: RuntimeSession scope match must include deployment_sc | story | — | a752ade |
| story-fix-csrf-trace-and-ssot-drift | Story: Close the CSRF attempted-evidence trace gap + fix voc | story | — | a752ade |
| story-fix-failurecode-execution-outcome-unknown | Fix: FailureCode proto missing `execution_outcome_unknown` | story | — | a752ade |
| story-fix-formal-model-disclosure-drift | Story: Fix disclosure drift and emitted-artifact issues in t | story | — | a752ade |
| story-fix-formal-model-genuine-checks | Story: Fix self-defining properties in the seed formal model | story | — | a752ade |
| story-fix-sessions-ingest-correctness | Story: Fix sessions ingest correctness (3 blockers from feat | story | — | a752ade |
| story-fix-sessions-multi-delta-atomicity | Story: Fix sessions multi-delta append atomicity (B5, regres | story | — | a752ade |
| story-fix-sessions-tombstone-key | Story: Fix session tombstone key to include full identity | story | — | a752ade |
| story-formal-model-command-lifecycle | Story: Author command_lifecycle.qnt (the fused terminal-race | story | — | a752ade |
| story-formal-model-realignment-adjacency | Story: V1 transition-adjacency strengthening (Unit CL) | story | — | a752ade |
| story-formal-model-realignment-elicitation | Story: Elicitation lifecycle model (Unit EL) | story | — | a752ade |
| story-formal-model-realignment-spawn | Story: Spawn authority (Unit SA — promote into authority.q | story | — | a752ade |
| story-formal-model-realignment-subscription | Story: Subscription authority (Unit SUB — promote into aut | story | — | a752ade |
| story-formal-model-realignment-traceability | Story: Traceability script + VR2 metadata realignment + VR4  | story | — | a752ade |
| story-formal-model-realignment-typed-correlation | Story: TypedCorrelation extension (Unit TC) | story | — | a752ade |
| story-protocol-idl-conformance-vectors | Story: Author the v0 conformance vectors | story | — | a752ade |
| story-protocol-idl-generation-wiring | Story: Wire up buf generate for Rust + TypeScript | story | — | a752ade |
| story-protocol-idl-proto-package | Story: Author the v0 .proto package | story | — | a752ade |
| story-protocol-idl-traceability-script | Story: CI traceability script + VERIFICATION.md reference | story | — | a752ade |
| story-review-provisional-semantics | Review four provisional semantic choices in committed v0 doc | story | — | a752ade |
| story-sessions-spawn-origin-field | Story: Add SessionRegistered.spawn_origin field (authority p | story | — | a752ade |
| story-v0-core-acceptance-elicitation-slot | Story: Elicitation-slot terminalization (A2 decoupled layer) | story | — | a752ade |
| story-v0-core-acceptance-observation-ingestion | Story: Observation ingestion and command-state reflection | story | — | a752ade |
| story-v0-core-acceptance-pipeline | Story: Acceptance pipeline and the three ports | story | — | a752ade |
| story-v0-core-acceptance-proptests | Story: Property tests for acceptance invariants | story | — | a752ade |
| story-v0-core-acceptance-replay | Story: Replay and in-memory index reconstruction | story | — | a752ade |
| story-v0-core-acceptance-state-machine | Story: Command state machine and transition validation | story | — | a752ade |
| story-v0-core-authority-grant-check | Story: IssuerContext port + GrantCheck impl (the acceptance  | story | — | a752ade |
| story-v0-core-authority-ingest | Story: Grant + revocation ingestion (writer) | story | — | a752ade |
| story-v0-core-authority-proptests | Story: Property tests for authority invariants (7 oracles +  | story | — | a752ade |
| story-v0-core-authority-registry | Story: Grant/revocation event model and AuthorityRegistry pr | story | — | a752ade |
| story-v0-core-authority-replay | Story: Replay and module wiring | story | — | a752ade |
| story-v0-core-authority-spawn-tail | Story: Descendant-grant-on-spawn log-tail reactor (order-ind | story | — | a752ade |
| story-v0-core-persistence-proptests | Story: Property tests for storage invariants | story | — | a752ade |
| story-v0-core-persistence-recovery | Story: Crash recovery and replay | story | — | a752ade |
| story-v0-core-persistence-rusqlite-impl | Story: rusqlite storage implementation + writer actor | story | — | a752ade |
| story-v0-core-persistence-workspace-and-port | Story: Workspace scaffolding and storage port trait | story | — | a752ade |
| story-v0-core-sessions-ingest | Story: Session report ingestion (the writer) | story | — | a752ade |
| story-v0-core-sessions-proptests | Story: Property tests for session invariants | story | — | a752ade |
| story-v0-core-sessions-registry | Story: Session delta events and the SessionRegistry projecti | story | — | a752ade |
| story-v0-core-sessions-replay-resolver | Story: Replay, TargetResolver impl, and module wiring | story | — | a752ade |
| story-v0-core-sessions-state-machine | Story: Session identity, state axes, and transition validati | story | — | a752ade |
| story-v0-pi-adapter-core-surface | Story: pi-adapter core-facing RPC surface | story | — | a752ade |
| story-v0-pi-adapter-pi-rpc-client | Story: pi-adapter Pi `AgentSession` driver (harvested in-pro | story | — | a752ade |
| story-v0-pi-adapter-translation | Story: pi-adapter translation + session registry + e2e | story | — | a752ade |
| story-v0-protocol-seam-grpc-server | Story: protocol-seam gRPC server crate | story | — | a752ade |
| story-v0-protocol-seam-proto-services | Story: protocol-seam proto service definitions | story | — | a752ade |
| story-v0-web-cockpit-elicitation-handling | Story: Cockpit Unit 4 — elicitation handling (three shapes | story | — | a752ade |
| story-v0-web-cockpit-markdown-rendering | Story: Cockpit Unit 3 — markdown rendering (the mobile-rea | story | — | a752ade |
| story-v0-web-cockpit-presentation-model-fold | Story: Cockpit Unit 2 — presentation model fold | story | — | a752ade |
| story-v0-web-cockpit-protocol-client-reconcile | Story: Cockpit Unit 1 — protocol client + cursor-reconcile | story | — | a752ade |
| story-v0-web-cockpit-shell-session-list-detail | Story: Cockpit Unit 5 — shell + session list + responsive  | story | — | a752ade |
| story-v0-web-server-rpc-bridge | Story: web-server Connect-Web RPC bridge + integration tests | story | — | a752ade |
| story-v0-web-server-scaffold | Story: web-server scaffold + core client | story | — | a752ade |
| story-v0-web-server-sessions | Story: web-server operator-session store + CSRF/auth guard | story | — | a752ade |
| story-verification-correction-alloy-and-toys | Demote ActorIdsUnique and relocate superseded toy artifacts | story | — | a752ade |
| story-verification-correction-command-lifecycle | Demote overclaiming command_lifecycle.qnt properties | story | — | a752ade |
| story-verification-correction-draft-formulas | Remove misleading draft formulas and demote SpawnCreatesDesc | story | — | a752ade |
| story-verification-correction-mutation-fragility-demotion | Remove leftover demoted formulas and demote mutation-fragile | story | — | a752ade |
| story-verification-correction-prose | Fix stale PROTOCOL.md prose and audit emitted TLA+ | story | — | a752ade |
| story-verification-correction-retained-semantics | Narrow retained promoted property semantics and fix stale mo | story | — | a752ade |
| story-verification-correction-session-elicitation | Demote overclaiming session_generation and elicitation_lifec | story | — | a752ade |
| story-verification-correction-trace-fidelity-demotion | Demote trace-fidelity-defective promoted authority propertie | story | — | a752ade |

## Ship record

- **Shipped:** 2026-07-24
- **Mapping:** tag-based — `git tag v0.1.0` (annotated), pushed to origin (code.s-nc.org/Kevoun/patchbay)
- **Items shipped:** 145
- **Gate findings:** security 5 (1 critical, 4 medium), tests 1 (high), cruft 1, docs 6, patterns 6 promoted — all gate-produced items done before ship
