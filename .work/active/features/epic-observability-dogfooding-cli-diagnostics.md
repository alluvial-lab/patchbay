---
id: epic-observability-dogfooding-cli-diagnostics
kind: feature
stage: done
tags: [observability, dogfooding]
parent: epic-observability-dogfooding
depends_on: [epic-observability-dogfooding-core-diagnostics]
release_binding: null
gate_origin: null
created: 2026-07-25
updated: 2026-07-27
---

# CLI diagnostics commands

## Brief

`audit-query`, `inspect-command`, and `adapter-status` shipped in v0.1.0 as
honest stubs that exit non-zero with a prerequisite message. This feature
fulfills them as real commands backed by the core-diagnostics query surface,
following the established CLI pattern (`session-health` queries core over
gRPC and renders a projection, with `--json` script-facing output).

This gives the operator a workstation-native inspection path: the CLI reaches
core the same way it already does for every other command, so diagnostics are
available from the workstation without SSHing into the VM.

It does NOT cover: the core-diagnostics surface itself
(`epic-observability-dogfooding-core-diagnostics`), cockpit presentation
(`epic-observability-dogfooding-cockpit-diagnostics`), or `event-inspect
<lsn>` (reserved seam).

## Epic context

- Parent epic: `epic-observability-dogfooding`
- Position in epic: consumer of `epic-observability-dogfooding-core-diagnostics`
  — depends on its query surface and generated contract types. Parallel with
  the cockpit-diagnostics consumer. Priority 3 in the epic's seed order.

## Simplification opportunity

- Deletes the three stub command bodies and their prerequisite messages —
  the stubs reference `feature-v0-cli Unit 3b`, a released artifact, and
  their existence is the spec/code divergence this epic closes.
- The three commands should share one diagnostics query/render path rather
  than growing three near-duplicate client/render stacks.

## Foundation references

- `docs/UX.md` — CLI conventions (script-facing output, diagnostic CLI role)
- `docs/SPEC.md` — post-v0.1.0 observability scope
- `docs/PROTOCOL.md` — Persistence and recovery (control surfaces never touch
  persistence directly)

## Architectural choice

Use **one generated-contract diagnostics command runner that constructs an
authority-domain `OperationKind.QUERY`, calls
`ControlService.QueryDiagnostics`, validates the common response envelope, and
renders a command-specific projection**. The three command modules own only
flag-to-query construction and their compact table/JSON views. They do not read
subscriptions, snapshots, SQLite, or raw events.

Three approaches were considered:

1. **Implement each command independently.** This is locally obvious, but would
   duplicate credential loading, query Operation creation, payload encoding,
   submission-outcome handling, response-oneof checks, JSON envelope shape,
   pagination notices, and table plumbing three times.
2. **Build a generic CLI RPC/renderer framework.** This could absorb future CLI
   methods, but it invents a framework before a second RPC family needs one and
   obscures the diagnostics-specific lifecycle and result-oneof checks.
3. **Chosen: one narrow diagnostics runner plus three projections.** The runner
   is generic only over the generated `QueryDiagnosticsResponse.result` oneof;
   generated types remain the contract source, while each command stays small
   and explicit. This is the least irreversible option and matches the existing
   query → projection → render pattern without three stacks.

The trickiest unit is the **shared query boundary**: a read is still an ordinary
accepted Operation, so the CLI must build the exact protobuf payload, preserve
submission exit semantics, reject a missing/wrong result oneof, and never print
an apparently successful empty result for a pre-acceptance rejection. It is
specified first below.

Codebase mapping was direct-read only: the feature is bounded to the CLI entry
point, Operation helpers, three stubs, output helper, and two existing CLI test
files.

## Design decisions

- **The CLI is a thin projection client.** It calls only
  `ControlService.QueryDiagnostics` with a `DiagnosticsQuery` payload on an
  authority-domain `query` Operation. It does not reconstruct diagnostics from
  `Subscribe`, `LoadSnapshot`, filesystem logs, or persistence.
- **Successful empty results are success, not errors.** Empty `AuditPage` and
  `AdapterStatusPage`, and `CommandInspectionResult { found: false }`, return
  exit code `0` with typed empty JSON or a header-only human table plus a clear
  no-match notice. This preserves the core's malformed-vs-empty distinction;
  every actual error remains non-zero.
