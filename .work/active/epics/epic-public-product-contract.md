---
id: epic-public-product-contract
kind: epic
stage: implementing
tags: [foundation, protocol, verification]
depends_on: [epic-foundation-hardening, epic-v0-1-0-implementation]
parent: null
created: 2026-07-10
updated: 2026-07-26
gate_origin: null
release_binding: null
---

# Epic: Public product contract and assurance calibration

## Status

Active after the v0.1.0 walking skeleton shipped. The remaining public-product features now consume running core, server, cockpit, CLI, Pi-adapter, persistence, and observability behavior rather than design-only scaffolding. The post-v0.1.0 agent-operations arc supplies the selected token-commune resource adapter to the portability proof; public compatibility, self-hosted operations, executable assurance, and publication governance continue on their declared dependency paths.

## Brief

Patchbay's first executable milestone exists to get its initial operator operational, but that milestone is not the product ceiling. Patchbay is intended to become a publishable, reliable self-hosted product that additional operators can deploy for themselves. The current generic `v0` label obscures that distinction and caused an adversarial review to conflate two different findings: machinery that is genuinely valueless at any scale, and architecture that is merely beyond the first personal-operability milestone but remains necessary for the intended public product.

This epic defines a SemVer product horizon, establishes the `v1.0.0` public contract, and recalibrates verification work accordingly. It removes or corrects artifacts that cannot earn their claims at any version while preserving seams that support a publishable, adapter-neutral, independently deployable system. It is not a rewrite to a TypeScript monolith, a rejection of Rust, or a retreat from Ports & Adapters, generated contracts, formal reasoning, authority modeling, or forward-compatible protocol design.

## Why this is epic-sized

The scope changes the project's stated audience and release horizon, defines public compatibility and deployment obligations, establishes release-blocking assurance policy, and creates a multi-feature cleanup/design arc across foundation prose, protocol contracts, formal models, conformance vectors, CI, deployment, and adapter strategy. The cleanup must be decomposed so that product-serving seams are not deleted under a v0-only overengineering argument.

## Strategic decisions

- **Who shares one v1 deployment?** `v1.0.0` supports one human operator per deployment. Many operators may independently self-host Patchbay. Multi-human shared deployments remain an explicit post-v1 seam.
- **What adapters prove v1?** `v1.0.0` targets Pi plus token-commune. Pi proves the runtime-session path; token-commune proves the materially distinct operational-resource path without putting LLM traffic through Patchbay. Patchbay does not accept an obligation to build uncompensated first-party-provider integrations, but the public adapter contract must permit adopters and providers to build them.
- **What does deployable by others mean?** `v1.0.0` is a reliable self-hosted product: one supported reference deployment path, documented installation and TLS/reverse-proxy guidance, operator and adapter enrollment/revocation, versioned configuration and storage migrations, upgrade and rollback expectations, backup/restore, diagnostics/health checks, and tested crash recovery. HA, federation, zero-downtime upgrades, multiple storage backends, and orchestration-specific packaging remain preserved post-v1 seams.
- **What becomes stable at 1.0?** The public compatibility contract covers the adapter protocol/capability contract, explicitly documented operator API, supported persisted-data migration path, documented configuration, script-facing CLI behavior, and canonical protocol semantics. Internal module APIs, raw database schema, UI structure, human-readable CLI formatting, undesignated internal web/core calls, and checker/file layout remain private. `0.x` may break with explicit migrations and release notes; `1.x` follows SemVer.
- **What assurance blocks 1.0?** Patchbay uses a property-graded hybrid. Every public safety claim requires executable implementation evidence. Formal coverage additionally blocks release for command terminal races, session-generation isolation, crash/replay/snapshot convergence, and multi-surface Elicitation races. Multi-human delegation, lease, federation, HA, and split-brain properties gate those future capabilities rather than `v1.0.0`.
- **What licensing model supports publication and adapter adoption?** Subject to legal review, the intended policy is `AGPL-3.0-or-later` for the application—including the coordination core, server, and web surface—and `Apache-2.0` for interoperability surfaces such as protocol schemas, generated clients, adapter SDKs, examples/templates, and official adapters. The boundary must continue to permit providers and adopters to build proprietary adapters without relicensing the application itself. Documentation licensing, generated-code notices, dependency compatibility, and contribution terms must be made explicit before publication. If future proprietary commercial licensing is desired, contributor relicensing rights must be established before accepting outside contributions; a DCO alone is not assumed to provide them.
- **Is Patchbay the public product name?** `Patchbay` is a provisional working name, not a committed public mark. Its descriptive fit is outweighed by substantial existing software and commercial use, weak search distinctiveness, and likely trademark crowding. Before public release, package/registry reservation, or contributor growth makes a rename expensive, the project must select and legally clear a more distinctive product identity. “Patchbay” may remain descriptive language beneath the new mark (for example, “a patchbay for agent sessions”) if counsel considers that use safe. The public identity gets a separate trademark policy rather than relying on the software licenses to govern branding.

