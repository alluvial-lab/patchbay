---
id: epic-agent-operations-resource-plane-cockpit-composition
kind: feature
stage: implementing
tags: [foundation, ux, protocol]
parent: epic-agent-operations-resource-plane
depends_on: [epic-agent-operations-resource-plane-resource-identity, epic-agent-operations-resource-plane-resource-state, epic-agent-operations-resource-plane-capability-manifest]
release_binding: null
gate_origin: null
created: 2026-07-30
updated: 2026-08-04
---

# Cockpit resource composition

## Brief

Render operational resources in the cockpit alongside runtime sessions, per
the Phase 4.6 mockup decision: **Resources as a peer destination** surfacing
two resource kinds (pooled token-commune pools + direct-provider usage
windows) under the single admission rule, plus a **session runtime-context
strip** whose resource-linkage cell links to a pool only when the sibling
session-runtime projection says provider = token-commune. Direct-provider
usage windows remain visible in Resources but are not linked from this
feature's session cell.

This feature delivers the resource-side rendering and linkage: the Resources
destination (list + detail, pooled/direct sections, mobile affordances),
`ResourceView`/collection/projection-decoder/navigation/detail-renderer, and
the runtime-context strip's **resource-linkage** (the usage cell). Grant-scope
labels extend to resources. The composition obeys the conformance floor:
resource domain health stays distinct from session connectivity/lifecycle;
stale/unknown/offline resource state never renders as live.

It does **not** own the provider concept itself (model-vs-provider split,
provider-switch reconfigure) — that is sibling session-runtime scope (see
parent epic Mockups). This feature renders the resource linkage the provider
concept consumes; the interactable provider/model pickers depend on sibling
work and are mocked here as the design direction, not implemented by this
feature in isolation.

## Epic context

- Parent epic: `epic-agent-operations-resource-plane`
- Position in epic: the UI-bearing consumer — depends on identity, state, and manifest; the conformance feature closes over it.

## Simplification opportunity

- Reuse the shared presentation-component layer (`StateBadge`, `CommandTimeline`, attention primitives) and the lockdown mockup's rail/destination pattern rather than a new navigation system.
- Keep resource health projection adapter-owned; do not coerce it into session connectivity/activity axes.

## Foundation references

- `docs/UX.md:13-62,85-135` — surface-neutral floor, shared presentation layer, required surfaces
- `docs/ARCHITECTURE.md` — human control surface plane
- `web-cockpit/src/domain/model.ts` — presentation model (session-shaped today; this adds `ResourceView`)
- `web-cockpit/src/ui/shell.ts`, `session-list.ts`, `session-detail.ts` — current session-centric surfaces

## Mockups

- Inherits design system: `.mockups/design-system/tokens.css`, `components.css`
- Screens: `.mockups/screens/epic-agent-operations-resource-plane/index.html`
  - `option-1.html` — Resources destination (pooled + direct-provider sections) — selected direction
  - `session-context.html` — runtime-context strip (interactable; resource-linkage in scope, provider concept sibling)
  - `session-context-mobile.html` — mobile pill-buttons + bottom sheet
- Selected: option-1 navigation + runtime-context strip resource-linkage (2026-07-30)

## Design decisions

