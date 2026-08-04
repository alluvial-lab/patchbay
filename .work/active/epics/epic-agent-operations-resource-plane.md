---
id: epic-agent-operations-resource-plane
kind: epic
stage: review
tags: [foundation, protocol, adapter, ux]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-26
updated: 2026-07-26
---

# Agent-operations resource plane

## Brief

Evolve Patchbay from a control plane centered exclusively on runtime sessions into an operator-owned agent-operations control plane: sessions remain the product center, while operational resources that materially govern agent availability, capability, or safe control become first-class adapter targets. The first pressure case is model-capacity infrastructure, but the architecture must admit a resource only when its state changes what the operator can ask an agent to do or requires human action to keep agent work operating.

This epic promotes the existing generic resource target seam into a designed identity, snapshot/revision, Observation, query, authority, and presentation path without pretending resources are runtime sessions. It also establishes the composition rule for the cockpit: canonical protocol and presentation primitives provide delivery, reconciliation, stale-state, authority, and attention honesty; adapter-shaped projections provide domain views above that floor. It does not create an arbitrary monitoring platform or a dynamically loaded third-party UI plugin system.

## Strategic decisions

- **What does Patchbay become after v0.1.0?** A personal agent-operations control plane for sessions and the operational resources that govern their availability, capability, and safe control; it does not become a generic infrastructure dashboard.
- **Are resources represented as sessions?** No. Runtime sessions and operational resources are distinct target categories with honest identities and state; resource health must not be coerced into session connectivity or activity.
- **How does adapter-specific UX fit surface-neutrality?** The shared conformance floor remains mandatory, while adapter-shaped projections may compose richer domain views without inventing protocol states or presenting stale data as live.
- **What is the near-term human authority model?** One human operator per Patchbay deployment remains the committed short-term shape; shared multi-human Patchbay authority is not required by the resource plane.
- **What qualifies as an operational resource?** It must materially affect agent capability/availability or require operator attention to keep agent work operating. Arbitrary service telemetry is outside the product boundary.

## Arc position

This is the foundation epic for the post-v0.1.0 agent-operations arc. `epic-token-commune-observer` depends on it and supplies the first real resource adapter. `epic-token-commune-control-attention` then exercises durable mutation and attention workflows over the same resource model.

## Capability outline

- resource identity and target-resolution semantics distinct from runtime-session identity;
- resource snapshot/revision and reconnect behavior with explicit partial/no-snapshot degradation;
- resource-scoped query, Observation, authority, audit, and subscription behavior;
- presentation bindings that keep resource domain health separate from connectivity and command lifecycle;
- cockpit navigation/composition for sessions plus operational resources;
- adapter-shaped projection contracts that remain bounded by the surface-neutral conformance floor;
- conformance evidence showing a resource adapter cannot bypass Patchbay authority, durability, or stale-state rules.

## Scope boundaries

- No multi-human shared Patchbay deployment, delegation, quorum approval, federation, or agent-to-agent work routing.
- No model request/data-plane proxying through Patchbay.
- No universal monitoring ontology or arbitrary dashboard/plugin marketplace.
- No requirement that every adapter expose resources; Pi may remain session-centered.
- Exact wire registries and presentation extension mechanics are design work for this epic, not preselected by this scope item.

## Simplification opportunity

Use the existing `TargetScopeKind = resource`, Operation/Observation envelopes, snapshot discipline, authority checks, and presentation primitives rather than creating a parallel control subsystem. Eliminate the temptation to synthesize fake runtime sessions, generations, or activity states for non-session resources. Keep one adapter projection path instead of separate one-off diagnostic and dashboard state stores.

## Mockups

Phase 4.6 design exploration produced interactive mocks at
`.mockups/screens/epic-agent-operations-resource-plane/` (see `index.html`):

- **Navigation decision: Resources as a peer destination** (`option-1.html`).
  Resources gets its own rail destination, parallel to Sessions — consistent
  with the lockdown mockup's established kind-as-destination pattern and
  extensible to future target kinds. Rejected alternatives: composed-into-
  session (loses detached-resource attention the control-attention epic needs)
  and unified-list-by-kind (its "operations" frame breaks for the reserved
  read-only third kind).
- **Resources destination surfaces two resource kinds** under the single
  admission rule: pooled token-commune pools (adminable) and direct-provider
  usage windows (read-only, e.g. Claude Code / Codex / GLM windows). Both
  materially govern agent availability.
- **Session runtime-context strip** (`session-context.html`, interactable):
  every session surfaces provider · model · context-remaining · usage-window.
  Provider and model are interactable pickers (selecting issues a `reconfigure`
  Operation — `model_set` within a provider, provider-switch for the fuel
  source). The usage cell links to a resource only when provider = token-commune.
- **Mobile affordance** (`session-context-mobile.html`): the strip collapses
  to tappable pill-buttons; pickers open a bottom sheet (the lockdown mock's
  established mobile pattern).

The exploration surfaced two adjacent concerns that are **sibling scope, not
this epic's** (recorded so the decomposition is honest about the boundary):

- **The provider concept** (model-vs-provider split, provider-switch
  reconfigure) is session-runtime, not resource-plane. This epic delivers the
  resource-side rendering and linkage; the provider concept itself is a
  sibling feature.
