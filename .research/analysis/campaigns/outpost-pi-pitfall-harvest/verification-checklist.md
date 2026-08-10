---
provenance: agent-synthesis
stage: adversarial-read
updated: 2026-08-09
campaign: outpost-pi-pitfall-harvest
verdict: NEEDS-REVISION
---

# Adversarial verification checklist — outpost_pi pitfall harvest

## Scope read

- Campaign synthesis: `parent.md`
- Five specialist briefs under `specialists/`
- All 38 matching `restart-*`, `herdr-*`, `mobile-*`, `transcript-*`, and `keyring-*` attestations
- Lead-provided lint result and its known local `source_path` reachability limitation

The `unreachable-source` reports for commit-pinned, multi-path, and commit-range local sources are not repeated below. Handle-to-attestation resolution is intact; the findings concern semantic support.

## Verdict

**NEEDS-REVISION.** Most of the harvest is carefully scoped, contradiction-aware, and correctly marks Patchbay seam proposals as `{inferred}` or `{extends}`. The transcript facet survives the semantic walk cleanly. Revision is still required for one overclaimed parent cross-band mapping, several mobile claims that exceed their cited attestation passages, detailed keyring-cause language absent from its attestation, and two narrower parent/Herdr aggregation claims.

## Job (a): Semantic citation-chain walk

### Findings requiring revision

1. **Parent M2 / “Convergence with the 2026-08-09 Patchbay adversarial review”: BLOCKER 5 is analogy, not field corroboration.**
   - Claim: the keyring incident directly prefigures or independently corroborates “restart strands descendant authority,” and records “identity not carried across replacement.”
   - Citation: `[keyring-incident]{7}`.
   - Attested fact: silent adoption of a divergent fallback key changed the local cryptographic principal and led to mesh eviction.
   - Gap: the incident does not involve restart-as-continuation, descendant grants, or authority stranded across runtime replacement. It supports an **analogous identity-continuity warning**, not direct field evidence for the descendant-authority blocker.
   - Required correction: classify this row as `{inferred: analogy}` and say explicitly that descendant-grant behavior is not source-attested here, or remove BLOCKER 5 from the claim that five blockers were converted to field-attested failures.

2. **Mobile “completion from lifecycle events” contains specifics not present in `[mobile-review]{5}`.**
   - Claim: `session_start` lacked a request id and reload/local-UI commands lacked a uniform completion signal.
   - The attestation passage supports only the broader statement “no robust ack; many commands give no completion signal.”
   - Required correction: either enrich the attestation with the source passages for those two specifics or narrow the brief to the attested no-robust-ack claim.

3. **Mobile typed-operation retention rationale is not supported by the cited ranges.**
   - Claim: `session_new` was retained because the daemon, restart protocol, and staggered app/extension deployments depended on its typed contract.
   - Citations: `[mobile-ops]{2}` and `{3}` attest SDK context gating and why compact works while new-session does not; they do not attest the daemon/deployment dependency rationale.
   - Required correction: add a passage that records the retention rationale or narrow the sentence to the source-attested dedicated-operation rescope.

4. **Mobile deadlock rationale over-attributes intent to the SDK.**
   - Claim: command gating is intentional “to avoid event-handler deadlocks.”
   - Citations: `[mobile-ops]{2}` and `{3}` establish “only safe in user-initiated commands” and the base-context/command-context split, but not the deadlock motive.
   - Required correction: remove “to avoid event-handler deadlocks,” mark it as an inference, or attest the SDK/source passage that states this rationale.

5. **Keyring exact-cause detail exceeds `[keyring-incident]{2}`.**
   - Claims in the opening disconfirming analysis, later disconfirming analysis, and Gap 1 say the source lists locked/gone/unfindable states, a destructive service rename, and a requested comparison against the Owner-stored key.
   - The attestation records `KeyRevoked`, keyring inaccessibility, divergent fallback, and unresolved cause, but its cited passage does not record those detailed diagnostic alternatives or requested checks.
   - Required correction: enrich `keyring-incident.md` with quote-bound passages for those details, or retain only the attested statement that the exact cause is unresolved.