## Design decisions

- **Does this epic define readiness or deliver it?** It owns the full v1-readiness program. Child features may remain blocked on missing implementation or qualified legal review, but they do not stop at prose or scaffolding while claiming the capability is delivered. Release binding remains late-bound.
- **Does naming block internal engineering?** No. Identity selection and clearance begin immediately in parallel, while the provisional working name may remain on unpublished internal artifacts. A cleared identity is a hard gate for public package/registry reservation and public release, not for unrelated internal engineering.
- **How are contributor relicensing rights handled before v1?** The project accepts no outside contribution until contributor terms are settled. With at most one or two invited collaborators expected, qualified counsel will choose appropriate terms before their first contribution. The project does not prematurely commit either to commercial dual-licensing rights through a CLA/assignment or to a DCO-only posture that may foreclose unilateral relicensing.
- **When does legal review become a release gate?** Not for `v0.1.0`. That milestone is for the initial operator's personal/internal use and is not a public distribution milestone. Qualified trademark and open-source legal review gate the first public release, public package/registry reservation, and any outside contribution. If public distribution is pulled forward to `v0.1.0`, the legal gate moves forward with it.
- **Does this epic need UI mockups?** No. It introduces no net-new screen, flow, component, or visual-system decision; it changes product, compatibility, operational, adapter, assurance, naming, and licensing obligations. The repository-wide mockup-first convention is installed for later UI-bearing work.

## Version horizon

- **`v0.1.0` — initial-operator walking skeleton.** One operator controls Pi-backed sessions through the responsive web cockpit and diagnostic CLI, proving the durable control loop and getting the initial operator operational. It is a personal/internal milestone, not a public distribution milestone, and does not require completed publication legal review.
- **`v0.x` — public-preview hardening.** Deployment, migrations, compatibility surfaces, implementation-backed conformance, adapter boundaries, and the second-adapter/reference-adapter proof mature while breaking changes remain permitted with explicit migration and release notes.
- **`v1.0.0` — reliable self-hosted public product.** Additional operators can independently deploy and operate Patchbay through the supported reference path; the designated public contracts and canonical semantics carry SemVer compatibility.
- **Post-v1 reserved capabilities.** Multi-human shared deployments, delegation workflows, federation, HA/multi-core coordination, replicated storage, zero-downtime upgrades, and broader surface/adapter ecosystems remain named seams promoted by demonstrated product pressure.

## v1 scope (consolidated, 2026-08-08)

Operator-confirmed consolidation of the v1.0.0 bar after the 2026-08-08 product brainstorm + the `v1-control-plane-and-spawn` research campaign. This is the working v1 picture (must / could / post-v1 — **no "should" tier**: everything is either in-v1, opportunistic, or later). It supersedes the earlier feature-scope sketch, which predated the resource-plane, token-commune-observer, and spawn research.

