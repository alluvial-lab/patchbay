---
id: feature-pi-parity-checklist
kind: feature
stage: drafting
tags: [adapter, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-session-identity-adapter-contract, feature-operator-presence-and-action-inventory]
created: 2026-06-28
updated: 2026-07-06
gate_origin: null
release_binding: null
---

# Feature: Define Pi migration and parity checklist

Pi is the first adapter because it lets the operator migrate from current Remote Pi-style workflows, but parity is not yet itemized. Define the migration floor without making Pi the core ontology.

## Scope

- Current Remote Pi workflow inventory.
- Required Pi adapter capabilities for v0.
- Session discovery, send prompt, stream/read replies, reconnect recovery, working/idle/stale/offline status.
- Commands such as cancel, compact, new/resume only as adapter-declared capabilities.
- Unsupported or deferred Remote Pi features.
- Mapping from Pi session metadata to Patchbay session identity.

## Acceptance criteria

- Add a Pi parity checklist to `docs/SPEC.md`, `docs/ARCHITECTURE.md`, or a dedicated adapter doc.
- The checklist is sufficient to decide when the operator can switch workflows.
- Pi-specific operations are represented as adapter capabilities, not core protocol states.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.

## Misroute note (2026-07-06)

Misrouted to prose-author; the work has a real design surface — retagged for feature-design. The `prose` tag was removed.

The prose-author black-box test was applied and initially passed (the `OperationKind` registry, adapter capability manifest shape, session identity tuple, and Pi action surface are all settled in `done` dependencies). But on second application the test fails: the scope line "Mapping from Pi session metadata to Patchbay session identity" contains a genuine semantic classification that cannot be made silently through prose, with real verification consequences.

The load-bearing design questions a `feature-design` pass must resolve:

1. **`session_new` classification.** Pi's `session_new` resets the attached session's conversation via `ctx.newSession()` *without spawning a new process* (grounded in `.research/attestation/pi-extension.md`). Is that a session *replacement* that bumps `session_generation` and tombstones the prior generation (triggering `GenerationMonotonic` and `LateGenerationInert`), or a same-generation `session-management` clear? The choice has durable correlation and audit consequences and cannot be made inside a prose checklist.
2. **Pi snapshot tier.** The adapter must declare a snapshot tier (`authoritative` / `partial` / `none` per `docs/PROTOCOL.md`). Pi's `session_sync` evidence suggests at least `partial`, but the tier drives the core's reconnect reconciliation contract; the checklist must not pin it in a foundation doc — the design pass must decide how the Pi adapter declares and the checklist records it.
3. **Provisioning seam.** `pi-supervisord` is out-of-band sysadmin (not an operator Operation in v0). The design must classify this as reserved/adapter-external without foreclosing a future supervisor OperationKind, and say so explicitly rather than as a prose aside.
4. **Pairing/queue messages.** `pair_request`, `queued_message_set`, `queued_message_clear` are transport/pairing, not agent-control Operations. The design must decide whether they map to the reserved Subscription/transport seam or are purely out-of-adapter-scope.

These are choosing-between-approaches / semantic-model-pinning commitments, not collapsed prose authoring. This is the same misroute pattern that hit `feature-session-identity-adapter-contract` (retagged `[prose]` → design on 2026-06-28) and that prompted the project-wide 2026-07-06 codification of the prose black-box test. Route to `feature-design`; do not advance the stage.
