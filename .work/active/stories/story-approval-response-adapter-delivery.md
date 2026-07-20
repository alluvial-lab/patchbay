---
id: story-approval-response-adapter-delivery
kind: story
stage: implementing
tags: [protocol, verification, foundation]
parent: feature-v0-approval-response-contract
depends_on: [story-approval-response-proto-message, story-approval-response-core-validation]
release_binding: null
gate_origin: null
created: 2026-07-19
updated: 2026-07-20
---

# Story: Pi-adapter delivery of the approval decision

Checkpoint for `feature-v0-approval-response-contract` Unit 3. Wires the
adapter to consume the typed decision.

## Deliverable

Wire the `APPROVAL_RESPONSE` arm in `DeliveryTranslator.deliver`
(`pi-adapter/src/delivery.ts`), currently `unsupported_command`. On delivery,
decode the `ApprovalResponsePayload` decision and resolve the pending approval
via the `ApprovalHandler` in `pi_session.ts`:

```typescript
case OperationKind.APPROVAL_RESPONSE: {
  const payload = decodeApprovalPayload(operation);  // content_type PROTOBUF + decode
  if (payload.decision === ApprovalDecision.APPROVED) {
    await session.resolveApproval(operation, /*approved*/ true);
  } else if (payload.decision === ApprovalDecision.DENIED) {
    await session.resolveApproval(operation, /*approved*/ false);
  } else {
    throw new UnsupportedCommandError(`approval decision ${payload.decision} not deliverable in v0.1.0`);
  }
  return {};
}
```

`ELICITATION_RESPONSE` (question side) stays `unsupported_command` — the
question-side producer is a separate follow-on.

The `ApprovalHandler` (today auto-approves via `() => true`) gains a real
resolution path: when an approval Elicitation is pending and a response arrives,
the handler returns the operator's decision (DENIED → the tool call is blocked
with "denied by operator"). The auto-approve default stays as a fallback for
tool calls arriving with no open approval gate (unchanged).

## Acceptance evidence

- [ ] `DeliveryTranslator.deliver` handles `APPROVAL_RESPONSE` (no longer
      `unsupported_command`): decodes the decision, resolves the approval.
- [ ] `ELICITATION_RESPONSE` stays `unsupported_command` (question-side producer
      deferred).
- [ ] DENIED resolution blocks the pending tool call; APPROVED allows it.
- [ ] Reserved decisions (100-103) reject as `unsupported_command` at delivery.
- [ ] `pi-adapter` builds + its tests pass (`npm run build && npm test`).

## Notes

- The adapter does NOT open approval Elicitations today (no `OpenElicitation`
  RPC in `adapter_control.proto`). This unit wires only *response delivery*.
  The producer side (adapter opening an approval Elicitation when a tool call
  needs a gate) is a separate follow-on — the cockpit tests against vectors +
  a fake transport, same as the question side.
- The response Operation's terminal state is `Completed` either way (it
  delivered a valid decision); the *tool* is blocked on DENIED, not the
  response. The Elicitation terminal is what differs (`Answered` vs `Declined`).
