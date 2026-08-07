---
id: epic-token-commune-observer-adapter-foundation-attachment-lifecycle
kind: story
stage: done
tags: [adapter, protocol, integration]
parent: epic-token-commune-observer-adapter-foundation
depends_on: [epic-token-commune-observer-adapter-foundation-gateway-client]
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-07
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

## Implementation notes

- Added the narrowed generated Connect client with evidence/token interception,
  exact manifest attachment, token-required success, same-generation
  single-flight reattachment, failed-token fencing, one authenticated retry,
  generic observation ingestion, and diagnostic-reporting bypass.
- Added the process composition root with attach-before-started ordering, one
  abort-owned run, signal cleanup in the environment bootstrap, and idempotent
  diagnostic flush/close. The gateway port is composed but never invoked or
  scheduled by this feature.
- No Pi session, transcript, runtime report, delivery translator, running, or
  successful-result code was carried into the package.
- Verification: integrated `npm test` passed with exact attachment evidence,
  identity/generation/manifest assertions, missing-token rejection, concurrent
  auth-refresh single flight, diagnostics bypass, start ordering, abort, and
  repeated-disposal coverage.
