#!/usr/bin/env node
import { readdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '../..');
const seedDir = path.join(repoRoot, 'specs', 'seed');
const vectorDir = path.join(repoRoot, 'contracts', 'vectors');
const verificationPath = path.join(repoRoot, 'docs', 'VERIFICATION.md');
const checkVectorsPath = path.join(repoRoot, 'contracts', 'scripts', 'check-vectors.mjs');

const GENERATED_BEGIN = '<!-- BEGIN GENERATED MODEL-PROMOTION TRACEABILITY -->';
const GENERATED_END = '<!-- END GENERATED MODEL-PROMOTION TRACEABILITY -->';
const REQUIRED_FIELDS = ['property', 'status', 'model', 'backend', 'invocation', 'bounds', 'expected', 'proto_fields', 'semantics'];
const VALID_STATUSES = new Set(['promoted', 'draft']);
const VECTOR_ARRAYS = [
  'CHECKED_MODEL_PROPERTIES',
  'STATED_NORMATIVE_PROPERTIES',
  'CHECKED_NORMATIVE_PROPERTIES',
];

function rel(filePath) {
  return path.relative(repoRoot, filePath).replaceAll(path.sep, '/');
}

function markdownEscape(value) {
  return String(value ?? '—').replaceAll('|', '\\|').replaceAll('\n', '<br>');
}

function listCell(values) {
  if (values.length === 0) return '—';
  return values.map(markdownEscape).join('<br>');
}

function parseStringArray(source, constName) {
  const match = source.match(new RegExp(`const\\s+${constName}\\s*=\\s*\\[([\\s\\S]*?)\\];`));
  if (!match) throw new Error(`Could not find ${constName} in ${rel(checkVectorsPath)}`);
  return [...match[1].matchAll(/'([^']+)'/g)].map((item) => item[1]);
}

async function readRegistryFromCheckVectors() {
  const source = await readFile(checkVectorsPath, 'utf8');
  const arrays = Object.fromEntries(VECTOR_ARRAYS.map((name) => [name, parseStringArray(source, name)]));
  const propertyTiers = new Map();
  const errors = [];

  for (const propertyId of arrays.CHECKED_MODEL_PROPERTIES) propertyTiers.set(propertyId, 'checked-model');
  for (const propertyId of arrays.STATED_NORMATIVE_PROPERTIES) {
    if (propertyTiers.has(propertyId)) {
      errors.push(`${propertyId} appears in both CHECKED_MODEL_PROPERTIES and STATED_NORMATIVE_PROPERTIES`);
    } else {
      propertyTiers.set(propertyId, 'stated-normative');
    }
  }
  for (const propertyId of arrays.CHECKED_NORMATIVE_PROPERTIES) {
    if (arrays.STATED_NORMATIVE_PROPERTIES.includes(propertyId)) {
      errors.push(`${propertyId} appears in both CHECKED_NORMATIVE_PROPERTIES and STATED_NORMATIVE_PROPERTIES`);
    }
    propertyTiers.set(propertyId, 'checked-normative');
  }

  return { arrays, propertyTiers, errors };
}

async function readPromotedVectorCoverage() {
  const entries = (await readdir(vectorDir, { withFileTypes: true }))
    .filter((entry) => entry.isFile() && entry.name.endsWith('.json'))
    .map((entry) => entry.name)
    .sort();

  const promotedByProperty = new Map();
  const errors = [];
  for (const filename of entries) {
    const fullPath = path.join(vectorDir, filename);
    let parsed;
    try {
      parsed = JSON.parse(await readFile(fullPath, 'utf8'));
    } catch (error) {
      errors.push(`${rel(fullPath)}: invalid JSON: ${error.message}`);
      continue;
    }
    if (parsed?.promotion_status === 'promoted' && typeof parsed.property_id === 'string') {
      const vectors = promotedByProperty.get(parsed.property_id) ?? [];
      vectors.push(parsed.vector_id ?? filename.replace(/\.json$/, ''));
      promotedByProperty.set(parsed.property_id, vectors);
    }
  }

  return { promotedByProperty, errors };
}

async function promotionFiles() {
  const entries = (await readdir(seedDir, { withFileTypes: true }))
    .filter((entry) => entry.isFile() && (entry.name.endsWith('.qnt') || entry.name.endsWith('.als')))
    .map((entry) => entry.name)
    .sort();
  return entries.map((entry) => path.join(seedDir, entry));
}

