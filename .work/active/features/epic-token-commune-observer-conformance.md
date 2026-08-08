---
id: epic-token-commune-observer-conformance
kind: feature
stage: done
tags: [adapter, verification]
parent: epic-token-commune-observer
depends_on:
  - epic-token-commune-observer-adapter-foundation
  - epic-token-commune-observer-snapshot-mapping
  - epic-token-commune-observer-polling-ingestion
  - epic-token-commune-observer-cockpit-panel
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-08
---

# token-commune observer conformance and end-to-end evidence

## Brief

Executable conformance vectors and end-to-end tests proving the token-commune
observer is honest and trustworthy — the gate the `control-attention` epic
explicitly waits for ("after the observer adapter is trustworthy in daily use"),
and a primary input to the sibling `epic-public-product-contract-adapter-portability-proof`
feature's cross-adapter v1 boundary proof.

It delivers: conformance vectors proving the observer reconnects and reconciles
within its real limits, snapshots at the declared PARTIAL tier (never claiming
authoritative), degrades honestly on event gaps (>50-event window) and
disconnect, source-authenticates its reports against the current adapter
generation, fully redacts the gateway credential from all Observations/payloads/
diagnostics/audit, and fails safely on adapter failure. It also covers the
real-core end-to-end path (adapter attaches, reports resources, cockpit renders,
disconnect degrades) analogous to the Pi adapter's real-core E2E.

This is the per-adapter correctness evidence, distinct from the cross-adapter
boundary proof (which consumes both Pi and token-commune to show neither's
concepts entered the core ontology).

## Epic context

- Parent epic: `epic-token-commune-observer`
- Position in epic: **closing evidence** — consumes the whole arc. Its vectors
  are a primary input to `epic-public-product-contract-adapter-portability-proof`
  (cross-adapter v1 proof) and the trust gate for `epic-token-commune-control-attention`.

## Simplification opportunity

- Reuse the conformance-vector harness and property-oracle pattern proven in the
  resource-plane `conformance` feature; do not re-prove core invariants — prove
  the adapter's honest behavior against them.
- Apply the same self-validating-evidence discipline (data-driven counts/
  registries that fail-closed on drift) learned in the resource-plane deep-lane
  review.

## Foundation references

