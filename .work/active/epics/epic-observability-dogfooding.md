---
id: epic-observability-dogfooding
kind: epic
stage: drafting
tags: [observability, dogfooding]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-25
---

# Epic: Observability for dogfooding

## Brief

v0.1.0 shipped the deliberately minimal observability slice: redacted process
audit lines, CLI `session-health`, and web current-`CommandState` presentation,
with the query- and monitoring-oriented seams reserved (`docs/SPEC.md` v0.1.0
observability scope; `docs/PROTOCOL.md` extension seams registry). The operator
is now dogfooding Patchbay as their daily driver and cannot inspect the system
while it runs: adapter-process diagnostics die with the process, the three
diagnostic CLI commands are honest stubs, and the cockpit shows nothing about
adapter or connection health.

This epic promotes the reserved observability seams needed for live
single-operator inspection — the scope act the v0.1.0 scope statement
anticipated. It succeeds the released `feature-observability-operator-admin`
(the v0.1.0 slice) and stays inside the standing constraints: the durable event
log remains the single source of truth, no second writer, no metrics pipeline
as the primary substrate.

It is deliberately **not** the v1.0.0 supported-diagnostics story. Documented
diagnostics and health checks for other self-hosting operators remain with
`epic-public-product-contract-self-hosted-operations`, which will inherit and
harden what this epic builds. No dependency edge is declared: this epic is
sequenced ahead of the v1.0.0 work and does not assume its output.

## Strategic decisions

- **Standalone near-term epic, not folded into `epic-public-product-contract`** —
  dogfooding observability serves the current operator now; the v1.0.0 public
  supported-diagnostics contract is a later hardening of the same machinery.
  Coupling them would bind the immediate unblock to a v1-staged epic.
- **Topology is single-VM** — core, pi-adapter, and Pi sessions run on one VM;
  the workstation side is only the cockpit (browser) and optionally the
  operator CLI. All collection and storage work is local to the VM; the
  workstation question is purely a surfacing question.
- **The cockpit is the primary inspection surface** — it is what the operator
  has open while dogfooding. Observability that exists only in VM-local files
  forces SSH-side inspection; cockpit-facing surfacing is therefore prioritized
  within the epic, not deferred to the tail.
- **The v0.1.0 rejections stand** — no dedicated per-command trace storage
  (SSOT/single-writer), no metrics pipeline as the primary observability
  substrate. Everything in this epic is a projection over, or an addition to,
  the existing durable event log and process audit lines.

## Decomposition seed (for epic-design)

In priority order, as agreed with the operator:

1. **Adapter durable log sink** — pi-adapter gains an env-configured durable
   diagnostics log (e.g. `PATCHBAY_ADAPTER_LOG`, defaulting to an XDG state
   dir) capturing attach, delivery, observation, and lifecycle errors that are
   currently process-local (`#observationError`, `TranscriptEventLog`,
   `#activeCommands`). Fastest unblock for live-testing inspection.
2. **Core-diagnostics query capability** — durable, queryable projections over
   the existing event log (audit records, command history, adapter status). A
   read-side projection; no second writer.
3. **Fulfill the CLI stubs** — `audit-query`, `inspect-command`,
   `adapter-status` become real commands backed by core-diagnostics, giving a
   workstation-native inspection path. Whether these route as `query`
   Operations or a CLI-local read is a design-time call (the no-lifecycle
   bypass read remains a reserved seam until decided).
4. **Adapter diagnostics forwarding + cockpit surfacing** — adapter diagnostics
   reported to core as payload (promoting the reserved adapter-specific
   diagnostics seam), presented in the cockpit's existing views: adapter
   health, connection state, recent diagnostic events. Single pane of glass on
   the machine the operator actually sits at. UI mocks route through
   epic-design Phase 4.6 against the existing design system
   (`.mockups/design-system/`).
5. **Deferred (remains reserved)**: per-command delivery-trace timeline UI,
   metrics (counters/histograms/throughput), dedicated health/status
   dashboard, raw `event-inspect <lsn>`, SIEM export and long-retention
   archives.

## Simplification opportunity

- Deletes the three honest CLI stubs (`cli/src/commands/audit-query.ts`,
  `inspect-command.ts`, `adapter-status.ts`) and the spec/code divergence they
  document — the stub messages reference `feature-v0-cli Unit 3b`, a released
  artifact.
- The process-local `TranscriptEventLog` may be subsumed or repositioned once a
  durable adapter sink exists; design should decide whether it survives as an
  in-memory ring feeding the sink or is deleted.
- No guarantees removed: SSOT and single-writer invariants are preserved by
  construction (projection-only reads).

## Extension pressure classification

- **Promoted reserved → committed** (this epic is the scope act): durable
  queryable audit log + core-diagnostics backing `audit-query` /
  `inspect-command` / `adapter-status`; adapter-specific diagnostics as
  payload. Both flips recorded in `docs/PROTOCOL.md` extension seams registry
  and `docs/SPEC.md` post-v0.1.0 observability scope.
- **Remains reserved**: delivery-trace timeline UI, metrics, dedicated
  health/status dashboard, `event-inspect`, SIEM, no-lifecycle bypass read of
  the audit log.
- **Remains rejected**: dedicated per-command trace storage; metrics pipeline
  as primary substrate.
- Parked-idea pressure: none of the four parked ideas is foreclosed — adapter
  diagnostics payload is adapter-declared, and cockpit presentation is a
  surface-declared feature, keeping adapter- and surface-neutrality intact.
