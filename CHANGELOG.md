# Changelog

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