- **Submission exit codes stay shared.** Accepted query results return `0`;
  pre-acceptance `rejected`, `failed`, and `unknown` use existing codes `2`,
  `3`, and `4`; local validation, transport, malformed RPC envelope, and wrong
  response-oneof errors use `1`. A rejected query never invokes a result
  renderer.
- **JSON has one stable common envelope.** All three commands emit exactly one
  JSON document on stdout: `{ submission, resultEventId, asOfLsn, result }`.
  Every `uint64`/LSN/generation is a decimal string, timestamps are RFC 3339 or
  `null`, absent generated fields are `null`, and enums use generated canonical
  names lowercased to `snake_case`. Successful `--json` writes no routine
  status to stderr.
- **Core defaults remain authoritative.** The CLI omits unspecified limits, so
  the core applies audit `100`, command-related audit `50`, and adapter `100`.
  Explicit CLI limits are positive integers and are locally bounded at the
  contract maxima (`500`, `200`, `500`) before the RPC; the core still validates
  every request.
- **Cursors are local-domain decimal values.** `--before-event` and
  `--audit-before-event` accept a positive decimal LSN and the CLI constructs
  the full `(configured authority_domain_id, LSN)` `EventId`. Adapter pagination
  uses the contract's opaque lexicographic adapter-id cursor unchanged.
- **Enum filters derive from generated registries.** `--kind` and
  `--failure-code` accept comma-separated generated enum names,
  case-insensitively with hyphen/underscore equivalence; unknown and
  `UNSPECIFIED` values fail locally. No handwritten audit-kind/failure list is
  introduced.
- **Audit target syntax is explicit and reversible.** `--target` accepts
  `authority-domain`, `fleet`, `actor=<id>`, `adapter=<id>`, `group=<value>`,
  `resource=<id>`, or the existing canonical runtime identity
  `adapter=...;scope=...;runtime=...;generation=...`. Exactly one generated
  `TargetScope` is built; percent-encoding in runtime identities is decoded by
  the inverse of `canonicalSessionIdentity`.
- **Query-operation IDs are not a new flag family.** Each invocation uses the
  existing `operationIds` generation path. The returned submission view exposes
  the query command id for inspection. Adding retry-control flags can be done
  later without changing result shape; duplicating `--command-id` semantics now
  would collide with the audit command-id filter and widen the interface without
  a demonstrated scripting need.

## UI fallback

No UI surface: these are terminal CLI commands governed by `docs/UX.md`; no
HTML/CSS mockup is applicable.

## Command and flag surface

### `audit-query`

```text
patchbay-cli audit-query
  [--kind KIND[,KIND...]]
  [--actor-id ID]
  [--endpoint-id ID]
  [--command-id ID]
  [--target TARGET]
  [--failure-code CODE[,CODE...]]
  [--reason-code CODE[,CODE...]]
  [--since RFC3339]
  [--until RFC3339]
  [--before-event LSN]
  [--limit 1..500]
  [--json]
```

`--since` maps to `occurred_from_inclusive`; `--until` maps to
`occurred_before_exclusive`; `--before-event` is the exclusive audit cursor.
Repeated semantic values use one comma-separated option because the current CLI
parser intentionally stores one value per named option. `--kind` is also the
outcome filter because `AuditEventKind` is outcome-bearing; the CLI does not
invent a second outcome vocabulary.

Human table columns are `LSN`, `TIME`, `KIND`, `ACTOR`, `ENDPOINT`, `COMMAND`,
`TARGET`, `FAILURE`, and `REASON`. JSON includes every safe generated
`AuditRecord` field, including device, correlation/source event, normalized
source network, and the operator-session hash rendered as lowercase hex; it
cannot include fields absent from the redacted generated contract. Pagination
is `{ hasMore, nextBeforeEvent }`, and the human renderer prints the equivalent
next-command hint to stderr only when `has_more` is true.

### `inspect-command`

```text
patchbay-cli inspect-command <command-id>
  [--audit-before-event LSN]
  [--audit-limit 1..200]
  [--json]
```

