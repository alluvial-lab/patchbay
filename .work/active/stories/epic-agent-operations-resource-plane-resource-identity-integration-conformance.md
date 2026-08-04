---
id: epic-agent-operations-resource-plane-resource-identity-integration-conformance
kind: story
stage: implementing
tags: [foundation, protocol, security, testing]
parent: epic-agent-operations-resource-plane-resource-identity
depends_on: [epic-agent-operations-resource-plane-resource-identity-polymorphic-target-resolution, epic-agent-operations-resource-plane-resource-identity-resource-authority-containment]
release_binding: null
gate_origin: null
created: 2026-08-03
updated: 2026-08-03
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
