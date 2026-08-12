---
id: epic-agent-operations-resource-plane-resource-identity-integration-conformance
kind: story
stage: done
tags: [foundation, protocol, security, testing]
parent: epic-agent-operations-resource-plane-resource-identity
depends_on: [epic-agent-operations-resource-plane-resource-identity-polymorphic-target-resolution, epic-agent-operations-resource-plane-resource-identity-resource-authority-containment]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-03
updated: 2026-08-04
---

# Close resource identity acceptance and compatibility evidence

## Checkpoint

Land Unit 4 from the parent design: enforce canonical resource shape before
grant evaluation, prove authorized+registered resource acceptance through the
existing durable pipeline, preserve existing control-surface audit targets on
Protobuf tag 8 without making them operational, and roll PROTOCOL, SECURITY,
VERIFICATION, and GLOSSARY assertions forward at an honest evidence tier.

## Acceptance evidence

- Malformed/legacy resource scopes reject with `validation_failed` before grant,
  resolver, dedup, or durable append; authorized but unknown typed resources
  reject `target_not_found` without append.
- An exact authorized and registered resource Operation is accepted without any
  runtime-session id/generation and deduplicates by the complete typed target.
- Cross-adapter and cross-kind collision attempts cannot authorize, resolve,
  append, or reach the wrong adapter.
- Runtime-session acceptance and authority-domain diagnostics remain green.
- Principal/endpoint/device revocation audit records preserve their target ids
  through generation, storage, filtering, and JSON output, but cannot resolve
  or satisfy a resource grant. Operational resource audit filters use canonical
  `adapter=...;resource-kind=...;resource=...`; `resource=ID` stays audit-only.
- Rust workspace tests/clippy, TypeScript contract/CLI tests, model/vector
  metadata checks, and generated drift checks pass.
- Foundation docs state the tuple and collision fence without claiming the
  promoted conformance evidence owned by the epic's closing conformance feature.

## Ordering constraints

Consumes both polymorphic resolution and authority containment. This checkpoint
closes their shared acceptance/delivery boundary; it must not absorb resource
snapshots/revisions, capability manifest fields, adapter projection schemas, or
cockpit rendering.

## Implementation notes

- Acceptance now validates canonical resource shape before issuer posture, grant, resolver, dedup, or append work. Nested identity on a non-resource target and any operational use of the legacy audit scalar fail with `validation_failed`.
- Added integrated evidence for exact authorized+registered acceptance without session fields, authorized-but-unknown `target_not_found`, malformed-before-port ordering, exact grant collision denial, and full-tuple target-key separation.
- Preserved control-surface audit compatibility: existing producers and stored tag-8 bytes use `legacy_audit_resource_id`; CLI JSON exposes that audit-only value separately from nested operational resource identity. Audit filtering keeps `resource=ID` audit-only and adds canonical `adapter=...;resource-kind=...;resource=...` parsing with percent encoding.
- Rolled PROTOCOL, SECURITY, VERIFICATION, UX, and GLOSSARY assertions forward. Verification explicitly labels this implementation-checked and leaves promoted vectors/formal assurance to the epic's closing conformance feature.
- No resource snapshots, revisions, manifests, projection schemas, report ingress, or cockpit rendering entered this checkpoint.

## Verification

- `cargo test --workspace` — all Rust core/server/unit/integration/doc tests passed, including 4 new resource-acceptance tests and existing runtime-session/diagnostics/audit regressions.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cd cli && npm test` — 37 passed.
- `cd contracts/ts && npm run build && npm run check:drift` — passed.
- `node contracts/scripts/check-vectors.mjs`, `check-models.mjs`, and `check-presentation.mjs` — passed; no resource evidence was falsely promoted.
