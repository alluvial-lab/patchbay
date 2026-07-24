---
id: feature-design-grant-shape
kind: feature
stage: done
tags: [security, protocol, verification]
parent: epic-foundation-hardening
depends_on: [feature-security-threat-model]
created: 2026-06-28
updated: 2026-06-29
gate_origin: null
release_binding: v0.1.0
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

## Design decisions

- **Delegation**: Remove the `parent_grant_id` / delegated-by field from the v0 grant record. Delegation remains a reserved future direction in prose only, not a live field with vacuous semantics. The verification property "delegation cannot create authority beyond its parent grant" moves out of required v0 authority-safety obligations into a precondition that must be satisfied before any delegation-backed behavior ships.
- **Grant subject**: `subject actor id` + optional `subject endpoint id` or `endpoint class`. Device stays in the identity model as an audit and revocation dimension (it is a normative verification variable and supports device-scoped revocation), but it is not a grant-matching field. This makes the grant record consistent with `docs/VERIFICATION.md`'s grant-matching list (issuer actor, optional endpoint, target scope, command kind, expiration, revocation generation).
- **Allowed actions**: Grants authorize canonical Patchbay command kinds (stable, registry-owned). The core validates the command kind is known to Patchbay at submission (Fail Fast boundary check). The core does not gate on adapter capabilities; it delivers to the adapter and the adapter accepts or rejects. Adapter capability declarations exist for UX display only (rendering unavailable actions), not as an authority or delivery gate. `unsupported_command` from an adapter is a delivery-layer, adapter-reported rejection; an unknown-to-Patchbay command kind is `validation_failed` at submission.
- **Web server as principal**: Compound issuer tuple. The grant subject is the operator actor (+ optional endpoint); the command-issuer audit tuple includes the web-server endpoint as the verified transport principal. The core independently verifies both "this came through the web surface" and "this operator authorized it". The exact wire/evidence shape for how operator-session evidence crosses the web↔core seam is deferred to `feature-web-core-protocol-seam`; this feature commits only to the requirement that the core must not trust a self-asserted operator identity.

## Architectural choice

Ratify a tightened v0 grant shape against the four design decisions above.

The rejected alternatives were:

1. **Status quo (all committed fields, including delegation seam)** — rejected because the `parent_grant_id` field carries no v0 semantics and the "delegation cannot exceed parent" model property is vacuous without delegation. A reserved-seam field that looks like a commitment misleads implementers and creates dead schema. The delegation direction is real but belongs to future multi-operator / federated-authority work, which `docs/SPEC.md` explicitly excludes from v0.
2. **Capability-set grants** — rejected because adapter capabilities are mutable, adapter-owned runtime declarations. Storing an adapter's capability set as a grant's authority would let authority silently widen or narrow when the adapter changes its declared capabilities, violating deny-by-default and operator-chosen authority. It also couples the authority layer to adapter-specific vocabulary, hurting adapter-neutrality.
3. **Web server as sole issuer/principal** — rejected because the core could not distinguish operators (relevant to audit and to future multi-operator), and the web server would be self-asserting the operator identity, contradicting the rule that sender identity is derived from verified context rather than self-asserted payload fields.
4. **Device as a grant-matching field** — rejected because device is too coarse as an authority boundary (a device may host multiple legitimate operator endpoints) and redundant with endpoint in single-operator v0. Device remains valuable as an audit and revocation grouping, which is preserved in the identity model.

The chosen shape aligns the grant record with the authority-safety model obligations, removes a vacuous field, and keeps adapter capability out of the authority layer while preserving its role in UX display.

## Implementation Units

### Unit 1: Revise the v0 grant record in protocol prose

**File**: `docs/PROTOCOL.md` (Authority grants section)

```text
GrantRecord {
  grant_id
  authority_domain_id
  subject_actor_id
  optional subject_endpoint_id | subject_endpoint_class
  target_scope
  allowed_command_kinds   // canonical Patchbay command kinds
  created_time, provenance
  optional expiration
  revocation_generation | revoked_time
  revocation_policy_for_accepted_commands
}
// parent_grant_id / delegated-by removed from v0
// subject_device_id removed from the grant record; device remains in identity model
```

