#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { readdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '../..');
const vectorDir = path.join(repoRoot, 'contracts', 'vectors');
const protoDir = path.join(repoRoot, 'contracts', 'proto', 'patchbay');
const verificationPath = path.join(repoRoot, 'docs', 'VERIFICATION.md');

// Property registry maintenance note:
// Update these lists whenever docs/VERIFICATION.md adds, removes, promotes, or
// demotes property ids. The generated traceability table below is the checked-in
// sync surface that makes drift visible during review.
const CHECKED_MODEL_PROPERTIES = [
  'BoundaryDedup',
  'browser_local_state_not_authority',
  'CsrfRejectsMissingProof',
  'CsrfRejectsUnauthenticated',
  'GenerationMonotonic',
  'NoAcceptedToCompleted',
  'RevokedSessionCannotCommand',
  'SessionReportSourceOrdering',
  'TerminalFinality',
];

// Checked-model properties become checked-normative only when a promoted
// product-seam vector independently covers the same property.
const CHECKED_NORMATIVE_PROPERTIES = [
  'SessionReportSourceOrdering',
];

const STATED_NORMATIVE_PROPERTIES = [
  'ActorIdsUnique',
  'AuthorityGraphAcyclic',
  'CommandDurability',
  'CompoundIssuer',
  'CrashNoAcceptedLost',
  'ElicitationCorrelationTyped',
  'ElicitationFirstAnswerWins',
  'ElicitationInvalidResponseRejected',
  'ElicitationPendingFinality',
  'ElicitationResponderAuthority',
  'ElicitationStaleTargetInert',
  'ElicitationTimeoutNeitherSuccessNorDenial',
  'ElicitationWithdrawalFinality',
  'FleetAuthorityForSpawn',
  'GrantAuthorityIsCommandKinds',
  'GrantAuthorityIsOperationKinds',
  'IdempotentLogReplay',
  'LabelsCannotOverrideIdentity',
  'LateEventNoRewrite',
  'LateGenerationInert',
  'LsnDeterminesTerminalWinner',
  'NoCommandWithoutGrant',
  'NoOperationWithoutGrant',
  'PreAppendTerminalChoice',
  'RetryAfterTerminalReturnsExisting',
  'RetryReusesIdAndKey',
  'RevocationPreventsFuture',
  'RevokeAllInvalidatesPriorSessionGeneration',
  'PrincipalRevocationPreventsFuture',
  'EndpointRevocationPreventsFuture',
  'DeviceRevocationPreventsFuture',
  'LockdownRejectsNewOperations',
  'LockdownReplayPersists',
  'LockdownEntryStalesSessions',
  'LockdownInvalidatesExistingOperatorSessions',
  'BootstrapOnlyExit',
  'SenderMatchesClaim',
  'SessionIdentityTuple',
  'SnapshotConsistentPrefix',
  'SnapshotCrossDomainRejected',
  'SnapshotStaleRejected',
  'SpawnCreatesDescendantGrant',
  'SpawnRevocationDoesNotCascade',
  'SubscriptionAudited',
  'SubscriptionCursorReplayAuthorized',
  'SubscriptionGrantChecked',
  'TimeoutNeitherSuccessNorDenial',
  'TypedCorrelation',
  'ResourceObservationSourceAuthenticated',
  'ResourceSnapshotCompletenessHonesty',
  'ResourceStaleNeverLive',
  'ResourceIdentityCollisionFenced',
  'ResourceCoreStateInjectionRejected',
  'TokenCommunePartialSnapshotHonesty',
  'TokenCommuneBoundedReconnectHonesty',
  'TokenCommuneDegradationHonesty',
  'TokenCommuneCurrentGenerationSourceAuthenticated',
  'TokenCommuneGatewayMemberKeyRedacted',
  'TokenCommuneAdapterFailureSafe',
  'TokenCommuneCockpitPresentationHonesty',
];

// Descriptive, non-formal ids that may appear in draft boundary vectors. They
// are intentionally not valid promotion targets because they do not identify a
// formal or reserved property in docs/VERIFICATION.md.
const DESCRIPTIVE_DRAFT_ONLY_PROPERTY_IDS = new Set(['boundary-validation']);

const IMPLEMENTATION_RUNNERS = Object.freeze({
  'rust-core': {
    command: 'cargo',
    args: ['test', '-q', '-p', 'patchbay-core', '--test', 'conformance_vectors', '--', '--nocapture'],
  },
  'rust-server': {
    command: 'cargo',
    args: ['test', '-q', '-p', 'patchbay-core-server', '--features', 'conformance-fault-injection', '--test', 'conformance_vectors', '--', '--nocapture'],
  },
  'web-cockpit': {
    command: 'npm',
    args: ['--prefix', 'web-cockpit', 'test', '--', '--test-name-pattern=conformance vector runner'],
  },
  'token-commune-adapter': {
    command: 'npm',
    args: ['--prefix', 'token-commune-adapter', 'run', 'test:conformance'],
  },
});

const TOKEN_COMMUNE_PROFILE = Object.freeze([
  {
    property: 'TokenCommunePartialSnapshotHonesty',
    vector: 'token-commune-partial-snapshot-honesty',
    checks: [{ runner: 'token-commune-adapter', case: 'partial_snapshot_honesty' }],
  },
  {
    property: 'TokenCommuneBoundedReconnectHonesty',
    vector: 'token-commune-bounded-reconnect-honesty',
    checks: [{ runner: 'token-commune-adapter', case: 'bounded_reconnect_honesty' }],
  },
  {
    property: 'TokenCommuneDegradationHonesty',
    vector: 'token-commune-degradation-honesty',
    checks: [
      { runner: 'token-commune-adapter', case: 'degradation_failed_poll_report' },
      { runner: 'rust-server', case: 'token_commune_degradation_projection' },
    ],
  },
  {
    property: 'TokenCommuneCurrentGenerationSourceAuthenticated',
    vector: 'token-commune-current-generation-source-authenticated',
    checks: [{ runner: 'rust-server', case: 'token_commune_current_generation_source_binding' }],
  },
  {
    property: 'TokenCommuneGatewayMemberKeyRedacted',
    vector: 'token-commune-gateway-key-redaction',
    checks: [{ runner: 'token-commune-adapter', case: 'gateway_key_redaction' }],
  },
  {
    property: 'TokenCommuneAdapterFailureSafe',
    vector: 'token-commune-unsupported-operation-terminalization',
    checks: [{ runner: 'token-commune-adapter', case: 'unsupported_operation_terminalization' }],
  },
  {
    property: 'TokenCommuneCockpitPresentationHonesty',
    vector: 'token-commune-cockpit-presentation-honesty',
    checks: [
      { runner: 'token-commune-adapter', case: 'cockpit_projection_fixture' },
      { runner: 'web-cockpit', case: 'token_commune_cockpit_presentation' },
    ],
  },
]);

const TOKEN_COMMUNE_BY_VECTOR = new Map(TOKEN_COMMUNE_PROFILE.map((entry) => [entry.vector, entry]));
const TOKEN_COMMUNE_BY_PROPERTY = new Map(TOKEN_COMMUNE_PROFILE.map((entry) => [entry.property, entry]));
const MUTATION_RUNNERS = new Set(['rust-server', 'token-commune-adapter', 'web-cockpit']);

function expectation(ok, detail) {
  return { ok: Boolean(ok), detail };
}

function expectedCases(vector) {
  const session = vector.expected_outcome?.session_case;
  const resource = vector.expected_outcome?.resource_case;
  if (session !== undefined && resource !== undefined) {
    return [
      { name: 'session_case', input: vector.input?.session_case, outcome: session },
      { name: 'resource_case', input: vector.input?.resource_case, outcome: resource },
    ];
  }
  return [{ name: 'single_case', input: vector.input?.resource_case ?? vector.input, outcome: resource ?? vector.expected_outcome }];
}

