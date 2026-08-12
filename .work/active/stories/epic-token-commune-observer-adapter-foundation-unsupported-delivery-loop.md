---
id: epic-token-commune-observer-adapter-foundation-unsupported-delivery-loop
kind: story
stage: done
tags: [adapter, protocol, integration]
parent: epic-token-commune-observer-adapter-foundation
depends_on: [epic-token-commune-observer-adapter-foundation-attachment-lifecycle]
release_binding: v0.2.0
gate_origin: null
created: 2026-08-05
updated: 2026-08-07
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
Operation, observes acknowledgement plus canonical unsupported failure terminalization, scans all
visible payload/diagnostic material for both secrets, and shuts down cleanly.
This is implementation evidence only; promoted conformance belongs to the
later conformance feature.

## Acceptance evidence

- Idle subscription remains pending until abort; a finite tail reconnects and is
  never accepted as liveness.
- Unsupported delivery is acknowledged then failed once with
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

## Implementation notes

- The delivery loop holds one subscription open, treats finite completion as
  unavailable, reconnects only the established retryable Connect failures,
  acknowledges before advancing its LSN cursor, and emits exactly one
  `UNSUPPORTED_COMMAND` result without running, translating, invoking the
  gateway, or claiming success.
- Added serial real-core evidence for the durable registration, idle stream,
  accepted adapter delivery, delivered→unsupported terminalization, secret
  absence, and clean abort/disposal. The current core does not resolve an
  adapter-scope target through ordinary `Submit`, so the test seeds one
  already-accepted durable adapter Operation using the same fixture technique
  as the Pi adapter's stale-delivery test; it does not fabricate a resource
  report before snapshot mapping exists.
- Historical implementation discovery (superseded by aggregate-review
  remediation): core ingestion originally terminalized this path as
  `FAILED + UNSUPPORTED_COMMAND`. The current real-core outcome is
  `REJECTED + UNSUPPORTED_COMMAND`, matching the canonical delivery-refusal
  semantics.
- Verification: integrated `npm test` passed, including acknowledge-before-
  unsupported ordering, finite-tail retry, abort/dispose behavior, exact real
  manifest assertions, core terminal failure code, and secret scans.
