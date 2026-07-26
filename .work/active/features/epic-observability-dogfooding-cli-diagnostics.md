---
id: epic-observability-dogfooding-cli-diagnostics
kind: feature
stage: drafting
tags: [observability, dogfooding]
parent: epic-observability-dogfooding
depends_on: [epic-observability-dogfooding-core-diagnostics]
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-25
---

# CLI diagnostics commands

## Brief

`audit-query`, `inspect-command`, and `adapter-status` shipped in v0.1.0 as
honest stubs that exit non-zero with a prerequisite message. This feature
fulfills them as real commands backed by the core-diagnostics query surface,
following the established CLI pattern (`session-health` queries core over
gRPC and renders a projection, with `--json` script-facing output).

This gives the operator a workstation-native inspection path: the CLI reaches
core the same way it already does for every other command, so diagnostics are
available from the workstation without SSHing into the VM.

It does NOT cover: the core-diagnostics surface itself
(`epic-observability-dogfooding-core-diagnostics`), cockpit presentation
(`epic-observability-dogfooding-cockpit-diagnostics`), or `event-inspect
<lsn>` (reserved seam).

## Epic context

- Parent epic: `epic-observability-dogfooding`
- Position in epic: consumer of `epic-observability-dogfooding-core-diagnostics`
  — depends on its query surface and generated contract types. Parallel with
  the cockpit-diagnostics consumer. Priority 3 in the epic's seed order.

## Simplification opportunity

- Deletes the three stub command bodies and their prerequisite messages —
  the stubs reference `feature-v0-cli Unit 3b`, a released artifact, and
  their existence is the spec/code divergence this epic closes.
- The three commands should share one diagnostics query/render path rather
  than growing three near-duplicate client/render stacks.

## Foundation references

- `docs/UX.md` — CLI conventions (script-facing output, diagnostic CLI role)
- `docs/SPEC.md` — post-v0.1.0 observability scope
- `docs/PROTOCOL.md` — Persistence and recovery (control surfaces never touch
  persistence directly)

<!-- The design pass on this feature (`/agile-workflow:feature-design`) will
fill in command flags, output shapes, and implementation units. -->
