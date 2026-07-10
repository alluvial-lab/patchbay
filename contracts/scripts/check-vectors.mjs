#!/usr/bin/env node
import { readdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '../..');
const vectorDir = path.join(repoRoot, 'contracts', 'vectors');
const verificationPath = path.join(repoRoot, 'docs', 'VERIFICATION.md');

// Property registry maintenance note:
// Update these lists whenever docs/VERIFICATION.md adds, removes, promotes, or
// demotes property ids. The generated traceability table below is the checked-in
// sync surface that makes drift visible during review.
const CHECKED_MODEL_PROPERTIES = [
  'ActorIdsUnique',
  'BoundaryDedup',
  'browser_local_state_not_authority',
  'CsrfRejectsMissingProof',
  'CsrfRejectsUnauthenticated',
  'ElicitationCorrelationTyped',
  'ElicitationFirstAnswerWins',
  'ElicitationInvalidResponseRejected',
  'ElicitationPendingFinality',
  'ElicitationResponderAuthority',
  'ElicitationStaleTargetInert',
  'ElicitationWithdrawalFinality',
  'FleetAuthorityForSpawn',
  'GenerationMonotonic',
  'NoAcceptedToCompleted',
  'RevokedSessionCannotCommand',
  'SpawnCreatesDescendantGrant',
  'SpawnRevocationDoesNotCascade',
  'SubscriptionAudited',
  'SubscriptionCursorReplayAuthorized',
  'SubscriptionGrantChecked',
  'TerminalFinality',
  'TypedCorrelation',
];

// Currently empty by docs/VERIFICATION.md: checked-model properties are not
// checked-normative product semantics until they also have at least one promoted
// conformance vector. The promoted-vector coverage gate (a) only fails for ids
// listed here, not for every checked-model property.
const CHECKED_NORMATIVE_PROPERTIES = [];

const STATED_NORMATIVE_PROPERTIES = [
  'AuthorityGraphAcyclic',
  'CommandDurability',
  'CompoundIssuer',
  'CrashNoAcceptedLost',
  'ElicitationTimeoutNeitherSuccessNorDenial',
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
  'SenderMatchesClaim',
  'SessionIdentityTuple',
  'SnapshotConsistentPrefix',
  'SnapshotCrossDomainRejected',
  'SnapshotStaleRejected',
  'TimeoutNeitherSuccessNorDenial',
];

// Descriptive, non-formal ids that may appear in draft boundary vectors. They
// are intentionally not valid promotion targets because they do not identify a
// formal or reserved property in docs/VERIFICATION.md.
const DESCRIPTIVE_DRAFT_ONLY_PROPERTY_IDS = new Set(['boundary-validation']);

// Placeholder for surfaced-contradiction checks. Once vectors are promoted,
// each promoted property should register a checker here that independently
// inspects vector.expected_outcome against the referenced model invariant.
// Until a checker exists, promoting a vector for that property fails closed.
const INVARIANT_EXPECTATION_CHECKS = Object.freeze({
  // Example future shape:
  // CommandDurability: (vector) => ({ ok: hasDurableAcceptedRecord(vector), detail: '...' }),
});

const PROPERTY_TIERS = new Map();
for (const id of CHECKED_MODEL_PROPERTIES) PROPERTY_TIERS.set(id, 'checked-model');
for (const id of STATED_NORMATIVE_PROPERTIES) {
  if (!PROPERTY_TIERS.has(id)) PROPERTY_TIERS.set(id, 'stated-normative');
}
for (const id of CHECKED_NORMATIVE_PROPERTIES) PROPERTY_TIERS.set(id, 'checked-normative');

const VALID_PROPERTY_IDS = new Set(PROPERTY_TIERS.keys());
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

async function updateVerificationMarkdown(vectors) {
  const current = await readFile(verificationPath, 'utf8');
  const generated = buildTraceabilityMarkdown(vectors);
  const begin = current.indexOf(GENERATED_BEGIN);
  const end = current.indexOf(GENERATED_END);

  let next;
  if (begin !== -1 && end !== -1 && end > begin) {
    next = `${current.slice(0, begin)}${generated}${current.slice(end + GENERATED_END.length)}`;
  } else {
    const insertionPoint = current.indexOf('\n## Model promotion rule');
    if (insertionPoint === -1) {
      throw new Error(`Could not find insertion point in ${rel(verificationPath)}; expected "## Model promotion rule"`);
    }
    next = `${current.slice(0, insertionPoint)}\n\n${generated}\n${current.slice(insertionPoint)}`;
  }

  if (next !== current) await writeFile(verificationPath, next);
}

function printSummary({ vectors, envelopeErrors, propertyErrors, coverageErrors, invariantErrors, invariantChecked }) {
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
  console.log(`- promoted invariant expectation checks run: ${invariantChecked.length}`);
  console.log(`- checked-model properties without promoted vectors (informational, not failing until checked-normative): ${checkedModelWithoutPromoted.length}`);
  console.log(`- traceability table target: ${rel(verificationPath)}`);

  const allErrors = [...envelopeErrors, ...propertyErrors, ...coverageErrors, ...invariantErrors];
  if (allErrors.length > 0) {
    console.error('\nFailures:');
    for (const error of allErrors) console.error(`- ${error}`);
  } else {
    console.log('\nAll vector checks passed.');
  }
}

async function main() {
  const { vectors, errors: envelopeErrors } = await readVectors();
  const propertyErrors = validatePropertyReferences(vectors);
  const coverageErrors = validatePromotedCoverage(vectors);
  const { errors: invariantErrors, checked: invariantChecked } = validatePromotedInvariantExpectations(vectors);

  await updateVerificationMarkdown(vectors);

  printSummary({ vectors, envelopeErrors, propertyErrors, coverageErrors, invariantErrors, invariantChecked });

  if ([...envelopeErrors, ...propertyErrors, ...coverageErrors, ...invariantErrors].length > 0) {
    process.exitCode = 1;
  }
}

main().catch((error) => {
  console.error(`check-vectors failed: ${error.stack ?? error.message}`);
  process.exitCode = 1;
});
