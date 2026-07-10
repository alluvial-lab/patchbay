---
id: epic-public-product-contract-publication-governance
kind: feature
stage: drafting
tags: [foundation]
parent: epic-public-product-contract
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-10
updated: 2026-07-10
---

# Publication identity and licensing governance

## Brief

Make the project publishable under a distinctive, cleared identity and an unambiguous licensing/contribution policy. The identity arc performs source-grounded collision screening, qualified trademark review, final-name selection, trademark policy, and the coordinated repository/package/domain/registry rename. The licensing arc obtains qualified open-source legal review of the intended `AGPL-3.0-or-later` application boundary and `Apache-2.0` interoperability boundary, then establishes license files, SPDX/notices, generated-output treatment, dependency compatibility, documentation licensing, proprietary-adapter permission, and contribution terms.

Naming begins immediately but runs in parallel with engineering: the provisional `Patchbay` name may remain internally while contracts are built, yet no public package/registry reservation or public release may proceed without the cleared identity. The project does not accept outside contributions until contributor terms are settled. Because only one or two invited pre-v1 collaborators are likely, counsel may choose a narrow agreement before their first contribution; the project does not prematurely assume either a CLA/assignment for commercial dual licensing or a DCO-only community posture. A DCO alone must not be represented as granting unilateral relicensing rights.

The two legal tracks are grouped as one publication-readiness capability to keep the epic within a coherent six-feature decomposition; feature design should keep their research, review, and acceptance evidence distinct. Legal conclusions must come from qualified counsel, not from this work item's prose.

## Epic context

- Parent epic: `epic-public-product-contract`
- Position in epic: independent, externally reviewed publication gate; it does not block internal engineering but must complete before public release, package registration, or outside contributions.
- Licensing-boundary inventory must coordinate with `epic-public-product-contract-public-compatibility` even though final-name work starts in parallel.

## Foundation references

- `docs/VISION.md` — public-product intent
- `docs/SPEC.md` — public contracts and proprietary-adapter seam
- `README.md` — current unlicensed/publication status
- `contracts/` — interoperability and generated-output surfaces requiring explicit treatment