function stripCommentPrefix(line) {
  return line.replace(/^\s*\/\/\s?/, '').trimEnd();
}

function parsePromotionBlock(rawBlock, filePath, indexInFile) {
  const fields = {};
  const fieldOrder = [];
  let currentKey = null;
  const lines = rawBlock.split('\n').map(stripCommentPrefix);

  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (line.length === 0 || line === '@promotion {' || line === '}') continue;
    const field = line.match(/^([A-Za-z_][A-Za-z0-9_-]*):\s*(.*)$/);
    if (field) {
      currentKey = field[1];
      if (Object.hasOwn(fields, currentKey)) {
        fields[currentKey] = `${fields[currentKey]} ${field[2].trim()}`.trim();
      } else {
        fields[currentKey] = field[2].trim();
        fieldOrder.push(currentKey);
      }
    } else if (currentKey) {
      fields[currentKey] = `${fields[currentKey]} ${line.trim()}`.trim();
    }
  }

  const location = `${rel(filePath)}#promotion-${indexInFile}`;
  return { fields, fieldOrder, filePath, location, rawBlock };
}

function extractPromotionBlocks(source, filePath) {
  const blocks = [];
  const regex = /^\s*\/\/\s*@promotion\s*\{[\s\S]*?^\s*\/\/\s*\}\s*$/gm;
  let match;
  while ((match = regex.exec(source)) !== null) {
    blocks.push(parsePromotionBlock(match[0], filePath, blocks.length + 1));
  }
  return blocks;
}

async function readPromotionBlocks() {
  const files = await promotionFiles();
  const blocks = [];
  for (const filePath of files) {
    const source = await readFile(filePath, 'utf8');
    blocks.push(...extractPromotionBlocks(source, filePath));
  }
  return blocks;
}

function deriveTier(status, promotedVectors) {
  if (status === 'promoted') return promotedVectors.length > 0 ? 'checked-normative' : 'checked-model';
  if (status === 'draft') return 'stated-normative';
  return 'unknown';
}

function validateBlocks(blocks, validPropertyIds, promotedByProperty) {
  const errors = [];
  const blockByProperty = new Map();

  for (const block of blocks) {
    const { fields, location } = block;
    for (const required of REQUIRED_FIELDS) {
      if (!Object.hasOwn(fields, required) || fields[required].length === 0) {
        errors.push(`${location}: missing required field ${required}`);
      }
    }
    if (Object.hasOwn(fields, 'tier')) {
      errors.push(`${location}: tier field is forbidden; product tier is derived from status + promoted vectors`);
    }
    if (Object.hasOwn(fields, 'property')) {
      if (!validPropertyIds.has(fields.property)) {
        errors.push(`${location}: unknown property ${fields.property}; update the registry or fix the @promotion block`);
      }
      if (blockByProperty.has(fields.property)) {
        errors.push(`${location}: duplicate @promotion property ${fields.property}; first seen at ${blockByProperty.get(fields.property).location}`);
      } else {
        blockByProperty.set(fields.property, block);
      }
    }
    if (Object.hasOwn(fields, 'status') && !VALID_STATUSES.has(fields.status)) {
      errors.push(`${location}: status must be one of ${[...VALID_STATUSES].join(', ')}, got ${fields.status}`);
    }
    if (fields.status === 'promoted') {
      if (!fields.invocation || fields.invocation.includes('<TBD')) {
        errors.push(`${location}: promoted block must have a concrete invocation`);
      } else if (!/\b(quint|java)\b/.test(fields.invocation)) {
        errors.push(`${location}: promoted invocation must name the tool (quint or java)`);
      }
      if (fields.language === 'quint' || fields.backend?.startsWith('apalache')) {
        const modelBasename = path.basename(fields.model ?? '');
        if (!fields.invocation?.includes(modelBasename)) {
          errors.push(`${location}: promoted Quint invocation must include model filename ${modelBasename}`);
        }
      }
    }
    if (fields.status === 'promoted' && (promotedByProperty.get(fields.property) ?? []).length > 0) {
      // Derived checked-normative is valid only because coverage exists; this branch documents the gate.
    }
  }

  return { errors, blockByProperty };
}

