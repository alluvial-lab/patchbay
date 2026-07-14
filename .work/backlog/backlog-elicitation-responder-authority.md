---
id: backlog-elicitation-responder-authority
kind: feature
stage: backlog
tags: [security, protocol, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Backlog: Elicitation responder authority enforcement

## Source
Authority design review #2 (finding G) + review #3 (finding R6). `feature-v0-core-authority` revision 3 documents `ElicitationResponderAuthority` (authority.qnt property 8) as an untested gap.

## Finding
The stated-normative property `ElicitationResponderAuthority` requires that response Operations (`approval-response`, `elicitation-response`) are accepted only when the verified issuer maps to the `Elicitation.expected_responder_actor`. But neither `GrantCheck` nor acceptance receives the referenced Elicitation or its expected responder; the elicitation projection (`core/src/acceptance/elicitation.rs`) checks response kind + correlation only, not responder-actor matching. Authority does not own response-Operation responder validation — it's an acceptance/elicitation concern.

Revision 3 honestly documents this as a gap rather than shipping a vacuous stand-in test. The obligation is real; it's owned by a future acceptance/elicitation responder-validation feature.

## Direction
Add an Elicitation lookup/validation port to response-Operation acceptance: when a response Operation is submitted, look up the correlated Elicitation, require the verified issuer actor to equal `Elicitation.expected_responder_actor`, and reject (deny-by-default) on mismatch. Test the mismatch. This is an acceptance-feature change (acceptance owns the response-Operation path).

## Priority
Not blocking for v0.1.0 authority. Becomes important when Elicitation response surfaces are exercised (the web cockpit answering approval/question prompts). Couples with the ingress features (verified issuer).
