---
id: epic-public-product-contract
kind: epic
stage: drafting
tags: [foundation, protocol, verification]
depends_on: [epic-foundation-hardening]
parent: null
created: 2026-07-10
updated: 2026-07-10
gate_origin: null
release_binding: null
---

# Epic: Public product contract and assurance calibration

## Brief

Patchbay's first executable milestone exists to get its initial operator operational, but that milestone is not the product ceiling. Patchbay is intended to become a publishable, reliable self-hosted product that additional operators can deploy for themselves. The current generic `v0` label obscures that distinction and caused an adversarial review to conflate two different findings: machinery that is genuinely valueless at any scale, and architecture that is merely beyond the first personal-operability milestone but remains necessary for the intended public product.

This epic defines a SemVer product horizon, establishes the `v1.0.0` public contract, and recalibrates verification work accordingly. It removes or corrects artifacts that cannot earn their claims at any version while preserving seams that support a publishable, adapter-neutral, independently deployable system. It is not a rewrite to a TypeScript monolith, a rejection of Rust, or a retreat from Ports & Adapters, generated contracts, formal reasoning, authority modeling, or forward-compatible protocol design.

## Why this is epic-sized

The scope changes the project's stated audience and release horizon, defines public compatibility and deployment obligations, establishes release-blocking assurance policy, and creates a multi-feature cleanup/design arc across foundation prose, protocol contracts, formal models, conformance vectors, CI, deployment, and adapter strategy. The cleanup must be decomposed so that product-serving seams are not deleted under a v0-only overengineering argument.

## Strategic decisions

- **Who shares one v1 deployment?** `v1.0.0` supports one human operator per deployment. Many operators may independently self-host Patchbay. Multi-human shared deployments remain an explicit post-v1 seam.
- **What adapters prove v1?** `v1.0.0` targets Pi plus one credible second adapter, preferably an open-source system with materially different semantics. Patchbay does not accept an obligation to build uncompensated first-party-provider integrations, but the public adapter contract must permit adopters and providers to build them. If no suitable second adapter exists, a materially distinct conformance reference adapter may prevent an indefinite release block.
- **What does deployable by others mean?** `v1.0.0` is a reliable self-hosted product: one supported reference deployment path, documented installation and TLS/reverse-proxy guidance, operator and adapter enrollment/revocation, versioned configuration and storage migrations, upgrade and rollback expectations, backup/restore, diagnostics/health checks, and tested crash recovery. HA, federation, zero-downtime upgrades, multiple storage backends, and orchestration-specific packaging remain preserved post-v1 seams.
- **What becomes stable at 1.0?** The public compatibility contract covers the adapter protocol/capability contract, explicitly documented operator API, supported persisted-data migration path, documented configuration, script-facing CLI behavior, and canonical protocol semantics. Internal module APIs, raw database schema, UI structure, human-readable CLI formatting, undesignated internal web/core calls, and checker/file layout remain private. `0.x` may break with explicit migrations and release notes; `1.x` follows SemVer.
- **What assurance blocks 1.0?** Patchbay uses a property-graded hybrid. Every public safety claim requires executable implementation evidence. Formal coverage additionally blocks release for command terminal races, session-generation isolation, crash/replay/snapshot convergence, and multi-surface Elicitation races. Multi-human delegation, lease, federation, HA, and split-brain properties gate those future capabilities rather than `v1.0.0`.

## Version horizon

- **`v0.1.0` — initial-operator walking skeleton.** One operator controls Pi-backed sessions through the responsive web cockpit and diagnostic CLI, proving the durable control loop and getting the initial operator operational.
- **`v0.x` — public-preview hardening.** Deployment, migrations, compatibility surfaces, implementation-backed conformance, adapter boundaries, and the second-adapter/reference-adapter proof mature while breaking changes remain permitted with explicit migration and release notes.
- **`v1.0.0` — reliable self-hosted public product.** Additional operators can independently deploy and operate Patchbay through the supported reference path; the designated public contracts and canonical semantics carry SemVer compatibility.
- **Post-v1 reserved capabilities.** Multi-human shared deployments, delegation workflows, federation, HA/multi-core coordination, replicated storage, zero-downtime upgrades, and broader surface/adapter ecosystems remain named seams promoted by demonstrated product pressure.

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
- Epic design decomposes the work into independently reviewable features for version vocabulary, public contract/deployment support, second-adapter proof, verification-program correction, and executable release assurance.
- Every proposed deletion records whether it is universally valueless or merely deferred beyond `v0.1.0`; no useful seam is removed under a v0-only argument.
- Weak or overclaiming verification artifacts are rewritten, demoted, moved to tooling documentation, or deleted; no strong product claim rests on metadata-only evidence.
- Public-facing safety claims are tied to implementation-backed conformance, with formal coverage added only where the property-grade policy requires it.

## Extension pressure classification

- **Committed `v0.1.0`:** one operator, one authoritative core, Pi-first control loop, durable local persistence behind ports, web cockpit, diagnostic CLI, generated contracts, and the safety kernel required to get the initial operator operational.
- **Committed `v1.0.0` horizon:** reliable self-hosted deployment for independent operators; Pi plus a credible second/reference adapter proof; stable designated public contracts; property-graded hybrid release assurance.
- **Reserved seams:** multi-human shared deployments, provider/adopter-authored proprietary integrations, federation, HA, replication, zero-downtime upgrades, delegation, lease-backed coordination, additional surfaces, and broader adapters.
- **Explicitly rejected as a Patchbay obligation:** uncompensated first-party-provider integration work as a prerequisite for `v1.0.0`. The adapter path remains open to providers and adopters.