function validateDerivedTiers({ propertyTiers, blockByProperty, promotedByProperty }) {
  const errors = [];
  const derivedTiers = new Map();

  for (const propertyId of [...propertyTiers.keys()].sort((a, b) => a.localeCompare(b))) {
    const block = blockByProperty.get(propertyId);
    const promotedVectors = promotedByProperty.get(propertyId) ?? [];
    const derivedTier = block ? deriveTier(block.fields.status, promotedVectors) : 'stated-normative';
    derivedTiers.set(propertyId, derivedTier);

    const registryTier = propertyTiers.get(propertyId);
    if (derivedTier !== registryTier) {
      const modelState = block ? `@promotion status:${block.fields.status} at ${block.location}` : 'reserved-unmodeled (no @promotion block)';
      errors.push(`${propertyId}: derived tier ${derivedTier} from ${modelState} + ${promotedVectors.length} promoted vector(s), but ${rel(checkVectorsPath)} registry/docs cache says ${registryTier}`);
    }

    if ((registryTier === 'checked-model' || registryTier === 'checked-normative') && !block) {
      errors.push(`${propertyId}: registry/docs classify as ${registryTier}, but no @promotion block was found`);
    }
    if (derivedTier === 'checked-normative' && promotedVectors.length === 0) {
      errors.push(`${propertyId}: checked-normative derivation requires at least one promoted vector`);
    }
  }

  return { errors, derivedTiers };
}

function parseVerificationGeneratedTable(markdown) {
  const classifications = new Map();
  const begin = markdown.indexOf('<!-- BEGIN GENERATED CONFORMANCE VECTOR TRACEABILITY -->');
  const end = markdown.indexOf('<!-- END GENERATED CONFORMANCE VECTOR TRACEABILITY -->');
  if (begin === -1 || end === -1 || end <= begin) return classifications;
  const block = markdown.slice(begin, end);
  for (const line of block.split('\n')) {
    const match = line.match(/^\| `([^`]+)` \| ([^|]+?) \|/);
    if (match) classifications.set(match[1], match[2].trim());
  }
  return classifications;
}

function validateVerificationMarkdown({ markdown, derivedTiers }) {
  const errors = [];
  const classifications = parseVerificationGeneratedTable(markdown);
  if (classifications.size === 0) {
    errors.push(`${rel(verificationPath)}: generated conformance-vector traceability table is missing; run check-vectors.mjs`);
    return errors;
  }

  for (const [propertyId, derivedTier] of derivedTiers) {
    const docTier = classifications.get(propertyId);
    if (!docTier) {
      errors.push(`${rel(verificationPath)}: generated conformance-vector table lacks ${propertyId}`);
    } else if (docTier !== derivedTier) {
      errors.push(`${rel(verificationPath)}: ${propertyId} is ${docTier} in generated conformance table but model-derived tier is ${derivedTier}`);
    }
  }

  return errors;
}

function buildTraceabilityMarkdown({ propertyTiers, blockByProperty, promotedByProperty, derivedTiers }) {
  const rows = [...propertyTiers.keys()].sort((a, b) => a.localeCompare(b)).map((propertyId) => {
    const block = blockByProperty.get(propertyId);
    const promotedVectors = promotedByProperty.get(propertyId) ?? [];
    const status = block?.fields.status ?? 'reserved-unmodeled';
    const model = block?.fields.model ?? '—';
    const backend = block?.fields.backend ?? '—';
    const invocation = status === 'reserved-unmodeled' ? '—' : (block?.fields.invocation ?? '—');
    const semantics = block?.fields.semantics ?? '—';
    return `| \`${markdownEscape(propertyId)}\` | ${markdownEscape(status)} | ${markdownEscape(derivedTiers.get(propertyId))} | ${markdownEscape(model)} | ${markdownEscape(backend)} | ${listCell(promotedVectors)} | ${markdownEscape(invocation)} | ${markdownEscape(semantics)} |`;
  });

  const modeled = [...blockByProperty.keys()].length;
  const reserved = [...propertyTiers.keys()].filter((propertyId) => !blockByProperty.has(propertyId)).length;
  const promotedBlocks = [...blockByProperty.values()].filter((block) => block.fields.status === 'promoted').length;
  const draftBlocks = [...blockByProperty.values()].filter((block) => block.fields.status === 'draft').length;

  return [
    GENERATED_BEGIN,
    '<!-- Generated by `node contracts/scripts/check-models.mjs`; do not edit this block by hand. -->',
    '',
    '### Generated model-promotion traceability table',
    '',
    'Source models: `specs/seed/*.qnt` and `specs/seed/*.als`. Product tier is derived from model `status` plus promoted conformance-vector coverage; model files do not store a `tier` field.',
    '',
    `Summary: ${modeled} modeled propert${modeled === 1 ? 'y' : 'ies'} (${promotedBlocks} promoted, ${draftBlocks} draft), ${reserved} reserved-unmodeled stated-normative propert${reserved === 1 ? 'y' : 'ies'}, ${[...promotedByProperty.keys()].length} propert${promotedByProperty.size === 1 ? 'y' : 'ies'} with promoted vector coverage.`,
    '',
    '| Property id | Model status | Derived tier | Model | Backend | Promoted vectors | Invocation | Semantics |',
    '|---|---|---|---|---|---|---|---|',
    ...rows,
    '',
    GENERATED_END,
  ].join('\n');
}