- **Compose resource state inside the existing browser presentation projection.** `PresentationModel` gains exact resource records and per-adapter-kind collection revisions; it does not gain a second store, polling cache, or server-translated dashboard DTO. This preserves one cursor/replay authority and keeps the generated resource contracts at the boundary.
- **Reconcile session and resource snapshots atomically at one replay horizon.** The two `LoadSnapshot` calls can observe different global LSNs. Reconciliation installs both snapshots only after replaying the visible durable prefix through `max(session_snapshot_lsn, resource_snapshot_lsn)`, skipping each axis only through its own snapshot LSN. A failed half-read leaves the whole cached projection unreconciled instead of claiming that a mixed-age model is current.
- **Use one closed local decoder registry keyed by the complete projection contract.** A decoder matches `ResourceKind` plus both `(schema_ref, content_type)` envelope descriptors. Known `provider_pool.*.v1` and `usage_window.*.v1` JSON projections decode into a cockpit-local discriminated union. Unknown descriptors or malformed bytes retain the canonical resource wrapper but install no adapter-domain projection and show an explicit unavailable/invalid projection state. Descriptor matching is not presented as semantic validation.
- **Keep open resource kinds separate from the two selected presentation compositions.** `provider_pool` and `usage_window` remain adapter-owned `ResourceKind` strings. `pooled-provider-pool` and `direct-provider-usage-window` are local decoded presentation variants, not new protocol enums or target categories.
- **Resource freshness dominates domain health.** `ResourceFreshnessState` becomes a registry-checked presentation primitive. A cached stale resource may show its last decoded domain values only as last reported; unknown shows no domain health; tombstoned resources are not active. Adapter `offline` remains adapter diagnostics, while resource detach/loss is represented by the core's stale/unknown resource projection. No resource is assigned session connectivity/activity.
- **The canonical wrapper includes Operation delivery and visible grant context.** `CommandView.target` becomes a session/resource union and the existing delivery renderer is extracted for reuse by resource detail. Grant labels and broad/exact scope matching are presentation-only explanations; they never gate an action or claim authority in place of the core.
- **Resources is a real peer destination on desktop and mobile.** It joins the rail and the mobile bottom tabs, with a two-pane desktop list/detail and a mobile list-to-detail drill-in. Unknown/invalid projections remain findable under an unavailable section rather than disappearing.
- **The session strip boundary is resource linkage only.** This feature adds a `SessionResourceLinkage` slot containing an exact resource identity and renders it only when it resolves to a decoded pooled-provider resource. It does not parse the opaque `SessionView.model`, invent `provider`, populate model/context fields, issue `reconfigure`, or implement provider/model pickers. The sibling session-runtime feature supplies the linkage only for `provider = token-commune`; direct-provider windows remain destination-only here.
- **Mobile scope follows the same boundary.** The usage linkage renders as a tappable pill that navigates to the resource. The mockup's adjacent provider/model pills and their bottom sheet are the signed-off sibling seam, not hidden scope for this feature; the implementation preserves compatible strip/slot styling and reuses the existing generic sheet primitive when that sibling lands.
- **Mockups are inherited, not regenerated.** The selected artifacts remain `.mockups/screens/epic-agent-operations-resource-plane/option-1.html`, `session-context.html`, and `session-context-mobile.html`.
- **Autopilot rationale.** These are the least-irreversible choices consistent with the generated resource contracts, the selected mocks, and the explicit provider-scope boundary. The codebase is bounded to the web cockpit plus the presentation checker, so direct reading was used instead of exploratory fan-out. Independent design-time advisory review was warranted by reconciliation and conformance risk but is unavailable in this delegated tool surface; the explicit `thorough` implementation review remains mandatory.

## Architectural choice

### Options considered

1. **Generated resource projection inside the existing model, with dual-snapshot reconciliation and local known compositors (chosen).** Resource events fold beside session events, snapshots repair each axis, and the shell renders canonical wrappers plus decoded local domain projections. This optimizes for cursor correctness, stale-state honesty, and reuse of the existing cockpit architecture. It costs a careful multi-view reconciliation change and a typed command-target refactor.
2. **Load a resource snapshot only when Resources opens.** This is initially smaller, but the destination would miss live resource events, cross-session links could point at an unloaded cache, and opening navigation would become an authority boundary. It also creates two freshness policies in one browser.
3. **Translate adapter projections into a fixed server-side cockpit JSON API.** This could simplify the DOM renderer, but it hand-copies generated contracts, moves surface policy into the web server, and turns every new local compositor into a server contract change. It conflicts with the shared TypeScript operator-domain and local-known-decoder direction.

The chosen approach extends existing seams instead of adding a resource-only client. The trickiest unit is the **resource projection and dual-view reconciliation boundary**: snapshot LSNs can differ, live normalized events must fold identically, and malformed adapter-domain bytes must fail closed without causing a permanent reconnect loop. That unit lands before navigation or rendering.

## Implementation Units

### Unit 1: Resource presentation model and exact local projection decoders

**Files**: `web-cockpit/src/domain/resource-projection.ts` (new), `web-cockpit/src/domain/model.ts`, `web-cockpit/tests/resource-projection.test.ts` (new), `web-cockpit/tests/model.test.ts`

**Story**: `epic-agent-operations-resource-plane-cockpit-composition-resource-projection-domain`