The positional id maps to `CommandInspectionQuery.command_id`; the two options
page only the related audit slice. Human output uses a `COMMAND` two-column
summary table, a `HISTORY` table (`LSN`, `TIME`, `STATE`, `FAILURE`,
`CORRELATIONS`), and an `AUDIT` table using the shared audit columns. JSON keeps
`found` explicit and otherwise projects the complete safe generated
`CommandInspection`: summary, accepted/current/terminal facts, lifecycle
history, and nested audit page. It never supplements the response from local
command records and never prints an Operation payload or idempotency key.

### `adapter-status`

```text
patchbay-cli adapter-status [adapter-id ...]
  [--after-adapter-id ID]
  [--limit 1..500]
  [--json]
```

Zero positional ids requests all adapters; one or more positional ids populate
`AdapterStatusQuery.adapter_ids`. `--after-adapter-id` is the exclusive opaque
lexicographic cursor. Human columns are `ADAPTER`, `ENDPOINT`, `GENERATION`,
`STATE`, `LIVE`, `STALE`, `OFFLINE`, `FAILED`, `SNAPSHOT`, `IDEMPOTENCY`, and
`ATTACHED_AT`. JSON projects the full safe `AdapterStatus`, including supported
OperationKinds/target-spec shapes, boolean capabilities, known failure modes,
last lifecycle record, and session counts; no attachment descriptor bytes can
enter the generated response. Pagination is `{ hasMore, nextAfterAdapterId }`.

## Implementation Units

### Unit 1: Shared diagnostics Operation and response boundary
**Files**: `cli/src/commands/diagnostics.ts`, `cli/src/commands/operations.ts`,
`cli/src/commands/sessions.ts`

```ts
import type {
  DiagnosticsQuery,
  QueryDiagnosticsResponse,
} from "@patchbay/contracts";
import type { ControlClient } from "../core-client.js";
import type { CredentialStore } from "../credentials.js";
import type { CliOutput } from "../main.js";
import type { TableSection } from "../output.js";

type ResultOneof = Exclude<
  QueryDiagnosticsResponse["result"],
  { case: undefined }
>;
export type DiagnosticsResultCase = ResultOneof["case"];
export type DiagnosticsResultFor<K extends DiagnosticsResultCase> =
  Extract<ResultOneof, { case: K }>["value"];

export interface DiagnosticsMeta {
  submission: QueryDiagnosticsResponse["submission"];
  resultEventId: QueryDiagnosticsResponse["resultEventId"];
  asOfLsn: string;
}

export interface HumanDiagnosticsView {
  sections: readonly TableSection[];
  notices?: readonly string[];
}

export interface DiagnosticsCommandSpec<K extends DiagnosticsResultCase> {
  query: DiagnosticsQuery;
  resultCase: K;
  json: boolean;
  jsonResult(value: DiagnosticsResultFor<K>): unknown;
  humanResult(value: DiagnosticsResultFor<K>): HumanDiagnosticsView;
}

export async function runDiagnosticsCommand<K extends DiagnosticsResultCase>(
  client: Pick<ControlClient, "queryDiagnostics">,
  store: CredentialStore,
  authorityDomainId: string,
  spec: DiagnosticsCommandSpec<K>,
  output: CliOutput,
): Promise<number>;

export function parsePositiveLimit(
  raw: string | undefined,
  maximum: number,
  option: string,
): number | undefined;
export function eventCursor(
  authorityDomainId: string,
  raw: string | undefined,
  option: string,
): EventId | undefined;
export function parseGeneratedEnumList(
  registry: Record<string | number, string | number>,
  raw: string | undefined,
  option: string,
): number[];
export function parseRfc3339(
  raw: string | undefined,
  option: string,
): Timestamp | undefined;
```

**Implementation Notes**:
- Add `authorityDomainTarget(authorityDomainId): TargetScope` and
  `parseCanonicalSessionTarget(value): TargetScope` as small exported helpers
  beside existing Operation/session target helpers; do not teach the generic
  `targetIdentity` runtime-session formatter about core-local targets.
- `runDiagnosticsCommand` obtains the verified CLI operation context, builds an
  authority-domain target, creates IDs and the standard five-minute validity
  window through existing helpers, sets `kind = OperationKind.QUERY`, and sets
  `payload = { content_type: PROTOBUF, schema_ref:
  "patchbay.DiagnosticsQuery", payload: toBinary(DiagnosticsQuerySchema,
  spec.query) }` before calling `client.queryDiagnostics({ operation })`.
