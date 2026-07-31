---
id: idea-combined-surface-vision
created: 2026-07-30
updated: 2026-07-30
tags: [vision, combined-surface, parking]
---

# Combined-surface vision (Patchbay × Workbench × token-commune) — parked

A multi-round vision exploration (five adversarial review rounds across GPT and
Kimi K3) investigated whether Patchbay, Workbench, and token-commune — all
co-owned — could compose into a single operator surface. The exploration is
parked here with its conclusion. The vision document and the five review reports
live in `docs/PITCH-combined-surface.md` (and the review transcripts are
preserved in the session log).

## The conclusion (what the reviews converged on)

The "three-product family" framing did not survive scrutiny. Across five rounds,
the reviews proved the composition was always Patchbay's durable delivery
contract viewed alongside other things — the delivery contract *is* Patchbay.
When the framing was honestly restructured to "Patchbay as durable hub,
Workbench/token-commune as adapter instances," the reviews found that:

- **Seam #3 (token-commune adapter) is already committed v1.0.0 product**
  (SPEC.md:26, :189) — it has its own drafting epics independent of this
  vision.
- **Seam #1 (public client API) is standalone Patchbay distribution work**
  required by the IDE path regardless of any ledger pane.
- **The commission-boundary lifecycle** the vision identified as the real gap
  is "Patchbay's existing standalone v0.1.0 pitch" — by the vision's own
  admission.

Subtract the committed/standalone work, and the vision's entire uncommitted
content was **one unbuilt, undesigned, unmocked ledger pane plus a validation
loop inside it.** That is a backlog item, not a vision. Keeping the vision
framing alive keeps four-seam program gravity attached to a one-pane bet.

## The disposition

The vision is parked, not killed. The committed work (token-commune adapter
chain, public product contract) proceeds on its own tracks. The combined-view
hypothesis — that viewing the delivery contract alongside git-backed work state
and pooled fuel changes an operator decision the incumbent gets wrong —
becomes a cheap, falsifiable mockup-first experiment if pressure materializes
(see `idea-ledger-pane-falsifiable-experiment`).

## What was genuinely learned (durable findings, recorded separately)

The reviews surfaced four findings that are independently valuable and
*independent of the vision's fate*:

1. **The protocol has no adapter kind for a foreign data source Patchbay reads
   out of git.** A git-backed Markdown ledger is neither a runtime session nor
   an operational resource. See `idea-third-adapter-kind-foreign-data-source`.
2. **The public-client-API promotion is not the same as multi-host
   reachability.** The latter is the split-deployment reserved seam. See
   `idea-public-client-api-vs-split-deployment`.
3. **`schema_ref` + `AttentionRequired` is not a validation engine.** A
   partial validator that validates only the cheap half manufactures the
   durable false confidence it was built to prevent. See
   `idea-correlation-grounding-validation-leak`.
4. **The demand question is empirical, not architectural.** See
   `idea-combined-surface-demand-research`.

## What does NOT need parking (already covered)

- Agents-as-principals → `idea-agent-to-agent-mesh-seam` (the α/β fork is
  already recorded there).
- Multi-human coordination → `idea-multi-human-coordination`.
- Desktop/IDE surface → `idea-desktop-app-surface`,
  `idea-harvest-remote-pi-extension-as-adapter`.

## Open question for the co-owners

Whether to revive the combined-surface framing later depends on whether the
ledger-pane experiment (if run) produces evidence that the combined view
changes operator decisions. Until then, this is a parked vision, not a
direction.
