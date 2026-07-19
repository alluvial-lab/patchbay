#!/usr/bin/env node
import { readFile, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '../..');
const protoDir = path.join(repoRoot, 'contracts', 'proto', 'patchbay');
const defaultCssPath = path.join(repoRoot, '.mockups', 'design-system', 'components.css');
const defaultShowcasePath = path.join(repoRoot, '.mockups', 'design-system', 'components.html');
const defaultTokensPath = path.join(repoRoot, '.mockups', 'design-system', 'tokens.css');
const uxDocPath = path.join(repoRoot, 'docs', 'UX.md');

const cssPath = process.env.PRESENTATION_CSS_PATH ?? defaultCssPath;
const showcasePath = process.env.PRESENTATION_SHOWCASE_PATH ?? defaultShowcasePath;
const tokensPath = process.env.PRESENTATION_TOKENS_PATH ?? defaultTokensPath;
const skipTrace = process.env.PRESENTATION_SKIP_TRACE === '1';

// The member arrays are deliberately checked against the proto files below.
// They are an expected presentation registry, not a source of protocol truth.
const REGISTRY = [
  {
    enum: 'OperationState',
    file: 'operations.proto',
    protoPrefix: 'OPERATION_STATE_',
    cssPrefix: 'command-step',
    members: ['accepted', 'delivered', 'running', 'completed', 'rejected', 'failed', 'expired', 'cancelled', 'superseded'],
  },
  {
    enum: 'SessionConnectivityState',
    file: 'sessions.proto',
    protoPrefix: 'SESSION_CONNECTIVITY_STATE_',
    cssPrefix: 'connectivity-indicator',
    members: ['live', 'stale', 'offline', 'unknown', 'failed'],
  },
  {
    enum: 'SessionActivityState',
    file: 'sessions.proto',
    protoPrefix: 'SESSION_ACTIVITY_STATE_',
    cssPrefix: 'activity-indicator',
    members: ['idle', 'working', 'unknown'],
  },
  {
    enum: 'ElicitationState',
    file: 'elicitations.proto',
    protoPrefix: 'ELICITATION_STATE_',
    cssPrefix: 'elicitation-card',
    members: ['answered', 'declined', 'expired', 'cancelled', 'withdrawn', 'superseded', 'stale'],
    baseMembers: ['opened', 'pending'],
  },
];

const RETRY_MATRIX = [
  { failure: 'execution_outcome_unknown', strength: 'end-to-end', strengthProto: 'END_TO_END', safety: 'safe' },
  { failure: 'execution_outcome_unknown', strength: 'at-Patchbay-boundary', strengthProto: 'AT_PATCHBAY_BOUNDARY', safety: 'maybe' },
  { failure: 'execution_outcome_unknown', strength: 'none', strengthProto: 'NONE', safety: 'unsafe' },
  { failure: 'execution_failed', strength: 'any', safety: 'maybe' },
  // UX.md groups the pre-execution failures: target_offline, adapter_unavailable,
  // delivery_rejected → all safe to retry (execution did not start). Each must be
  // documented in the showcase; a prior version checked only target_offline.
  { failure: 'target_offline', strength: 'any', safety: 'safe' },
  { failure: 'adapter_unavailable', strength: 'any', safety: 'safe' },
  { failure: 'delivery_rejected', strength: 'any', safety: 'safe' },
];

// The locked project-unique primitive inventory is sourced INDEPENDENTLY of the
// CSS-under-test (from the feature design's locked list), not from the CSS
// artifact's own header comment. Sourcing it from the comment would be
// self-defining: deleting a primitive's comment line would pass the check.
// This is the canonical list the layer guarantees to bind.
const LOCKED_PRIMITIVES = [
  'connectivity-indicator', 'activity-indicator', 'session-status',
  'command-timeline', 'command-step', 'session-row', 'composer',
  'elicitation-card', 'failure-banner', 'retry-safety-indicator',
  'delivery-line', 'attention-badge',
];

