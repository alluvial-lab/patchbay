---
id: epic-token-commune-observer-cockpit-panel
kind: feature
stage: review
tags: [adapter, ux]
parent: epic-token-commune-observer
depends_on: [epic-token-commune-observer-snapshot-mapping, epic-token-commune-observer-polling-ingestion]
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-08
---

# token-commune cockpit resource panel and CLI projection

## Brief

The **surface-declared** token-commune resource panel, composed above the
conformance floor using Patchbay's shared presentation primitives, plus CLI
text-table projections over the same metadata. This is the UI-bearing feature of
the epic and the one net-new screen surface.

It delivers: a calm **per-provider** panel — one row per provider, each
showing commune health (pool capacity remaining + fresh/exhausted/auth_broken
state + reset), the operator's **per-provider draw** (`limitFraction` +
`consumedUnits`), model availability, and fingerprint state — nested beneath
the canonical Patchbay wrapper via a local known decoder/compositor for the
manifest-bound projection schema (never loading adapter-supplied renderer code).
There is deliberately **no aggregate-draw hero**: draw is meaningfully
per-provider (an operator can be flush on anthropic and dry on openai-codex),
and `/commune/me` already returns draw as a per-provider array. Per-pool
**contributions are shown as unattributed aggregates** (count + total declared
share) with an honest "contributors not exposed" note until token-commune adds
attribution (the lead external prerequisite); the contributor roster is an
additive future promotion, not blocked on this feature. Grant-gated member/admin
view affordances apply as local defense-in-depth (upstream has no read-scope
distinction today). Honest stale/unknown/partial presentation never styles stale
data as live. CLI query/inspect projections are text tables over the same
metadata. (Draw-enforcement/calibration status is reported honestly when present
but is **not** a required UI element.)

It does NOT cover mutations, approval cards, re-onboarding elicitations, or
admin command affordances — those belong to the `control-attention` epic and are
out of scope for the read-only observer.

## Epic context

- Parent epic: `epic-token-commune-observer`
- Position in epic: **presentation consumer** — consumes the flowing resource
  state + Observations from `snapshot-mapping` / `polling-ingestion`. The
  conformance evidence in `conformance` proves the presentation honesty.

## Simplification opportunity

- Reuse the shared presentation-component layer (`StateBadge`,
  `CommandTimeline`, resource projection decoders, the cockpit-composition
  primitives from the resource-plane epic) — Tier-1 floor affordances come free;
  only the token-commune-specific domain projection decoder + panel layout is
  new.
- Do not duplicate allocation/quota/role logic in Patchbay; the panel renders
  adapter-reported metadata only.

## Foundation references

- `docs/UX.md` — surface-declared affordances compose above the conformance
  floor; the presentation conformance check enforces the floor structurally.
- `docs/ARCHITECTURE.md` — adapter-shaped domain projections compose above, not
  instead of, the canonical wrapper; Patchbay does not load adapter-provided
  renderer code.
- `docs/SECURITY.md` — member/admin visibility is governed by both upstream
  credentials and Patchbay grants; only metadata flows.
- Blueprint: `web-cockpit/` (resource projection decoders, cockpit composition
  from the resource-plane epic); `cli/`.

## Mockups

- Screens: `.mockups/screens/epic-token-commune-observer-cockpit-panel/option-7.html` — **selected MVP direction** (2026-08-05)
- Comparison set: `option-1`..`option-6` are exploratory iterations (busy → calm → per-provider); `option-7` is canonical.
- Design system: inherits `.mockups/design-system/tokens.css` + `components.css` (Nostromo/LCARS amber console).

**Selected direction — calm per-pool list (MVP).** One row per provider-pool, three Patchbay-summarized signals:
1. **draw allowance** — `limitFraction` from `/commune/me`; the operator's per-provider allowance against others' pooled capacity (may be admin-set via decree).
2. **credential-health distribution** — count of the pool's contributions by health state (fresh / exhausted / auth_broken); native token-commune data.
3. **capacity** — the highest `5h`-window `usedFraction` among the pool's anonymous contributions; `5h` is Patchbay's display window, not necessarily the provider's binding window.

Plus a Patchbay-synthesized verdict (runnable / pool exhausted / telemetry stale / auth broken) — owned as a synthesis of credential health + capacity + model availability, not a native state.