```ts
// web-cockpit/src/domain/resource-projection.ts
import type { PayloadContentType, PayloadEnvelope } from "@patchbay/contracts";

export interface ResourceIdentityView {
  adapterId: string;
  resourceKind: string;
  resourceId: string;
}

export interface ProjectionDescriptor {
  schemaRef: string;
  contentType: PayloadContentType;
}

export interface ProviderPoolProjection {
  kind: "pooled-provider-pool";
  displayName: string;
  providerLabel: string;
  health: "serving" | "degraded" | "exhausted" | "paused" | "unknown";
  remainingPercent?: number;
  resetLabel?: string;
  contributionCount?: number;
  serviceLabel?: string;
  controlPosture: "administration-capable";
}

export interface UsageWindowProjection {
  kind: "direct-provider-usage-window";
  displayName: string;
  providerLabel: string;
  health: "ok" | "low" | "exhausted" | "unknown";
  remainingPercent?: number;
  resetLabel?: string;
  accountLabel?: string;
  planLabel?: string;
  windowLabel?: string;
  burnRateLabel?: string;
  activeSessionCount?: number;
  controlPosture: "read-only";
}

export type DecodedResourceProjection = ProviderPoolProjection | UsageWindowProjection;

export type ResourceProjectionResult =
  | { status: "decoded"; value: DecodedResourceProjection }
  | { status: "unsupported"; projection: ProjectionDescriptor }
  | { status: "invalid"; projection: ProjectionDescriptor; reason: "projection_decode_failed" }
  | { status: "unavailable" };

export interface ResourceProjectionDecoder {
  resourceKind: string;
  resourcePayload: ProjectionDescriptor;
  projectionPayload: ProjectionDescriptor;
  decode(payload: Uint8Array): DecodedResourceProjection;
}

export const RESOURCE_PROJECTION_DECODERS: readonly ResourceProjectionDecoder[];

export function decodeResourceProjection(
  identity: ResourceIdentityView,
  resourcePayload: PayloadEnvelope | undefined,
  projectionPayload: PayloadEnvelope | undefined,
): ResourceProjectionResult;
```

```ts
// additions in web-cockpit/src/domain/model.ts
export interface ResourceCollectionView {
  adapterId: string;
  resourceKind: string;
  completeness: AdapterSnapshotSupport;
  sourceAdapterGeneration: bigint;
  revisionLsn: bigint;
  observedAt?: Date;
  reconciled: boolean;
}

export interface ResourceView {
  identity: ResourceIdentityView;
  freshness: ResourceFreshnessState;
  sourceAdapterGeneration: bigint;
  revisionLsn: bigint;
  observedAt?: Date;
  tombstoned: boolean;
  replacedBy?: ResourceIdentityView;
  hasCachedPayload: boolean;
  reconciled: boolean;
  projection: ResourceProjectionResult;
}

export interface SessionResourceLinkage {
  usageResource: ResourceIdentityView;
}

export function resourceKey(identity: ResourceIdentityView): string;
export function resourceCollectionKey(adapterId: string, resourceKind: string): string;
export function rendersResourceCurrent(resource: ResourceView): boolean;
```

**Implementation notes**:

- The registry contains exactly two v1 decoder descriptors: `provider_pool.payload.v1` + `provider_pool.projection.v1`, and `usage_window.payload.v1` + `usage_window.projection.v1`, all JSON. Registration fails fast on duplicate decoder keys. Adding another local compositor is one registry entry plus decoder tests, never dynamic adapter code.
- Decoder helpers require a JSON object, bounded non-empty labels, finite percentages in `[0, 100]`, non-negative integer counts, and known domain-health values. Malformed types and unknown health values reject the projection. No decoded field selects a CSS class directly; the renderer maps the local union to bounded presentation tones.
- The complete descriptor pair is matched before bytes are decoded. `unsupported` and `invalid` do not retain or expose raw payload text. The canonical identity/freshness/revision wrapper remains available for diagnosis.
- `resourceKey` uses the same length-prefixed collision-proof composition as `sessionKey`; it never joins the tuple with an ambiguous slash.
- `SessionResourceLinkage` deliberately contains no provider/model fields. Its only contract is an exact resource identity supplied by the sibling session-runtime projection.

**Acceptance criteria**:

- [ ] Valid provider-pool and usage-window projections decode to different local variants while `ResourceKind` stays an open string.
- [ ] A wrong resource kind, wrong payload descriptor, wrong projection descriptor, unknown content type, malformed JSON, out-of-range percent, or unknown domain-health value installs no decoded domain projection.
- [ ] Adapter/kind/id collisions produce distinct resource keys.
- [ ] `rendersResourceCurrent` is true only for reconciled, non-tombstoned `CURRENT` records; stale/unknown/unreconciled records cannot receive current styling.

### Unit 2: Resource event folding and atomic multi-view reconnect

**Files**: `web-cockpit/src/domain/model.ts`, `web-cockpit/src/domain/reconcile.ts`, `web-cockpit/tests/model.test.ts`, `web-cockpit/tests/reconcile.test.ts`

**Story**: `epic-agent-operations-resource-plane-cockpit-composition-resource-reconciliation`

```ts
// web-cockpit/src/domain/model.ts
export interface PresentationModel {
  // existing fields
  resources: Map<string, ResourceView>;
  resourceCollections: Map<string, ResourceCollectionView>;
}

export interface SnapshotBaselines {
  session: SessionSnapshot;
  resource: ResourceSnapshot;
}

export function replaceFromSnapshots(
  snapshots: SnapshotBaselines,
  replayEvents: readonly SubscribeEvent[],
): PresentationModel;

export function foldResourceState(
  model: PresentationModel,
  event: ResourceStateEvent,
  lsn: bigint,
): void;
```

```ts
// web-cockpit/src/domain/reconcile.ts
export interface ReconcileProjection {
  markUnreconciled(reason: "stream-break" | "event-gap"): void;
  replaceFromSnapshots(
    snapshots: SnapshotBaselines,
    replayEvents: readonly SubscribeEvent[],
  ): void | Promise<void>;
  replaceSecuritySnapshot?(snapshot: SecuritySnapshot): void | Promise<void>;
  foldEvent(event: SubscribeEvent): void | Promise<void>;
}
```

**Implementation notes**:

- Replace the current `RESOURCE_STATE` decode-and-ignore branch. Snapshot resource records and normalized events go through the same identity, enum, revision, and decoder helpers.
- `foldResourceState` requires the event authority domain to match the enclosing event, every identity adapter to match `source_adapter_id`, every identity kind to have a corresponding view update when required, and `from_revision_lsn` to match the cached record. Upsert sets current with decoded-or-failed projection; unknown clears cached projection; tombstone preserves cached data as stale or no-payload unknown; freshness changes never create payload.
- `markUnreconciled` keeps decoded cached values but changes effective resource confidence to stale (or unknown when no cache), clears collection reconciliation, and never rewrites a domain-health value to session offline/failed.
- Reconciliation requests both `SnapshotViewKind.SESSION` and `SnapshotViewKind.RESOURCE`, verifies each response discriminator/domain/payload LSN, then replays the visible prefix through the larger LSN. Session-state events at or below the session snapshot LSN and resource-state events at or below the resource snapshot LSN are skipped independently; later events for either axis fold normally. Commands, Observations, Elicitations, and lockdown events replay across the whole prefix.
- Build the replacement model off to the side and assign only after both snapshots and replay validate. Continue from the larger LSN; events committed afterward arrive on the next subscription.
- Projection-decoder failure is a resource-local `invalid` result, not a thrown stream-fold error. Missing generated fields, cross-domain data, impossible revisions, or malformed normalized resource events do throw and leave the model unreconciled.

**Acceptance criteria**:

- [ ] Live `RESOURCE_STATE` upsert/freshness/unknown/tombstone/replacement events update only their exact identity and collection.
- [ ] Restart repair with unequal session/resource snapshot LSNs produces the same final presentation state as folding the durable visible prefix once.
- [ ] A failed second snapshot or invalid replay never installs a half-reconciled model or advances the cursor.
- [ ] Stream break marks current cached resources stale and no-payload resources unknown until the atomic snapshot replacement succeeds.
- [ ] Existing session, command, Elicitation, diagnostics, security, and filtered-LSN-hole reconciliation behavior remains green.

### Unit 3: Shared target and Operation-delivery presentation

**Files**: `web-cockpit/src/domain/model.ts`, `web-cockpit/src/ui/operation-delivery.ts` (new), `web-cockpit/src/ui/session-detail.ts`, `web-cockpit/tests/model.test.ts`, `web-cockpit/tests/shell.test.ts`

**Story**: `epic-agent-operations-resource-plane-cockpit-composition-shared-resource-rendering`