- A missing submission is a malformed RPC response. For non-accepted
  submissions, emit the existing `submissionView` (inside the common JSON
  envelope when `--json`) and return `exitCodeForSubmission` without reading a
  result. For accepted responses, require `result_event_id`, `as_of_lsn`, the
  configured result case, and matching authority-domain ids; otherwise throw a
  protocol error handled by `run` as exit `1`.
- The JSON envelope contains JSON-safe projected values only; never
  `JSON.stringify` a generated message containing `bigint` directly. Human
  notices go to stderr after table sections, while table rows stay on stdout.
- Obvious CLI syntax is validated before the RPC, but server rejections are not
  rewritten: malformed/oversized/cross-domain requests returned as
  `SubmissionOutcome.REJECTED` remain exit `2` with the core's canonical
  failure code and diagnostic message.

**Acceptance Criteria**:
- [ ] Every command submits one authenticated authority-domain `query`
  Operation whose protobuf payload decodes to the requested
  `DiagnosticsQuery`; no command calls `submit`, `subscribe`, `loadSnapshot`, or
  persistence for diagnostics.
- [ ] Accepted responses render only the expected typed oneof; rejection,
  missing result, wrong oneof, missing event/LSN, and cross-domain event ids
  cannot masquerade as a successful empty result.
- [ ] Successful JSON is one JSON-safe stdout document with no routine stderr;
  submission failures retain existing exit-code meanings.

---

### Unit 2: Audit filters and safe audit projection
**Files**: `cli/src/commands/audit-query.ts`,
`cli/src/commands/diagnostics.ts`, `cli/src/commands/sessions.ts`

```ts
export interface AuditQueryOptions {
  kinds?: string;
  actorId?: string;
  endpointId?: string;
  commandId?: string;
  target?: string;
  failureCodes?: string;
  reasonCodes?: string;
  since?: string;
  until?: string;
  beforeEvent?: string;
  limit?: string;
  json: boolean;
}

export async function auditQueryCommand(
  client: Pick<ControlClient, "queryDiagnostics">,
  store: CredentialStore,
  authorityDomainId: string,
  options: AuditQueryOptions,
  output: CliOutput,
): Promise<number>;

export function auditRecordView(record: AuditRecord): AuditRecordView;
export function auditPageView(page: AuditPage): {
  records: AuditRecordView[];
  page: { hasMore: boolean; nextBeforeEvent: EventIdView | null };
};
export function parseAuditTarget(
  raw: string | undefined,
  authorityDomainId: string,
): TargetScope | undefined;
```

**Implementation Notes**:
- Build `AuditQuerySchema` directly from parsed generated enum members, exact
  typed ids, target scope, protobuf timestamps, exclusive EventId cursor, and
  optional limit. Reject empty ids, empty CSV elements, duplicate enum members,
  invalid RFC 3339/no-timezone values, `since >= until`, and malformed target
  expressions locally.
- Runtime-session target parsing is the strict inverse of
  `canonicalSessionIdentity`: detect the four-key semicolon form before the
  simple `adapter=<id>` form, require exactly adapter/scope/runtime/generation,
  reject duplicate/unknown keys, percent-decode each component, and require a
  non-negative integer generation.
- Views use the shared generated `enumLabel`, EventId/TargetScope formatters, and
  timestamp formatter. Human tables intentionally omit lower-value columns for
  width; the JSON projection includes every field in the already-redacted
  generated record.

**Acceptance Criteria**:
- [ ] Every `AuditQuery` filter and pagination field has a documented CLI flag
  and reaches the corresponding generated field without a parallel filter DTO
  or enum registry.
- [ ] `--since` is inclusive, `--until` and `--before-event` are exclusive, and
  omitted `--limit` lets core apply `100` while explicit values cannot exceed
  `500`.
- [ ] A valid empty page returns `0`; non-empty table and JSON views expose
  pagination without leaking payloads, secrets, or raw generated bigints.

---

### Unit 3: Command inspection and adapter status projections
**Files**: `cli/src/commands/inspect-command.ts`,
`cli/src/commands/adapter-status.ts`, `cli/src/commands/diagnostics.ts`

