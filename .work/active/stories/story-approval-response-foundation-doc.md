---
id: story-approval-response-foundation-doc
kind: story
stage: implementing
tags: [protocol, verification, foundation]
parent: feature-v0-approval-response-contract
depends_on: [story-approval-response-core-validation]
release_binding: null
gate_origin: null
created: 2026-07-19
updated: 2026-07-20
---

# Story: Foundation-doc roll-forward (PROTOCOL.md)

Checkpoint for `feature-v0-approval-response-contract` Unit 5. Resolves the
internal tension in the committed spec + records the surface-reject reserved
seam.

## Deliverable

Roll `docs/PROTOCOL.md` forward in place (rolling-foundation — no "previously"
prose):

1. **Line 277** (`declined` def): reconcile "without satisfying it." A DENIED
   approval *did* satisfy the slot (a valid decision was delivered). Reword so
   `declined` reads as "the operator answered with a declining decision" (a
   satisfied slot, negative valence), consistent with line 310 ("`answered`
   does not imply the underlying tool/action succeeded; it only means the
   response slot was satisfied").
2. **Add the disambiguation** between operator `Declined` (ElicitationState —
   an answer) and machine `Rejected` (CommandState — the system refusing a
   command, PROTOCOL:110). Make explicit that `Rejected` never terminalizes an
   Elicitation.
3. **Line 156** (approval-response row): confirm "Completion updates the
   Elicitation terminal (`answered` or `declined`)" reads correctly under the
   decision-driven resolution (Completion = `Completed`; the decision is
   decoded from the typed `ApprovalResponsePayload` and picks the terminal).
4. **Reserved seam** (line 312 area): add "surface-reject (operator surface
   signals it cannot handle an elicitation)" to the reserved-seams list, noting
   it is distinct from operator approve/decline and from machine rejection, and
   that v0.1.0 leaves an unrenderable elicitation `pending` until timeout/withdraw.

## Acceptance evidence

- [ ] PROTOCOL:277 no longer contradicts PROTOCOL:310 for DENIED approvals.
- [ ] The operator-`Declined` vs machine-`Rejected` disambiguation is explicit.
- [ ] PROTOCOL:156 reflects the decision-driven resolution (decision decoded
      from `ApprovalResponsePayload`).
- [ ] The surface-reject reserved seam is named in the reserved-seams list.

## Notes

- Rolling-foundation: edit in place. No "previously"/"originally" prose. Git is
  the audit trail.
- This story depends on Unit 2 (core-validation) because the doc must reflect
  the settled decision-driven semantics, not precede them.