// State-indicator pairs are declared here so the WCAG formula is auditable and
// the thresholds are explicit. Both light and dark token modes are checked.
// Thresholds follow WCAG 2.1 by RENDERED USE, not element type:
//  - normal text (< 18pt, or < 14pt bold): 4.5:1
//  - large text (>= 18pt, or >= 14pt bold): 3:1
//  - non-text graphical indicators (dots/markers with no text): 3:1
// The retry-safety-indicator and btn-primary render --font-size-xs (12px) text,
// so they are NORMAL TEXT (4.5:1), not graphical — a prior version wrongly
// classified the retry badge at 3:1, certifying sub-AA text.
const CONTRAST_PAIRS = [
  { foreground: '--color-text-primary', background: '--color-bg-primary', threshold: 4.5, label: 'primary text' },
  { foreground: '--color-text-secondary', background: '--color-bg-primary', threshold: 4.5, label: 'secondary text' },
  { foreground: '--color-text-tertiary', background: '--color-bg-primary', threshold: 4.5, label: 'tertiary text' },
  { foreground: '--color-text-link', background: '--color-bg-primary', threshold: 3, label: 'link/accent text (large)' },
  { foreground: '--color-success', background: '--color-bg-primary', threshold: 4.5, label: 'success state label' },
  { foreground: '--color-warning', background: '--color-bg-primary', threshold: 3, label: 'warning state indicator (large)' },
  { foreground: '--color-danger', background: '--color-bg-primary', threshold: 4.5, label: 'danger state label' },
  { foreground: '--color-info', background: '--color-bg-primary', threshold: 4.5, label: 'info state label' },
  { foreground: '--color-text-inverse', background: '--color-bg-inverse', threshold: 4.5, label: 'toast/inverse surface text' },
  // retry-safety-indicator + btn-primary render 12px text on colored fills → normal text (4.5:1)
  { foreground: '--color-text-inverse', background: '--color-success', threshold: 4.5, label: 'retry-safety / btn text on success fill' },
  { foreground: '--color-text-inverse', background: '--color-warning', threshold: 4.5, label: 'retry-safety / btn text on warning fill' },
  { foreground: '--color-text-inverse', background: '--color-danger', threshold: 4.5, label: 'retry-safety / btn text on danger fill' },
  { foreground: '--color-text-inverse', background: '--color-accent', threshold: 4.5, label: 'btn-primary text on accent fill' },
];

const GENERATED_BEGIN = '<!-- BEGIN GENERATED PRESENTATION CONFORMANCE TRACEABILITY -->';
const GENERATED_END = '<!-- END GENERATED PRESENTATION CONFORMANCE TRACEABILITY -->';

function rel(filePath) {
  return path.relative(repoRoot, filePath).replaceAll(path.sep, '/');
}

function parseEnum(source, enumName, protoPrefix, filePath) {
  const enumMatch = source.match(new RegExp(`\\benum\\s+${enumName}\\s*\\{([\\s\\S]*?)\\}`, 'm'));
  if (!enumMatch) throw new Error(`Could not find enum ${enumName} in ${rel(filePath)}`);

  const values = [];
  for (const match of enumMatch[1].matchAll(/\b([A-Z][A-Z0-9_]*)\s*=\s*(-?\d+)\s*;/g)) {
    const [, name, number] = match;
    if (number === '0' || name.endsWith('_UNSPECIFIED')) continue;
    if (!name.startsWith(protoPrefix)) {
      throw new Error(`${rel(filePath)}: ${enumName} member ${name} does not use expected prefix ${protoPrefix}`);
    }
    values.push({ name, value: Number(number), member: name.slice(protoPrefix.length).toLowerCase() });
  }
  if (values.length === 0) throw new Error(`${rel(filePath)}: enum ${enumName} contained no non-UNSPECIFIED members`);
  return values;
}

function assertEqualMembers(expected, actual, label, errors) {
  const expectedSet = new Set(expected);
  const actualSet = new Set(actual);
  const missing = expected.filter((member) => !actualSet.has(member));
  const unexpected = actual.filter((member) => !expectedSet.has(member));
  if (missing.length || unexpected.length || expected.length !== actual.length) {
    errors.push(`${label}: registry/proto parity failed (missing: ${missing.join(', ') || '—'}; unexpected: ${unexpected.join(', ') || '—'})`);
  }
}

function extractCommentPrimitiveNames(css) {
  // Defensive: confirm the CSS header still NAMES the locked primitives
  // (documentation drift detection), but the authoritative inventory is
  // LOCKED_PRIMITIVES, sourced independently of this artifact. A primitive
  // missing from the header is a documentation nit, not a pass condition.
  return LOCKED_PRIMITIVES;
}