```ts
export interface InspectCommandOptions {
  commandId: string;
  auditBeforeEvent?: string;
  auditLimit?: string;
  json: boolean;
}
export async function inspectCommandCommand(
  client: Pick<ControlClient, "queryDiagnostics">,
  store: CredentialStore,
  authorityDomainId: string,
  options: InspectCommandOptions,
  output: CliOutput,
): Promise<number>;

export interface AdapterStatusOptions {
  adapterIds: readonly string[];
  afterAdapterId?: string;
  limit?: string;
  json: boolean;
}
export async function adapterStatusCommand(
  client: Pick<ControlClient, "queryDiagnostics">,
  store: CredentialStore,
  authorityDomainId: string,
  options: AdapterStatusOptions,
  output: CliOutput,
): Promise<number>;

export function commandInspectionView(
  result: CommandInspectionResult,
): CommandInspectionView;
export function adapterStatusPageView(
  page: AdapterStatusPage,
): AdapterStatusPageView;
```

**Implementation Notes**:
- Require a non-empty inspect positional id and unique, non-empty adapter ids.
  Omit command audit/adapters limits when absent; bound them at `200` and `500`.
  Preserve `after_adapter_id` as an opaque string and do not infer liveness from
  the last lifecycle record.
- Render `found: false` as a normal empty result. Render adapter
  `UNKNOWN` as `unknown`, never as attached/live; show each session-count axis
  independently.
- Use shared audit view/table functions for the inspection's nested audit page.
  Capability JSON derives only from `AdapterCapabilitySummary`; the table uses
  snapshot/idempotency registry labels and does not expose the omitted
  attachment descriptor.

**Acceptance Criteria**:
- [ ] Inspect sends exactly the positional command id and optional audit cursor/
  limit; `found: false` is explicit and exits `0` rather than becoming a
  transport or target-not-found error.
- [ ] Adapter status supports all adapters or an exact id set, preserves the
  opaque adapter cursor, and renders `UNKNOWN|ATTACHED|DETACHED|FAILED` without
  fabricating liveness.
- [ ] Command and adapter JSON contain the full safe generated projections but
  no Operation payload, idempotency key, or attachment descriptor bytes.

---

### Unit 4: Dispatch, shared tables, and operator documentation
**Files**: `cli/src/main.ts`, `cli/src/output.ts`,
`cli/src/commands/session-health.ts`, `docs/UX.md`, `docs/RUNBOOK.md`,
`docs/SECURITY.md`

```ts
export interface TableSection {
  title?: string;
  headers: readonly string[];
  rows: readonly (readonly string[])[];
}

export function printTableSection(
  section: TableSection,
  output: CliOutput,
): void;
export function eventIdView(eventId: EventId | undefined): EventIdView | null;
export function timestampView(value: Timestamp | undefined): string | null;
export function targetScopeView(value: TargetScope | undefined): unknown;
```

**Implementation Notes**:
- Extend `VALUE_OPTIONS` with the exact flags above, validate command positional
  counts (`audit-query` 0, `inspect-command` 1, `adapter-status` 0..many), pass
  the authenticated `ControlClient`, `CredentialStore`, configured domain, and
  parsed options to the async commands, and replace all three stub help lines
  with the real syntax and cursor semantics.
- Move the small width/padding table implementation out of
  `session-health.ts` into `output.ts`; `session-health` and all diagnostics
  reuse it. A section with zero rows still prints its headers. Do not add a
  formatting dependency or ANSI/color behavior.
- Roll `docs/UX.md` and `docs/RUNBOOK.md` forward from current-stub wording to
  the implemented command/JSON/exit contract. Remove any remaining current-stub
  assertion in `docs/SECURITY.md` after the upstream core-diagnostics edit,
  while retaining its canonical redaction rules. Keep the v0.1.0 historical
  statement in `CHANGELOG.md` unchanged.

**Acceptance Criteria**:
- [ ] Help text is sufficient to discover every filter, time bound, cursor,
  limit, and `--json`; unknown options and wrong positional counts still fail
  before network access without echoing secret values.
- [ ] All four table-producing CLI commands use one formatter; table stdout has
  no status chatter and JSON mode emits no table lines.
