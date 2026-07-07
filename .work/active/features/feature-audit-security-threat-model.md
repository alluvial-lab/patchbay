---
id: feature-audit-security-threat-model
kind: feature
stage: drafting
tags: [security, foundation]
parent: epic-retroactive-design-gate-audit
depends_on: [feature-security-threat-model]
created: 2026-07-07
updated: 2026-07-07
gate_origin: null
release_binding: null
---

# Feature: Retroactive design-gate audit — v0 security, principal, and threat model

## Brief

`feature-security-threat-model` slipped through to `done` tagged `[prose]`, structurally skipping the design gate. Its scope defines the v0 threat model, the principal model (operator/device/browser-session/endpoint/adapter/actor), enrollment and revocation posture, grant shape and authorization algorithm, replay protection and command-issuer binding, emergency revocation and audit, and forbidden v0 deployments (e.g. internet-exposed unauthenticated core). These are security-architecture choices with high reversal cost and high propagation risk — exactly the class where a skipped alternatives evaluation is most dangerous.

3 downstream dependents (`feature-verification-contract-authority`, `feature-design-grant-shape`, `feature-lease-scope-decision`).

## What to read

- The target: `.work/active/features/feature-security-threat-model.md` (read FULLY — "Scope," "Implementation notes" recording review fixes: "endpoint/device enrollment posture, login throttling/authenticator setup, AGENTS orientation entry, audit-record terminology, device/principal glossary terms, actor/endpoint alignment, `Secure` cookie requirement wording, adapter-core trust-boundary language, deployment HTTPS clarification, and delegation parent-grant seam").
- The docs it produced: `docs/SECURITY.md` (v0 authority domain, principal model, grant posture, browser security, adapter trust boundary, audit, reserved seams), `docs/PROTOCOL.md` (grants/revocation terminology), `docs/VERIFICATION.md` (authority safety variables), `docs/GLOSSARY.md`.
- The checked models: `specs/seed/authority.qnt`, `specs/seed/csrf_browser.qnt`, `specs/seed/patchbay-relational.als` (verify their properties match the security posture).
- The 3 downstream dependents listed above.
- Foundation context: `docs/SECURITY.md`, `docs/PROTOCOL.md`, `docs/VERIFICATION.md`, `AGENTS.md`, `.agents/rules/`.
- The research that grounded it: `.research/` for `feature-research-web-control-security` (the security research engagement).

## Scope

1. **Alternatives evaluation** for each load-bearing security decision:
   - V0 threat model and explicit out-of-scope adversaries (which adversaries were considered and rejected vs omitted).
   - Principal model: operator/device/browser-session/endpoint/adapter/actor (vs flatter / vs richer-from-start).
   - Device/control-surface enrollment and revocation posture (the choices made — alternatives for enrollment ceremony, revocation speed, revocation propagation).
   - Grant shape and authorization algorithm (cross-ref `feature-design-grant-shape` which re-opened this — verify the audit doesn't duplicate; cover what that feature did *not*).
   - Replay protection and command-issuer binding (the compound-issuer model: operator + transport endpoint — alternatives?).
   - Emergency revocation and audit events.
   - Forbidden v0 deployments (internet-exposed unauthenticated core) — was this a conscious rejection with rationale?
   - The review-fix items (login throttling, authenticator setup, `Secure` cookie, adapter-core trust boundary, HTTPS clarification, delegation parent-grant seam) — each was a design decision made in-review; each likely has no alternatives record.
2. **Faulty-assumption hunt.** Re-derive each from current first principles. Flag any accident-of-prose. Pay special attention to: the adapter-core trust boundary (Patchbay controls remote/headless agents and potentially shell/job adapters — was the trust posture against a malicious/correct adapter actually pinned, or assumed?); whether the compound-issuer binding has an edge case the prose lane left open; whether the single-operator assumption baked in a security posture multi-human would have to reverse (not just defer).
3. **Propagation check** across the 3 dependents. Did `feature-verification-contract-authority` assume a security-posture property the skipped gate would have surfaced? Did `feature-design-grant-shape` already resolve the grant-shape open question, or is there residual debt? Did `feature-lease-scope-decision` (just done) inherit a security assumption about lessor authority?
4. **Verdict.** `holds` / `holds-with-caveats` / `faulty-assumption-found`.

## Acceptance criteria

- [ ] Every load-bearing security decision has a recorded alternatives evaluation.
- [ ] Each review-fix item (login throttling, authenticator, `Secure` cookie, trust boundary, HTTPS, delegation seam) has an alternatives record (likely missing — they were in-review fixes).
- [ ] The adapter-core trust posture (malicious vs correct adapter) is explicitly classified.
- [ ] Propagation check across the 3 dependents recorded.
- [ ] Verdict recorded; any `faulty-assumption-found` produced a filed corrective item with re-opening `depends_on`.

## Notes

Routes through `feature-design`. No pre-mortem per operator direction. Coordinate with `feature-design-grant-shape` (done) on grant shape — that feature re-opened and resolved the grant field list; don't re-decide, cover only what it did not. Security findings have high reversal cost — a `faulty-assumption-found` verdict here is the highest-priority corrective of the four audits.
