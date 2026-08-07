---
id: epic-token-commune-observer-adapter-foundation-attachment-lifecycle
kind: story
stage: implementing
tags: [adapter, protocol, integration]
parent: epic-token-commune-observer-adapter-foundation
depends_on: [epic-token-commune-observer-adapter-foundation-gateway-client]
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-05
---

# Attach the adapter and compose its long-lived process

## Design checkpoint

Implement the narrowed `PatchbayCoreClient` and `AdapterProcess` composition
root. Reuse the Pi architecture for ConnectRPC, evidence/token headers, generated
Attach request, token-required success, same-generation single-flight reattach,
one retry after `Unauthenticated`, diagnostics bypass, abort/signal ownership,
and idempotent disposal. Register `tokenCommuneCapabilityManifest()` exactly.

The core client intentionally has no SessionReport, transcript, Pi session,
DeliveryTranslator, running, or successful-result API. The process composes the
validated config, opaque gateway credential/client, local+forwarded diagnostics,
and core client, but starts no poll scheduler in this feature.

## Acceptance evidence

- Attach tests prove exact adapter id, endpoint id, authority domain, adapter
  generation, configured-local attachment evidence, manifest, and empty
  attachment descriptor.
- Attachment is accepted only when the core issues both accepted result and an
  attachment token.
- Concurrent auth failures cause one reattach; a newer token fences a stale
  failed-token caller, and all retry uses the same adapter generation.
- Diagnostic reports bypass auth refresh and remain best-effort/non-recursive.
- Start/abort/dispose tests prove attach-before-started ordering, one active run,
  idempotent disposal, and no leaked RPC/file/drain work.

## Ordering constraint

Depends on the complete gateway client so the composition root is final-shaped.
The next checkpoint adds the held-open delivery behavior; do not add polling,
resource mapping, or successful Operation execution here.