6. **Herdr contradiction table adds an unattested purpose to exact-PID matching.**
   - Claim: the original exact-child fence existed “so concurrent wrappers would not consume each other's intent.”
   - Citation: `[herdr-wrapper-tty-fix]{1}` supports exact PID-scoped matching and the TTY regression, but not that stated purpose.
   - Required correction: cite `[restart-wrapper-foreground-regression]{4}`, which records that surrounding intent, or mark the purpose as inferred and keep `[herdr-wrapper-tty-fix]` only for the mechanism.

### Claims that survive

- Restart P1–P13: the cited attestations semantically support the recorded ESM-cache behavior, timer/successor race, non-exclusive claim failure, settlement limitations, ingress window, wrapper foreground regression, readiness failure, PID-hunting mitigations, and false-green ENOENT teardown race. Patchbay prescriptions are consistently marked inferred.
- Herdr’s cwd-relocation, PTY backpressure, text-injection failure, ancestry discovery, bulk timing/naming failures, split restart ownership, and schema-archeology findings are supported and appropriately bounded to the observed installation/version.
- Transcript claims about adjacent-record divergence, arrival-order lifecycle reduction, timestamp provenance, first-writer-wins limits, hook multiplicity, enumerate-first audit, mesh-card authority, and custom-entry feasibility are semantically supported. The brief also correctly disclaims that the proposed durable architecture had shipped.
- Keyring’s observed failure chain, unverified fallback-continuity gap, absent keyring-to-file write-through, fail-loud no-file branch, Owner-membership containment, and `/new` non-causality are supported.
- Parent M1’s individual bullets, M3, and M5 are source-grounded as composed findings, subject to the aggregation-language correction under Job (b).

## Job (b): Claim-shapes the lint missed

1. **Parent M1 says every proxy inference “failed in the field.”** `{inferred: aggregates}` honestly marks composition, but “failed in the field” is too uniform: the mobile editor seam included a successful probe followed by a durability review/reversal, and the wrapper cross-marker case is an exposed gap without a recorded multi-wrapper interference failure. Revise to “was observed to fail, was reproduced, or was rejected/exposed during review.”

2. **Parent M2/M4 similarly turn gaps and rejected designs into universal recorded failures.**
   - M2 says each facet records an incarnation fence failing; the keyring case is an authority-continuity analogy, and wrapper cross-consumption remains untested.
   - M4 says each conflation produced a recorded failure; some entries are review-discovered distinctions or unresolved tensions rather than observed failures.
   - Keep the useful cross-facet synthesis, but use “failure, exposed gap, or rejected design” and preserve the distinction per bullet.

3. **Mobile seam decision 6 contains an uncited source-attribution shape.** “The failed command-picker plan also discovered that extension command discovery does not imply native TUI command enumeration” is presented as campaign evidence, while only the following Patchbay recommendation is marked `{inferred}`. Add a source-direct citation/passage or recast the whole statement as an inference from the bounded operation inventory.

No other uncited named-feature or comparative-as-description shape surfaced in the restart, transcript, or keyring findings.

## Job (c): Coherence-read for smoothed contradictions

- **Parent BLOCKER 5 mapping smooths two different authority failures:** local principal substitution after keyring failure versus descendant authority across process restart. They must remain side-by-side as analogy, not be merged into one field-attested lifecycle failure.
- The specialist contradiction sections otherwise preserve important corrections rather than smoothing them: settlement versus lock/flush; exact PID versus foreground terminal ownership; editor probe versus durable API; live ordering fix versus restart durability; intended key mirror versus current no-write-through behavior.
- No cross-facet contradiction omitted from the parent was found beyond the over-smoothed BLOCKER 5 analogy.

## Job (d): Noise-domination / relevance-weighting