**Honesty model (locked during mockup):**
- **No derived pool-aggregate %.** A pool-level "% remaining" was explicitly rejected as a fabricated metric; capacity shows only a real per-window reading (highest 5h utilization), honestly labeled.
- Capacity readings are per-contribution × per-window × individually nullable; null/stale/auth-broken states render distinctly (e.g. "no readings", "7m old · stale").
- Credential freshness vs telemetry staleness are distinct axes (a pool can be credential-fresh with stale capacity telemetry) — never presented as contradictory.
- Model IDs come from the live `/v1/models` catalog (the mock's are illustrative placeholders; note `gpt-5.6` aliases are rejected upstream — use `gpt-5.5` / `gpt-5.3-codex-spark` etc.).
- The footer owns every derivation ("Patchbay summaries from per-contribution readings; no native pool aggregate; verdicts are a Patchbay synthesis; polled/partial; contributor identities + stable contribution IDs not exposed").

**Out of MVP (parked):**
- Per-contribution × per-window drill-down — omitted by Patchbay MVP choice (anonymous per-contribution readings already exist upstream; the drill-down is buildable now and gains contributor names when attribution lands).
- Draw-enforcement/calibration status — reported honestly when present but **not** a required UI element.

Adversarially reviewed (cross-model) + visually self-verified via headless render; passed.

## Design decisions

- **One Patchbay-owned compositor serves web and CLI:** add a small pure `@patchbay/operator-domain` package for the manifest-bound token-commune decoder and synthesis. The cockpit and CLI adapt canonical resource views into this module rather than maintaining two verdict implementations.
- **Reuse the existing data ingress:** snapshot and live `RESOURCE_STATE` folds already call `web-cockpit/src/domain/resource-projection.ts::decodeResourceProjection`. Extend that local registry with the two exact token-commune contracts, then compose provider-pool and member-draw records from `PresentationModel.resources`; add no HTTP route, browser poller, server DTO, or second cache.
- **Join only on adapter id plus exact native provider:** one decoded `token-commune.provider-pool` anchors a row. Member draw joins only under the same adapter with equal native `provider`. Resource-id parsing, display/member names, and array position never establish the join.
- **No fabricated stale threshold:** display native reading age from `observedAt`; call it stale only when its canonical wrapper is stale/unreconciled. The source declares no minutes-based telemetry SLA.
- **Draw ambiguity fails closed:** exactly one same-provider report yields `limitFraction`, `consumedUnits`, and reset. Zero is unavailable; multiple reports are ambiguous. Never average, sum, or silently select.
- **Fail-closed verdict companions:** the selected outcomes remain `runnable`, `pool-exhausted`, `telemetry-stale`, and `auth-broken`. `model-unavailable` and `unknown` cover evidence that cannot honestly fit those four; neither is mislabeled healthy, exhausted, or stale.
- **Local grant gating is deny-by-default defense-in-depth:** a row needs a visible live `query` grant whose strict scope contains the pool; member draw is withheld independently unless its resource is covered. Core subscription/snapshot authorization remains authoritative. No trustworthy upstream member/admin role is projected, so this observer infers no admin mode and exposes no admin controls.
- **CLI reads canonical snapshots:** add `resource-query` and `resource-inspect` over `LoadSnapshot(RESOURCE)` plus the security snapshot for local gating. Human output is text tables; JSON exposes safe summaries, never raw envelopes, member names, contribution keys/ids, or credentials.
- **Dispatch rationale:** direct-read only. The surface is bounded to the landed resource projection, token-commune contracts, shell, and CLI snapshot precedent. This worker exposes no independent subagent/peer mechanism; Part IV advisory review is recorded as degraded and non-blocking, not mislabeled independent.

## Architectural choice

1. **Shared pure local decoder/compositor with surface adapters (chosen).** A small Patchbay-owned TypeScript package validates both manifest-bound JSON projections, joins canonical wrappers, calculates the three signals, and synthesizes the verdict. Web and CLI own only snapshot/view adaptation and rendering. This gives one honesty rule set without loading adapter renderer code.
2. **Separate web and CLI compositors.** Fewer package files initially, but two copies of descriptor matching, null taxonomy, highest-5h selection, and verdict precedence would drift.
3. **Server-translated dashboard DTO.** One response for both surfaces, but surface policy would become a server/core contract and bypass the resource projection seam.

The first approach is the least irreversible sound option and realizes the existing shared-operator-domain seam with one bounded module. The trickiest unit is the compositor: it joins independently partial kinds, retains separate freshness axes, selects a real per-contribution 5h reading without manufacturing a pool aggregate, and yields a deterministic verdict from incomplete evidence.

## Honesty and synthesis rules

### Three displayed signals

1. **Draw allowance:** preserve `limitFraction` and format it as a percent only at presentation; retain `consumedUnits` and nullable reset. Never aggregate providers or multiple reports.
2. **Credential health:** independently recount native `fresh`/`exhausted`/`auth_broken` rows and reject supplied counts that disagree. Render anonymous counts only; never expose `subKey`, contribution ids, owners, health reasons, or identity-like hashes.
3. **Highest 5h utilization:** inspect only anonymous pool contributions' native readings with `window === "5h"` and non-null `usedFraction`; select the maximum real fraction. Ties choose newest valid `observedAt`, then stable input order. Distinguish no listing, no 5h readings, all-null 5h readings, stale cached reading, and current reading. Never convert the maximum to pool remaining, sum, mean, or weighted aggregate.

Model labels preserve every exact currently decoded `/v1/models` id together with its native `available` boolean, so an unavailable model can still be labeled honestly. Only `available === true` rows satisfy runnable evidence. No alias is synthesized; Patchbay displays `gpt-5.6` only if that exact id somehow arrives from the live catalog, never as a local alias.

### Verdict rule

Apply this precedence:

1. `telemetry-stale` when the pool wrapper is stale, tombstoned, or unreconciled.
2. `unknown` when the wrapper/projection is unknown/invalid or required contribution/model slices are unavailable.
3. `auth-broken` when no contribution is fresh and at least one is natively `auth_broken`.
4. `model-unavailable` when the model catalog is reported with no available model.
5. `pool-exhausted` when at least one contribution is listed and every listed contribution is exhausted, or when at least one contribution is listed and every listed contribution has a non-null native 5h reading at `usedFraction === 1`.
6. `runnable` only with at least one fresh contribution, one exact available model, and one native 5h reading below 1.
7. `unknown` for all remaining partial or contradictory evidence.

Draw does not determine the pool verdict: it is the operator's allowance, not evidence of pool capability. The footer states the rule and owns every synthesis.

## Stable interfaces and implementation units

### Unit 1: Shared manifest-bound decoder

**Files:** `operator-domain/package.json` (new), `operator-domain/tsconfig.json` (new), `operator-domain/src/token-commune.ts` (new), `operator-domain/tests/token-commune.test.ts` (new), both consumer `package.json`/lockfiles.

**Story:** `epic-token-commune-observer-cockpit-panel-projection-decoder`

```ts
import { AdapterSnapshotSupport, ResourceFreshnessState, type PayloadEnvelope } from "@patchbay/contracts";

export const TOKEN_COMMUNE_PRESENTATION_CONTRACT = {
  providerPool: {
    resourceKind: "token-commune.provider-pool",
    payloadSchema: "patchbay.token_commune.provider_pool.payload.v1",
    projectionSchema: "patchbay.token_commune.provider_pool.projection.v1",
  },
  memberDraw: {
    resourceKind: "token-commune.member-draw",
    payloadSchema: "patchbay.token_commune.member_draw.payload.v1",
    projectionSchema: "patchbay.token_commune.member_draw.projection.v1",
  },
} as const;

export interface SurfaceResourceIdentity { adapterId: string; resourceKind: string; resourceId: string }

export type TokenCommuneProjection =
  | { kind: "token-commune-provider-pool"; provider: string;
      contributionListing: ContributionListing; credentialHealthCounts: CredentialHealthCounts;
      statusTelemetry: ProviderStatusTelemetry; modelCatalog: ProviderModelCatalog;
      fingerprint: ProviderFingerprint; capacityAggregation: "none" }
  | { kind: "token-commune-member-draw"; memberDisplayName: string;
      provider: string; reports: readonly MemberDrawReport[] };

export type TokenCommuneDecodeResult =
  | { status: "decoded"; value: TokenCommuneProjection }
  | { status: "invalid"; reason: "projection_decode_failed" }
  | { status: "unsupported" }
  | { status: "unavailable" };

export function decodeTokenCommuneProjection(
  identity: SurfaceResourceIdentity,
  resourcePayload: PayloadEnvelope | undefined,
  projectionPayload: PayloadEnvelope | undefined,
): TokenCommuneDecodeResult | undefined;
```

**Notes and acceptance:** return `undefined` only for a non-token kind. Match exact kind plus both JSON descriptors before parsing. Validate bounded strings, closed discriminants, finite fractions `[0,1]`, non-negative safe units/counts, RFC 3339 times, nullable reading fields, exact model booleans, and literal `capacityAggregation: "none"`. Invalid output retains no raw bytes. Exact landed descriptors decode; every mismatch is unsupported; contradictory health counts/rows and malformed data fail closed.

### Unit 2: Pool-signal compositor

**Story:** `epic-token-commune-observer-cockpit-panel-pool-compositor`

```ts
export interface TokenCommuneResourceInput {
  identity: SurfaceResourceIdentity;
  freshness: ResourceFreshnessState;
  completeness: AdapterSnapshotSupport;
  observedAt?: Date;
  reconciled: boolean;
  tombstoned: boolean;
  projection: TokenCommuneDecodeResult;
}

export type DrawAllowance =
  | { state: "current" | "stale"; limitFraction: number; consumedUnits: number; resetsAt: string | null }
  | { state: "unavailable" | "ambiguous" | "unknown" };
export type Capacity5h =
  | { state: "current" | "stale"; usedFraction: number; observedAt: string; resetsAt: string | null }
  | { state: "no-5h-readings" | "reading-unavailable" | "unknown" };
export interface CredentialHealthSummary {
  state: "current" | "stale" | "unknown";
  fresh: number; exhausted: number; authBroken: number; contributionCount: number;
}
export type TokenCommuneVerdict = "runnable" | "pool-exhausted" | "telemetry-stale" | "auth-broken" | "model-unavailable" | "unknown";
export interface TokenCommunePoolSummary {
  key: string; provider: string; poolIdentity: SurfaceResourceIdentity; drawIdentity?: SurfaceResourceIdentity;
  completeness: AdapterSnapshotSupport; poolObservedAt?: Date; drawObservedAt?: Date;
  draw: DrawAllowance; credentials: CredentialHealthSummary; capacity5h: Capacity5h;
  models: readonly { id: string; available: boolean }[];
  modelState: "current" | "stale" | "unknown";
  verdict: TokenCommuneVerdict;
}
export function composeTokenCommunePools(resources: readonly TokenCommuneResourceInput[]): readonly TokenCommunePoolSummary[];
```

**Notes and acceptance:** sort by `(adapterId, provider)`; join only exact adapter/provider. Do not use member name, resource-id parsing, contribution subkeys, or array position. Distinguish stale/current per axis and ambiguous draw. Output contains one selected native 5h fraction, every exact model id with its availability, and no capacity aggregate or contributor identity.

### Unit 3: Verdict synthesis

**Story:** `epic-token-commune-observer-cockpit-panel-verdict-synthesis`

```ts
export function synthesizeTokenCommuneVerdict(input: {
  poolCurrent: boolean;
  sourceEvidenceComplete: boolean;
  credentials: CredentialHealthSummary;
  capacity5h: Capacity5h;
  contributionCapacityFacts: readonly {
    health: "fresh" | "exhausted" | "auth_broken";
    fiveHourUsedFraction: number | null | undefined;
  }[];
  modelState: "current" | "stale" | "unknown";
  availableModelCount: number;
}): TokenCommuneVerdict;
```

**Acceptance:** exact precedence above; stale dominates positive styling; one maximum at 100% does not exhaust a pool if another contribution is usable; runnable requires health, capacity, and model evidence; draw is excluded.

### Unit 4: Cockpit data and grant integration

**Files:** `web-cockpit/src/domain/resource-projection.ts`, `domain/model.ts`, `ui/resource-view.ts`, `ui/target-scope.ts`, and corresponding tests.

**Story:** `epic-token-commune-observer-cockpit-panel-cockpit-integration`

```ts
export type DecodedResourceProjection = ProviderPoolProjection | UsageWindowProjection | TokenCommuneProjection;
export interface TokenCommunePanelInput {
  summaries: readonly TokenCommunePoolSummary[]; refreshedAt?: Date; partial: boolean; selectedKey?: string;
}
export function resourceHasLocalQueryAffordance(
  model: PresentationModel, identity: ResourceIdentityView, now: Date,
): boolean;
export function tokenCommunePanelInput(
  model: PresentationModel, selectedKey: string | undefined, now: Date,
): TokenCommunePanelInput;
```

**Notes and acceptance:** delegate recognized descriptors from the existing closed decoder. Adapt active token records and exact collection completeness; unknown/invalid/stale records remain visible as honest summaries. A live non-expired grant must contain `OperationKind.QUERY` and a strict exact/adapter/fleet/domain scope; check pool and draw independently. Non-token resources retain generic rendering. Recognized token resources are not duplicated into generic detail; `openResource` selects/highlights the row. Snapshot and live folds use the same existing data path; colliding provider labels across adapters do not join.

### Unit 5: Option-7 panel component

**Files:** `web-cockpit/src/ui/token-commune-panel.ts` (new), `ui/resource-view.ts`, `ui/shell.css`, `tests/token-commune-panel.test.ts` (new), resource/shell tests.

**Story:** `epic-token-commune-observer-cockpit-panel-panel-component`

```ts
export interface TokenCommunePanelOptions extends TokenCommunePanelInput { formatNow?: Date }
export function renderTokenCommunePanel(document: Document, options: TokenCommunePanelOptions): HTMLElement;
```

**Notes and acceptance:** reproduce option-7's eyebrow, heading, verdict counts, responsive provider rows, exact column order, labels, and calm chrome. Preserve exact live model ids and native availability, including an honestly labeled unavailable model. Stale rows use established stale surface and last-reported/age text; unknown gets no positive indicator. Credential `fresh` remains a native axis independent of capacity staleness. Footer states: native `limitFraction`; maximum real anonymous 5h reading; 5h is Patchbay's display window; verdict precedence is Patchbay synthesis; polling/PARTIAL and wrapper/reading ages; no native aggregate, contributor identity/stable id, or MVP drill-down. Rows are not contribution controls. Accessible text retains canonical identity/freshness/completeness without anonymous keys. Tests distinguish stale, unknown, null/no-reading, auth-broken, model-unavailable, exhausted, and runnable; prohibit member/subkey/raw JSON/aggregate exposure; and prove aliases are not fabricated while allowing any exact id present in the source fixture.

### Unit 6: CLI query and inspect

**Files:** `cli/src/commands/resources.ts` (new), `cli/src/commands/token-commune-projection.ts` (new), `commands/diagnostics.ts`, `main.ts`, `output.ts`, `tests/resource-projection.test.ts` (new), diagnostic tests.

**Story:** `epic-token-commune-observer-cockpit-panel-cli-projection`

```ts
export interface ResourceQueryOptions { adapterId?: string; provider?: string; json: boolean }
export interface ResourceInspectOptions { identity: string; json: boolean }
export async function resourceQueryCommand(
  client: Pick<ControlClient, "loadSnapshot" | "loadSecuritySnapshot">,
  authorityDomainId: string, options: ResourceQueryOptions, output: CliOutput,
): Promise<number>;
export async function resourceInspectCommand(
  client: Pick<ControlClient, "loadSnapshot" | "loadSecuritySnapshot">,
  authorityDomainId: string, options: ResourceInspectOptions, output: CliOutput,
): Promise<number>;
export function parseCanonicalResourceIdentity(value: string): SurfaceResourceIdentity;
```

**Notes and acceptance:** `resource-query [--adapter-id ID] [--provider PROVIDER] [--json]` validates resource/security snapshot framing, locally grant-filters, and prints `PROVIDER`, `DRAW`, `CREDENTIALS`, `5H CAPACITY`, `VERDICT`, `FRESHNESS`, `MODELS`. Empty authorized results succeed explicitly. `resource-inspect <adapter=...;resource-kind=...;resource=...> [--json]` prints canonical identity/revision/completeness/times first, then identical signals/verdict and derivation note; it is not a contribution drill-down. Extract diagnostics' existing percent-encoded resource parser. JSON uses decimal strings/RFC3339/null and never raw bytes/subkeys/member identity. CLI and web call the shared compositor.

### Unit 7: Mutation-sensitive honesty evidence

**Files:** shared-module, cockpit, and CLI tests above.

**Story:** `epic-token-commune-observer-cockpit-panel-honesty-evidence`

**Acceptance:** focused witnesses fail if production averages/sums/inverts capacity; treats null as zero; selects a non-5h window; styles stale current; joins provider labels across adapters; picks the first divergent draw report; trusts supplied health counts; ignores model availability; lets fresh wording override stale wrapper; exposes a subkey/member name; or removes required footer derivations. Run shared package build/tests, full cockpit and CLI tests, contract drift/presentation checks, and `git diff --check`. No formal promotion is claimed for synthesis/UI.

## Implementation order

1. `projection-decoder`
2. `pool-compositor` depends on `projection-decoder`
3. `verdict-synthesis` depends on `pool-compositor`
4. `cockpit-integration` depends on `verdict-synthesis`
5. `panel-component` depends on `cockpit-integration`
6. `cli-projection` depends on `verdict-synthesis` and may proceed beside web integration under the same owner
7. `honesty-evidence` depends on `panel-component` and `cli-projection`

These are durable checkpoints, not seven worker assignments. One feature owner should carry the shared decoder, browser adapter, CLI adapter, and parity evidence coherently.

## Simplification

- Reuse canonical resource snapshot/live folds, `ResourceView`, collection completeness, freshness, Resources destination, strict scope helpers, grant inventory, generated contracts, table printer, and design tokens.
- Extract the CLI canonical resource parser instead of duplicating its grammar.
- Replace illustrative generic token-pool rendering only for recognized token resources; retain generic code for genuinely different contracts.
- Add no core/server schema, RPC, database, browser polling, adapter import, dynamic renderer, remaining-capacity meter, contribution drill-down, mutation/admin action, inferred role, or enforcement UI.
- No existing test is obsolete; generic resource tests remain regression coverage.

## Testing

- Shared decoder fixtures protect exact dual descriptors, semantic validation, byte non-retention, health recount, exact model ids, and draw ambiguity.
- Compositor tests protect join isolation, separate ages/freshness, null/no-5h/no-listing taxonomy, maximum-native-5h selection, no aggregate, and verdict precedence.
- Component tests protect option-7 labels/order/footer and absence of forbidden contributor, aggregate, alias, member, and raw-payload text.
- Grant tests protect pool versus draw gating, broad/exact scopes, expiry/revocation, and no locally fabricated authority/admin action.
- CLI tests protect snapshot framing, identity parsing, table/JSON redaction, not-found behavior, and shared-summary parity.
- Verification commands: operator-domain build/tests; `cd web-cockpit && npm test`; `cd cli && npm test`; contract drift/presentation checks; `git diff --check`.

## Risks

- **Independent partial axes:** pool and draw can disagree in age/presence. Preserve both; a current pool may show stale/unavailable draw. Fallback is omit draw, not pool.
- **Old reading inside current poll:** show native reading and wrapper ages separately. Without an upstream freshness contract, do not invent stale-after duration; insufficient evidence yields unknown.
- **Shared package breadth:** two real consumers justify one module, but do not migrate unrelated cockpit code or create a plugin framework.
- **Grant inventory lag:** local gating fails closed and hides data; core authorization remains the real boundary.
- **Pessimistic maximum is not health:** label it highest utilization; verdict uses all contribution facts and never treats one 100% maximum alone as pool exhaustion.
- **No role/attribution contract:** infer neither admin nor contributor identity. Anonymous counts and no admin controls are the safe fallback.

## Other agent review

- Invoked because: this design joins partial resources, decodes untrusted schema-bound bytes, synthesizes an operator verdict, and must align web/CLI honesty.
- Effective weight: **thorough** (explicit caller/autopilot override).
- Skipped/degraded: no subagent/peer review tool or different-class endpoint is exposed. Independent design review could not run; Part IV makes this non-blocking. Direct source, schema, foundation, selected-mock, and pre-mortem evidence was used, and no pass is labeled independent/cross-model.
- Fixed/active blockers: one shared verdict path, no invented age threshold, explicit draw ambiguity, fail-closed unknown/model-unavailable, per-resource grant gating, and no exhaustion inference from one maximum.
- Parked: upstream roles/read scopes, contributor attribution/stable ids, binding-window metadata, explicit reading freshness, and drill-down.
- Rejected: server DTOs, dynamic renderers, pool remaining percentages, inferred admin mode, and first-report draw selection.

## UI surface

The signed-off `.mockups/screens/epic-token-commune-observer-cockpit-panel/option-7.html` is inherited unchanged and is the implementation authority. No fallback mockup is needed. The component uses existing tokens/chrome and implements only the calm list; parked drill-down and enforcement calibration remain absent.

## Extension pressure classification

- **Committed post-v0.1 direction:** Patchbay-owned manifest decoder; shared web/CLI summaries; option-7 list; native draw/health/model/capacity; explicit Patchbay verdict; local query-grant gating; PARTIAL/polled/anonymous footer.
- **Reserved seams:** more local compositors, trustworthy member/admin scope, attribution/stable ids, binding-window/freshness metadata, drill-down, admin controls, enforcement calibration.
- **Explicitly rejected here:** dynamic adapter code, core token-commune states, pool remaining/average/weighted capacity, contributor/admin inference, model aliases, fabricated stale SLA, mutations, and adapter-specific RPC.
- **Non-foreclosure:** variants remain adapter-owned beneath the existing operational-resource wrapper; the pure local package permits later known compositors without dynamic plugins. No Pi core enum, closed surface set, second-operator assumption, federation behavior, or parked mesh/desktop/skin requirement is added.

## Review handoff

Effective implementation review weight is **thorough** (explicit caller). Child stories close directly on green verification; integrated feature review then iterates review → receiver adjudication → fix/verify → fresh-context review until no receiver-confirmed material current-cycle blocker remains. Reviewer findings are proposals, not authority. Autopilot completion keeps this weight unchanged.

## Implementation notes

- **Execution capability:** one owning worker, `openai-codex/gpt-5.6-sol`, high reasoning, selected by the explicit autopilot delegation. No sub-worker, peer, or dynamic renderer was used. The cohesive owner kept the shared decoder/compositor, cockpit adapter, option-7 component, CLI adapter, and cross-surface evidence aligned through the seven declared dependency checkpoints.
- Added the pure `@patchbay/operator-domain` package. It validates both exact manifest contracts, strips identity-bearing contribution detail from surface types, joins only exact `(adapterId, provider)`, preserves independent draw/credential/telemetry evidence, emits only the highest real anonymous 5h reading, and owns the freshness-first Patchbay verdict.
- Extended the cockpit's existing resource decoder/fold path and Resources destination without adding transport or cache state. Visible live `query` grants gate pool and member draw independently; recognized token resources render once through the signed-off option-7 list, with stale/unknown/partial/auth-broken distinctions and every derivation owned in the footer.
- Added CLI `resource-query`/`resource-inspect` over canonical resource and security snapshots, reusing the diagnostics identity grammar and shared compositor. Text and JSON remain safe projections with no raw envelope, member label, contribution identity, credentials, inferred role, or admin action.
- **Implementation judgment:** the explicit implementation brief says the upstream-rejected bare `gpt-5.6` alias must never render. The decoder therefore rejects it and the panel independently withholds it; no local alias is synthesized. Exact admitted catalog ids and native availability remain unchanged, and nullable upstream-model provenance is never fabricated.
- All seven child stories advanced directly to `done`. This feature advances only to `review` at the caller's explicit boundary; this worker did not self-review. Effective review weight remains **thorough** for the delegated reviewer under Part IV.

## Verification

- `cd operator-domain && npm test` — **7/7 passed**.
- `cd web-cockpit && npm run build` — clean; `npm test` — **113/113 passed** (baseline 105, +8).
- `cd cli && npm test` — **42/42 passed**.
- `cd contracts/ts && npm run check:drift && npm run check:presentation` — passed; presentation axe-core scan passed.
- Panel-local axe-core scan — **0 critical violations**.
- Self-mutation check — **3/3 mutants killed and reverted**: adapter-less join, inverted highest-5h comparator, and removed freshness-first verdict.
- `git diff --check` — passed. No formal/model-checked promotion is claimed.