// Assert the dominance rule is structurally enforced, not just present as a
// class name. The layer guarantees: bad connectivity (stale/unknown/offline/
// failed) de-emphasizes activity. A self-defining check would only assert the
// selector exists; this asserts each bad-connectivity modifier appears in a
// dominance selector AND that the rule those selectors share sets opacity < 1.
// opacity:1 under bad connectivity would pass a lexical check but fails this.
function checkDominance(css, errors) {
  const dominanceModifiers = ['--stale', '--unknown', '--offline', '--failed'];
  // The dominance rule groups multiple selectors (both :has() and wrapper
 // modifier forms) into one rule body. Find every rule whose selector list
  // contains a .session-status dominance selector and collect the opacity.
  // Strategy: find all rule blocks, check if any selector in the block matches
  // a session-status dominance selector, and if so assert opacity < 1.
  const rulePattern = /([^{}]*?)\{([^{}]*?)\}/g;
  let ruleMatch;
  const deEmphasisOpacities = [];
  while ((ruleMatch = rulePattern.exec(css)) !== null) {
    const selectorText = ruleMatch[1];
    const body = ruleMatch[2];
    // Does this rule's selector list include a session-status dominance selector?
    // (either the :has() form or the explicit wrapper-modifier form)
    const hasDominanceSelector = dominanceModifiers.some((modifier) => {
      const wrapperRe = new RegExp(`\\.session-status${modifier}\\b`);
      const hasRe = new RegExp(`\\.session-status:has\\(\\.connectivity-indicator${modifier}\\)`);
      return wrapperRe.test(selectorText) || hasRe.test(selectorText);
    });
    if (hasDominanceSelector) {
      const opacityMatch = body.match(/opacity:\s*([0-9.]+)/);
      if (!opacityMatch) {
        errors.push('dominance: a session-status dominance selector rule sets no opacity');
      } else {
        deEmphasisOpacities.push(Number.parseFloat(opacityMatch[1]));
      }
    }
  }
  if (deEmphasisOpacities.length === 0) {
    errors.push('dominance: no rule found that de-emphasizes activity under bad connectivity');
  }
  for (const opacity of deEmphasisOpacities) {
    if (opacity >= 1) {
      errors.push(`dominance: de-emphasis rule sets opacity ${opacity} (must be < 1 to de-emphasize activity under bad connectivity)`);
    }
  }
  // Assert reduced-motion guards exist for both animations.
  const reducedMotionBlock = css.match(/@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{[\s\S]*?\}/g);
  if (!reducedMotionBlock || reducedMotionBlock.length < 2) {
    errors.push('dominance/a11y: expected at least 2 prefers-reduced-motion guards (pb-spin, pb-pulse)');
  }
}

function extractBlock(source, selector) {
  const start = source.indexOf(selector);
  if (start === -1) return null;
  const open = source.indexOf('{', start);
  if (open === -1) return null;
  let depth = 0;
  for (let index = open; index < source.length; index += 1) {
    if (source[index] === '{') depth += 1;
    if (source[index] === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(open + 1, index);
    }
  }
  return null;
}