```ts
// web-cockpit/src/domain/model.ts
export type OperationTargetView =
  | { kind: "runtime-session"; identity: SessionIdentity }
  | { kind: "operational-resource"; identity: ResourceIdentityView };

export interface CommandView {
  // existing fields
  target?: OperationTargetView;
}

export function operationTargetFromScope(scope: TargetScope | undefined): OperationTargetView | undefined;
```

```ts
// web-cockpit/src/ui/operation-delivery.ts
export interface OperationDeliveryActions {
  cancel?(command: CommandView): void | Promise<void>;
  interrupt?(command: CommandView): void | Promise<void>;
}

export function renderOperationDelivery(
  document: Document,
  command: CommandView,
  actions?: OperationDeliveryActions,
  lockdownActive?: boolean,
): HTMLElement;

export function operationStateName(state: OperationState): string;
export function operationKindLabel(kind: OperationKind): string;
```

**Implementation notes**:

- Parse generated target scopes by `TargetScopeKind`; resource targets require the complete nested adapter/kind/id tuple and reject mixed/partial shapes. Session-only Observation/Elicitation helpers continue to require a runtime-session target rather than accepting the new union accidentally.
- Extract the existing delivery line, failure vocabulary mapping, command-state label mapping, and contextual cancel/interrupt controls without changing their behavior or state CSS. Session detail consumes the extracted component; resource detail can render accepted resource Operations with the same lifecycle and failure semantics.
- Resource UI still introduces no administrative submission builder. Future pool Operations appear correctly once another feature submits them because the durable command projection is already target-polymorphic.

**Acceptance criteria**:

- [ ] Resource-target Operations project with their exact resource tuple and appear only in that resource's timeline.
- [ ] Session-target Operations and cancel/interrupt actions retain existing behavior.
- [ ] Partial, legacy audit-only, or mixed resource target scopes never become a `CommandView` resource target.
- [ ] Operation lifecycle, failure, and retry semantics are rendered by one shared component, not a resource-specific copy.

### Unit 4: Resources destination, detail wrapper, grant labels, and responsive linkage

**Files**: `web-cockpit/src/ui/resource-view.ts` (new), `web-cockpit/src/ui/target-scope.ts` (new), `web-cockpit/src/ui/runtime-resource-link.ts` (new), `web-cockpit/src/ui/shell.ts`, `web-cockpit/src/ui/security-view.ts`, `web-cockpit/src/ui/session-detail.ts`, `web-cockpit/src/ui/icons.ts`, `web-cockpit/src/ui/shell.css`, `contracts/scripts/check-presentation.mjs`, `.mockups/design-system/components.css`, `.mockups/design-system/components.html`, `docs/UX.md` (generated traceability block only), `web-cockpit/tests/resource-view.test.ts` (new), `web-cockpit/tests/security-view.test.ts`, `web-cockpit/tests/shell.test.ts`

**Story**: `epic-agent-operations-resource-plane-cockpit-composition-session-resource-linkage`

```ts
// web-cockpit/src/ui/resource-view.ts
export interface ResourceDestinationOptions {
  selectedKey?: string;
  mobileDetailOpen: boolean;
  lockdownActive: boolean;
  onSelect(resource: ResourceView): void;
  onBack(): void;
}

export interface ResourceDestinationComponent {
  readonly element: HTMLElement;
  setMobile(mobile: boolean): void;
}

export function renderResourceDestination(
  document: Document,
  model: PresentationModel,
  options: ResourceDestinationOptions,
): ResourceDestinationComponent;
```

```ts
// web-cockpit/src/ui/target-scope.ts
export function formatTargetScope(scope: TargetScope | undefined): string;
export function scopeMayContainResource(
  scope: TargetScope | undefined,
  identity: ResourceIdentityView,
): boolean;

// web-cockpit/src/ui/runtime-resource-link.ts
export interface RuntimeResourceLinkOptions {
  resource: ResourceView | undefined;
  onOpen(identity: ResourceIdentityView): void;
}

export function renderRuntimeResourceLink(
  document: Document,
  options: RuntimeResourceLinkOptions,
): HTMLElement;
```

**Implementation notes**:

- Add `resources` to `CockpitDestination`, the desktop rail, and the mobile bottom tabs. `CockpitShell` keeps a selected resource key, supports `openResource(identity)`, and gives Resources its own mobile list/detail drill-in without reusing or duplicating the session sidebar.
- The active list groups decoded pooled pools and direct usage windows in the selected order. Invalid/unsupported/unknown projections appear in an explicit unavailable section. Tombstones are hidden from the active list but remain addressable from a stale link/replacement detail.
- Every detail begins with the canonical wrapper: full resource identity, source adapter generation, resource and collection revision LSNs, observed time, snapshot completeness, freshness, replacement/tombstone state, visible matching grant scopes, and resource-target Operation delivery. Adapter-domain cards render beneath it only for `decoded` projections.
- Domain-health labels are always subordinate to freshness. Stale cached values use “last reported”; unknown renders no meter/health; a tombstone renders retired/replaced context. Read-only versus administration-capable is descriptive projection posture only. No control is enabled from that value.
- `formatTargetScope` replaces the private session-only formatter in `security-view.ts` and adds exact resource labels. `scopeMayContainResource` recognizes exact resource, same-adapter, fleet, and authority-domain scopes only for explanatory grant listing; it is named `MayContain` and accompanied by “core enforced” copy so it cannot be mistaken for an authorization decision.
- Add `ResourceFreshnessState` to the registry-derived presentation checker with `.resource-freshness--current|stale|unknown` CSS and concrete showcase examples. `shell.css` owns layout and adapter-domain health styles; it must not rebind canonical freshness members.
- The session usage fragment is rendered only when `SessionView.resourceLinkage` exists and resolves to a decoded `pooled-provider-pool`. Current and stale pools remain navigable with honest freshness text; missing/tombstoned/invalid targets render a disabled unavailable pill. A direct `usage_window` identity is never turned into a link by this component.
- On mobile the usage fragment is a pill-button and Resources drills list → detail. The provider/model pills and bottom-sheet picker from `session-context-mobile.html` remain a documented sibling slot; this feature neither adds placeholder pickers nor issues `reconfigure`.

**Acceptance criteria**:

- [ ] Desktop and mobile expose Resources as a peer destination; mobile list/detail and back navigation use the same `ResourceView` renderer.
- [ ] Pooled and direct projections render the selected mock composition; invalid/unsupported resources remain visible without exposing raw bytes.
- [ ] Full resource identity precedes any future action, and resource Operation delivery uses canonical lifecycle/failure components.
- [ ] Current/stale/unknown freshness has registry/CSS/showcase parity; stale, unknown, tombstoned, and unreconciled resources cannot look current.
- [ ] Security and resource detail show exact resource grant-scope labels while UI grant matching remains non-authoritative.
- [ ] A token-commune pool linkage opens the exact Resources detail; direct-provider windows never receive a session link in this feature.
- [ ] Existing Sessions/Security destinations, panel-collapse preferences, lockdown read-only behavior, and mobile More menu remain green.

## Implementation Order

1. `epic-agent-operations-resource-plane-cockpit-composition-resource-projection-domain`
2. `epic-agent-operations-resource-plane-cockpit-composition-resource-reconciliation` after the local model/decoder contract is stable.
3. `epic-agent-operations-resource-plane-cockpit-composition-shared-resource-rendering` after resource targets exist in the presentation model.
4. `epic-agent-operations-resource-plane-cockpit-composition-session-resource-linkage` after reconciliation and shared Operation rendering can support the complete destination.
5. Advance child stories directly to `done` on green checkpoint evidence, then review the integrated feature at effective weight `thorough` until a fresh-context pass yields no receiver-confirmed material current-cycle blocker.

One feature owner should normally carry all four checkpoints: `model.ts`, reconciliation, shell state, and renderer tests overlap, and splitting workers would create more merge risk than parallelism value.

## Simplification

- Replace the temporary `RESOURCE_STATE` decode-and-ignore branch with the real projection; do not leave a shadow compatibility path.
- Reuse `PresentationModel`, one durable cursor, generated `ResourceSnapshot`/`ResourceStateEvent`, the shell rail/bottom tabs, existing mobile drill-in/sheet patterns, locked design tokens, and shared delivery primitives.
- Extract the existing Operation-delivery renderer and target-scope formatter instead of copying them into resource UI.
- Do not add a server endpoint, protocol field, resource-specific command state, resource connectivity/activity enum, polling timer, browser persistence cache, dynamic adapter renderer, or provider/model reconfigure code.
- Keep unknown/invalid local projection handling as one canonical-wrapper fallback rather than one fallback per adapter.