- **Keyring headless fallback sentence uses a less-relevant locator.** `[keyring-storage]{2}` establishes a failed keyring operation, while the attestation summary and later fail-loud/fallback passage carry the platform split. Cite the passage that actually records headless/file-only fallback (and retain the core-keyring fail-loud qualifier).
- **Parent keyring root-cause composition would benefit from `[keyring-storage]{4}`.** `[keyring-incident]{3}` proves the divergent file key in the incident; `[keyring-storage]{4}` is the more relevant evidence that current resolution accepts an existing file without continuity comparison. This still would not prove descendant-grant behavior.
- Otherwise, the briefs generally foreground the most relevant evidence: implementation/review attestations for restart fencing, the corrective commit for PTY command failure, SDK definitions for mobile context capability, the systematic sweep for transcript provenance, and storage/tests for keyring remediation boundaries.

## Job (e): Quote-context walk

No separate quote-context stripping finding surfaced.

- The restart brief explicitly narrows “process restart is the only way” to this fetched ESM adapter/loader case.
- The Herdr brief preserves that the PTY stall was code-server-owned and version-bounds the restart-script defects.
- The mobile brief preserves the successful editor-seam probe while denying that it proves a durable contract.
- The transcript brief preserves design-versus-shipped status.
- The keyring brief marks the platform cause ambiguous; the problem there is unattested diagnostic detail, not removal of a source qualifier.

## Job (f): Analytical-tier-inheritance walk

- No citation targets a specialist brief, parent synthesis, or prior campaign artifact.
- `herdr-concepts` and `herdr-state` from the prior `v1-control-plane-and-spawn` campaign are mentioned only as lens and are not cited by the Herdr specialist. No inherited Herdr framing was laundered into source-attested prose.
- The parent’s Patchbay blocker comparison is explicitly analytical and is marked `{inferred: cross-band}` in its dedicated section. M2’s earlier duplicate blocker paragraph should repeat that local marker for clarity, especially when correcting BLOCKER 5, but the defect is overextended analogy rather than a lens citation.

## Job (g): Line-reference / locator walk

- All cited handles resolve to attestation files. The known local-source reachability parser limitation is not a locator failure.
- Spot-checked ranges exist under the campaign’s attestation convention and derive the narrow claims in the restart and transcript facets.
- Locator-to-claim mismatches remain for `[mobile-review]{5}`, `[mobile-ops]{2}{3}`, `[keyring-incident]{2}`, `[keyring-storage]{2}`, and `[herdr-wrapper-tty-fix]{1}` as described above. These are semantic range defects even though the handles resolve.
- No nonexistent cited range surfaced in the transcript facet’s 29 clean chains.

## Job (h): Thin-attestation semantic complement

No attestation is wholesale too thin for its intended facet: all 38 have source metadata, a descriptive summary, and source passages or source-code facts. Four applications are nevertheless per-claim thin:

- `mobile-review` for the exact request-id/command examples;
- `mobile-ops` for deployment-retention and deadlock-motive claims;
- `keyring-incident` for the detailed unresolved-cause checklist;
- `herdr-wrapper-tty-fix` for the concurrency-prevention rationale.

Enriching those attestations or narrowing the prose will restore the claim → locator → attestation chain.

## Required revision actions

1. Downgrade the parent’s BLOCKER 5 row from direct field corroboration to an explicitly marked analogy, and adjust the “five blockers” count/wording.
2. Replace parent M1/M2/M4’s universal “each failed” wording with failure/gap/rejected-design distinctions.
3. Narrow or re-attest the mobile request-id examples, `session_new` retention rationale, deadlock motive, and command-enumeration attribution.
4. Narrow or re-attest the keyring incident’s detailed causal alternatives and requested diagnostic checks; fix the headless-fallback locator.
5. Add the source-direct concurrency-intent citation to the Herdr contradiction row or mark that purpose inferred.
6. Re-run citation lint after revision; do not treat the known rich-`source_path` reachability warnings as grounding failures unless a handle or semantic chain actually breaks.
