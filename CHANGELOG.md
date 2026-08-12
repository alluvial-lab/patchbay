# Changelog

## v0.2.0

### Features

- **Revocation lifecycle** — added durable grant expiry/revocation, operator-session generation fences, principal/endpoint/device controls, all-session revocation, and restart-stable security lockdown with explicit bootstrap-channel recovery.
- **Operational-resource plane** — introduced typed `(adapter, kind, resource)` identity, exact resource authority, capability-manifest admission, authenticated resource reporting, replay/snapshot materialization, resource-targeted Operations, and cockpit composition alongside sessions.
- **token-commune observer** — added a materially distinct resource adapter with gateway attachment diagnostics, polling/dedup/reconnect handling, provider-pool/member/draw projections, CLI/cockpit views, degradation honesty, and executable cross-layer conformance evidence.
- **Core diagnostics dogfooding** — added durable redacted audit and command-inspection projections, principal-gated diagnostic queries, adapter diagnostic forwarding, and cockpit/CLI operational views.
- **Recovery checkpointing** — added scheduled, crash-safe session checkpoints with rejection repair, bounded fallback, generation validation, and file-backed restart evidence.
- **Adapter report ordering** — whole-session reports now carry authenticated producer-generation/revision cursors; stale or reordered reports are fenced before durable mutation and replay restores the watermark.
- **Authority completion** — made descendant-grant issuance crash-safe and idempotent, completed live spawn composition, and made matching-grant selection deterministic and auditable.

### Fixes

- Repaired session replay-domain binding, shared replay-prefix validation, resource reconciliation prefix handling, and durable snapshot generation/compatibility semantics.
- Fixed authority-writer atomicity/retry evidence and rebuilt missing grant-identity index rows during bootstrap so seeded grants remain writable and selectable after restart.
- Fixed expired-session startup, exact resource-grant selection in CLI submissions, chat activity state, tool-call argument previews, cockpit render amplification, and scroll anchoring.
- Scoped every Rust test tempfile under `target/test-tmp` at process load and through Cargo environment configuration, eliminating `/tmp` leakage across binaries and newly added tests.

### Security

- Upgraded the Pi SDK/runtime dependency chain and Fastify routing transitive dependencies past high-severity Undici, brace-expansion, fast-uri, and find-my-way advisories; migrated the Pi adapter to the current `ModelRuntime` SDK surface.
- Added source-authenticated report ordering, exact resource-source containment, revocation controls, and lockdown adversaries to the executable security conformance profile.

### Documentation

- Rolled protocol, security, verification, architecture, UX, and operator guidance forward for revocation, resource identities/state, source cursors, diagnostics, token-commune behavior, recovery checkpoints, and extension-seam classifications.

### Internal

- Expanded generated-contract drift checks, shared conformance runners, mutation-accounting guards, real-core E2E coverage, and CI coverage for all TypeScript consumers.

## v0.1.0 (2026-07-24)

Initial-operator walking skeleton: one operator controls Pi-backed agent
sessions through the responsive web cockpit and diagnostic CLI. Personal/internal
milestone — not a public distribution.

### Features

- **Coordination core (Rust)** — durable command log with replay/snapshot
  convergence (SQLite WAL), authority model (grants, principals, compound
  issuers, revocation), session registry with generation fencing, acceptance
  pipeline with idempotent submission.
- **Protocol seam** — generated contracts from proto (dual Rust/TS codegen),
  gRPC adapter + control services, fencing tokens, conformance vectors,
  extension-seams registry.
- **Formal assurance** — Quint/TLA+/Alloy models for command lifecycle,
  session/elicitation races, crash/replay convergence; property tests and
  mutation oracles wired to the implementation.
- **Pi adapter (TS)** — session hosting with fencing-token auth, translation
  layer, ordered report tail; long-lived `ReceiveDeliveries` subscription
  (replaces polling fallback) with stream-drop staleness and running-command
  reconciliation to `failed(execution_outcome_unknown)`.
- **Web cockpit** — responsive operator surface: session list/detail, transcript
  streaming with per-message delta ordering, elicitation handling, markdown
  rendering, Lucide icon system bound into the presentation conformance floor.
- **CLI** — setup/login (throttled), session-health, instruct, cancel/interrupt;
  0600 credential store. `audit-query`/`inspect-command`/`adapter-status`
  ship as documented stubs.
- **Session model field** — agent model surfaces end-to-end (proto → core →
  adapter → cockpit/CLI) with `SessionModelChanged` for mid-session switches.

### Fixes

- Pre-release live-test wave: CSRF method-case 403s, fetch binding, transcript
  delta dedup dropping ~46% of streamed deltas, thinking-block leakage,
  per-session report serialization, viewport/scroll bounds, Enter-to-send.
- Review wave: model-change reports no longer fabricate idle state or collapse
  rapid switches (A→B→C ordering preserved); session-report append +
  projection update serialized under one lock (unreplayable-log race closed);
  reconnect catch-up can no longer re-execute terminal commands.
- Stale e2e harness migrated to the continuous-delivery model.

### Security

- Operation validity windows enforced at acceptance (expired/not-yet-valid
  rejected; zero-skew v0.1.0 policy).
- SQLite state files created/tightened to 0600.
- CLI rejects secrets on argv (env/prompt only); loopback proxy HTTPS
  verification is explicit and fail-closed.
- Deployment credentials rotated after a session-note leak; note convention
  established (tracked notes never carry live secrets).

### Documentation

- Foundation docs rolled to shipped truth: README layout, ARCHITECTURE seams,
  SECURITY deployment topology, RUNBOOK (honest v0.1.0 limitations: transport
  black-hole corner, core-fault conflation, running-rot semantics).