**Definition of done** — the operator's real daily workflow (herdr + outpost_pi) runs on Patchbay, and the known collaborator (NKlisch) can deploy + operate it: spawn/restart agents from any surface (mobile included); durable control across devices/machines; the two reference adapters (Pi + token-commune) prove the boundary via executable conformance; one supported self-hosted reference path; the public contract + assurance gates green.

**MUST (in v1):**
- *Adapters:* `research-handoff-spawn` (logical-target + generation + restart-as-continuation; adapter-owned Project seam); `research-handoff-pi-adapter-capability` (cursor recovery, restart-as-upgrade-boundary, minimum manifest); `epic-token-commune-control-attention` (mutations/approval/re-onboard — lives in its own epic; v1 adapter-proof names "attention/control").
- *Product shell:* `self-hosted-operations` (dogfood-first reference path: Docker primary + bare-metal fallback); `public-compatibility` (SemVer surface); `publication-governance` (SemVer + release process); `executable-release-assurance` (the formal-coverage gates + the campaign's ~25 conformance vectors); `adapter-portability-proof` (Pi + token-commune cross-adapter boundary proof; blocked on control-attention).
- *Surface:* mobile-responsive web cockpit at switch-quality (the outpost_pi replacement; responsive-web-over-Tailscale validated by the Cline precedent; mobile = control/monitor/review).

**COULD (opportunistic, only if it fits):** native mobile (Expo, or evolve outpost_pi's app — push/background/Tailscale split-tunnel beyond responsive-web); IDE extension (desktop control surface); extra mobile ergonomics (one-tap spawn/restart, theming, log export).

**post-v1 (later — soft, not forbidden):** mesh / agent-send · Elicitations (operator prefers conversational `instruct` flows) · multi-human / federation / HA / split-brain / second storage backend · broader adapter ecosystem + plugin marketplace · OKF third adapter kind · core `ProjectRef` promotion (only if cross-adapter project routing is needed) · richer token-commune views (gated on the contributor-attribution external prerequisite, not Patchbay work).

## Work classification rule

Every adversarial-review finding entering this epic must be classified into exactly one bucket before implementation:

1. **Remove or correct at every version** — the artifact is vacuous, self-defining, non-independent, falsely named, metadata-only while presented as assurance, semantically stale, or a recurring process step with no decision/evidence yield.
2. **Preserve seam; defer implementation** — the capability is not needed for `v0.1.0` but supports the public `v1.0.0` contract or an explicit post-v1 direction and is expensive to retrofit.
3. **Keep as current product machinery** — the capability directly supports `v0.1.0` or the committed `v1.0.0` public product.

"Too much for the initial operator" is not sufficient evidence for bucket 1.

## Initial remove-or-correct candidates

These candidates remain subject to feature design and adversarial confirmation; the epic does not pre-judge rewrite versus deletion where a genuine property intent exists.

- Formal properties whose names materially overclaim their formulas, including the current `CommandDurability`, `PreAppendTerminalChoice`, `LsnDeterminesTerminalWinner`, and overly abstract boundary-dedup checks. Rewrite them to model the claimed failure boundary or remove/demote the claim.
- The current Alloy actor-identity assertion that checks a constraint already imposed as a fact. Remove the tautological check while preserving Alloy as the reserved relational tool for real delegation, authority-graph, routing, and lease/fencing problems.
- Presentation of Quint-emitted TLA+ artifacts as an independent verification lane. Generated inspection artifacts may remain where useful, but only independently authored and independently checked models count as separate evidence.
- Model/vector promotion or traceability machinery that validates metadata while being described as behavioral evidence. Preserve honest evidence grades; require actual checker execution and executable vectors before product claims advance.
- Semantically stale model/prose traceability claims. Generated tables do not substitute for reconciliation of the assertions themselves.
- Recurring retroactive design-audit patterns that consume full design passes without surfacing a decision, correction, or evidence gain. Preserve targeted adversarial review where the mutation/failure surface justifies it.
- Toy formal examples appearing in the product-verification inventory. They may move to tooling documentation if still useful.

## Seams and principles explicitly preserved

- **Ports & Adapters:** domain semantics remain independent of storage, HTTP, deployment packaging, and any single harness.
- **Single Source of Truth:** protocol variants, states, failure vocabularies, capabilities, and authority kinds retain canonical registries from which validation, routing, contracts, and presentation derive.
- **Generated Contracts:** Protobuf/Buf and generated boundary types remain the cross-language/public-contract posture; this epic may reduce or complete contract surfaces but does not replace them with hand-copied DTOs.
- **Fail Fast:** boundary validation, authenticated identity derivation, target/generation checks, and internal precondition assertions remain mandatory.
- Stable session generations, durable acceptance, idempotent retry, typed correlation, authoritative snapshots/reconnect, and honest stale/unknown outcomes.
- Actor, endpoint, grant, revocation, authority-domain, adapter-capability, delegation, lease, federation, non-operator sender, and additional-surface seams needed by the public product horizon. Reserved does not mean implemented in `v0.1.0`.
- Rust orientation and the current logical core/control-surface boundary remain standing architecture unless a separately scoped design decision changes them.
- Property-graded formal reasoning remains; checker count and file format are not product goals.

## Release-assurance policy

A property may be described as:

- **Specified** — required by canonical prose/contracts.
- **Model-checked** — established only for the bounded abstract model.
- **Implementation-checked** — exercised against running product code.
- **Release-verified** — carries every evidence form required by its risk grade.

For a formally gated v1 property, release verification requires: a model that represents the claimed failure boundary; a formula whose name matches what it proves; adversarial mutation/non-vacuity evidence; at least one executable vector against the implementation; shared traceable property identity; and CI that runs the real checker and executable test rather than metadata validation alone.

## Acceptance outcomes

- Foundation docs consistently distinguish `v0.1.0`, public-preview `v0.x`, `v1.0.0`, and post-v1 reserved capabilities.
- `v1.0.0` has an explicit audience, adapter proof, deployment support floor, compatibility contract, and assurance gate.
- Epic design decomposes the work into independently reviewable features for version vocabulary, public contract/deployment support, second-adapter proof, verification-program correction, executable release assurance, public-product naming/clearance, and repository licensing formalization.
- Every proposed deletion records whether it is universally valueless or merely deferred beyond `v0.1.0`; no useful seam is removed under a v0-only argument.
- Weak or overclaiming verification artifacts are rewritten, demoted, moved to tooling documentation, or deleted; no strong product claim rests on metadata-only evidence.
- Public-facing safety claims are tied to implementation-backed conformance, with formal coverage added only where the property-grade policy requires it.
- A distinctive public product name is selected only after preliminary collision screening and qualified trademark review; repository, package, domain, and registry identities are updated before the first public release.
- Legal review confirms the intended AGPL-application/Apache-interoperability boundary, dependency compatibility, generated-output treatment, and proprietary-adapter permission before license files and public claims are finalized.
- The repository carries unambiguous per-surface license files and SPDX/noticing guidance, contribution terms appropriate to the chosen future-relicensing posture, and a separate trademark policy before accepting public contributions.

## Decomposition

The epic is split by capability rather than implementation layer. The already-authored version vocabulary is folded into the public compatibility contract. Compatibility and self-hosting operations are separate so the stable-boundary work does not become an oversized deployment feature; verification correction remains independent from positive executable assurance so honest cleanup is not blocked on product code. Naming and licensing are grouped as one publication-governance capability, with distinct legal tracks inside it, to leave room for both public-product capability arcs within the six-feature epic limit.

### Child features

- `epic-public-product-contract-public-compatibility` — designate and enforce the stable v1 public/private compatibility boundary and SemVer ceremonies — depends on: `[]`
- `epic-public-product-contract-self-hosted-operations` — deliver one tested install, secure operation, migration, upgrade/rollback, backup/restore, diagnostics, and recovery path — depends on: `[epic-public-product-contract-public-compatibility]`
- `epic-public-product-contract-adapter-portability-proof` — prove the public adapter boundary across Pi runtime sessions and token-commune operational resources — depends on: `[epic-public-product-contract-public-compatibility, epic-token-commune-control-attention]`
- `epic-public-product-contract-verification-claim-correction` — re-inventory and correct, demote, relocate, or remove verification artifacts whose claims exceed their evidence — depends on: `[]`
- `epic-public-product-contract-executable-release-assurance` — run real checkers and implementation-backed evidence for public claims and the four formally gated kernels — depends on: `[epic-public-product-contract-public-compatibility, epic-public-product-contract-self-hosted-operations, epic-public-product-contract-adapter-portability-proof, epic-public-product-contract-verification-claim-correction]`
- `epic-public-product-contract-publication-governance` — clear the public identity and legally formalize licensing, trademark, generated-output, dependency, and contribution policy — depends on: `[]`
- `research-handoff-spawn` *(research-origin: v1-control-plane-and-spawn; added 2026-08-08)* — the v1-must spawn lifecycle (logical-target + generation + restart-as-continuation; adapter-owned Project seam) that enables the operator's workflow migration off herdr — depends on: `[]`
- `research-handoff-pi-adapter-capability` *(research-origin: v1-control-plane-and-spawn; added 2026-08-08)* — Pi adapter cursor recovery, restart-as-upgrade-boundary, restart-as-continuation orchestration, minimum capability manifest — depends on: `[research-handoff-spawn]`

*Cross-epic v1-must:* `epic-token-commune-control-attention` (mutations/approval/re-onboard) is a v1-must that lives in its own epic; `adapter-portability-proof` depends on it.

### Decomposition risks

- **Missing implementation prerequisite.** Self-hosted operations and executable release assurance cannot complete until the core, persistence, control surfaces, packaging, and adapters exist. Their design may proceed, but metadata, prose, or fixtures cannot stand in for running evidence.
- **External legal gates.** Final identity clearance and licensing/contribution conclusions require qualified counsel. Publication governance can prepare inventories and options but must remain incomplete rather than fabricate legal certainty.
- **Broad operational surface.** Compatibility and self-hosted operations were split because combining API/config/migration stability with installation/backup/recovery would exceed a comfortable feature. Each feature-design pass must still split further into implementation units without creating layer-only work.
- **Stale cleanup inventory.** Foundation hardening already fixed some adversarial findings. Verification correction must inspect current HEAD and classify each artifact afresh instead of replaying the epic's initial candidate list.
- **Cross-repository reference dependency.** token-commune is the selected second adapter, but its external read/mutation contracts, scoped credentials, event identity/cursors, and idempotency semantics are owned in a separate repository. Patchbay must consume an explicit external contract and must not turn sibling-repo implementation coupling into portability evidence.
- **Critical path.** Executable assurance intentionally waits for the public contract, operational recovery behavior, adapter proof, and corrected verification claims. The long path is evidence of the actual product dependency, not a reason to weaken the gate.

## Extension pressure classification

- **Committed `v0.1.0`:** one operator, one authoritative core, Pi-first control loop, durable local persistence behind ports, web cockpit, diagnostic CLI, generated contracts, and the safety kernel required to get the initial operator operational.
- **Committed `v1.0.0` horizon:** reliable self-hosted deployment for independent operators; Pi plus token-commune adapter proof across session and resource shapes; stable designated public contracts; property-graded hybrid release assurance; a cleared distinctive public identity; and a legally reviewed AGPL-application/Apache-interoperability licensing policy.
- **Reserved seams:** multi-human shared deployments, provider/adopter-authored proprietary integrations, federation, HA, replication, zero-downtime upgrades, delegation, lease-backed coordination, additional surfaces, and broader adapters.
- **Explicitly rejected as a Patchbay obligation:** uncompensated first-party-provider integration work as a prerequisite for `v1.0.0`. The adapter path remains open to providers and adopters.