**Implementation Notes**:
- Remove the "Under design review" note from the Authority grants section once this design is implemented.
- Drop the `parent grant id / delegated-by field reserved for future delegation` line from the field list. Add a one-line note that delegation is a reserved future direction, not a v0 field.
- Drop the `optional subject device id` line from the grant record field list. Device remains a normative identity variable; clarify that device is used for audit and revocation grouping, not grant matching.
- Change "allowed command kinds or adapter capability set" to "allowed command kinds" and clarify that adapter capabilities are not part of grant authority (they are UX-display declarations; the adapter is the authority on its own support at delivery time).
- Preserve the existing deny-by-default, grant-checks-before-acceptance, and revocation-policy wording.

**Acceptance Criteria**:
- [ ] `docs/PROTOCOL.md` grant record lists the tightened v0 fields and no `parent_grant_id`.
- [ ] Prose states delegation is a reserved future direction, not a v0 field.
- [ ] Prose states adapter capabilities are not grant authority and explains the display-vs-delivery distinction.
- [ ] The under-design-review marker is removed for this decision only.

---

### Unit 2: Align the security grant shape and issuer model

**File**: `docs/SECURITY.md` (Grant shape and Command authorization sections)

```text
IssuerEvidence {
  operator_actor_id        // grant subject
  operator_session_evidence // verified by the core, not self-asserted
  transport_endpoint_id     // the web-server endpoint, verified as a principal
}
```