async function updateVerificationMarkdown(generated) {
  const current = await readFile(verificationPath, 'utf8');
  const begin = current.indexOf(GENERATED_BEGIN);
  const end = current.indexOf(GENERATED_END);

  let next;
  if (begin !== -1 && end !== -1 && end > begin) {
    next = `${current.slice(0, begin)}${generated}${current.slice(end + GENERATED_END.length)}`;
  } else {
    const insertionPoint = current.indexOf('\n<!-- BEGIN GENERATED CONFORMANCE VECTOR TRACEABILITY -->');
    if (insertionPoint === -1) {
      throw new Error(`Could not find insertion point in ${rel(verificationPath)}; expected generated conformance-vector block`);
    }
    next = `${current.slice(0, insertionPoint)}\n\n${generated}\n${current.slice(insertionPoint)}`;
  }

  if (next !== current) {
    await writeFile(verificationPath, next);
    return true;
  }
  return false;
}

function printSummary({ blocks, propertyTiers, blockByProperty, derivedTiers, generatedChanged, allErrors }) {
  const tierCounts = new Map();
  for (const tier of derivedTiers.values()) tierCounts.set(tier, (tierCounts.get(tier) ?? 0) + 1);

  console.log('Model-promotion check summary');
  console.log(`- promotion blocks read: ${blocks.length}`);
  console.log(`- registered formal property ids: ${propertyTiers.size}`);
  console.log(`- modeled properties: ${blockByProperty.size}`);
  console.log(`- reserved-unmodeled properties: ${propertyTiers.size - blockByProperty.size}`);
  console.log(`- derived checked-model properties: ${tierCounts.get('checked-model') ?? 0}`);
  console.log(`- derived checked-normative properties: ${tierCounts.get('checked-normative') ?? 0}`);
  console.log(`- derived stated-normative properties: ${tierCounts.get('stated-normative') ?? 0}`);
  console.log(`- traceability table target: ${rel(verificationPath)}${generatedChanged ? ' (updated)' : ' (already current)'}`);

  if (allErrors.length > 0) {
    console.error('\nFailures:');
    for (const error of allErrors) console.error(`- ${error}`);
  } else {
    console.log('\nAll model-promotion checks passed.');
  }
}

async function main() {
  const { propertyTiers, errors: registryErrors } = await readRegistryFromCheckVectors();
  const { promotedByProperty, errors: vectorErrors } = await readPromotedVectorCoverage();
  const blocks = await readPromotionBlocks();
  const { errors: blockErrors, blockByProperty } = validateBlocks(blocks, new Set(propertyTiers.keys()), promotedByProperty);
  const { errors: tierErrors, derivedTiers } = validateDerivedTiers({ propertyTiers, blockByProperty, promotedByProperty });
  const verificationMarkdown = await readFile(verificationPath, 'utf8');
  const verificationErrors = validateVerificationMarkdown({ markdown: verificationMarkdown, derivedTiers });
  const generated = buildTraceabilityMarkdown({ propertyTiers, blockByProperty, promotedByProperty, derivedTiers });
  const generatedChanged = await updateVerificationMarkdown(generated);

  const driftErrors = generatedChanged
    ? [`${rel(verificationPath)} generated model-promotion traceability was stale and has been regenerated; re-run after committing the updated block`]
    : [];
  const allErrors = [...registryErrors, ...vectorErrors, ...blockErrors, ...tierErrors, ...verificationErrors, ...driftErrors];

  printSummary({ blocks, propertyTiers, blockByProperty, derivedTiers, generatedChanged, allErrors });

  if (allErrors.length > 0) process.exitCode = 1;
}

main().catch((error) => {
  console.error(`check-models failed: ${error.stack ?? error.message}`);
  process.exitCode = 1;
});