## Testing

- **Decoder boundary tests** protect exact kind + dual-descriptor matching and semantic JSON validation. Their value is preventing untrusted adapter bytes from becoming rendered domain state.
- **Projection/reconnect interface tests** protect live/snapshot equivalence, unequal snapshot horizons, full-tuple identity, revision checks, tombstone finality, and stale dominance. This is the highest-risk surface.
- **UI integration tests** protect peer navigation, pooled/direct/unavailable grouping, canonical wrapper fields, resource Operation delivery, mobile drill-in, and the token-commune-only linkage seam.
- **Grant-label tests** protect complete resource identity display and explicitly broad/exact explanatory scopes without using those helpers for action availability.
- **Presentation conformance** protects exact `ResourceFreshnessState` proto/CSS/showcase parity and accessibility through the existing checker. No new test is added for trivial DOM helpers or generated serialization.
- **Regression suite**: `cd web-cockpit && npm test`, `node contracts/scripts/check-presentation.mjs`, contract generated drift/build, and repository model/vector metadata checks. No existing test should be deleted or weakened; there is no obsolete test identified for removal at design time.

## Risks

- **Two snapshot RPCs do not share one transaction.** Without per-axis replay thresholds, a later resource snapshot could make session state falsely current through the same cursor. The atomic max-horizon algorithm is mandatory; fallback is to keep the entire cached projection unreconciled, not install half the reads.
- **The first real adapters may refine projection v1 fields.** The decoder registry is intentionally local and versioned. Unsupported descriptors get a generic canonical wrapper until the adapter and cockpit add a reviewed decoder together; the fallback is never dynamic code or best-effort JSON probing.
- **Malformed durable domain payload can be replayed forever.** Semantic decoder failures become local invalid projections so the stream continues safely. Only malformed generated resource events break reconciliation, correctly surfacing core/storage corruption instead of hiding it.
- **The provider sibling is not yet represented in the session wire/model.** This feature must not infer provider from `provider/modelId`. The usage link remains unpopulated in production until the sibling supplies `SessionResourceLinkage`; renderer fixture coverage proves the seam without pretending provider switching shipped.
- **Grant summaries can be mistaken for authority.** Resource detail labels them as visible snapshot context, uses no grant result to enable actions, and keeps every future mutation on the existing Submit/core authorization path.
- **Command target polymorphism can regress session rendering.** The discriminated union and shared renderer land before the destination; existing session command/timeline/property-style tests remain required.

## Extension pressure classification

- **Committed post-v0.1 direction:** a Resources peer destination; exact resource collection and detail projection; local schema-bound pooled/direct compositors inside the canonical identity/revision/freshness/grant/Operation wrapper; registry-derived resource-freshness presentation; responsive web/mobile composition; token-commune pool linkage from a session-provided resource slot.
- **Reserved seams:** additional local decoders, direct-provider runtime linkage, provider/model/context session metadata, provider-switch/model-set `reconfigure`, provider/model mobile bottom sheets, native Expo composition, per-resource action catalogs, and dynamically registered local renderer catalogs. Promotion requires the owning adapter/session-runtime contract.
- **Explicitly rejected for this arc:** inferring provider from the opaque model string, linking direct usage windows from this feature, adapter-supplied HTML/CSS/code, capability- or UI-derived authority, coercing resource health into session axes, silently hiding unknown schemas, and a second browser/server resource cache.

## Other agent review

- Invoked because: this is a UI-bearing foundation feature with multi-view reconciliation, untrusted projection decoding, and presentation-conformance implications.
- Fixed/active blockers: the design closes mixed-snapshot cursor skew, stale-domain-health dominance, unknown-schema fallback, resource command lifecycle reuse, grant-label non-authority, and the provider-scope boundary.
- Parked: none from this design pass; provider/model/session-runtime behavior is already an explicit sibling seam in the parent and this item.
- Rejected: on-open polling, server-translated cockpit DTOs, direct-provider session links, and dynamic adapter renderers.
- Skipped/degraded: this delegated worker exposes no independent subagent/peer dispatch mechanism, so design-time advisory review could not run. Direct source verification and the pre-mortem above were completed. Effective implementation review weight is `thorough`, explicitly supplied by the autopilot caller, and is not degraded.