- [ ] Standing UX/security/runbook prose no longer claims the three commands are
  current stubs and documents that empty query results are successful while
  submission/transport errors are non-zero.

---

### Unit 5: CLI contract and regression evidence
**Files**: `cli/tests/output-diagnostics.test.ts`,
`cli/tests/scripting-commands.test.ts`, `cli/tests/helpers.ts`

```ts
export function diagnosticsResponse<K extends DiagnosticsResultCase>(
  resultCase: K,
  value: DiagnosticsResultFor<K>,
  overrides?: Partial<QueryDiagnosticsResponse>,
): QueryDiagnosticsResponse;
```

**Implementation Notes**:
- Replace the obsolete “honest non-zero stubs” test; do not retain tests for
  behavior this feature intentionally deletes.
- In `scripting-commands.test.ts`, decode the submitted payload with
  `DiagnosticsQuerySchema` and assert the query Operation's authority-domain
  target, kind, protobuf content type/schema ref, standard validity window, and
  verified issuer. One shared boundary test protects all three commands; do not
  repeat the same envelope assertions per command.
- In `output-diagnostics.test.ts`, use generated-message fixtures for: all audit
  filters and pagination; inspection found/not-found and nested history/audit;
  adapter unknown/attached state and pagination; table/JSON shapes; valid empty
  exit `0`; rejection exit `2`; and wrong/missing result oneof fail-closed.
- Assert JSON is parseable with LSN/generation strings and contains sentinel
  safe summaries but none of the canonical secret/prompt/attachment sentinels.
  Parser tests cover maxima, malformed timestamps/cursors, generated enum
  normalization, target syntax, and help text. Do not snapshot whole tables;
  assert headers and representative values so harmless spacing changes stay
  private.

**Acceptance Criteria**:
- [ ] `npm test --prefix cli` builds against generated diagnostics contracts and
  protects query construction, output/exit semantics, pagination, empty
  results, and response-envelope validation.
- [ ] Tests fail if a command bypasses the shared query runner, renders a
  mismatched oneof as empty, stringifies a bigint directly, or leaks a seeded
  sensitive payload/descriptor.
- [ ] No test duplicates upstream core validation/projection algorithms; CLI
  tests stop at flag→wire and generated response→presentation boundaries.

## Implementation Order

1. Land against the completed upstream generated diagnostics contract; implement
   Unit 1 and its shared test fixture first.
2. Replace all three stubs through Units 2-3, using the same runner and audit
   view rather than independent client/render paths.
3. Wire dispatch/help and consolidate table output in Unit 4; roll current
   operator/security prose forward.
4. Complete Unit 5 boundary/regression cases and run `npm test --prefix cli`,
   generated-contract drift checks, and the repository TypeScript test suite.

No child stories are spawned. The three commands are one cohesive CLI boundary:
they share the same generated contract, runner, output envelope, parser, and
test fixtures, and there is no durable intermediate checkpoint that would make
implementation or verification clearer.

## Simplification

- Replace the synchronous stub bodies in
  `cli/src/commands/{audit-query,inspect-command,adapter-status}.ts`; delete the
  prerequisite message and released-item reference entirely.
- Replace the three stub dispatch branches/help lines in `cli/src/main.ts` with
  real async command calls and exact syntax.
- Move the private `session-health` table formatter to `cli/src/output.ts` and
  reuse it rather than adding three table implementations.
- Share one audit-record view/table between `audit-query` and
  `inspect-command`; share one diagnostics Operation/response path across all
  three commands.
- Do not add a local diagnostics repository, raw-event decoder, SQLite access,
  alternate HTTP endpoint, generic renderer framework, ANSI/table dependency,
  or handwritten protocol enum list.
- Delete the obsolete stub regression test. Retain existing submission-output
  and secret-argument tests unchanged except where shared helpers are reused.

## Testing

- **Wire-boundary test:** protects the highest-risk seam: authenticated normal
  query lifecycle, authority-domain target, and exact generated protobuf
  payload. It is more valuable than isolated tests of trivial command wrappers.
- **Response-boundary table tests:** protect accepted/rejected distinction,
  oneof fail-closed behavior, empty-result success, JSON-safe 64-bit values, and
  pagination cursors.
