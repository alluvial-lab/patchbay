---
id: epic-revocation-lifecycle-session-principal-revocation-conformance-foundation
kind: story
stage: done
tags: [security, foundation, verification]
parent: epic-revocation-lifecycle-session-principal-revocation
depends_on: [epic-revocation-lifecycle-session-principal-revocation-web-session-plane, epic-revocation-lifecycle-session-principal-revocation-cli-controls]
release_binding: null
gate_origin: null
created: 2026-07-27
updated: 2026-07-27
---

# Integrated revocation conformance and foundation

## Checkpoint

Run the integrated real-process/restart/conformance matrix after both control
surfaces consume the core contract, then roll the foundation docs forward and
close the absorbed session-record-fields gap without historical appendices.

This checkpoint owns Unit 5 in the parent feature. The parent design is
authoritative for the test-risk mapping, docs touched, property-status honesty,
accepted-work policy, and recovery boundary.

## Acceptance evidence

- Real-process and replay tests prove old-session generation, exact principal,
  endpoint, and device evidence reject before Operation acceptance and
  subscription establishment; unrelated endpoints remain usable.
- Accepted-before-revocation work may continue under this plane's `continue`
  policy, and command/audit history is never deleted or rewritten.
- Model checks, guard-removal mutations, non-vacuity runs, vectors, generated
  traceability, generated-contract drift, Rust workspace/clippy, TypeScript
  workspace, and restart tests are green. Any property lacking genuine model +
  promoted-vector evidence remains honestly stated-normative.
- `SECURITY.md`, `PROTOCOL.md`, `VERIFICATION.md`, `UX.md`, `GLOSSARY.md`, and
  `RUNBOOK.md` describe implemented revocation, distinguish operator-session
  generation from runtime-session generation, list CLI recovery, and remove the
  stale “only #1 implemented” status.
- `backlog-session-record-fields-gap` is dispositioned as absorbed; no duplicate
  follow-up remains.

## Ordering constraints

Runs only after both web and CLI checkpoints. As a `[verification]` story, its
review uses the project deep lane and independently attacks atomic source/audit
ordering, replay, model genuineness, and self-lockout recovery claims.

## Implementation notes
- Execution capability: inline integrated verification; the cross-boundary gRPC tests and foundation-doc roll-forward required one owner to reconcile model status and recovery claims.
- Review weight: deep verification lane.
- Files changed: `server/tests/grpc_smoke.rs`, `server/src/service.rs`, `docs/{SECURITY,PROTOCOL,VERIFICATION,UX,GLOSSARY,RUNBOOK}.md`, and `.work/archive/backlog-session-record-fields-gap.md`.
- Tests added/removed: real gRPC principal/endpoint/device rejection plus subscription-denial matrix; restart/re-login generation-floor test; no tests removed.
- Simplification: one integrated scope matrix exercises all three credential fences and unrelated-surface continuity; no separate recovery mechanism was introduced.
- Discrepancies from design: the session/principal Quint properties and vectors remain draft/stated-normative, as the generated traceability correctly reports; implementation evidence is not being mislabeled as model promotion. Integrated testing also exposed and removed a duplicate endpoint decision-gate acquisition.
- Adjacent issues parked: none; the session-record-fields backlog item is archived as absorbed.
- Verification: workspace Rust tests, workspace clippy, contracts model/vector/drift/presentation checks, web-server, CLI, web-cockpit, and Pi adapter test suites passed. The first parallel workspace test run encountered a transient doctest artifact race with clippy; the sequential rerun passed completely.
