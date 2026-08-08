# Verification checklist — v1-control-plane-and-spawn

Gate history for the campaign (full rigor: specialists → lint → cross-synth → adversarial-read → evaluator → spot-check).

## Specialists (3, multi-path)
- `peer-protocol-deep-dive` (sol) — brief + attestations: mission-control-src, amux-outbox, happy-relay, codeagent-mobile. 1 blocking acquisition (CodeAgent Mobile backend).
- `spawn-lifecycle` (luna) — brief + attestations: pi-sessions, pi-rpc, pi-sdk, herdr-concepts, herdr-state, coder-workspaces. No acquisition candidates (Devin/Daytona/amux unacquired → held as gaps, not training-recall).
- `pi-adapter-probe` (luna) — brief + attestations: pi-rpc, pi-sessions, pi-extensions, pi-loader, pi-sdk. No acquisition candidates.

Shared-source attestation handles (pi-rpc/pi-sessions/pi-sdk attested by two specialists) reconciled — citation chains resolve cleanly.

## Lint
`lint-citations.py` on all three specialist briefs + parent: **88 resolved, 0 broken, 0 thin, 0 pattern flags** (final). Deprecation note: some attestations omit `substrate_confidence` (defaults source-direct; future-MAJOR fail-closed) — non-blocking.

## Cross-synthesis
`parent.md` composed across the three facets (moat verdict + spawn lifecycle direction + Pi capability + adapter-owned Project seam + consolidated conformance vectors + contradictions).

## Gates
- **Adversarial-read pass 1 — NEEDS-REVISION:** moat verdict overclaimed given CodeAgent backend gated; broad uniqueness language; wrong/thin Pi locators; lifecycle block + "Pi supports directly" read as source-attested; Contradictions delegated to facet briefs; "decided" overstated; `get_entries` over-broad; cross-peer under-cited; project-framing + authoring-history leakage (portability).
- **Revision (sol):** all 10 addressed — corpus-bounded verdict + CodeAgent-gate propagated; composition markers added; Pi locators fixed + attestations extended (pi-rpc {6-9}, pi-loader {2-3}, pi-sessions {2-3}, pi-extensions {7-8}); Contradictions rebuilt in-parent; "decided" → "proposed direction"; `get_entries` scoped; Reader-context block added (portability); authoring-history stripped.
- **Evaluator (isolated) — NEEDS-REVISION (same class):** overclaim + portability + framing — addressed by the revision.
- **Adversarial-read pass 2 — NEEDS-REVISION (narrow convergence correction):** every major prior finding resolved; residue = two verdict-summary locator sets (MC approvals needs `{1}`; amux needs `{1}{2}{6}`, drop auth-passage `{10}`) + a "1–3 axes" line that understated peer coverage (subtly overstating the moat) + one namesake-framing nit.
- **Surgical correction (lead, inline):** the two locator sets fixed; the axis-count replaced with accurate per-peer partial-coverage framing; namesake editorially hedged. Re-lint clean.

## Spot-check (lead)
Revised `parent.md` is portable (Reader-context defines the five core concepts), corpus-bounded (no unconditional moat language; CodeAgent gate propagated), composition-marked (`{extends}`/`{inferred}` on lifecycle/convergence/Pi-feasibility), and its citation chains resolve (88, 0 broken/thin). Second-pass narrow residual surfaced (not silently looped) and surgically resolved.

## Status
**CONVERGED.** The second adversarial pass was a narrow precision correction; applied + re-linted clean. No material current-cycle groundedness issue remains. Acquisition gap (CodeAgent Mobile backend) recorded in `acquisitions.md`, proposed for the `research-acquisition-queue` at the operator-confirmed handoff gate.