- **Projection examples:** one representative rich record per response family
  protects redaction-preserving field mapping and registry-derived enum labels;
  upstream owns projection correctness and redaction enforcement.
- **Parser regressions:** protect RFC 3339 inclusivity/exclusivity mapping,
  positive maxima, decimal cursor/domain construction, strict runtime target
  identity parsing, and generated enum normalization.
- **Test removal:** remove only the stub-message test. Do not create tests for
  padding implementation details, every enum member, every absent field, or
  core filter algorithms.

## Risks

- **Upstream generated names are the integration dependency.** The upstream
  feature body is stable, but generated oneof property spelling is compile-time
  authority. Mitigation: implement only after the dependency lands, import from
  `@patchbay/contracts`, and adjust local type extraction to generated output
  rather than adding aliases or handwritten DTOs.
- **A submission rejection could be mistaken for an empty diagnostic result.**
  This is the main production failure mode. Mitigation: the shared runner checks
  submission outcome before result presence and requires the exact result oneof
  for every accepted response.
- **JSON becomes a future public compatibility surface.** Raw protobuf messages
  contain bigint values and generator-shaped oneofs that are poor script
  contracts. Mitigation: one explicit envelope, decimal strings, normalized
  enums, nulls, and focused shape tests; human table formatting remains private.
- **Target-filter parsing can misaddress a query.** Mitigation: one strict target
  grammar, generated `TargetScope`, an exact inverse for canonical runtime
  identities, and fail-before-RPC behavior. Fallback: omit `--target` and use
  actor/command/time filters; never guess a malformed target.
- **Diagnostic queries audit themselves and results are prefix-sensitive.** The
  CLI must expose the core's `as_of_lsn`/result event and must not hide or
  recompute records to create a cleaner-looking page.
- **Long identifiers can wrap human tables.** Silent truncation would be worse
  for diagnostics, so tables preserve full values; `--json` is the robust
  script/large-value path.
- **Least certain:** whether operators will need explicit query idempotency flags
  during dogfooding. The response exposes the generated query command id and the
  core preserves retry semantics, but this feature avoids a colliding flag
  family until a concrete retry workflow exists.

## Extension pressure classification

- **Committed post-v0.1.0:** the three CLI commands as thin control-surface
  projections over the generated core-diagnostics `query` Operation, including
  script-facing JSON and bounded cursor pagination. This implements the already
  promoted observability registry row; it does not add a new protocol variant.
- **Remains reserved:** `event-inspect`, no-lifecycle/bypass reads, metrics,
  delivery-trace timeline UI, SIEM/retention, and dedicated health dashboard.
- **Remains rejected:** direct persistence access and a second diagnostics state
  store. The command presentation is CLI-specific above the surface-neutral
  generated response, so future desktop/mobile surfaces and other adapters are
  not foreclosed.

## Implementation summary

- Implemented the shared diagnostics query runner and generated-contract
  protobuf boundary in `cli/src/commands/diagnostics.ts`, including the
  authority-domain query Operation, credential context, five-minute validity
  window, JSON-safe envelope, oneof validation, domain checks, and shared
  exit-code handling.
- Replaced the three stubs with flag-to-query builders and safe projections in
  `cli/src/commands/audit-query.ts`, `cli/src/commands/inspect-command.ts`,
  and `cli/src/commands/adapter-status.ts`. Added generated-enum parsing,
  RFC3339/cursor/limit validation, strict canonical target parsing, complete
  redacted JSON views, compact tables, and pagination notices.
- Added shared table/event/timestamp/target formatting to `cli/src/output.ts`,
  reused it from `session-health`, and wired dispatch, positional validation,
  flags, and help text in `cli/src/main.ts`. Removed only the obsolete stub
  regression test.
- Verification: `cd cli && npm test` passed (16 tests, build plus Node test
  runner). A direct generated-wire smoke check decoded the submitted payload
  as `patchbay.DiagnosticsQuery` and confirmed `query` kind, authority-domain
  target, and protobuf schema ref. `cd contracts/ts && npm run check:drift`
  reports pre-existing generated-contract drift and was not repaired because
  generated contracts are outside this worker's write scope; its incidental
  generated-file changes were reverted.
