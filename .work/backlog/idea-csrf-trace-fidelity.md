---
id: idea-csrf-trace-fidelity
kind: backlog
created: 2026-07-01
updated: 2026-07-01
tags: [verification, security, foundation]
research_refs: []
---

# Backlog: CSRF/authority invariants should verify against attempted-evidence state, not recorded trace

Filed from the deep re-review of `feature-formal-model-seed`. The CSRF invariants (`csrf_browser.qnt`) check `accepted.implies(lastProof == csrfProofs.get(lastSession))`, where `lastSession`/`lastProof` are recorded by the action. This catches a broken acceptance *predicate* (the B2 fix), but not a broken acceptance *recording* — an action that lies about what was submitted (sets `lastProof` to the bound proof regardless of input) would fool the invariant.

The general pattern for safety-claiming models that verify server-side acceptance: the invariant should check against the **attempted/request evidence** (the raw submitted values, captured as distinct state), not the action's recorded trace variables. This is the same "independent oracle" discipline as the B1 fix, extended to the evidence-recording layer.

Applies to:
- `csrf_browser.qnt` CSRF invariants (the immediate case — also filed as B2-trace in `story-fix-alloy-relational-assertions`).
- `authority.qnt` `CompoundIssuer` (when promoted) — the transport-endpoint-vs-operator-actor verification should likewise check attempted evidence, not recorded acceptance.

Not blocking the seed feature's advancement once the B5/B6 blockers and B2-trace/B4-overclaim are fixed, but the pattern should be applied to any future server-side-acceptance model.
