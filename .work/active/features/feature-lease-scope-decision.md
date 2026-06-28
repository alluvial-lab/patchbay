---
id: feature-lease-scope-decision
kind: feature
stage: drafting
tags: [prose, protocol, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-security-threat-model]
---

# Feature: Decide lease scope for v0

Leases appear as core concepts in the current docs, but review questioned whether they are premature without a first concrete use case or fencing model.

## Scope

- Decide whether leases are included in v0 or explicitly deferred.
- If included, name the first lease use case.
- Define authority domain, lessor authority, lease epochs/fencing tokens, partition behavior, and adapter obligations.
- If deferred, revise docs so leases are future coordination concepts rather than v0 implementation obligations.

## Acceptance criteria

- `docs/SPEC.md` states whether leases are v0 or post-v0.
- `docs/PROTOCOL.md` no longer presents underspecified lease safety as an immediate guarantee.
- `docs/GLOSSARY.md` defines `authority domain` if the term remains.
- `docs/VERIFICATION.md` models only lease properties that are in scope.

## Related parked ideas

- `idea-multi-human-coordination` — v0 remains single-operator unless this feature decides otherwise, but the foundation should not foreclose future multi-human authority domains, grants, audit, handoffs, or third-party coordination surfaces.