function everyExpectedCase(vector, detail, check) {
  const failed = expectedCases(vector).find((entry) => !check(entry));
  return expectation(failed === undefined, failed === undefined ? detail : `${failed.name}: ${detail}`);
}

// These check raw expected examples only. Package runners separately execute the
// same vector fields against product seams; keeping the two checks independent
// prevents a successful implementation test from laundering a contradictory
// expected outcome.
const INVARIANT_EXPECTATION_CHECKS = Object.freeze({
  CommandDurability: (vector) => everyExpectedCase(
    vector,
    'acceptance must produce an accepted durable record before delivery',
    ({ name, outcome }) => outcome?.submission_result?.outcome === 'SUBMISSION_OUTCOME_ACCEPTED'
      && outcome?.submission_result?.operation_state === 'OPERATION_STATE_ACCEPTED'
      && outcome?.durable_record?.operation_state === 'OPERATION_STATE_ACCEPTED'
      && (name !== 'resource_case' || outcome?.durable_before_delivery === true),
  ),
  NoCommandWithoutGrant: (vector) => everyExpectedCase(
    vector,
    'a missing grant must reject before append and delivery',
    ({ outcome }) => outcome?.submission_result?.outcome === 'SUBMISSION_OUTCOME_REJECTED'
      && outcome?.submission_result?.failure_code === 'FAILURE_CODE_AUTHORIZATION_DENIED'
      && outcome?.durable_acceptance_record_created === false
      && outcome?.delivered_to_adapter === false,
  ),
  IdempotentLogReplay: (vector) => expectation(
    vector.input?.initial?.lsn === 1
      && vector.input?.initial?.adapter_generation === 1
      && vector.input?.replacement?.lsn === 2
      && vector.input?.replacement?.adapter_generation === 2
      && vector.input?.covered_refeed?.lsn === 1
      && vector.input?.covered_refeed?.adapter_generation === 1
      && vector.input?.lower_generation_next_candidate?.lsn === 3
      && vector.input?.lower_generation_next_candidate?.adapter_generation === 1
      && vector.input?.sibling_prefix_probe?.lsn === 3
      && vector.input?.sibling_prefix_probe?.stored_event_kind === 'STORED_EVENT_KIND_OBSERVATION'
      && vector.input?.failed_replacement?.lsn === 4
      && vector.input?.failed_replacement?.adapter_generation === 2
      && JSON.stringify(vector.input?.failed_replacement?.tombstone_identity) === JSON.stringify(['token-commune', 'provider_pool', 'new'])
      && vector.input?.failed_replacement?.tombstone_from_revision_lsn === 2
      && JSON.stringify(vector.input?.failed_replacement?.paired_upsert_identity) === JSON.stringify(['token-commune', 'provider_pool', 'old'])
      && vector.input?.failed_replacement?.upsert_from_revision_lsn === 2
      && JSON.stringify(vector.input?.retired_mutation_candidates) === JSON.stringify(['upsert', 'unknown', 'tombstone'])
      && vector.expected_outcome?.initial_applied === true
      && vector.expected_outcome?.replacement_applied_atomically === true
      && vector.expected_outcome?.retired_identity_tombstoned === true
      && vector.expected_outcome?.replacement_identity_active === true
      && vector.expected_outcome?.covered_refeed_result === 'success_no_change'
      && vector.expected_outcome?.covered_refeed_applied_through_lsn === 2
      && vector.expected_outcome?.lower_generation_next_result === 'corrupt_log'
      && vector.expected_outcome?.rejected_candidate_applied_through_lsn === 2
      && vector.expected_outcome?.sibling_probe_advanced_prefix === true
      && vector.expected_outcome?.sibling_probe_resource_state_unchanged === true
      && vector.expected_outcome?.final_applied_through_lsn === 3
      && vector.expected_outcome?.failed_replacement_result === 'terminal_tombstone'
      && JSON.stringify(vector.expected_outcome?.failed_replacement_error_identity) === JSON.stringify(['token-commune', 'provider_pool', 'old'])
      && vector.expected_outcome?.failed_replacement_applied_through_lsn === 3
      && vector.expected_outcome?.failed_replacement_full_projection_unchanged === true
      && vector.expected_outcome?.failed_replacement_views_unchanged === true
      && vector.expected_outcome?.failed_replacement_durable_event_count === 3
      && JSON.stringify(vector.expected_outcome?.retired_mutations_rejected) === JSON.stringify(['upsert', 'unknown', 'tombstone'])
      && vector.expected_outcome?.durable_event_count === 3
      && vector.expected_outcome?.projection_unchanged_after_each_rejection === true
      && vector.expected_outcome?.hot_equals_fresh_replay === true
      && vector.expected_outcome?.fresh_replays_equal === true
      && vector.expected_outcome?.covered_prefix_replay_is_idempotent === true,
    'exact covered-record replay must be inert, while lower-generation, failed-replacement, and terminal candidates preserve the cursor-bearing projection and durable prefix',
  ),
  SessionReportSourceOrdering: (vector) => expectation(
    JSON.stringify(vector.input?.primary_reports) === JSON.stringify([
      { adapter_generation: 1, revision: 1, model: 'A' },
      { adapter_generation: 1, revision: 3, model: 'B' },
      { adapter_generation: 1, revision: 2, model: 'A' },
    ])
      && vector.input?.initial_attachment_generation === 1
      && vector.input?.runtime_session_generation === 1
      && vector.input?.adapter_generation_reset?.attachment_generation === 2
      && vector.input?.adapter_generation_reset?.accepted_revision === 1
      && vector.input?.adapter_generation_reset?.old_adapter_generation === 1
      && vector.input?.runtime_generation_reset?.session_generation === 2
      && vector.input?.runtime_generation_reset?.accepted_revision === 1
      && vector.input?.runtime_generation_reset?.old_session_generation === 1
      && JSON.stringify(vector.expected_outcome?.primary?.accepted_models) === JSON.stringify(['A', 'B'])
      && vector.expected_outcome?.primary?.delayed_status === 'QUARANTINED_RUNTIME_EVIDENCE'
      && vector.expected_outcome?.primary?.session_state_event_count === 2
      && vector.expected_outcome?.primary?.audit_kind === 'AUDIT_EVENT_KIND_STALE_EVENT_IGNORED'
      && vector.expected_outcome?.primary?.audit_failure_code === 'FAILURE_CODE_STALE_EVENT'
      && vector.expected_outcome?.primary?.audit_reason_code === 'runtime_evidence_stale_source_order'
      && vector.expected_outcome?.primary?.snapshot_model === 'B'
      && vector.expected_outcome?.primary?.snapshot_adapter_generation === 1
      && vector.expected_outcome?.primary?.snapshot_revision === 3
      && vector.expected_outcome?.primary?.hot_equals_replay === true
      && vector.expected_outcome?.adapter_generation_reset?.accepted_model === 'C'
      && vector.expected_outcome?.adapter_generation_reset?.snapshot_adapter_generation === 2
      && vector.expected_outcome?.adapter_generation_reset?.snapshot_revision === 1
      && vector.expected_outcome?.adapter_generation_reset?.old_producer_status === 'QUARANTINED_RUNTIME_EVIDENCE'
      && vector.expected_outcome?.adapter_generation_reset?.old_producer_mutated === false
      && vector.expected_outcome?.runtime_generation_reset?.accepted_model === 'D'
      && vector.expected_outcome?.runtime_generation_reset?.snapshot_session_generation === 2
      && vector.expected_outcome?.runtime_generation_reset?.snapshot_adapter_generation === 2
      && vector.expected_outcome?.runtime_generation_reset?.snapshot_revision === 1
      && vector.expected_outcome?.runtime_generation_reset?.old_runtime_status === 'QUARANTINED_RUNTIME_EVIDENCE'
      && vector.expected_outcome?.runtime_generation_reset?.old_runtime_mutated === false,
    'A/r1 then B/r3 must fence delayed A/r2 with stale audit and B/r3 snapshot; only newer adapter/runtime generations reset revision',
  ),
  SnapshotStaleRejected: (vector) => everyExpectedCase(
    vector,
    'an older snapshot must be rejected and replaced by the current view',
    ({ name, input, outcome }) => outcome?.snapshot_decision?.accepted === false
      && outcome?.snapshot_decision?.failure_code === 'FAILURE_CODE_STALE_EVENT'
      && Number(outcome?.snapshot_decision?.replacement_required_from_lsn?.value)
        > Number(input?.cached_snapshot?.snapshot_lsn?.value)
      && (name !== 'resource_case'
        || (outcome?.requested_view_kind === 'SNAPSHOT_VIEW_KIND_RESOURCE'
          && outcome?.returned_view_kind === 'SNAPSHOT_VIEW_KIND_RESOURCE')),
  ),
  ResourceObservationSourceAuthenticated: (vector) => expectation(
    vector.expected_outcome?.authenticated_owner?.observation_appended === true
      && vector.expected_outcome?.unauthenticated?.observation_appended === false
      && vector.expected_outcome?.stale_token?.observation_appended === false
      && vector.expected_outcome?.cross_adapter_target?.observation_appended === false
      && vector.expected_outcome?.forged_claim?.authority_changed === false,
    'only the current owning adapter channel may append evidence, and evidence cannot create authority',
  ),
  ResourceSnapshotCompletenessHonesty: (vector) => expectation(
    vector.expected_outcome?.authoritative_omission === 'tombstoned'
      && vector.expected_outcome?.partial_cached_omission === 'stale'
      && vector.expected_outcome?.none_cached_omission === 'stale'
      && vector.expected_outcome?.no_payload_omission === 'unknown'
      && vector.expected_outcome?.delta_omission === 'unchanged'
      && vector.expected_outcome?.hot_equals_replay === true,
    'snapshot tiers and delta omission must follow the independent completeness truth table',
  ),
  ResourceStaleNeverLive: (vector) => expectation(
    vector.expected_outcome?.current_eligibility?.reconciled === true
      && vector.expected_outcome?.current_eligibility?.tombstoned === false
      && vector.expected_outcome?.current_eligibility?.freshness === 'RESOURCE_FRESHNESS_STATE_CURRENT'
      && vector.expected_outcome?.disallowed_states_render_current === false
      && vector.expected_outcome?.adapter_health_overrides_freshness === false,
    'current presentation requires reconciled, non-tombstoned current resource state',
  ),
  ResourceIdentityCollisionFenced: (vector) => expectation(
    vector.expected_outcome?.exact_tuple_authorized === true
      && vector.expected_outcome?.changed_adapter_authorized === false
      && vector.expected_outcome?.changed_kind_authorized === false
      && vector.expected_outcome?.changed_resource_id_authorized === false,
    'resource authority must compare every identity-tuple dimension',
  ),
  ResourceCoreStateInjectionRejected: (vector) => expectation(
    vector.expected_outcome?.stored_event_kind === 'STORED_EVENT_KIND_OBSERVATION'
      && vector.expected_outcome?.resource_registry_changed === false
      && vector.expected_outcome?.resource_resolved === false
      && vector.expected_outcome?.adapter_assigned_lsn_accepted === false,
    'opaque Observation bytes must remain evidence and never become core-owned resource state',
  ),
  TokenCommunePartialSnapshotHonesty: (vector) => expectation(
    Array.isArray(vector.expected_outcome?.views)
      && vector.expected_outcome.views.length === 2
      && vector.expected_outcome.views.every((view) => view?.completeness === 'ADAPTER_SNAPSHOT_SUPPORT_PARTIAL')
      && vector.expected_outcome?.omission_is_tombstone === false
      && vector.expected_outcome?.capacity_aggregation === 'none',
    'token-commune snapshots must expose exactly two PARTIAL views without tombstone or aggregate claims',
  ),
  TokenCommuneBoundedReconnectHonesty: (vector) => expectation(
    vector.expected_outcome?.initial_baseline_replayed === false
      && vector.expected_outcome?.acknowledge_only_after_core_acceptance === true
      && vector.expected_outcome?.saturated_gap_reason === 'window-saturated-without-anchor'
      && vector.expected_outcome?.fabricated_missed_count === null
      && vector.expected_outcome?.report_precedes_events === true,
    'latest-50 reconciliation must be baseline-safe, acknowledgement-aware, gap-explicit, and report-first',
  ),
  TokenCommuneDegradationHonesty: (vector) => expectation(
    vector.expected_outcome?.missed_poll_emits_empty_partial_views === true
      && vector.expected_outcome?.disconnect_cached_state === 'stale'
      && vector.expected_outcome?.disconnect_no_payload_state === 'unknown'
      && vector.expected_outcome?.omitted_partial_identity_removed === false
      && vector.expected_outcome?.polling_establishes_liveness === false,
    'failed polls and disconnects must preserve rows while degrading cache confidence honestly',
  ),
  TokenCommuneCurrentGenerationSourceAuthenticated: (vector) => expectation(
    vector.expected_outcome?.current_exact_tuple_appended === true
      && vector.expected_outcome?.stale_token_appended === false
      && vector.expected_outcome?.stale_generation_appended === false
      && vector.expected_outcome?.cross_owner_appended === false
      && vector.expected_outcome?.payload_claim_overrides_source === false,
    'only current authenticated attachment evidence for the exact owned target may append',
  ),
  TokenCommuneGatewayMemberKeyRedacted: (vector) => expectation(
    vector.expected_outcome?.secret_absent === true
      && Array.isArray(vector.expected_outcome?.scanned_sinks)
      && ['resource-reports', 'observations', 'diagnostics', 'audit-query', 'snapshots', 'subscriptions', 'sqlite-bytes']
        .every((sink) => vector.expected_outcome.scanned_sinks.includes(sink)),
    'the gateway member key and common encodings must be absent from every external and durable sink',
  ),
  TokenCommuneAdapterFailureSafe: (vector) => expectation(
    vector.expected_outcome?.durable_delivered_count === 1
      && vector.expected_outcome?.durable_rejected_count === 1
      && vector.expected_outcome?.terminal_state === 'OPERATION_STATE_REJECTED'
      && vector.expected_outcome?.failure_code === 'FAILURE_CODE_UNSUPPORTED_COMMAND'
      && vector.expected_outcome?.completed_count === 0
      && vector.expected_outcome?.nonterminal_after_recovery === false
      && vector.expected_outcome?.pending_precedes_later_delivery === true,
    'unsupported delivery must converge to exactly one delivered then rejected/unsupported terminal history',
  ),
  TokenCommuneCockpitPresentationHonesty: (vector) => expectation(
    vector.expected_outcome?.stale_renders_live === false
      && vector.expected_outcome?.unknown_rows_visible === true
      && vector.expected_outcome?.cross_provider_model_runnable === false
      && vector.expected_outcome?.cross_provider_model_visible === false
      && vector.expected_outcome?.safe_fingerprint_visible === true
      && vector.expected_outcome?.total_declared_share_visible === true
      && vector.expected_outcome?.draw_consumed_units_visible === true
      && vector.expected_outcome?.draw_reset_visible === true
      && vector.expected_outcome?.capacity_reset_visible === true
      && vector.expected_outcome?.old_reading_age_visible_under_current_wrapper === true
      && vector.expected_outcome?.no_5h_readings_telemetry === 'unavailable'
      && vector.expected_outcome?.all_null_5h_readings_telemetry === 'unavailable'
      && vector.expected_outcome?.resource_events_visible === true
      && vector.expected_outcome?.competing_cross_adapter_draw_joined === false
      && vector.expected_outcome?.current_capacity_used_fraction === 0.8
      && vector.expected_outcome?.forbidden_alias_visible === false
      && vector.expected_outcome?.private_fields_visible === false
      && vector.expected_outcome?.dynamic_renderer_executed === false
      && vector.expected_outcome?.verdict_owner === 'Patchbay',
    'the local cockpit compositor must preserve carried capabilities, source-time, resource-event, stale/unknown/model/privacy/verdict/renderer honesty',
  ),
});