- **Direct-provider-usage adapters** (Claude Code / Codex / GLM window
  observers) are adapter work parallel to the token-commune observer epic,
  consuming this epic's foundation. The foundation must make the resource-
  adapter concept general enough to admit them.

## Decomposition

Split by capability: the resource foundation (identity, state, manifest)
lands first; the cockpit composition consumes it; conformance evidence closes
the arc. The provider concept and direct-provider-usage adapters are sibling
scope (see Mockups), so they do not appear as child features here.

### Child features

- `epic-agent-operations-resource-plane-resource-identity` — typed resource identity, target-resolution polymorphism, grant-containment refinement — depends on: `[]`
- `epic-agent-operations-resource-plane-resource-state` — resource snapshot, revision, completeness tier, typed report ingress, delta folding, replay, LoadSnapshot — depends on: `[resource-identity]`
- `epic-agent-operations-resource-plane-capability-manifest` — adapter manifest extensions (resource kinds, target categories, projection schemas, per-resource snapshot tier); target-category registry extensible for the reserved OKF third kind — depends on: `[resource-identity]`
- `epic-agent-operations-resource-plane-cockpit-composition` — Resources destination (pooled + direct-provider sections), runtime-context strip resource-linkage, mobile affordances, grant-scope labels — depends on: `[resource-identity, resource-state, capability-manifest]`
- `epic-agent-operations-resource-plane-conformance` — vectors proving a resource adapter cannot bypass authority, durability, or stale-state rules — depends on: `[resource-identity, resource-state, capability-manifest, cockpit-composition]`

### Decomposition risks

- `resource-identity` refactors `TargetResolver` (hard-coded to session fields today) — touches the acceptance pipeline, but the port is already named `TargetResolver`, so polymorphism is the intended shape.
- `resource-state` adds a `StoredEventKind` resource variant → durable resource state + replay; the storage port is already opaque-domain-keyed, so the materializer pattern extends.
- `capability-manifest` must support multiple resource kinds (pooled + direct-provider) while keeping the target-category registry extensible for the reserved OKF third kind — must not accidentally admit foreign data sources while staying honest about operational resources.

## Extension pressure classification

- **Committed post-v0.1.0 direction:** first-class operational resources; personal one-operator control; adapter-shaped projections above the conformance floor.
- **Reserved seams:** dynamically loaded third-party surface plugins, multi-human shared authority, resource-to-resource coordination, and a broad external adapter ecosystem.
- **Explicitly rejected for this arc:** representing resource health as session connectivity/activity, turning Patchbay into generic monitoring, or making adapter-specific state part of the core protocol registry without a promotion ceremony.

## Interaction with parked ideas

- **`idea-third-adapter-kind-foreign-data-source`** — pressure-test input for the
  presentation-extension-mechanics design. This epic owns "exact wire registries
  and presentation extension mechanics" and reserves "dynamically loaded
  third-party surface plugins." The parked finding argues a git-backed Markdown
  work ledger is neither a runtime session nor an operational resource under
  SPEC's admission rule, so if the cockpit ever renders an external work
  ledger, the design must decide whether it fits the resource-adapter kind or
  needs a *third* projection-source kind. **Open Knowledge Format (OKF v0.2)**
  is the candidate format for that reserved third kind (vendor-neutral
  markdown+frontmatter knowledge bundles); this epic's `capability-manifest`
  child keeps the target-category registry extensible so an OKF-knowledge-
  bundle category is an additive future promotion, not a rearchitecture. The
  epic's presentation-mechanics design should engage this question rather than
  discovering it later.
- **`idea-correlation-grounding-validation-leak`** — pressure-test input for
  the presentation/validation path. If any cockpit projection here proposes
  convention-based correlation (e.g. a work item id carried in an opaque
  payload) plus attention-flag validation, the parked finding records that
  `schema_ref` is an uninterpreted string and `AttentionRequired` is a
  signaling record, not a validation engine. A partial validator that
  validates only the cheap half manufactures the durable false confidence it
  was built to prevent. The design must not reach for that pattern without a
  real validation contract.

## Child features reviewed and complete

All five child features are `done` after thorough review:

- `resource-identity` (done) — typed resource identity, target-resolution polymorphism, grant-containment.
- `resource-state` (done) — resource snapshot/revision, typed report ingress, delta folding, replay, LoadSnapshot; thorough convergence review.
- `capability-manifest` (done) — adapter manifest resource-kind/category/projection-schema extensions; target-category registry extensible for the reserved OKF third kind.
- `cockpit-composition` (done) — Resources peer destination, canonical wrapper + local decoders, dual-snapshot atomic reconciliation, runtime-context resource linkage; thorough convergence review (3 passes; 3 contract blockers fixed: mixed diagnostic targets, reverse-horizon oracle, historical diagnostic path).
- `conformance` (done) — executable, mutation-sensitive conformance vectors + property oracles proving a resource adapter cannot bypass authority, durability, or stale-state rules; deep-lane convergence (6 passes; all drift classes now data-driven-guarded).

Advanced to `review` for the deeper aggregate review over the whole agent-operations resource-plane arc.
