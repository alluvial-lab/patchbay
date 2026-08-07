---
id: epic-token-commune-observer-adapter-foundation-unsupported-delivery-loop
kind: story
stage: implementing
tags: [adapter, protocol, integration]
parent: epic-token-commune-observer-adapter-foundation
depends_on: [epic-token-commune-observer-adapter-foundation-attachment-lifecycle]
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-05
---

# Keep delivery liveness open and reject all Operations honestly

## Design checkpoint

Complete `AdapterProcess.run` with one long-lived authenticated
`ReceiveDeliveries` subscription. Preserve the last acknowledged delivery LSN,
treat finite clean completion as `Unavailable`, and reconnect only retryable
Connect transport failures. For every unexpected delivery: require the
Operation, record only safe ids/enums, durably acknowledge it, advance the cursor
only after acknowledgement succeeds, then ingest an `UNSUPPORTED_COMMAND`
outcome. Never report `running`, invoke the gateway, translate payloads, or claim
successful execution.

Add one bounded serial real-core E2E test that reads back the durable manifest,
keeps an idle stream open, delivers one adapter-targeted committed `query`
Operation, observes acknowledgement plus canonical unsupported rejection, scans all
visible payload/diagnostic material for both secrets, and shuts down cleanly.
This is implementation evidence only; promoted conformance belongs to the
later conformance feature.

## Acceptance evidence

- Idle subscription remains pending until abort; a finite tail reconnects and is
  never accepted as liveness.
- Unsupported delivery is acknowledged then rejected once with
  `FailureCode.UNSUPPORTED_COMMAND`, without a `running` state or gateway call.
- Cursor/reattach tests prove acknowledged history is not re-acknowledged after
  stream replacement or token refresh.
- Real-core registration contains exactly two PARTIAL resource capabilities,
  four JSON descriptors, no runtime-session category/tier, no OperationKinds,
  and an empty attachment descriptor.
- Gateway key and attachment evidence are absent from diagnostics, durable
  registration/Observations, error strings, and test output.
- SIGINT/SIGTERM-equivalent abort and repeated disposal leave no active RPC,
  timer, file handle, or diagnostic drain.

## Ordering constraint

Final checkpoint in this feature. Do not implement snapshot mapping, polling,
resource reports, cockpit UI, mutation translation, or promoted conformance
vectors.
