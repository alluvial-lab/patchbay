---
id: elicitation-responder-validation
kind: feature
stage: drafting
tags: [security, protocol]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-08-09
updated: 2026-08-09
---

# Elicitation responder authority validation

## Brief
Close the responder-authority gap split out of `authority-provenance-hardening`. Absorbs:

- `backlog-elicitation-responder-authority` — **OPEN** (highest-risk silent-check-drop in the set): response Operations (`approval-response`, `elicitation-response`) must be accepted only when the verified issuer maps to `Elicitation.expected_responder_actor`; but the projection retains `expected_responder_actor` (`elicitation.rs:50-56`) while the `ActiveElicitation` port omits it (`ports.rs:110-119`), so `validate_response_payload` cannot compare (`pipeline.rs:247-263`). *Src:* authority review #2(G)+#3(R6).

## Direction
Add an Elicitation lookup/validation port to response-Operation acceptance: on a response Operation, look up the correlated Elicitation, require the verified issuer actor to equal `expected_responder_actor`, deny-by-default on mismatch. This is an acceptance-owned change (acceptance owns the response-Operation path). **Keep it a distinct fail-fast acceptance/Elicitation check** — do NOT fold it into a shared grant primitive (a valid grant is not authority to answer an Elicitation intended for another actor; the review's explicit warning).

## Foundation references
- `docs/PROTOCOL.md` — Elicitation responder matching (`:329-332`); response-Operation acceptance
- `docs/VERIFICATION.md` — `ElicitationResponderAuthority` (stated-normative, untested)
- Code: `core/src/acceptance/elicitation.rs`, `core/src/acceptance/ports.rs`, `core/src/acceptance/pipeline.rs`
