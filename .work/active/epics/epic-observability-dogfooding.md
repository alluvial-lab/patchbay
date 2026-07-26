---
id: epic-observability-dogfooding
kind: epic
stage: review
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

## Decomposition seed (realized above — retained for the record of the agreed priority order)

1. Adapter durable log sink → `epic-observability-dogfooding-adapter-log-sink`
2. Core-diagnostics query capability → `epic-observability-dogfooding-core-diagnostics`
3. Fulfill the CLI stubs → `epic-observability-dogfooding-cli-diagnostics`
4. Adapter diagnostics forwarding + cockpit surfacing → `epic-observability-dogfooding-cockpit-diagnostics`
5. Deferred (remains reserved): delivery-trace timeline UI, metrics, dedicated health/status dashboard, `event-inspect <lsn>`, SIEM export.

<details><summary>Original seed prose</summary>

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

</details>

## Design decisions

- **Diagnostics queries route through the core; no CLI-local bypass read.**
  The operator CLI runs on the workstation while the SQLite store lives on
  the VM, so a persistence-local read cannot serve the primary case — and
  PROTOCOL already bars control surfaces from touching persistence directly.
  `session-health` sets the pattern (gRPC query against core, rendered
  projection). The no-lifecycle bypass-read seam stays reserved.
- **Audit records gain durable persistence in core storage.** The committed
  "durable queryable audit log" cannot be served from stderr-only lines.
  The core writes redacted audit records to storage behind the existing
  ports (single-writer preserved); the SECURITY redaction list applies
  unchanged; stderr lines remain as process diagnostics.
- **No mockups at epic tier.** Cockpit surfacing composes adapter health
  into existing views reusing the current CommandState/status presentation
  patterns (mockup-first convention skip rule: feature-level UI cleanly
  reusing existing components). The dedicated health/status dashboard —
  the UI that would warrant mocks — remains a reserved seam. Feature design
  on `epic-observability-dogfooding-cockpit-diagnostics` re-evaluates and
  falls back to `/ux-ui-design:screens` if composition proves insufficient.
- **Adapter log default location: XDG state dir** (`~/.local/state/patchbay/`),
  overridable via `PATCHBAY_ADAPTER_LOG`. Routine, reversible; durable across
  reboots unlike `/tmp`, outside the repo unlike the CWD.

## Decomposition

Split by capability along the inspection path: one adapter-local producer
(log sink), one core foundation (durable audit records + query surface), and
two parallel consumers (CLI, cockpit). The log sink and core-diagnostics are
independent and can start immediately; both consumers wait only on
core-diagnostics. This shape was chosen over a layer split
(contracts/core/surface features) because each consumer capability needs its
contract, core, and surface pieces to agree on one vocabulary.

### Child features

- `epic-observability-dogfooding-adapter-log-sink` — pi-adapter durable, configurable diagnostics log file — depends on: `[]`
- `epic-observability-dogfooding-core-diagnostics` — durable audit records + queryable diagnostics projections in the core — depends on: `[]`
- `epic-observability-dogfooding-cli-diagnostics` — fulfill `audit-query` / `inspect-command` / `adapter-status` against core-diagnostics — depends on: `[epic-observability-dogfooding-core-diagnostics]`
- `epic-observability-dogfooding-cockpit-diagnostics` — adapter diagnostics forwarded to core as payload + cockpit presentation of adapter health — depends on: `[epic-observability-dogfooding-core-diagnostics]`

### Simplification arcs

- `epic-observability-dogfooding-cli-diagnostics` — deletes the three honest
  stub commands and the released-artifact references in their messages.
- `epic-observability-dogfooding-adapter-log-sink` — repositions or deletes
  the process-local `TranscriptEventLog`.
- `epic-observability-dogfooding-core-diagnostics` — consolidates audit
  emission behind one sink abstraction (`StderrLoginAuditSink` becomes one
  implementation, not the only channel).

### Decomposition risks

- **Core-diagnostics is the critical path** — both consumers block on it, and
  it carries the epic's hardest design questions (audit-record storage
  schema, query operation shapes, generalizing the login-audit trait to the
  full audit vocabulary). Mitigation: its contract types land first in its
  own design pass, unblocking consumer design before consumer implementation.
- **Cockpit-diagnostics spans contract + core + web** — wide but one
  capability; if its design pass finds it exceeding the 5-15 unit sizing
  rule, split the presentation units into a child story rather than slicing
  by layer.
- **Priority vs dependency tension** — cockpit surfacing is the highest
  dogfooding value but sits at the end of the longest chain. The adapter log
  sink (priority 1) and CLI (priority 3) deliver inspection value earlier;
  accept the ordering rather than distort the dependency graph.

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

## Aggregate completion note (2026-07-26)

All four child features are `done`. Realized delivery:

- `adapter-log-sink` — durable JSONL diagnostics sink (bounded async queue,
  rotation, XDG/env path, structural redaction, total error normalization);
  TranscriptEventLog deleted. Review: 3 blockers found and fixed.
- `core-diagnostics` — durable audit records as typed domain decisions with
  atomic source+audit commit, versioned mutation-free migrations, resumable
  QueryDiagnostics lifecycle with bounded prefixes, full SECURITY producer
  coverage (lockdown deferred — no decision surface exists). Review: 6
  blockers found and fixed.
- `cli-diagnostics` — audit-query / inspect-command / adapter-status as real
  commands; shared runner; exit-code discipline; UX.md/RUNBOOK.md rolled
  forward. Review: 5 blockers found and fixed.
- `cockpit-diagnostics` — adapter ReportDiagnostics ingestion (atomic
  Observation + audit), best-effort abortable forwarding sharing the
  AdapterDiagnostics port, cockpit composition into existing views with
  monotonic as_of_lsn merge and explicit reconciliation signaling. Review: 5
  blockers found and fixed.

Verification at epic close: cargo workspace (30 suites) + clippy clean;
cli 26, web-server 25, web-cockpit 53, pi-adapter 24, e2e walking skeleton,
contracts vectors/models — all green. One intermittent pi-adapter e2e flake
parked (idea-pi-adapter-e2e-intermittent-flake); pre-existing
generated-contract drift parked (idea-generated-contract-drift-ci-gap);
lockdown audit producers deferred to the lockdown capability.