const PROPERTY_TIERS = new Map();
for (const id of CHECKED_MODEL_PROPERTIES) PROPERTY_TIERS.set(id, 'checked-model');
for (const id of STATED_NORMATIVE_PROPERTIES) {
  if (!PROPERTY_TIERS.has(id)) PROPERTY_TIERS.set(id, 'stated-normative');
}
for (const id of CHECKED_NORMATIVE_PROPERTIES) PROPERTY_TIERS.set(id, 'checked-normative');

const VALID_PROPERTY_IDS = new Set(PROPERTY_TIERS.keys());
const MODEL_GENERATED_BEGIN = '<!-- BEGIN GENERATED MODEL-PROMOTION TRACEABILITY -->';
const MODEL_GENERATED_END = '<!-- END GENERATED MODEL-PROMOTION TRACEABILITY -->';
const GENERATED_BEGIN = '<!-- BEGIN GENERATED CONFORMANCE VECTOR TRACEABILITY -->';
const GENERATED_END = '<!-- END GENERATED CONFORMANCE VECTOR TRACEABILITY -->';

function rel(filePath) {
  return path.relative(repoRoot, filePath).replaceAll(path.sep, '/');
}

function markdownEscape(value) {
  return String(value).replaceAll('|', '\\|').replaceAll('\n', '<br>');
}