- `docs/VERIFICATION.md` — property-graded baseline; conformance-vector rigor.
- `docs/SPEC.md` — "v1 adapter proof" (Pi + token-commune must prove through
  executable conformance that the boundary supports both shapes without either
  adapter's concepts entering the core ontology).
- `docs/SECURITY.md` — credential redaction; no-log rules.
- `.agents/skills/patterns/` and the resource-plane `conformance` feature for
  the vector/property-oracle harness shape.
- `pi-adapter/tests/e2e.test.ts` for the real-core E2E shape.

## Key design decisions (inherited)

- **Honesty over coverage theater.** Vectors must be genuinely
  mutation-sensitive: each promoted claim must fail when the invariant it
  asserts is broken. A vector that passes whether or not the invariant holds is
  a defect (the recurring anti-pattern from the resource-plane deep-lane review).
- **Prove the gaps, not hide them.** The >50-event window, partial-only tier,
  composite-identity collision risk, and no-read-scope credential are real
  limitations; vectors must prove the adapter reports them honestly, not paper
  over them.

## Mockups

- Inherits the signed-off token-commune surface at
  `.mockups/screens/epic-token-commune-observer-cockpit-panel/option-7.html`.
- No new surface is introduced. This feature verifies the existing panel over
  real adapter-shaped data, so feature-level fallback mocking is skipped.

## Design decisions

- **Assurance scope is adapter-specific implementation evidence.** Add seven
  `TokenCommune*` stated-normative property ids and promoted executable examples;
  do not add a formal model or re-prove the generic resource-plane invariants.
  The sibling portability proof consumes this evidence and remains responsible
  for the cross-adapter ontology claim.
- **Extend the one corpus and runner protocol.** Add a
  `token-commune-adapter` package runner and token-commune cases to the existing
  core/server/web runners. There is no adapter-only vector directory, manifest,
  traceability table, or second promotion command.
- **Every promoted claim carries exact mutation accounting.** Add optional
  `mutation_witnesses` to the shared envelope. For this certification profile it
  is mandatory and non-empty. A runner reports each killed witness id and the
  umbrella checker requires exact equality with the vector-declared set before
  it updates traceability.
- **Reference oracles consume raw scenario facts.** Oracles encode the external
  latest-50/PARTIAL/current-generation/redaction/presentation rules directly.
  They may not call the production projector, event-window classifier,
  diagnostic sanitizer, terminalization loop, verdict synthesizer, decoder, or
  renderer whose behavior they judge.
- **Real-core evidence uses the real external boundary.** The E2E starts the
  Rust core plus a local HTTP gateway, loads a real `0600` member-key file, uses
  the generated adapter RPCs, and reads core snapshots/subscriptions/audit plus
  the adapter diagnostic file. Fake ports remain only for deterministic mutant
  witnesses.
- **Presentation shares vector data without a runtime package dependency.** The
  adapter runner proves that raw gateway input produces the vector's exact
  schema-bound resource projection; the web runner renders that same promoted
  expected projection through the local decoder/compositor. The cockpit never
  imports adapter code or loads adapter-supplied renderer material.
- **Autopilot rationale.** These are the least irreversible choices consistent
  with the brief and landed harness. All seven claims can remain
  implementation-checked until formal promotion is independently justified;
  no external contract or product direction is being silently widened.

## Codebase mapping

Direct reading covered the complete shared conformance envelope/checker and its
Rust core/server/web runners, the resource-plane conformance design and deep
review record, the token-commune process/core client/projector/poller/window/
diagnostic paths, its current real-core E2E, the Pi E2E blueprint, the shared
operator-domain decoder/verdict code, and the option-7 DOM renderer/tests. The
current token E2E proves only manifest registration, one unsupported delivery,
and a coarse secret scan. It does not bind polling output to the real resource
snapshot, exercise disconnect/reconnect/generation fencing, or prove every
redaction/presentation sink. The feature is cross-package but its seams are now
bounded, so direct reading was used rather than exploratory fan-out.

## Architectural choice

### Options considered

1. **Extend the shared executable-vector profile with a token adapter runner,
   exact mutation ledger, and shared real-core E2E (chosen).** Each JSON vector
   drives product seams and an independent oracle; the existing checker owns
   registration, execution, mutation accounting, proto validation, and generated
   traceability. This optimizes for one evidence source and fails closed on
   drift, at the cost of a small additive runner protocol.
2. **Keep all evidence in `token-commune-adapter/tests/` and summarize it in
   prose.** This is mechanically smaller, but the tests are not linked to the
   promoted corpus or property ids, and prose counts can pass after a scenario
   silently disappears. It repeats the resource-plane's former green-tick
   anti-pattern.
3. **Create a token-commune certification package with its own fixtures and
   report.** This could isolate adapter concerns, but duplicates the runner,
   promotion, property, and traceability registries and makes cross-package
   presentation evidence harder to bind. It is rejected.

The chosen design treats the existing corpus as the expected-example authority
and adds only one new package runner. The trickiest unit is the **self-validating
oracle/mutation bridge**: a vector must fail if its scenario is ignored, if an
expected outcome is reflected back from production, if a mutation is not run,
or if a runner reports an unrelated green case. That bridge lands first.

## Conformance vector set

| Vector | Property id | Real adapter behavior scenario | Independent property oracle | Claim-breaking mutations that must fail |
|---|---|---|---|---|
| `token-commune-partial-snapshot-honesty` | `TokenCommunePartialSnapshotHonesty` | A mixed-success poll produces both exact token-commune views as snapshot reports at `PARTIAL`; failed slices are `unavailable`, omissions carry no tombstone meaning, and no pool aggregate is invented. | From raw endpoint availability and literal resource kinds, require exactly two PARTIAL views, only listed upserts, explicit unavailable/not-reported slices, `capacityAggregation = none`, and no authoritative/aggregate/tombstone claim. | PARTIAL→AUTHORITATIVE; drop a declared view; reuse a prior successful slice; coerce missing telemetry to zero; synthesize average/remaining capacity. |
| `token-commune-bounded-reconnect-honesty` | `TokenCommuneBoundedReconnectHonesty` | Initial latest-50 page is a non-replayed baseline; overlapping pages emit only newly acknowledged ids; a saturated/no-anchor rollover emits a gap before visible facts; reconnect reports before event repair. | A small set/sequence reference model over raw pages, acknowledgements, and process boundaries computes emitted ids and gap reasons without calling `LatestEventWindowTracker`. | Replay initial history; acknowledge before core acceptance; suppress the saturated/no-overlap gap; estimate a missed count; process events before the reconnect report. |
| `token-commune-degradation-honesty` | `TokenCommuneDegradationHonesty` | Failed polls emit empty PARTIAL views, a >50/no-anchor event window emits explicit unknown continuity, abnormal delivery-stream loss degrades cached resources to stale, and a generation-2 reconnect restores only newly reported identities. | A four-step raw-state oracle (`reported`, `poll-missed`, `disconnected`, `reconnected`) allows current only after accepted current-generation evidence; cached disconnect state is stale, no-payload state unknown, and omission never removes the row under PARTIAL. | Skip the empty report so unknown pools disappear; carry the previous endpoint response; leave disconnect state current; let polling silence establish liveness; promote all cached identities on reconnect. |
| `token-commune-current-generation-source-authenticated` | `TokenCommuneCurrentGenerationSourceAuthenticated` | Current token+generation reports and STATUS Observations append; stale attachment token, generation 1 after generation 2 attaches, cross-adapter sender/target, and payload-claimed source append no resource/Observation state. | Compare authenticated adapter id, exact target owner, attachment epoch, and generation from independent request fixtures; accepted append count is one only for the current exact tuple. | Ignore generation equality; accept the prior token; trust payload sender/generation; compare resource local id without adapter/kind. |
| `token-commune-gateway-key-redaction` | `TokenCommuneGatewayMemberKeyRedacted` | A high-entropy sentinel member key is used by the real HTTP client and appears in hostile gateway errors/fields; scans cover resource reports/snapshots, generic Observations, local and forwarded diagnostics, subscription payloads, audit/diagnostic queries, and raw durable event bytes. | The oracle owns the original sentinel and recursively scans bytes, UTF-8/base64/JSON forms, bearer form, URL-encoded form, and key path. Structural allowlists independently reject secret-bearing fields in adapter payload/diagnostic schemas. | Put the key in a resource payload; remove local secret replacement; forward arbitrary error text; persist Authorization in audit; render or CLI-print raw envelope bytes. |
| `token-commune-unsupported-operation-terminalization` | `TokenCommuneAdapterFailureSafe` | Unexpected accepted Operations survive both a retryable terminal-report failure and a hard adapter replacement after acknowledgement: redelivery/reconnect finishes unsupported terminalization before later work, and core history ends in exactly one durable `DELIVERED` then one `FAILED/UNSUPPORTED_COMMAND`. | From raw lifecycle events require one durable `DELIVERED` before one terminal `FAILED`, no `COMPLETED`, exact failure code, no duplicate durable acknowledgement, and no accepted/delivered command left non-terminal after either recovery path. | Clear pending state before successful terminalization; filter delivered-but-nonterminal work from replacement redelivery; advance to later delivery first; emit `COMPLETED`; use `ADAPTER_UNAVAILABLE`; append duplicate delivery/terminal transitions. |
| `token-commune-cockpit-presentation-honesty` | `TokenCommuneCockpitPresentationHonesty` | The exact adapter projection fixture renders current, stale, unknown/invalid, cross-provider-model, forbidden-alias, and hostile renderer/contributor fields through the local operator-domain and option-7 panel. | Literal summary/DOM expectations require stale dominance, unknown anchoring, exact provider-local runnable evidence, no contributor/member/subkey or `gpt-5.6`, verdict provenance text, and no hostile HTML/script/renderer execution. | Ignore wrapper freshness; drop unknown rows; provider-only model join; accept `gpt-5.6`; expose contributor/member/subkey; trust adapter verdict; accept `rendererUrl`/HTML/script or use dynamic import/`innerHTML`. |

The seven token-specific vectors are resource-only by construction. They do not
add a fake session case merely to satisfy a count. The shared dual-case
`command-acceptance`, `failure-missing-grant`, and `snapshot-reconciliation`
runners retain their existing fail-closed requirement to execute both session
and resource cases; this feature does not weaken it.

## Implementation Units

### Unit 1: Shared profile registration and exact mutation accounting

**Files:** `contracts/vectors/README.md`,
`contracts/scripts/check-vectors.mjs`, `docs/VERIFICATION.md`,
`token-commune-adapter/package.json`,
`token-commune-adapter/tests/conformance-vectors.test.ts` (new),
`token-commune-adapter/tests/conformance-oracles.ts` (new)

**Story:** `epic-token-commune-observer-conformance-harness-registry-guards`

```ts
export interface MutationWitness {
  mutation_id: string;
  runner: "token-commune-adapter" | "web-cockpit";
  invariant: string;
}

export interface TokenCommuneConformanceVector {
  vector_id: string;
  property_id: TokenCommunePropertyId;
  promotion_status: "draft" | "promoted";
  implementation_checks: readonly { runner: string; case: string }[];
  mutation_witnesses: readonly MutationWitness[];
  input: unknown;
  expected_outcome: unknown;
}

export type TokenCommunePropertyId =
  | "TokenCommunePartialSnapshotHonesty"
  | "TokenCommuneBoundedReconnectHonesty"
  | "TokenCommuneDegradationHonesty"
  | "TokenCommuneCurrentGenerationSourceAuthenticated"
  | "TokenCommuneGatewayMemberKeyRedacted"
  | "TokenCommuneAdapterFailureSafe"
  | "TokenCommuneCockpitPresentationHonesty";
```

**Implementation notes:**

- Add `token-commune-adapter` to `IMPLEMENTATION_RUNNERS`; dispatch it once and
  retain the existing exact `PATCHBAY_CONFORMANCE_EXECUTED` equality check.
- Accept `mutation_witnesses` additively for the whole corpus, but require a
  non-empty, unique set for every promoted `TokenCommune*` vector. Parse
  `PATCHBAY_CONFORMANCE_MUTATION_KILLED=<vector>:<mutation>` and require exact
  equality with the declared union. Missing, duplicate, unexpected, or
  unreported kills fail before traceability generation.
- One checker profile registry binds the seven property ids to the seven exact
  vector ids and permitted runner cases. Property lists, generated docs, vector
  counts, implementation counts, and mutation counts derive from it; do not add
  prose-maintained numeric claims.
- Add property-specific static expected-outcome checkers. No generic “truthy” or
  “runner exited zero” promotion rule is acceptable.

**Acceptance criteria:**

- [ ] Removing/renaming a vector, property, implementation case, or mutation id
      fails closed and leaves `docs/VERIFICATION.md` byte-identical.
- [ ] A token vector cannot promote without scenario execution and at least one
      exact killed mutation witness.
- [ ] A runner cannot satisfy a request by reporting an unrequested or duplicate
      id; all current resource/session vectors remain green.
- [ ] Assurance wording remains “promoted vector + implementation-checked; not
      model-checked,” never checked-normative or release-verified.

### Unit 2: Phase 1 completeness vectors and independent oracles

**Files:** the first three new vector JSON files in the table,
`token-commune-adapter/tests/conformance-vectors.test.ts`,
`token-commune-adapter/tests/conformance-oracles.ts`, and focused existing
poller/window/projector tests

**Story:** `epic-token-commune-observer-conformance-phase-1-completeness-vectors`

```ts
export interface ReconnectReferenceStep {
  process: number;
  page: { historyMode: "latest-50-no-cursor"; ids: readonly string[] };
  acknowledged: readonly string[];
  reportAccepted: boolean;
}

export interface ReconnectReferenceOutcome {
  baselineOnly: boolean;
  emittedIds: readonly string[];
  gap: null | "initial-baseline" | "window-discontinuity"
    | "window-saturated-without-anchor" | "history-became-empty";
  reportPrecedesEvents: boolean;
}

export function reconnectReferenceModel(
  steps: readonly ReconnectReferenceStep[],
): readonly ReconnectReferenceOutcome[];

export function assertPartialSnapshotOracle(input: unknown, observed: unknown): void;
export function assertDegradationOracle(input: unknown, observed: unknown): void;
```

**Implementation notes:**

- Use literal resource kinds, endpoint availability, page ids, and expected
  state; do not import the projector registries or event-window classifier into
  the oracle file.
- The production runner executes the real projector, poller, tracker, and core
  sink seams. Each mutant transforms the production observation then reuses the
  same oracle, so a passing production assertion that also accepts its mutant is
  a blocker.
- Include deterministic witnesses for the review-discovered Retry-After hot
  loop, disappearing-unknown pool, pre-ack dedup, fail-open fingerprint/source
  decode, and PARTIAL→AUTHORITATIVE regressions.

**Acceptance criteria:**

- [ ] All PARTIAL, latest-50, missed-poll, disconnect, and reconnect outcomes in
      the first three vectors execute from vector fields rather than hard-coded
      package fixtures.
- [ ] Every named mutation is rejected by the same independent oracle that
      accepts production.
- [ ] Report-before-event ordering and no fabricated missed count/liveness are
      explicit observations, not explanatory prose.

### Unit 3: Real-core attach → report → degrade → reconnect E2E

**Files:** `token-commune-adapter/tests/e2e.test.ts`,
`token-commune-adapter/tests/fixtures/conformance-gateway.ts` (new)

**Story:** `epic-token-commune-observer-conformance-real-core-e2e`

```ts
export interface ScriptedGatewayStep {
  responses: Readonly<Record<GatewayEndpoint, { status: number; body: unknown }>>;
  expectedAuthorization: string;
}

export class ScriptedTokenCommuneGateway {
  start(steps: readonly ScriptedGatewayStep[]): Promise<URL>;
  advance(): void;
  close(): Promise<void>;
}

async function loadResourceSnapshot(
  control: ReturnType<typeof makeControlClient>,
): Promise<ResourceSnapshot>;
```

**Implementation notes:**

- Expand the serial E2E to use a local HTTP server, actual gateway decoder,
  actual `0600` credential loader, actual adapter process, real Rust core and
  SQLite storage, generated attach/report/subscribe/load-snapshot APIs, and
  fixed gateway fixtures.
- Flow: generation 1 attach → mixed current PARTIAL report → initial event
  baseline → overlapping event → abnormal stream loss → stale snapshot →
  generation 2 attach → no-anchor/gap report → listed-only recovery. Assert old
  generation/token attempts append nothing.
- Scan every externally visible and durable representation for the credential
  sentinel, including raw SQLite payload blobs and rotated/current diagnostics.
  Scan failures print only sink names, never the sentinel.

**Acceptance criteria:**

- [ ] The real resource snapshot shows exact PARTIAL collection revisions and
      expected current→stale→listed-current transitions.
- [ ] The latest-50 limit is visible as gap evidence, not repaired history.
- [ ] Generation-1 evidence after generation 2 is inert and audited/redacted.
- [ ] No key encoding appears in any report, Observation, snapshot, diagnostic,
      query result, audit projection, or durable blob.

### Unit 4: Phase 2 source-authentication and redaction adversaries

**Files:** the current-generation and gateway-key vector files,
`token-commune-adapter/tests/conformance-vectors.test.ts`,
`server/tests/conformance_vectors.rs`, existing credential/diagnostic/core-client
tests, and `docs/SECURITY.md` only if the canonical no-log list requires an
in-place clarification

**Story:** `epic-token-commune-observer-conformance-phase-2-security-adversaries`

```ts
export interface SecretScanTarget {
  name: string;
  bytes: Uint8Array;
}

export function assertSecretAbsent(
  originalSecret: string,
  targets: readonly SecretScanTarget[],
): void;

export function expectedCurrentGenerationAcceptance(input: {
  authenticatedAdapterId: string;
  currentGeneration: bigint;
  requestAdapterId: string;
  requestGeneration: bigint;
  ownsTarget: boolean;
  tokenEpochCurrent: boolean;
}): boolean;
```

**Implementation notes:**

- The server runner owns authenticated current/stale token and generation
  attempts through `AdapterControlService`; the adapter runner owns local
  identity rejection and the complete secret sink inventory.
- Include hostile gateway values that contain raw/bearer/url/base64-like forms
  and error objects whose names/codes contain the key. Structural diagnostics
  remain allowlisted; no arbitrary message/cause/stack field is added to make
  the test convenient.
- The source oracle consumes request fixtures, not accepted events written by
  the accepting action (trace-fidelity discipline).

**Acceptance criteria:**

- [ ] Only current exact adapter/generation/target/token evidence appends.
- [ ] The sentinel scan covers all listed sinks and fails under each redaction
      bypass mutant.
- [ ] No adapter identity, credential, or token-commune field enters a core
      enum/state registry.

### Unit 5: Phase 2 failure and presentation adversaries

**Files:** the unsupported-terminalization and cockpit vector files,
`token-commune-adapter/tests/conformance-vectors.test.ts`,
`token-commune-adapter/tests/e2e.test.ts`,
`web-cockpit/tests/conformance-vectors.test.ts`,
`operator-domain/tests/token-commune.test.ts`,
`web-cockpit/tests/token-commune-panel.test.ts`

**Story:** `epic-token-commune-observer-conformance-phase-2-failure-presentation-adversaries`

```ts
export interface LifecycleFact {
  state: "accepted" | "delivered" | "failed" | "completed";
  failureCode?: "unsupported_command" | "adapter_unavailable";
  eventLsn: bigint;
}

export function assertUnsupportedTerminalization(
  facts: readonly LifecycleFact[],
): void;

export function assertTokenCommunePresentation(
  vector: TokenCommuneConformanceVector,
  summary: TokenCommunePoolSummary,
  root: HTMLElement,
): void;
```

**Implementation notes:**

- Exercise two loss points after the real core has durably acknowledged
  delivery: one retryable `failUnsupported` failure in the same process, and one
  hard process replacement before the terminal report. Prove pending or
  core-redelivered nonterminal work completes before later delivery, with
  idempotent durable acknowledgement; do not add a production checkpoint store
  or new command state unless real-core evidence proves the existing durable
  inbox cannot recover the latter case.
- The web runner consumes the exact adapter projection fixture already checked
  by the adapter runner. It renders current/stale/unknown and hostile extra-field
  cases through the real decoder/compositor/panel.
- Hostile projection keys (`contributors`, `member`, `subKey`, `gpt-5.6`,
  `verdict`, `rendererUrl`, `html`, `script`) must either fail decoding or remain
  absent/inert. Assert no dynamic renderer request and no `innerHTML` path.

**Acceptance criteria:**

- [ ] Unsupported work reaches exactly one delivered and one failed/unsupported
      terminal transition despite the injected mid-terminalization failure.
- [ ] Every terminalization mutant leaves a detectable lifecycle mismatch.
- [ ] Stale is never live/runnable; unknown rows remain visible; cross-provider
      models cannot make a pool runnable; no contributor/alias/dynamic renderer
      material reaches the DOM.
- [ ] Verdict wording explicitly remains Patchbay-owned synthesis.

### Unit 6: Promotion, generated traceability, and deep-lane closeout

**Files:** `contracts/scripts/check-vectors.mjs`, all seven vectors,
`docs/VERIFICATION.md`, and the package tests above

**Story:** `epic-token-commune-observer-conformance-promotion-closeout`

**Implementation notes:**

- Promote only after both phase-1 completeness and phase-2 adversarial vectors
  execute, every declared mutation is reported killed, and the real-core flow is
  green. If a vector reveals a production mismatch, fix the owning boundary;
  never weaken the vector to mirror the bug.
- Generate the existing traceability block and one token-commune evidence table
  from the profile registry. Record paths and killed mutation ids, not manually
  maintained totals.
- Run the project `[verification]` deep lane for every child and then the
  integrated feature at effective weight `thorough`: completeness convergence
  first, adversarial convergence second, with reviewers attacking field
  consumption, oracle independence, runner accounting, and surviving mutants.

**Acceptance criteria:**

- [ ] Seven exact token-commune vectors are promoted and traced to seven
      implementation-evidence property ids with exact scenario/mutation runs.
- [ ] Full adapter/core/server/operator-domain/web/contracts verification is
      green without skip, retry masking, weakened expectation, or hard-coded
      runner success.
- [ ] Documentation makes no formal, checked-normative, portability, or
      release-verification claim beyond the evidence actually delivered.

## Real-core E2E scenarios

1. **Attach and report:** load a real key, authenticate the HTTP gateway call,
   attach generation 1, ingest the projector's two PARTIAL views, and load the
   real core resource snapshot.
2. **Bounded event repair:** establish an initial baseline, emit one overlapping
   new event exactly once, then roll past the anchor and observe explicit gap
   evidence with no estimated count.
3. **Missed poll and disconnect:** make all snapshot reads fail, require an empty
   PARTIAL report and stale/unknown preservation, then drop the real delivery
   stream and require stale cached resources rather than live or disappearance.
4. **Reconnect and source fence:** attach generation 2, accept its listed-only
   PARTIAL report, retain omitted resources stale, and reject old token/
   generation/cross-owner evidence with no state append.
5. **Failure-safe delivery:** durably accept two unsupported Operations; for one,
   fail the first terminal report and reconnect in-process, and for the other
   replace the adapter process after acknowledgement. Both must converge to one
   durable delivered then one failed/unsupported terminal outcome before later
   delivery.
6. **Redact and render:** recursively scan all process/core/durable outputs for
   the gateway key; pass the exact promoted adapter projection through the local
   operator-domain/web runner and prove stale/unknown/contributor/model/verdict/
   dynamic-renderer honesty.

## Implementation Order

1. `epic-token-commune-observer-conformance-harness-registry-guards`
2. `epic-token-commune-observer-conformance-phase-1-completeness-vectors`
3. `epic-token-commune-observer-conformance-real-core-e2e`
4. `epic-token-commune-observer-conformance-phase-2-security-adversaries`
5. `epic-token-commune-observer-conformance-phase-2-failure-presentation-adversaries`
6. `epic-token-commune-observer-conformance-promotion-closeout`

The chain intentionally makes the completeness evidence and real happy/degraded
flow exist before adversarial claims are promoted. One feature owner should
normally carry all six checkpoints because the vector profile, E2E fixture,
mutation ledger, and promotion record share one truth boundary.

## Simplification

- Extend the existing vector checker, package-runner protocol, generated table,
  real core test utilities, gateway port, projector, poller, attachment client,
  resource snapshot path, operator-domain compositor, and option-7 renderer.
- Do not add a token-only corpus, formal model, core RPC, resource enum,
  connectivity state, heartbeat, adapter-side database/cursor, dynamic UI
  plugin, model alias, contributor projection, or authoritative snapshot tier.
- Keep the existing narrow unit/property tests as fast mutation witnesses; the
  promoted vectors and E2E bind them to product seams rather than replacing
  them with duplicate branch tests.
- Consolidate repeated process bootstrap/snapshot/subscription helpers in the
  token E2E fixture. Do not copy the full Pi test helper surface.
- No existing test is identified for deletion. Tautological reflected-output
  assertions discovered during implementation are replaced by vector-driven
  independent-oracle assertions rather than retained beside them.

## Testing

- **Shared checker integrity:** envelope/property/proto validation, exact runner
  requests/reports, profile registry parity, static expected-outcome checks,
  mutation-kill equality, generated-doc drift, and no-write-on-failure.
- **Adapter properties:** deterministic raw-input reference oracles over the
  real projector/poller/window/client/process seams protect PARTIAL, bounded
  reconnect, degradation, generation binding, and failure terminalization.
- **Security regression:** high-entropy sentinel and structural allowlists
  protect every payload/Observation/diagnostic/audit/storage output. The test
  logs sink names only.
- **Real-process E2E:** local gateway + Node adapter + Rust core + SQLite proves
  attach/report/degrade/reconnect/source-fence/terminalization/redaction.
- **Presentation interface:** the shared vector fixture crosses adapter output
  validation into operator-domain decode/verdict and real DOM rendering; exact
  selectors/text protect stale dominance, unknown anchoring, contributor/model
  exclusion, verdict ownership, and static local composition.
- **Mutation evidence:** every promoted vector declares at least one named
  mutation and the checker requires its exact kill report. Manual production
  edits for the highest-risk redaction, pending-terminalization, generation,
  PARTIAL, and stale-rendering guards are also executed/reverted and recorded.

Verification commands include `node contracts/scripts/check-vectors.mjs`,
`npm --prefix token-commune-adapter test`, focused real E2E, Rust core/server
conformance and workspace tests, `npm --prefix operator-domain test`,
`npm --prefix web-cockpit test`, contract drift/presentation/model checks,
clippy, and `git diff --check`.

## Risks

- **Self-validating theater (highest risk):** an executor can ignore vector
  fields or an oracle can mirror production. Exact field consumption, raw-input
  reference models, expected-output static checks, and required killed-mutant
  reports are all promotion gates. Fallback: leave the vector draft and the
  feature incomplete.
- **Credential scan false confidence:** scanning only JSON misses raw/binary or
  encoded copies. The sentinel oracle covers raw bytes and common transport
  encodings, while structural payload/diagnostic allowlists reduce dependence
  on string replacement. JavaScript cannot promise memory zeroization; the claim
  is strictly no emitted/persisted/surfaced key.
- **Real E2E race/flakiness:** stream-drop degradation and retryable
  terminalization are asynchronous. Synchronize on committed LSN/state, use
  fixed clocks and scripted endpoints, run one serial process test, and never
  hide failure behind retry.
- **No cursor means no complete reconnect proof:** the oracle proves only the
  honest latest-50 behavior. It must not evolve into an unlimited-history or
  exactly-once-across-process-restart claim.
- **Generation reattach interaction:** the client automatically retries an
  unauthenticated request by reattaching the same generation. After a newer
  generation wins, the old client must remain rejected; tests must not refresh
  it into apparent validity.
- **Presentation proof can drift from adapter output:** the shared vector fixture
  bridges the packages, and the adapter runner must prove its exact bytes before
  the web runner consumes them. A hand-built unrelated DOM fixture is not
  acceptance evidence.
- **Design advisory degradation:** this worker exposes no subagent/peer review
  mechanism. Independent design review could not run and is not mislabeled.
  The explicit thorough two-phase implementation review remains mandatory.

## Extension pressure classification

- **Committed post-v0.1 direction:** promoted implementation evidence for the
  current read-only token-commune observer's PARTIAL snapshots, latest-50
  reconnect limits, stale degradation, current-generation source binding,
  gateway-key no-emission, unsupported-operation terminalization, and static
  local cockpit presentation.
- **Reserved seams:** upstream cursor/pagination/push, authoritative snapshots,
  scoped read-only credentials, stable source ids, durable cross-process event
  dedup, heartbeat/age liveness, formal promotion, and third-party certification
  profiles.
- **Explicitly rejected here:** claiming cross-adapter portability from this
  evidence alone, a token-only harness, polling-as-streaming, fabricated
  aggregate/liveness/history, adapter-supplied renderer code, and any key in
  payload/diagnostic/audit output.
- **Non-foreclosure check:** all token variants remain adapter-owned below the
  existing resource/presentation registries; authority-domain/generation/resource
  tuple demarcators remain intact. No second-operator, federation, Pi ontology,
  closed control-surface set, or parked mesh/desktop/skin requirement is added.

## Other agent review

- Invoked because: this design promotes security-, durability-, reconnect-, and
  presentation-claiming evidence across JavaScript, Rust, SQLite, and the web
  DOM.
- Effective weight: **thorough** (explicit caller) plus the project-specific
  `[verification]` two-phase deep lane.
- Skipped/degraded: no independent subagent/peer mechanism is exposed in this
  delegated worker. Part IV makes design-time advisory non-blocking; direct
  foundation, prior deep-review, code, test, and pre-mortem evidence was used,
  and no pass is labeled independent or cross-model.
- Fixed/active blockers in design: exact mutation reporting; raw-input oracles;
  no-write-on-check failure; real gateway+core E2E; full key sink inventory;
  retryable post-ack terminalization; shared adapter-to-web vector fixture.
- Parked: formal promotion and stronger upstream cursor/snapshot/read-scope
  guarantees; they are reserved prerequisites rather than hidden requirements.
- Rejected: metadata-only promotion, hard-coded unrelated runners, a token-only
  report, and cross-adapter proof claims.

## Implementation notes

- Ownership/topology: one cohesive owning worker (`openai-codex/gpt-5.6-sol`, high reasoning) implemented the six-story dependency chain sequentially with no sub-worker fan-out. Direct reading covered the shared checker, package runners, token adapter seams, real-core E2E utilities, operator compositor, panel, and foundation; splitting would have weakened the exact profile/projection/mutation truth boundary.
- Review boundary: effective `review_weight = thorough`, explicit caller override. This worker implemented and verified only, then advanced the feature to `review`; the separate `[verification]` deep-lane reviewer owns two-phase completeness→adversarial convergence and no implementation pass is mislabeled review.
- Promoted vectors:
  - `token-commune-partial-snapshot-honesty`
  - `token-commune-bounded-reconnect-honesty`
  - `token-commune-degradation-honesty`
  - `token-commune-current-generation-source-authenticated`
  - `token-commune-gateway-key-redaction`
  - `token-commune-unsupported-operation-terminalization`
  - `token-commune-cockpit-presentation-honesty`
- Harness/evidence: corpus, promotion, scenario, mutation, and proto-reference totals are derived by the checker rather than retained here. Exact requested/executed and declared/killed sets, profile parity, proto refs, generated docs, and every retained count assertion fail closed before traceability comparison.
- Independent oracles: literal two-view/PARTIAL and no-aggregate checks; set/sequence latest-50 model; degradation confidence truth table; attempted attachment-token/generation/ownership tuple; multi-encoding secret byte scan; durable lifecycle facts; and literal summary/DOM expectations. None imports the production helper it judges.
- Real-core E2E: real local HTTP gateway and Authorization, real `0600` credential loader, actual adapter process, generated RPC clients, Rust core, and SQLite execute mixed PARTIAL report, event baseline/overlap, missed poll, abnormal stream loss/stale snapshot, generation-2 reconnect/50-window gap/listed recovery, stale-generation and cross-owner rejection, retryable and replacement-process unsupported terminalization, diagnostic query, full sink redaction, and cockpit-bound exact projection evidence.
- Highest-stakes redaction result: the member-key sentinel and bearer/URL/base64/JSON/hex forms are absent from resource envelopes, Observations/subscriptions, local/forwarded diagnostics, diagnostic/audit query output, ResourceSnapshot bytes, and raw SQLite bytes. A successful upstream response that reflects credential material now fails closed as `invalid-response` before decode.
- Mutation evidence: every declared witness was exactly killed. Production-module copies now carry each claim-breaking projector, tracker/poller, sanitizer, terminalization-loop, compositor, or renderer mutation; Rust conformance mutants drive the real source-ingress/core-storage and degradation-projection seams. Every witness runs baseline production first, then the mutant through the same oracle.
- Final verification:
  - `npm --prefix contracts/ts run check:vectors`: derived corpus/promotion/check/proto/mutation totals — pass.
  - `check:drift`, `check:presentation` (5 registries + axe), `check:models` (8 checked-model / 0 checked-normative / 60 stated-normative) — pass.
  - `cargo test --workspace`: 345 listed tests including doctests — pass; `cargo clippy --workspace --all-targets -- -D warnings` — pass.
  - `npm --prefix token-commune-adapter test`: 60/60 — pass; both real-core E2Es green.
  - `npm --prefix operator-domain test`: 9/9 — pass.
  - `npm --prefix web-cockpit test`: 114/114 — pass; real token panel vector included.
  - `git diff --check` — pass.
- Verification execution note: an initial parallel command batch created transient build-artifact contention (the web build observed operator-domain `dist/` while that package was rebuilding, and Cargo doctest linkage overlapped contract regeneration). The authoritative commands were rerun sequentially and passed; no test expectation or production behavior was weakened.
- Story commits: `7029c6d`, `b9f1b08`, `d5c13e8`, `a313872`, `f037e21`, `8df3a72`. No unrelated changes or deferred production defects were folded into the feature.
- Assurance classification: promoted vector + implementation-checked only. No formal/model-checked, checked-normative, cross-adapter portability, or release-verified claim is made.

## Deep-lane review remediation (2026-08-08)

- Replaced observation-editing, constant-comparison, and sentinel-buffer mutation theater with baseline→compiled-production-mutant→same-oracle witnesses. The TypeScript harness mutates copied compiled production graphs and the Rust runner exercises mutant source ingress and real resource storage/replay.
- Reconnect evidence now runs the real poller/tracker and derives report/gap/event ordering from sink call traces and committed acknowledgements. Degradation combines the real failed-poll report with a Rust four-step core projection trace, including a CURRENT resource immediately before abnormal disconnect.
- Terminalization scenarios enqueue pending and later Operations together; both retry and replacement recovery assert the pending unsupported terminal LSN precedes the later command's first delivered LSN.
- Redaction now injects every declared hostile representation through real gateway fields, scans actual reports, Observations, local/forwarded diagnostics, query-shaped output, snapshots/subscriptions, and live plus checkpointed/closed SQLite db/WAL/SHM files. The real-process E2E separately scans core queries, subscription output, resource snapshot, diagnostics, and every core SQLite file.
- The real-process E2E proves initial-baseline no replay, report→gap→event durable LSN order, CURRENT immediately before abnormal stream loss, stale caused by that loss, and final snapshot composition through operator-domain and the real panel renderer.
- The promoted presentation scenario now has unequal multi-contribution readings, a native member draw, and a competing same-provider cross-adapter draw. Its join witness mutates the actual compositor, while aggregate-vs-native behavior is distinguishable at the rendered summary.
- `check:vectors` is read-only and compares generated traceability byte-for-byte. `generate:vectors` is the separate write command; check failure never repairs the artifact. Prose-maintained totals were removed from this work item.
