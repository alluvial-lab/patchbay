---
id: feature-design-grant-shape
kind: feature
stage: drafting
tags: [security, protocol, verification]
parent: epic-foundation-hardening
depends_on: [feature-security-threat-model]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Design: v0 grant shape and delegation seam

The concrete grant field list is currently committed v0 behavior in `docs/PROTOCOL.md` (authority grants) and `docs/SECURITY.md` (grant shape), including a `parent grant id / delegated-by` seam. It was decided inside prose features (`feature-security-threat-model` plus review fixes) without a design pass. This feature reopens it as a deliberate design decision.

## What is under design review

The grant record fields currently committed:

- grant id;
- authority domain id;
- subject actor id;
- optional subject device id;
- optional subject endpoint id or endpoint class;
- target scope;
- allowed command kinds or adapter capability set;
- creation time and provenance;
- optional expiration;
- revocation generation or revoked time;
- revocation policy for already accepted commands;
- optional parent grant id / delegated-by field reserved for future delegation.

## Alternatives to evaluate

- **Minimal v0 grant** — drop device/endpoint class, drop the delegation seam; model the operator's authority as a single implicit grant, add fields only when a concrete need arrives.
- **Endpoint-scoped grants** — keep device/endpoint fields but drop the delegation seam.
- **Capability-set grants** — model the grant as a capability set rather than command-kind list, for cleaner adapter-capability alignment.
- **Delegation-in-v0** — keep the parent-grant field and actually define delegation semantics, not just reserve the seam.
- **Status quo** — keep all committed fields as-is.

## Design questions to resolve

- Is delegation a v0 concern at all, or should the `parent grant id` seam be removed entirely until a multi-operator or delegated-authority need arrives? (The field was added during a review-fix pass with no design discussion.)
- Do device and endpoint both need to be grant subjects in v0's single-operator model, or is endpoint sufficient?
- Should grants reference adapter capability sets directly, or stay command-kind-oriented?
- What does the authority-safety formal model (`docs/VERIFICATION.md` authority safety) actually require the grant to carry? Work backward from the model obligations, not forward from a guessed field list.
- How does the grant shape interact with the web↔core protocol seam (the web server is itself a principal with a grant to the core)?

## Relationship to committed docs

Grant shape is committed in `docs/PROTOCOL.md` (authority grants), `docs/SECURITY.md` (grant shape, revocation), and `docs/VERIFICATION.md` (authority safety variables). A design pass ratifies or revises the fields; docs roll forward accordingly. The committed shape stays as provisional v0 behavior until the design pass concludes.

## Acceptance criteria

- Grant shape is a deliberate design choice, not a prose artifact.
- The delegation question (in-v0 vs. reserved-seam vs. removed) is explicitly resolved.
- Fields are justified by the authority-safety model obligations or removed.
- `docs/VERIFICATION.md` authority-safety variables align with the chosen shape.
- The web-server-as-principal interaction is addressed (cross-reference `feature-web-core-protocol-seam`).