**Implementation Notes**:
- Remove the "Under design review" equivalent wording for grant shape (the grant-shape marker lives in PROTOCOL.md; ensure SECURITY.md's grant shape section matches the tightened fields).
- Update the Grant shape field list to drop `parent_grant_id` and `subject_device_id`, matching PROTOCOL.md.
- In Command authorization and replay resistance, clarify the compound issuer: the core authorizes the operator actor against the grant; the command-issuer audit tuple includes the web-server (or CLI) endpoint as the verified transport principal. The core must not trust a self-asserted operator identity.
- Add a cross-reference to `feature-web-core-protocol-seam` noting that the exact wire/evidence shape for operator-session evidence crossing the web↔core seam is deferred to that feature.
- Preserve revocation model wording except where it references the removed device-matching field (device revocation still works via endpoint/device identity; it is not a grant-matching field).

**Acceptance Criteria**:
- [ ] `docs/SECURITY.md` grant shape matches the tightened PROTOCOL.md fields.
- [ ] Command authorization describes the compound issuer and the no-self-asserted-identity rule.
- [ ] Cross-reference to `feature-web-core-protocol-seam` is present.

---

### Unit 3: Align verification authority-safety obligations

**File**: `docs/VERIFICATION.md` (Authority safety section)

```text
GrantMatching checks:
  issuer_actor, optional_endpoint, target_scope,
  command_kind, expiration, revocation_generation

// removed from required v0 obligations:
//   Delegation cannot create authority beyond its parent grant
//   -> moved to "precondition before delegation-backed behavior ships"
```

**Implementation Notes**:
- Keep grant matching as issuer actor + optional endpoint + target scope + command kind + expiration + revocation generation (already correct in the doc). Confirm it does not require device matching.
- Move the "Delegation cannot create authority beyond its parent grant" property out of the required authority-safety list into a clearly labeled precondition for future delegation-backed behavior.
- Keep the normative variable list (`Actor`, `Device`, `Endpoint`, `OperatorSession`, `Grant`, `GrantScope`, `CommandKind`, `Target`, `TargetGeneration`, `RevocationGeneration`, `CommandIssuer`, `AuthorityDomain`). `Device` remains because it is an identity/audit/revocation variable even though it is not a grant-matching field.
- Add a property stating the core must verify transport-endpoint identity independently of operator identity (compound issuer), so the core does not trust a self-asserted operator.

**Acceptance Criteria**:
- [ ] `docs/VERIFICATION.md` grant-matching list excludes device.
- [ ] Delegation property is moved to a delegation precondition, not a required v0 obligation.
- [ ] A compound-issuer verification property is present.
- [ ] Normative variable list still includes `Device` with an audit/revocation role.

---

### Unit 4: Record the display-vs-delivery capability distinction

**File**: `docs/PROTOCOL.md` (Adapter capabilities section) and/or `docs/SECURITY.md`

**Implementation Notes**:
- Clarify that adapter capability declarations are advisory for control-surface UX (rendering unavailable actions) and are not an authority or delivery gate.
- Clarify that the core delivers a command kind to the adapter and the adapter accepts or rejects based on its own support at delivery time; `unsupported_command` is an adapter-reported delivery-layer rejection.
- No change to the failure vocabulary itself; the existing `unsupported_command` and `validation_failed` terms already cover the two layers.

**Acceptance Criteria**:
- [ ] Prose states adapter capability declarations are UX-display-only, not a delivery or authority gate.
- [ ] Prose states the adapter is the authority on its own support, reported at delivery time.

## Implementation Order

1. Update `docs/PROTOCOL.md` (grant record + delegation note + adapter-capabilities distinction).
2. Update `docs/SECURITY.md` (grant shape + compound issuer + web-core seam cross-reference).
3. Update `docs/VERIFICATION.md` (grant matching, move delegation property, add compound-issuer property).

No child stories are spawned. This is a single-stride documentation/verification design with tight cohesion across three foundation docs; stories would add overhead rather than useful parallelism.

## Testing

There is no implementation code yet. Verification for this design is by document consistency:

- confirm `docs/PROTOCOL.md`, `docs/SECURITY.md`, and `docs/VERIFICATION.md` describe the same tightened grant shape;
- confirm `parent_grant_id` appears nowhere as a v0 field;
- confirm device is present as an identity variable but absent from grant-matching;
- confirm the delegation property is a precondition, not a required v0 obligation;
- confirm the compound-issuer rule and the no-self-asserted-identity rule are stated in both SECURITY and VERIFICATION.

## Risks

- **Web↔core seam dependency**: This design asserts that the core must verify operator-session evidence independently of the web-server endpoint. The exact wire shape is deferred to `feature-web-core-protocol-seam`; if that feature cannot carry such evidence, this grant model needs revision. Mitigation: the requirement is recorded now so the seam feature must satisfy it.
- **Revocation model coupling**: Device revocation and endpoint revocation (SECURITY.md revocation actions) rely on device/endpoint identity, which is preserved in the identity model. If a future revocation design needs device-scoped grants, the grant subject may need to widen; that is a future decision, not a v0 one.
- **Adapter capability drift in UX**: Because capability declarations are advisory, a stale declaration could gray out an available action or enable a disabled-looking one. The failure mode is a real answer from the adapter at delivery time, handled by the existing failure vocabulary, which is acceptable and far better than the core silently gating on stale capability state.

## Implementation notes

- Files changed: `docs/PROTOCOL.md`, `docs/SECURITY.md`, `docs/VERIFICATION.md`, `.work/active/features/feature-design-grant-shape.md`.
- Tests added: none; this is foundation-doc implementation.
- Discrepancies from design: none.
- Adjacent issues parked: none.
- Verification: confirmed via `rg` that `parent_grant_id`/`delegated-by` appears only in "intentionally absent" explanatory notes (not as a live field); delegation property moved to a clearly labeled precondition section; grant-shape under-design-review marker removed while `feature-session-identity-adapter-contract` and the four `story-review-provisional-semantics` markers remain intact; adapter-capability display-vs-delivery wording is consistent across all three docs.

## Review (2026-06-29)

**Verdict**: Approve with comments

**Blockers** (resolved in review stride):
- `docs/PROTOCOL.md` `SubmissionOutcome = rejected` and the failure-vocabulary `unsupported_command` row still implied a core/submission-layer capability gate, contradicting the new canonical-command-kind + adapter-delivery-authority split. Fixed: submission-layer now cites "unknown-to-Patchbay command kind" (`validation_failed`); `unsupported_command` row is delivery-layer only and notes the core does not gate on cached adapter capability.

**Important** (resolved in review stride):
- `docs/SECURITY.md` rejected-direction line conflated missing grant with missing adapter capability ("best-effort hidden delivery when a grant or adapter capability is absent"). Fixed: now "when a grant is absent" only, since adapter capability is advisory.

**Nits** (applied):
- Grant-definition phrasing tightened from "an actor or endpoint" to "a subject (an actor, optionally narrowed to an endpoint or endpoint class)" in `docs/PROTOCOL.md`, `docs/SECURITY.md`, and `docs/GLOSSARY.md`.

**Notes**: Deep substrate feature review performed by one fresh-context cross-model reviewer on `openai-codex/gpt-5.5` per operator request (implementor was GLM 5.2). Reviewer identified one blocker and one important finding, both genuine contradictions between the new capability distinction and pre-existing failure-vocabulary / rejected-direction wording. All findings were applied in the review stride per the nit-triage convention; no follow-up items filed. Re-verification confirmed `unsupported_command` is now delivery-layer only across PROTOCOL and VERIFICATION, and no `actor or endpoint` grant-definition phrasing remains.