function listCell(values) {
  if (values.length === 0) return '—';
  return values.map(markdownEscape).join('<br>');
}

function assertArrayOfStrings(value) {
  return Array.isArray(value) && value.every((item) => typeof item === 'string');
}

function stripProtoCommentsAndStrings(source) {
  return source.replace(
    /"(?:\\.|[^"\\])*"|\/\*[\s\S]*?\*\/|\/\/[^\n]*/g,
    (match) => (match.startsWith('"') ? '""' : ' '),
  );
}

function closingBrace(source, openIndex) {
  let depth = 1;
  for (let index = openIndex + 1; index < source.length; index += 1) {
    if (source[index] === '{') depth += 1;
    if (source[index] === '}') depth -= 1;
    if (depth === 0) return index;
  }
  throw new Error(`unterminated proto declaration at offset ${openIndex}`);
}

function directProtoDeclarations(source) {
  const declarations = [];
  const pattern = /\b(message|enum)\s+([A-Za-z_]\w*)\s*\{/g;
  let match;
  while ((match = pattern.exec(source)) !== null) {
    const openIndex = pattern.lastIndex - 1;
    const closeIndex = closingBrace(source, openIndex);
    declarations.push({
      kind: match[1],
      name: match[2],
      start: match.index,
      end: closeIndex + 1,
      body: source.slice(openIndex + 1, closeIndex),
    });
    pattern.lastIndex = closeIndex + 1;
  }
  return declarations;
}

function sourceWithoutDeclarations(source, declarations) {
  let result = '';
  let cursor = 0;
  for (const declaration of declarations) {
    result += source.slice(cursor, declaration.start);
    result += ' '.repeat(declaration.end - declaration.start);
    cursor = declaration.end;
  }
  return result + source.slice(cursor);
}

function registerProtoDeclarations(source, packageName, parentName, schema) {
  const declarations = directProtoDeclarations(source);
  for (const declaration of declarations) {
    const localName = parentName ? `${parentName}.${declaration.name}` : declaration.name;
    const qualifiedName = `${packageName}.${localName}`;
    const nestedDeclarations = directProtoDeclarations(declaration.body);
    const declarationBody = sourceWithoutDeclarations(declaration.body, nestedDeclarations);

    if (declaration.kind === 'message') {
      if (schema.messages.has(qualifiedName) || schema.enums.has(qualifiedName)) {
        throw new Error(`duplicate proto declaration ${qualifiedName}`);
      }
      const fields = new Set();
      const fieldPattern = /\b(?:(?:optional|required|repeated)\s+)?(?:map\s*<[^;{}]+>|\.?[A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)\s+([A-Za-z_]\w*)\s*=\s*\d+\b/g;
      for (const match of declarationBody.matchAll(fieldPattern)) fields.add(match[1]);
      schema.messages.set(qualifiedName, fields);
    } else {
      if (schema.enums.has(qualifiedName) || schema.messages.has(qualifiedName)) {
        throw new Error(`duplicate proto declaration ${qualifiedName}`);
      }
      const members = new Set();
      const memberPattern = /(?:^|;)\s*([A-Za-z_]\w*)\s*=\s*-?\d+\b/gm;
      for (const match of declarationBody.matchAll(memberPattern)) members.add(match[1]);
      schema.enums.set(qualifiedName, members);
    }

    registerProtoDeclarations(declaration.body, packageName, localName, schema);
  }
}

async function readProtoSchema() {
  const entries = (await readdir(protoDir, { withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name.endsWith('.proto'))
    .map((entry) => entry.name)
    .sort();
  if (entries.length === 0) throw new Error(`${rel(protoDir)}: no *.proto schema files found`);

  const schema = { messages: new Map(), enums: new Map() };
  for (const filename of entries) {
    const source = stripProtoCommentsAndStrings(await readFile(path.join(protoDir, filename), 'utf8'));
    const packageName = source.match(/\bpackage\s+([A-Za-z_]\w*(?:\.[A-Za-z_]\w*)*)\s*;/)?.[1];
    if (packageName === undefined) throw new Error(`${filename}: proto package declaration is missing`);
    registerProtoDeclarations(source, packageName, '', schema);
  }
  return schema;
}

function protoReferenceResolves(reference, schema) {
  const declarations = [...schema.messages.keys(), ...schema.enums.keys()]
    .filter((name) => reference === name || reference.startsWith(`${name}.`))
    .sort((left, right) => right.length - left.length);

  for (const declaration of declarations) {
    const member = reference.slice(declaration.length + 1);
    if (reference === declaration) return true;
    if (member.includes('.')) continue;
    if (schema.messages.get(declaration)?.has(member)) return true;
    if (schema.enums.get(declaration)?.has(member)) return true;
  }
  return false;
}

function validatePromotedProtoReferences(vectors, schema) {
  const errors = [];
  let checked = 0;
  for (const vector of vectors.filter((item) => item.promotion_status === 'promoted')) {
    for (const reference of vector.proto_fields_constrained ?? []) {
      checked += 1;
      if (!protoReferenceResolves(reference, schema)) {
        errors.push(`vector ${vector.vector_id}: proto reference ${reference} does not resolve in the schema`);
      }
    }
  }
  return { errors, checked };
}

function validateImplementationChecks(vector, filename) {
  const errors = [];
  const checks = vector.implementation_checks;
  if (checks === undefined) {
    if (vector.promotion_status === 'promoted') {
      errors.push(`${filename}: promoted vectors require a non-empty implementation_checks array`);
    }
    return errors;
  }
  if (!Array.isArray(checks)) {
    return [`${filename}: implementation_checks must be an array`];
  }
  if (vector.promotion_status === 'promoted' && checks.length === 0) {
    errors.push(`${filename}: promoted vectors require a non-empty implementation_checks array`);
  }
  const seen = new Set();
  for (const [index, check] of checks.entries()) {
    if (!check || typeof check !== 'object' || Array.isArray(check)) {
      errors.push(`${filename}: implementation_checks[${index}] must be an object`);
      continue;
    }
    if (!(check.runner in IMPLEMENTATION_RUNNERS)) {
      errors.push(`${filename}: implementation_checks[${index}] has unknown runner ${String(check.runner)}`);
    }
    if (typeof check.case !== 'string' || check.case.length === 0) {
      errors.push(`${filename}: implementation_checks[${index}].case must be a non-empty string`);
      continue;
    }
    const id = `${check.runner}:${check.case}`;
    if (seen.has(id)) errors.push(`${filename}: duplicate implementation check ${id}`);
    seen.add(id);
  }
  return errors;
}

function validateMutationWitnesses(vector, filename) {
  const errors = [];
  const witnesses = vector.mutation_witnesses;
  const profile = TOKEN_COMMUNE_BY_VECTOR.get(vector.vector_id);
  if (witnesses === undefined) {
    if (profile && vector.promotion_status === 'promoted') {
      errors.push(`${filename}: promoted token-commune vectors require non-empty mutation_witnesses`);
    }
    return errors;
  }
  if (!Array.isArray(witnesses)) return [`${filename}: mutation_witnesses must be an array`];
  if (profile && vector.promotion_status === 'promoted' && witnesses.length === 0) {
    errors.push(`${filename}: promoted token-commune vectors require non-empty mutation_witnesses`);
  }
  const seen = new Set();
  for (const [index, witness] of witnesses.entries()) {
    if (!witness || typeof witness !== 'object' || Array.isArray(witness)) {
      errors.push(`${filename}: mutation_witnesses[${index}] must be an object`);
      continue;
    }
    if (typeof witness.mutation_id !== 'string' || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(witness.mutation_id)) {
      errors.push(`${filename}: mutation_witnesses[${index}].mutation_id must be a non-empty kebab-case id`);
      continue;
    }
    if (!MUTATION_RUNNERS.has(witness.runner)) {
      errors.push(`${filename}: mutation_witnesses[${index}] has unsupported runner ${String(witness.runner)}`);
    }
    if (typeof witness.invariant !== 'string' || witness.invariant.trim().length === 0) {
      errors.push(`${filename}: mutation_witnesses[${index}].invariant must be a non-empty string`);
    }
    if (seen.has(witness.mutation_id)) errors.push(`${filename}: duplicate mutation witness ${witness.mutation_id}`);
    seen.add(witness.mutation_id);
  }
  return errors;
}

function validateEnvelope(vector, filename) {
  const errors = [];
  const required = [
    'vector_id',
    'property_id',
    'promotion_status',
    'proto_fields_constrained',
    'description',
    'input',
    'expected_outcome',
    'invariant_check',
  ];

  for (const field of required) {
    if (!(field in vector)) errors.push(`${filename}: missing required field ${field}`);
  }

  if (typeof vector.vector_id !== 'string' || vector.vector_id.length === 0) {
    errors.push(`${filename}: vector_id must be a non-empty string`);
  } else if (`${vector.vector_id}.json` !== filename) {
    errors.push(`${filename}: vector_id must match filename without .json`);
  }

  if (typeof vector.property_id !== 'string' || vector.property_id.length === 0) {
    errors.push(`${filename}: property_id must be a non-empty string`);
  }

  if (!['draft', 'promoted'].includes(vector.promotion_status)) {
    errors.push(`${filename}: promotion_status must be draft or promoted`);
  }

  if (!assertArrayOfStrings(vector.proto_fields_constrained)) {
    errors.push(`${filename}: proto_fields_constrained must be an array of strings`);
  }

  if (typeof vector.description !== 'string' || vector.description.length === 0) {
    errors.push(`${filename}: description must be a non-empty string`);
  }

  if (typeof vector.input !== 'object' || vector.input === null || Array.isArray(vector.input)) {
    errors.push(`${filename}: input must be an object`);
  }

  if (typeof vector.expected_outcome !== 'object' || vector.expected_outcome === null || Array.isArray(vector.expected_outcome)) {
    errors.push(`${filename}: expected_outcome must be an object`);
  }

  if (typeof vector.invariant_check !== 'string' || vector.invariant_check.length === 0) {
    errors.push(`${filename}: invariant_check must be a non-empty string`);
  }

  errors.push(...validateImplementationChecks(vector, filename));
  errors.push(...validateMutationWitnesses(vector, filename));
  return errors;
}

async function readVectors() {
  const entries = (await readdir(vectorDir, { withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
    .map((entry) => entry.name)
    .sort();

  const vectors = [];
  const errors = [];
  const ids = new Map();

  for (const filename of entries) {
    const fullPath = path.join(vectorDir, filename);
    let parsed;
    try {
      parsed = JSON.parse(await readFile(fullPath, 'utf8'));
    } catch (error) {
      errors.push(`${filename}: invalid JSON: ${error.message}`);
      continue;
    }

    const envelopeErrors = validateEnvelope(parsed, filename);
    errors.push(...envelopeErrors);

    if (typeof parsed.vector_id === 'string') {
      if (ids.has(parsed.vector_id)) {
        errors.push(`${filename}: duplicate vector_id ${parsed.vector_id} also used by ${ids.get(parsed.vector_id)}`);
      } else {
        ids.set(parsed.vector_id, filename);
      }
    }

    vectors.push({ ...parsed, filename, fullPath });
  }

  if (entries.length === 0) errors.push(`${rel(vectorDir)}: no *.json vector files found`);

  return { vectors, errors };
}

function validateTokenCommuneProfile(vectors) {
  const errors = [];
  const byVector = new Map(vectors.map((vector) => [vector.vector_id, vector]));
  for (const profile of TOKEN_COMMUNE_PROFILE) {
    const vector = byVector.get(profile.vector);
    if (!vector) {
      errors.push(`token-commune profile: missing exact vector ${profile.vector}`);
      continue;
    }
    if (vector.property_id !== profile.property) {
      errors.push(`${vector.filename}: token-commune profile requires property_id ${profile.property}`);
    }
    const actual = (vector.implementation_checks ?? [])
      .map((check) => `${check.runner}:${check.case}`).sort();
    const expected = profile.checks.map((check) => `${check.runner}:${check.case}`).sort();
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
      errors.push(`${vector.filename}: implementation_checks must equal the profile; expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    }
  }
  for (const vector of vectors) {
    if ((vector.vector_id.startsWith('token-commune-') || vector.property_id.startsWith('TokenCommune'))
        && !TOKEN_COMMUNE_BY_VECTOR.has(vector.vector_id)) {
      errors.push(`${vector.filename}: token-commune vector/property is outside the exact certification profile`);
    }
    const propertyProfile = TOKEN_COMMUNE_BY_PROPERTY.get(vector.property_id);
    if (propertyProfile && propertyProfile.vector !== vector.vector_id) {
      errors.push(`${vector.filename}: ${vector.property_id} is bound only to ${propertyProfile.vector}`);
    }
  }
  const statuses = new Set(TOKEN_COMMUNE_PROFILE.map((profile) => byVector.get(profile.vector)?.promotion_status));
  if (statuses.has('promoted') && statuses.has('draft')) {
    errors.push('token-commune profile: partial promotion is forbidden; all seven exact vectors promote together');
  }
  return errors;
}

function validatePropertyReferences(vectors) {
  const errors = [];

  for (const vector of vectors) {
    if (DESCRIPTIVE_DRAFT_ONLY_PROPERTY_IDS.has(vector.property_id)) {
      if (vector.promotion_status !== 'draft') {
        errors.push(`${vector.filename}: descriptive property_id ${vector.property_id} is draft-only and cannot be promoted`);
      }
      continue;
    }

    if (!VALID_PROPERTY_IDS.has(vector.property_id)) {
      errors.push(`${vector.filename}: unknown property_id ${vector.property_id}; update the registry or fix the vector`);
    }
  }

  return errors;
}

function exactRegistryErrors(label, expectedValues, actualValues) {
  const expected = new Set(expectedValues);
  const actual = new Set(actualValues);
  const missing = [...expected].filter((id) => !actual.has(id)).sort();
  const unexpected = [...actual].filter((id) => !expected.has(id)).sort();
  const errors = [];
  if (missing.length > 0) errors.push(`${label}: docs are missing ${missing.join(', ')}`);
  if (unexpected.length > 0) errors.push(`${label}: docs declare unregistered ids ${unexpected.join(', ')}`);
  if (actual.size !== actualValues.length) errors.push(`${label}: docs declare duplicate property ids`);
  return errors;
}

const COUNT_WORDS = Object.freeze({
  zero: 0,
  one: 1,
  two: 2,
  three: 3,
  four: 4,
  five: 5,
  six: 6,
  seven: 7,
  eight: 8,
  nine: 9,
  ten: 10,
  eleven: 11,
  twelve: 12,
  thirteen: 13,
  fourteen: 14,
  fifteen: 15,
  sixteen: 16,
  seventeen: 17,
  eighteen: 18,
  nineteen: 19,
  twenty: 20,
});

function validateAssertedCount(markdown, { label, pattern, derived, source }) {
  const match = markdown.match(pattern);
  if (match === null) {
    return [`${rel(verificationPath)}: ${label} assertion is missing or malformed; derived ${derived} from ${source}`];
  }
  const token = match[1].toLowerCase();
  const asserted = /^\d+$/.test(token) ? Number(token) : COUNT_WORDS[token];
  if (asserted === undefined) {
    return [`${rel(verificationPath)}: ${label} assertion is not a supported count: asserted ${JSON.stringify(match[1])}, derived ${derived} from ${source}`];
  }
  if (asserted !== derived) {
    return [`${rel(verificationPath)}: ${label} mismatch: asserted ${asserted}, derived ${derived} from ${source}`];
  }
  return [];
}

function validateTokenCommuneMutationLedger(markdown, vectors) {
  const errors = [];
  const heading = '### token-commune observer conformance evidence (implementation-checked)';
  const start = markdown.indexOf(heading);
  const end = start === -1 ? -1 : markdown.indexOf('\n### ', start + heading.length);
  if (start === -1 || end === -1) {
    return [`${rel(verificationPath)}: token-commune mutation ledger section is missing or malformed`];
  }

  const section = markdown.slice(start, end);
  const expectedHeader = '| Property id | Executable vector | Product seam and independent oracle | Declared killed mutation ids (validated from vectors) | Assurance tier |';
  if (!section.includes(expectedHeader)) {
    errors.push(`${rel(verificationPath)}: token-commune mutation ledger header is missing or malformed`);
  }

  const byProperty = new Map(vectors.map((vector) => [vector.property_id, vector]));
  const rows = new Map();
  for (const line of section.split('\n')) {
    if (!line.startsWith('| `TokenCommune')) continue;
    const columns = line.slice(2, -2).split(' | ');
    const property = columns[0]?.match(/^`([^`]+)`$/)?.[1];
    if (property === undefined || columns.length !== 5) {
      errors.push(`${rel(verificationPath)}: malformed token-commune mutation ledger row: ${line}`);
      continue;
    }
    if (rows.has(property)) {
      errors.push(`${rel(verificationPath)}: duplicate token-commune mutation ledger row for ${property}`);
      continue;
    }
    rows.set(property, columns);
  }

  const expectedProperties = new Set(TOKEN_COMMUNE_PROFILE.map((profile) => profile.property));
  for (const property of rows.keys()) {
    if (!expectedProperties.has(property)) {
      errors.push(`${rel(verificationPath)}: token-commune mutation ledger declares unregistered property ${property}`);
    }
  }

  for (const profile of TOKEN_COMMUNE_PROFILE) {
    const vector = byProperty.get(profile.property);
    const columns = rows.get(profile.property);
    if (vector === undefined) {
      errors.push(`token-commune mutation ledger: missing vector declaration for ${profile.property}`);
      continue;
    }
    if (columns === undefined) {
      errors.push(`${rel(verificationPath)}: token-commune mutation ledger is missing ${profile.property}`);
      continue;
    }

    const documentedVector = columns[1]?.match(/^`([^`]+)`$/)?.[1];
    if (documentedVector !== vector.vector_id) {
      errors.push(`${rel(verificationPath)}: ${profile.property} ledger vector mismatch: expected ${vector.vector_id}, got ${String(documentedVector)}`);
    }

    const documentedMutations = [...columns[3].matchAll(/`([^`]+)`/g)].map((match) => match[1]);
    const declaredMutations = (vector.mutation_witnesses ?? []).map((witness) => witness.mutation_id);
    if (JSON.stringify(documentedMutations) !== JSON.stringify(declaredMutations)) {
      errors.push(`${rel(verificationPath)}: ${profile.property} mutation ledger mismatch: expected ${JSON.stringify(declaredMutations)} from ${vector.filename}, got ${JSON.stringify(documentedMutations)}`);
    }
  }

  return errors;
}

function validateEvidenceCounts(markdown, vectors, implementationExecuted, mutationsKilled) {
  const tokenVectorIds = new Set(TOKEN_COMMUNE_PROFILE.map((profile) => profile.vector));
  const resourceVectors = vectors.filter((vector) => vector.promotion_status === 'promoted' && !tokenVectorIds.has(vector.vector_id));
  const resourceChecks = resourceVectors.flatMap((vector) => vector.implementation_checks ?? []);
  const resourcePropertyCount = STATED_NORMATIVE_PROPERTIES.filter((id) => id.startsWith('Resource')).length;
  const errors = [
    ...validateAssertedCount(markdown, {
      label: 'resource promoted-vector count',
      pattern: /The resource-plane corpus promotes\s+([a-z]+|\d+)\s+executable examples\b/i,
      derived: resourceVectors.length,
      source: 'non-token promoted vectors',
    }),
    ...validateAssertedCount(markdown, {
      label: 'resource implementation-check count',
      pattern: /The umbrella checker runs\s+([a-z]+|\d+)\s+exact\s+package\s+checks\b/i,
      derived: resourceChecks.length,
      source: 'non-token promoted implementation checks',
    }),
    ...validateAssertedCount(markdown, {
      label: 'resource-property count',
      pattern: /None of the\s+([a-z]+|\d+)\s+new\s+resource\s+properties has a promoted formal model\b/i,
      derived: resourcePropertyCount,
      source: 'registered Resource* properties',
    }),
  ];
  const tokenPromoted = vectors.filter((vector) => vector.promotion_status === 'promoted' && tokenVectorIds.has(vector.vector_id));
  if (tokenPromoted.length > 0) {
    errors.push(
      ...validateAssertedCount(markdown, {
        label: 'token-commune promoted-vector count',
        pattern: /The token-commune profile promotes\s+([a-z]+|\d+)\s+executable examples\b/i,
        derived: tokenPromoted.length,
        source: 'token-commune profile registry',
      }),
      ...validateAssertedCount(markdown, {
        label: 'token-commune implementation-check count',
        pattern: /Its runners execute\s+([a-z]+|\d+)\s+exact\s+scenario checks\b/i,
        derived: implementationExecuted.filter((id) => tokenVectorIds.has(id.split(':')[1])).length,
        source: 'token-commune profile execution',
      }),
      ...validateAssertedCount(markdown, {
        label: 'token-commune mutation-witness count',
        pattern: /and kill\s+([a-z]+|\d+)\s+declared mutation witnesses\b/i,
        derived: mutationsKilled.filter((id) => tokenVectorIds.has(id.split(':')[1])).length,
        source: 'exact token-commune mutation-kill reports',
      }),
    );
  }
  return errors;
}

function validateVerificationPropertyRegistry(markdown) {
  const errors = [];
  const modelBegin = markdown.indexOf(MODEL_GENERATED_BEGIN);
  const modelEnd = markdown.indexOf(MODEL_GENERATED_END);
  if (modelBegin === -1 || modelEnd === -1 || modelEnd <= modelBegin) {
    return [`${rel(verificationPath)}: generated model-promotion property registry block is missing or malformed`];
  }

  const modelBlock = markdown.slice(modelBegin, modelEnd);
  const modelRows = [...modelBlock.matchAll(/^\| `([^`]+)` \| [^|]* \| (checked-model|checked-normative|stated-normative) \|/gm)]
    .map((match) => ({ id: match[1], tier: match[2] }));
  errors.push(...exactRegistryErrors(
    `${rel(verificationPath)} generated property registry`,
    PROPERTY_TIERS.keys(),
    modelRows.map((row) => row.id),
  ));
  for (const row of modelRows) {
    const expectedTier = PROPERTY_TIERS.get(row.id);
    if (expectedTier !== undefined && row.tier !== expectedTier) {
      errors.push(`${rel(verificationPath)}: property ${row.id} is ${row.tier} in docs but ${expectedTier} in check-vectors.mjs`);
    }
  }

  const checkedStart = markdown.indexOf('Current checked-model properties:');
  const checkedEnd = markdown.indexOf('\n\nThe session/principal revocation model', checkedStart);
  if (checkedStart === -1 || checkedEnd === -1) {
    errors.push(`${rel(verificationPath)}: checked-model prose registry is missing`);
  } else {
    const checkedIds = [...markdown.slice(checkedStart, checkedEnd).matchAll(/`([^`]+)`/g)]
      .map((match) => match[1])
      .filter((id) => !id.endsWith('.qnt'));
    errors.push(...exactRegistryErrors(
      `${rel(verificationPath)} checked-model prose registry`,
      CHECKED_MODEL_PROPERTIES,
      checkedIds,
    ));
  }

  const resourceLine = markdown.match(/^- Operational-resource adapter boundaries:.*$/m)?.[0];
  if (resourceLine === undefined) {
    errors.push(`${rel(verificationPath)}: operational-resource prose property registry is missing`);
  } else {
    const resourceIds = [...resourceLine.matchAll(/`([^`]+)`/g)].map((match) => match[1]);
    const registeredResourceIds = STATED_NORMATIVE_PROPERTIES.filter((id) => id.startsWith('Resource'));
    errors.push(...exactRegistryErrors(
      `${rel(verificationPath)} operational-resource prose property registry`,
      registeredResourceIds,
      resourceIds,
    ));
  }

  if (CHECKED_NORMATIVE_PROPERTIES.length === 0
      && !markdown.includes('No properties are currently checked-normative')) {
    errors.push(`${rel(verificationPath)}: checked-normative prose registry must explicitly declare that it is empty`);
  }
  return errors;
}

function validatePromotedCoverage(vectors) {
  const errors = [];
  const promotedByProperty = new Map();

  for (const vector of vectors) {
    if (vector.promotion_status !== 'promoted') continue;
    const existing = promotedByProperty.get(vector.property_id) ?? [];
    existing.push(vector);
    promotedByProperty.set(vector.property_id, existing);
  }

  for (const propertyId of CHECKED_NORMATIVE_PROPERTIES) {
    if ((promotedByProperty.get(propertyId) ?? []).length === 0) {
      errors.push(`checked-normative property ${propertyId} lacks a promoted conformance vector`);
    }
  }

  return errors;
}

function validatePromotedInvariantExpectations(vectors) {
  const errors = [];
  const checked = [];

  for (const vector of vectors.filter((item) => item.promotion_status === 'promoted')) {
    const checker = INVARIANT_EXPECTATION_CHECKS[vector.property_id];
    if (typeof checker !== 'function') {
      errors.push(`${vector.filename}: promoted vector references ${vector.property_id}, but no invariant expectation checker is registered yet`);
      continue;
    }

    const result = checker(vector);
    checked.push(vector);
    if (!result?.ok) {
      errors.push(`${vector.filename}: expected_outcome contradicts ${vector.property_id}: ${result?.detail ?? 'no detail provided'}`);
    }
  }

  return { errors, checked };
}

function requestedImplementationChecks(vectors) {
  const byRunner = new Map();
  for (const vector of vectors.filter((item) => item.promotion_status === 'promoted')) {
    for (const check of vector.implementation_checks ?? []) {
      const requests = byRunner.get(check.runner) ?? [];
      requests.push({ vector_id: vector.vector_id, case: check.case });
      byRunner.set(check.runner, requests);
    }
  }
  return byRunner;
}

function requestedMutationWitnesses(vectors) {
  const byRunner = new Map();
  for (const vector of vectors.filter((item) => item.promotion_status === 'promoted')) {
    for (const witness of vector.mutation_witnesses ?? []) {
      const requests = byRunner.get(witness.runner) ?? [];
      requests.push({ vector_id: vector.vector_id, mutation_id: witness.mutation_id });
      byRunner.set(witness.runner, requests);
    }
  }
  return byRunner;
}

function runImplementationChecks(vectors) {
  const errors = [];
  const executed = [];
  const mutationsKilled = [];
  const checksByRunner = requestedImplementationChecks(vectors);
  const mutationsByRunner = requestedMutationWitnesses(vectors);
  const runners = new Set([...checksByRunner.keys(), ...mutationsByRunner.keys()]);
  for (const runner of runners) {
    const requests = checksByRunner.get(runner) ?? [];
    const mutationRequests = mutationsByRunner.get(runner) ?? [];
    const spec = IMPLEMENTATION_RUNNERS[runner];
    const result = spawnSync(spec.command, spec.args, {
      cwd: repoRoot,
      env: {
        ...process.env,
        PATCHBAY_CONFORMANCE_REQUESTS: JSON.stringify(requests),
        PATCHBAY_CONFORMANCE_MUTATIONS: JSON.stringify(mutationRequests),
      },
      encoding: 'utf8',
      maxBuffer: 50 * 1024 * 1024,
    });
    const output = `${result.stdout ?? ''}${result.stderr ?? ''}`;
    if (result.error) {
      errors.push(`${runner}: runner could not start: ${result.error.message}`);
      continue;
    }
    if (result.status !== 0) {
      errors.push(`${runner}: runner exited ${result.status}\n${output.trim()}`);
      continue;
    }
    const reported = [...output.matchAll(/^PATCHBAY_CONFORMANCE_EXECUTED=(.+)$/gm)].map((match) => match[1].trim());
    const expected = requests.map((request) => `${request.vector_id}:${request.case}`).sort();
    const actual = [...reported].sort();
    if (new Set(reported).size !== reported.length) {
      errors.push(`${runner}: runner reported duplicate executed check ids`);
    }
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
      errors.push(`${runner}: executed check ids did not match request; expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    } else {
      executed.push(...reported.map((id) => `${runner}:${id}`));
    }

    const killed = [...output.matchAll(/^PATCHBAY_CONFORMANCE_MUTATION_KILLED=(.+)$/gm)].map((match) => match[1].trim());
    const expectedKilled = mutationRequests.map((request) => `${request.vector_id}:${request.mutation_id}`).sort();
    const actualKilled = [...killed].sort();
    if (new Set(killed).size !== killed.length) {
      errors.push(`${runner}: runner reported duplicate killed mutation ids`);
    }
    if (JSON.stringify(actualKilled) !== JSON.stringify(expectedKilled)) {
      errors.push(`${runner}: killed mutation ids did not match request; expected ${JSON.stringify(expectedKilled)}, got ${JSON.stringify(actualKilled)}`);
    } else {
      mutationsKilled.push(...killed.map((id) => `${runner}:${id}`));
    }
  }
  return { errors, executed, mutationsKilled };
}

function buildTraceabilityMarkdown(vectors) {
  const byProperty = new Map();
  const protoFieldsByProperty = new Map();

  for (const vector of vectors) {
    const propertyVectors = byProperty.get(vector.property_id) ?? [];
    propertyVectors.push(vector);
    byProperty.set(vector.property_id, propertyVectors);

    const fields = protoFieldsByProperty.get(vector.property_id) ?? new Set();
    for (const field of vector.proto_fields_constrained ?? []) fields.add(field);
    protoFieldsByProperty.set(vector.property_id, fields);
  }

  const formalPropertyIds = [...PROPERTY_TIERS.keys()].sort((a, b) => a.localeCompare(b));
  const descriptiveIds = [...DESCRIPTIVE_DRAFT_ONLY_PROPERTY_IDS]
    .filter((id) => byProperty.has(id))
    .sort((a, b) => a.localeCompare(b));

  const rows = [...formalPropertyIds, ...descriptiveIds].map((propertyId) => {
    const tier = PROPERTY_TIERS.get(propertyId) ?? 'descriptive boundary validation (draft-only)';
    const propertyVectors = (byProperty.get(propertyId) ?? []).sort((a, b) => a.vector_id.localeCompare(b.vector_id));
    const vectorLinks = propertyVectors.map((vector) => {
      const href = `../contracts/vectors/${vector.filename}`;
      return `[${vector.vector_id}](${href}) (${vector.promotion_status})`;
    });
    const fields = [...(protoFieldsByProperty.get(propertyId) ?? new Set())].sort((a, b) => a.localeCompare(b));
    return `| \`${markdownEscape(propertyId)}\` | ${markdownEscape(tier)} | ${listCell(vectorLinks)} | ${listCell(fields)} |`;
  });

  const vectorCount = vectors.length;
  const promotedCount = vectors.filter((vector) => vector.promotion_status === 'promoted').length;
  const checkedNormativeCount = CHECKED_NORMATIVE_PROPERTIES.length;

  return [
    GENERATED_BEGIN,
    '<!-- Generated by `node contracts/scripts/check-vectors.mjs`; do not edit this block by hand. -->',
    '',
    '### Generated conformance-vector traceability table',
    '',
    `Source vectors: \`contracts/vectors/*.json\`. CI check: \`node contracts/scripts/check-vectors.mjs\` (or \`npm run check:vectors\` from \`contracts/ts/\`).`,
    '',
    `Summary: ${vectorCount} vector(s), ${promotedCount} promoted vector(s), ${checkedNormativeCount} checked-normative propert${checkedNormativeCount === 1 ? 'y' : 'ies'} requiring promoted-vector coverage. Current checked-normative coverage gate is ${checkedNormativeCount === 0 ? 'empty by design' : 'active'}.`,
    '',
    '| Property id | Classification | Vectors | `.proto` fields/enums exercised by vectors |',
    '|---|---|---|---|',
    ...rows,
    '',
    GENERATED_END,
  ].join('\n');
}

async function expectedVerificationMarkdown(vectors) {
  const current = await readFile(verificationPath, 'utf8');
  const generated = buildTraceabilityMarkdown(vectors);
  const begin = current.indexOf(GENERATED_BEGIN);
  const end = current.indexOf(GENERATED_END);

  if (begin !== -1 && end !== -1 && end > begin) {
    return `${current.slice(0, begin)}${generated}${current.slice(end + GENERATED_END.length)}`;
  }
  const insertionPoint = current.indexOf('\n## Model promotion rule');
  if (insertionPoint === -1) {
    throw new Error(`Could not find insertion point in ${rel(verificationPath)}; expected "## Model promotion rule"`);
  }
  return `${current.slice(0, insertionPoint)}\n\n${generated}\n${current.slice(insertionPoint)}`;
}

async function traceabilityDriftErrors(vectors) {
  const current = await readFile(verificationPath, 'utf8');
  const expected = await expectedVerificationMarkdown(vectors);
  return current === expected ? [] : [
    `${rel(verificationPath)}: generated conformance-vector traceability is stale; run npm --prefix contracts/ts run generate:vectors`,
  ];
}

async function writeVerificationMarkdown(vectors) {
  const expected = await expectedVerificationMarkdown(vectors);
  await writeFile(verificationPath, expected);
}

function printSummary({ vectors, envelopeErrors, profileErrors, propertyErrors, protoErrors, protoReferencesChecked, registryErrors, coverageErrors, invariantErrors, invariantChecked, implementationErrors, implementationExecuted, mutationsKilled, evidenceErrors }) {
  const promotedVectors = vectors.filter((vector) => vector.promotion_status === 'promoted');
  const checkedModelWithoutPromoted = CHECKED_MODEL_PROPERTIES.filter(
    (propertyId) => !vectors.some((vector) => vector.property_id === propertyId && vector.promotion_status === 'promoted'),
  );

  console.log('Conformance vector check summary');
  console.log(`- vectors read: ${vectors.length}`);
  console.log(`- valid formal property ids registered: ${VALID_PROPERTY_IDS.size}`);
  console.log(`- descriptive draft-only ids allowlisted: ${[...DESCRIPTIVE_DRAFT_ONLY_PROPERTY_IDS].join(', ')}`);
  console.log(`- checked-normative properties requiring promoted vectors: ${CHECKED_NORMATIVE_PROPERTIES.length}`);
  console.log(`- promoted vectors: ${promotedVectors.length}`);
  console.log(`- promoted proto references resolved: ${protoReferencesChecked}`);
  console.log(`- promoted invariant expectation checks run: ${invariantChecked.length}`);
  console.log(`- implementation checks executed: ${implementationExecuted.length}`);
  console.log(`- mutation witnesses killed: ${mutationsKilled.length}`);
  console.log(`- checked-model properties without promoted vectors (informational, not failing until checked-normative): ${checkedModelWithoutPromoted.length}`);
  console.log(`- traceability table target: ${rel(verificationPath)}`);

  const allErrors = [...envelopeErrors, ...profileErrors, ...propertyErrors, ...protoErrors, ...registryErrors, ...coverageErrors, ...invariantErrors, ...implementationErrors, ...evidenceErrors];
  if (allErrors.length > 0) {
    console.error('\nFailures:');
    for (const error of allErrors) console.error(`- ${error}`);
  } else {
    console.log('\nAll vector checks passed.');
  }
}

async function main() {
  const writeMode = process.argv.includes('--write');
  const unknownArgs = process.argv.slice(2).filter((argument) => argument !== '--write');
  if (unknownArgs.length > 0) throw new Error(`unknown arguments: ${unknownArgs.join(', ')}`);
  const [{ vectors, errors: envelopeErrors }, protoSchema] = await Promise.all([
    readVectors(),
    readProtoSchema(),
  ]);
  const profileErrors = validateTokenCommuneProfile(vectors);
  const propertyErrors = validatePropertyReferences(vectors);
  const { errors: protoErrors, checked: protoReferencesChecked } = validatePromotedProtoReferences(vectors, protoSchema);
  const verificationMarkdown = await readFile(verificationPath, 'utf8');
  const registryErrors = [
    ...validateVerificationPropertyRegistry(verificationMarkdown),
    ...validateTokenCommuneMutationLedger(verificationMarkdown, vectors),
  ];
  const coverageErrors = validatePromotedCoverage(vectors);
  const { errors: invariantErrors, checked: invariantChecked } = validatePromotedInvariantExpectations(vectors);
  const staticErrors = [...envelopeErrors, ...profileErrors, ...propertyErrors, ...protoErrors, ...registryErrors, ...coverageErrors, ...invariantErrors];
  const { errors: implementationErrors, executed: implementationExecuted, mutationsKilled } = staticErrors.length === 0
    ? runImplementationChecks(vectors)
    : { errors: [], executed: [], mutationsKilled: [] };
  const evidenceErrors = staticErrors.length === 0 && implementationErrors.length === 0
    ? validateEvidenceCounts(verificationMarkdown, vectors, implementationExecuted, mutationsKilled)
    : [];
  const traceabilityErrors = staticErrors.length === 0 && implementationErrors.length === 0 && evidenceErrors.length === 0
    ? await traceabilityDriftErrors(vectors)
    : [];

  if (writeMode && staticErrors.length === 0 && implementationErrors.length === 0 && evidenceErrors.length === 0) {
    await writeVerificationMarkdown(vectors);
    traceabilityErrors.length = 0;
  }

  printSummary({
    vectors,
    envelopeErrors,
    profileErrors,
    propertyErrors,
    protoErrors,
    protoReferencesChecked,
    registryErrors,
    coverageErrors,
    invariantErrors,
    invariantChecked,
    implementationErrors,
    implementationExecuted,
    mutationsKilled,
    evidenceErrors: [...evidenceErrors, ...traceabilityErrors],
  });

  if ([...staticErrors, ...implementationErrors, ...evidenceErrors, ...traceabilityErrors].length > 0) process.exitCode = 1;
}

main().catch((error) => {
  console.error(`check-vectors failed: ${error.stack ?? error.message}`);
  process.exitCode = 1;
});