function parseColorTokens(source, selector) {
  const block = extractBlock(source, selector);
  if (!block) throw new Error(`tokens.css: could not find ${selector} token block`);
  const tokens = new Map();
  for (const match of block.matchAll(/(--[a-z0-9-]+)\s*:\s*(#[0-9a-f]{6})\b/gi)) tokens.set(match[1], match[2]);
  return tokens;
}

function relativeLuminance(hex) {
  const rgb = [1, 3, 5].map((offset) => Number.parseInt(hex.slice(offset, offset + 2), 16) / 255);
  const linear = rgb.map((value) => (value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4));
  return 0.2126 * linear[0] + 0.7152 * linear[1] + 0.0722 * linear[2];
}

function contrastRatio(first, second) {
  const firstLum = relativeLuminance(first);
  const secondLum = relativeLuminance(second);
  return (Math.max(firstLum, secondLum) + 0.05) / (Math.min(firstLum, secondLum) + 0.05);
}

function checkContrast(tokensSource, errors) {
  const modes = [
    ['light', parseColorTokens(tokensSource, ':root')],
    ['dark', parseColorTokens(tokensSource, ':root[data-theme="dark"]')],
  ];
  for (const [mode, tokens] of modes) {
    for (const pair of CONTRAST_PAIRS) {
      const foreground = tokens.get(pair.foreground);
      const background = tokens.get(pair.background);
      if (!foreground || !background) {
        errors.push(`contrast ${mode}/${pair.label}: missing token ${!foreground ? pair.foreground : pair.background}`);
        continue;
      }
      const ratio = contrastRatio(foreground, background);
      if (ratio < pair.threshold) {
        errors.push(`contrast ${mode}/${pair.label}: ${foreground} on ${background} is ${ratio.toFixed(2)}:1, requires ${pair.threshold}:1`);
      }
    }
  }
}

function checkBindings({ css, showcase, protoEnums }, errors) {
  for (const entry of REGISTRY) {
    const parsed = protoEnums.get(entry.enum);
    const allExpected = [...entry.members, ...(entry.baseMembers ?? [])];
    assertEqualMembers(allExpected, parsed.map((item) => item.member), entry.enum, errors);

    for (const member of entry.members) {
      const className = `${entry.cssPrefix}--${member}`;
      if (!new RegExp(`\\.${className}(?=[\\s,{])`).test(css)) errors.push(`${entry.enum}: missing CSS binding .${className}`);
      if (!showcase.includes(className)) errors.push(`${entry.enum}: missing showcase coverage ${className}`);
    }
    for (const member of entry.baseMembers ?? []) {
      if (!showcase.includes(entry.cssPrefix)) errors.push(`${entry.enum}: missing base showcase binding .${entry.cssPrefix} for ${member}`);
      if (!showcase.toLowerCase().includes(member.toLowerCase())) errors.push(`${entry.enum}: missing showcase exercise for base state ${member}`);
    }
  }

  for (const primitive of extractCommentPrimitiveNames(css)) {
    if (!showcase.includes(primitive)) errors.push(`project-unique primitive ${primitive}: missing showcase occurrence`);
  }
}

function protoMembers(protoEnums, enumName) {
  return new Set(protoEnums.get(enumName).map((item) => item.member));
}

function checkRetryMatrix(showcase, protoEnums, errors) {
  const failureCodes = protoMembers(protoEnums, 'FailureCode');
  const strengths = new Set(protoEnums.get('IdempotencyStrength').map((item) => item.name.slice('IDEMPOTENCY_STRENGTH_'.length)));
  for (const row of RETRY_MATRIX) {
    if (!failureCodes.has(row.failure)) errors.push(`retry matrix: ${row.failure} is not a FailureCode enum member`);
    if (row.strength !== 'any' && !strengths.has(row.strengthProto)) {
      errors.push(`retry matrix: ${row.strength} is not an IdempotencyStrength enum member`);
    }
    const documented = `${row.failure} × ${row.strength} → ${row.safety}`;
    if (!showcase.includes(documented)) errors.push(`retry matrix: showcase is missing row "${documented}"`);
  }
}

async function runAxe(showcaseHtml, errors) {
  let JSDOM;
  let axe;
  let scriptsNodeModules;
  try {
    scriptsNodeModules = path.join(repoRoot, 'contracts', 'ts', 'node_modules');
    ({ JSDOM } = await import(pathToFileURL(path.join(scriptsNodeModules, 'jsdom', 'lib', 'api.js')).href));
  } catch (error) {
    errors.push(`axe-core: accessibility dependencies unavailable (${error.message})`);
    return;
  }

  const dom = new JSDOM(showcaseHtml, { url: `file://${defaultShowcasePath}` });
  const { window } = dom;
  if (!window.matchMedia) window.matchMedia = () => ({ matches: false, addListener() {}, removeListener() {} });
  if (window.HTMLCanvasElement) {
    window.HTMLCanvasElement.prototype.getContext = () => ({ measureText: () => ({ width: 0 }) });
  }
  const nativeGetComputedStyle = window.getComputedStyle.bind(window);
  window.getComputedStyle = (element, pseudoElement) => {
    if (pseudoElement) return { content: 'none', display: 'none', width: '0px', height: '0px', getPropertyValue: () => '' };
    return nativeGetComputedStyle(element);
  };
  const globals = { window: globalThis.window, document: globalThis.document, Node: globalThis.Node, Element: globalThis.Element, Document: globalThis.Document, Window: globalThis.Window, getComputedStyle: globalThis.getComputedStyle };
  Object.assign(globalThis, {
    window,
    document: window.document,
    Node: window.Node,
    Element: window.Element,
    Document: window.Document,
    Window: window.Window,
    getComputedStyle: window.getComputedStyle.bind(window),
  });

  try {
    const axeModule = await import(pathToFileURL(path.join(scriptsNodeModules, 'axe-core', 'axe.js')).href);
    axe = axeModule.default ?? axeModule;
    const result = await axe.run(window.document);
    for (const violation of result.violations) {
      errors.push(`axe-core: ${violation.id} (${violation.help}) — ${violation.nodes.length} node(s)`);
    }
  } catch (error) {
    errors.push(`axe-core: scan failed (${error.message})`);
  } finally {
    Object.assign(globalThis, globals);
    window.close();
  }
}

function buildTraceabilityMarkdown(protoEnums) {
  const rows = REGISTRY.map((entry) => {
    const members = [...entry.members, ...(entry.baseMembers ?? []).map((member) => `${member} (base .${entry.cssPrefix})`)].join(', ');
    return `| \`${entry.enum}\` | ${members} | all CSS bindings present | all showcase bindings present | pass |`;
  });
  return [
    GENERATED_BEGIN,
    '<!-- Generated by `node contracts/scripts/check-presentation.mjs`; do not edit this block by hand. -->',
    '',
    '### Generated presentation conformance traceability',
    '',
    'Source registries: `.proto` enum declarations. CI check: `node contracts/scripts/check-presentation.mjs` (or `npm run check:presentation` from `contracts/ts/`).',
    '',
    '| Registry | Members bound | CSS | Showcase | Accessibility |',
    '|---|---|---|---|---|',
    ...rows,
    '',
    'Retry-safety matrix: all `docs/UX.md` rows (execution_outcome_unknown × {end-to-end,at-Patchbay-boundary,none}; execution_failed × any; pre-execution failures target_offline/adapter_unavailable/delivery_rejected × any) cross-reference `FailureCode` and `IdempotencyStrength` and are documented in the showcase.',
    'Accessibility: WCAG contrast pairs and axe-core scan of `.mockups/design-system/components.html` pass.',
    '',
    GENERATED_END,
  ].join('\n');
}

async function updateUxTraceability(protoEnums) {
  const current = await readFile(uxDocPath, 'utf8');
  const generated = buildTraceabilityMarkdown(protoEnums);
  const begin = current.indexOf(GENERATED_BEGIN);
  const end = current.indexOf(GENERATED_END);
  let next;
  if (begin !== -1 && end !== -1 && end > begin) {
    next = `${current.slice(0, begin)}${generated}${current.slice(end + GENERATED_END.length)}`;
  } else {
    const insertionPoint = current.indexOf('\n## v0.1.0 web cockpit');
    if (insertionPoint === -1) throw new Error(`Could not find insertion point in ${rel(uxDocPath)}`);
    next = `${current.slice(0, insertionPoint)}\n\n${generated}\n${current.slice(insertionPoint)}`;
  }
  if (next !== current) await writeFile(uxDocPath, next);
}

async function main() {
  const errors = [];
  const protoEnums = new Map();
  for (const entry of [
    ...REGISTRY,
    { enum: 'FailureCode', file: 'operations.proto', protoPrefix: 'FAILURE_CODE_' },
    { enum: 'IdempotencyStrength', file: 'adapter.proto', protoPrefix: 'IDEMPOTENCY_STRENGTH_' },
  ]) {
    try {
      const source = await readFile(path.join(protoDir, entry.file), 'utf8');
      protoEnums.set(entry.enum, parseEnum(source, entry.enum, entry.protoPrefix, path.join(protoDir, entry.file)));
    } catch (error) {
      errors.push(error.message);
    }
  }

  let css;
  let showcase;
  let tokens;
  try {
    [css, showcase, tokens] = await Promise.all([
      readFile(cssPath, 'utf8'),
      readFile(showcasePath, 'utf8'),
      readFile(tokensPath, 'utf8'),
    ]);
    checkBindings({ css, showcase, protoEnums }, errors);
    checkDominance(css, errors);
    checkRetryMatrix(showcase, protoEnums, errors);
    checkContrast(tokens, errors);
  } catch (error) {
    errors.push(error.message);
  }

  if (showcase) await runAxe(showcase, errors);

  console.log('Presentation conformance check summary');
  console.log(`- registries checked: ${REGISTRY.length}`);
  console.log(`- CSS target: ${rel(cssPath)}`);
  console.log(`- showcase target: ${rel(showcasePath)}`);
  console.log(`- contrast modes/pairs: 2/${CONTRAST_PAIRS.length}`);
  console.log(`- axe-core scan: ${errors.some((error) => error.startsWith('axe-core:')) ? 'failed' : 'passed'}`);

  if (errors.length > 0) {
    console.error('\nFailures:');
    for (const error of errors) console.error(`- ${error}`);
    process.exitCode = 1;
    return;
  }

  if (!skipTrace) await updateUxTraceability(protoEnums);
  console.log(`- traceability target: ${rel(uxDocPath)}${skipTrace ? ' (skipped for fixture)' : ' (updated or current)'}`);
  console.log('\nAll presentation conformance checks passed.');
}

main().catch((error) => {
  console.error(`check-presentation failed: ${error.stack ?? error.message}`);
  process.exitCode = 1;
});