- Deviation: `docs/UX.md`, `docs/RUNBOOK.md`, and `docs/SECURITY.md` were not
  edited because the worker's explicit write scope permits only `cli/` and
  this feature file. The implementation therefore leaves the prose roll-forward
  for the owning documentation scope rather than touching forbidden files.

## Review findings (standard pass 1, 2026-07-26 — independent reviewer: gpt-5.6-sol)

Verdict: blockers-found. Receiver-confirmed blockers (fix before `done`):

1. **Post-acceptance query failure misclassified** — accepted + FAILED +
   execution_failed (no result envelope) falls through to "incomplete
   envelope" exit 1 instead of lifecycle-failure exit 3; rendering must also
   require OperationState.COMPLETED. Fix: branch on submission outcome AND
   operation state; unexpected nonterminal states fail closed with 1.
2. **Per-command option grammar not enforced** — global option admission lets
   `adapter-status --kind` / `session-health --limit` pass silently; empty
   cursor values silently omitted; duplicate enum flags silently deduped.
   Fix: per-command allowlists, direct cursor pass-through, duplicate-enum
   rejection, help text documents target grammar + cursor/time inclusivity.
3. **JSON projections omit pre-existing safe contract fields** —
   `AuditRecord.adapter_diagnostic`, `AdapterCapabilitySummary
   .diagnostic_reporting`, `AdapterStatus.recent_diagnostics` all existed in
   the generated contracts before the implementation commit and are omitted
   from JSON output. Fix: explicit JSON-safe projections via generated enums.
4. **No diagnostics regression evidence** — the commit only deleted the stub
   test; the designed wire/exit/empty/oneof/pagination/JSON-safety/parser/
   redaction tests were never added. Fix: add them.
5. **UX.md + RUNBOOK.md still describe the commands as stubs** — now-false
   current-state assertions; rolling them forward was a design unit that the
   worker's write scope (orchestrator error) excluded. Fix: roll both docs
   forward to the implemented command/JSON/empty/pagination/exit contract.

Parked notes: runtime-target percent-encoding reversibility; unused
`makeEventCursor`/`enumDisplay` re-exports (fix worker may delete as cleanup).

## Review resolution

1. **Post-acceptance failure misclassification** — `runDiagnosticsCommand`
   now branches on both `SubmissionOutcome` and `OperationState`: accepted +
   completed requires the expected typed result envelope, accepted + failed
   emits the submission failure detail and exits 3, and every other accepted
   lifecycle state fails closed with exit 1. Pre-acceptance outcomes retain
   exits 2/3/4 and transport/protocol errors remain exit 1. Evidence:
   `cli/src/commands/diagnostics.ts`; regression tests cover typed-empty,
   rejection, accepted failure, and nonterminal lifecycle cases.
2. **Per-command option grammar** — `cli/src/main.ts` now validates command
   allowlists before client construction/network access, duplicate generated
   enum options are rejected, and explicit cursor values are not dropped by
   truthiness checks. Help documents target syntax and inclusive/exclusive
   time/cursor semantics. Evidence: option-grammar, duplicate-enum, empty
   opaque-cursor, omitted-limit, and help assertions in
   `cli/tests/output-diagnostics.test.ts`.
3. **JSON projections** — `auditRecordView` now includes the generated
   `adapter_diagnostic` detail, capability projections include generated
   `diagnostic_reporting`, and adapter status projections include
   `recent_diagnostics`; enum and timestamp formatting uses existing safe
   helpers. Evidence: `cli/src/commands/diagnostics.ts` projection and
   redaction regression tests.
4. **Regression evidence** — added generated-contract fixtures and 10
   diagnostics boundary tests covering wire decoding, defaults, exit paths,
   lifecycle oneof validation, safe projections, redaction, parser grammar,
   duplicate enums, and cursor preservation. `cd cli && npm test`: 26 tests
   pass.
5. **Docs roll-forward** — `docs/UX.md` and `docs/RUNBOOK.md` now describe
   all three commands, flags, JSON envelope, typed-empty success, pagination
   defaults/maxima, and exit codes 0/1/2/3/4 without current-stub claims.

The runtime-target percent-encoding reversibility note remains parked by
request. No files outside the declared write scope were changed.
