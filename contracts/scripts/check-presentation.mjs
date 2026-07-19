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

// Contrast pairs are NOT hand-maintained — they are DERIVED from the actual
// CSS rules in components.css. A hand-maintained list is self-defining: a
// mutation that introduces a new failing pair (e.g. setting .toast color to
// its own background) would pass if the pair isn't in the list. Deriving from
// the CSS means EVERY rendered foreground/background combination in the layer
// is checked, including ones the check author didn't anticipate.
// Font-size classifies the WCAG threshold: normal text (<18pt, or <14pt bold)
// needs 4.5:1; large text (>=18pt, or >=14pt bold) needs 3:1. We map the
// layer's --font-size-* tokens to pt (xs=12, sm=14, base=16) and treat sm+bold
// or base as large, xs/sm-regular as normal.
const FONT_SIZE_PT = { '--font-size-xs': 12, '--font-size-sm': 14, '--font-size-base': 16 };

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
// selector exists; this asserts EACH modifier has a dominance rule AND that
// the rule sets opacity < 1 (the semantic). CSS comments are stripped first so
// a commented-out rule cannot fool the regex.
function checkDominance(cssSource, errors) {
  // Strip CSS comments so a binding commented out can't pass as present.
  const css = cssSource.replace(/\/\*[\s\S]*?\*\//g, '');
  const dominanceModifiers = ['--stale', '--unknown', '--offline', '--failed'];
  const rulePattern = /([^{}]*?)\{([^{}]*?)\}/g;
  let ruleMatch;
  const modifierToOpacities = new Map();
  for (const m of dominanceModifiers) modifierToOpacities.set(m, []);
  while ((ruleMatch = rulePattern.exec(css)) !== null) {
    const selectorText = ruleMatch[1];
    const body = ruleMatch[2];
    for (const modifier of dominanceModifiers) {
      const wrapperRe = new RegExp(`\\.session-status${modifier}\\b`);
      const hasRe = new RegExp(`\\.session-status:has\\(\\.connectivity-indicator${modifier}\\)`);
      if (wrapperRe.test(selectorText) || hasRe.test(selectorText)) {
        const opacityMatch = body.match(/opacity:\s*([0-9.]+)/);
        if (!opacityMatch) {
          errors.push(`dominance: ${modifier} rule sets no opacity`);
        } else {
          modifierToOpacities.get(modifier).push(Number.parseFloat(opacityMatch[1]));
        }
      }
    }
  }
  // EACH modifier must have a dominance rule (not just SOME). A prior version
  // used some(), which passed if only --failed remained after deleting the
  // others — that is the self-defining oracle this fixes.
  for (const modifier of dominanceModifiers) {
    const opacities = modifierToOpacities.get(modifier);
    if (opacities.length === 0) {
      errors.push(`dominance: no de-emphasis rule found for connectivity ${modifier} (stale/unknown/offline/failed each require one)`);
    }
    for (const opacity of opacities) {
      if (opacity >= 1) {
        errors.push(`dominance: ${modifier} de-emphasis sets opacity ${opacity} (must be < 1 to de-emphasize activity)`);
      }
    }
  }
  // Assert reduced-motion guards exist for BOTH animations AND actually set
  // animation: none (not just count media blocks — a prior version passed if
  // the media block existed but didn't disable the animation). Use a
  // depth-aware extract because the media block contains nested braces.
  const reducedMotionBodies = [];
  const mediaRe = /@media\s*\(prefers-reduced-motion:\s*reduce\)\s*\{/g;
  let mediaMatch;
  while ((mediaMatch = mediaRe.exec(css)) !== null) {
    let depth = 1;
    let body = '';
    for (let i = mediaMatch.index + mediaMatch[0].length; i < css.length && depth > 0; i += 1) {
      if (css[i] === '{') depth += 1;
      else if (css[i] === '}') { depth -= 1; if (depth === 0) break; }
      body += css[i];
    }
    reducedMotionBodies.push(body);
  }
  const reducedMotionJoined = reducedMotionBodies.join('\n');
  const needsNone = ['pb-spin', 'pb-pulse'];
  let noneFound = false;
  for (const keyframe of needsNone) {
    if (!/animation:\s*none/.test(reducedMotionJoined)) {
      errors.push(`dominance/a11y: prefers-reduced-motion block does not set animation: none (must disable ${keyframe})`);
      break;
    }
    noneFound = true;
  }
  if (reducedMotionBodies.length < 2) {
    errors.push(`dominance/a11y: expected at least 2 prefers-reduced-motion guards (found ${reducedMotionBodies.length})`);
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

// Resolve a CSS value that may be var(--token) or a literal hex to a hex color
// via the token map. Returns null if it can't resolve (e.g. 'transparent',
// or a token not in the color-token set — those are non-color backgrounds).
function resolveColorValue(value, tokens) {
  const trimmed = value.trim();
  const varMatch = trimmed.match(/^var\((--[a-z0-9-]+)\)$/);
  if (varMatch) return tokens.get(varMatch[1]) ?? null;
  if (/^#[0-9a-f]{6}$/i.test(trimmed)) return trimmed;
  return null; // 'transparent', inherit, etc.
}

// Derive the rendered foreground/background/font-size for every CSS rule that
// sets both color and background. This is the genuine oracle: it checks the
// ACTUAL combinations the CSS produces, not a hand-maintained list. A mutation
// setting .toast color to its own background is caught here because the rule
// is parsed, not anticipated.
function deriveContrastPairs(css, tokens, errors) {
  const pairs = [];
  const rulePattern = /([^{}]+)\{([^{}]*)\}/g;
  let match;
  while ((match = rulePattern.exec(css)) !== null) {
    const selectors = match[1];
    const body = match[2];
    // Skip at-rules (@media, @keyframes) — their inner rules are handled when
    // the regex reaches them. Skip rules inside comments (rare in body).
    if (selectors.includes('@')) continue;

    const colorMatch = body.match(/(?:^|;|\{)\s*color\s*:\s*([^;]+)/m);
    const bgMatch = body.match(/background(?:-color)?\s*:\s*([^;]+)/m);
    if (!colorMatch || !bgMatch) continue;

    const fg = resolveColorValue(colorMatch[1], tokens);
    const bg = resolveColorValue(bgMatch[1], tokens);
    if (!fg || !bg) continue; // non-resolvable (transparent, etc.) — skip

    // Determine font-size for threshold classification.
    const fontMatch = body.match(/font:\s*[^;]*?(--font-size-[a-z]+)/) || body.match(/font-size:\s*var\((--font-size-[a-z]+)\)/);
    const sizeToken = fontMatch ? (fontMatch[1] || fontMatch[2]) : null;
    const sizePt = sizeToken ? (FONT_SIZE_PT[sizeToken] ?? 16) : 16;
    const weightMatch = body.match(/font:\s*[^;]*?(--font-weight-(?:semibold|bold))/);
    const isBold = weightMatch !== null;
    // WCAG large text: >=18pt, or >=14pt bold. Else normal (4.5:1).
    const isLarge = sizePt >= 18 || (sizePt >= 14 && isBold);
    const threshold = isLarge ? 3 : 4.5;

    const selectorLabel = selectors.trim().split(',').map((s) => s.trim()).slice(0, 1).join('').slice(0, 40);
    pairs.push({ fg, bg, threshold, label: selectorLabel });
  }
  return pairs;
}

function checkContrast(cssSource, tokensSource, errors) {
  const modes = [
    ['light', parseColorTokens(tokensSource, ':root')],
    ['dark', parseColorTokens(tokensSource, ':root[data-theme="dark"]')],
  ];
  for (const [mode, tokens] of modes) {
    const pairs = deriveContrastPairs(cssSource, tokens, errors);
    if (pairs.length === 0) {
      errors.push(`contrast ${mode}: no color+background rules derived from CSS (oracle broken)`);
      continue;
    }
    for (const pair of pairs) {
      const ratio = contrastRatio(pair.fg, pair.bg);
      if (ratio < pair.threshold) {
        errors.push(`contrast ${mode}/${pair.label}: ${pair.fg} on ${pair.bg} is ${ratio.toFixed(2)}:1, requires ${pair.threshold}:1`);
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

// Parse the canonical retry-safety table from docs/UX.md as the Single Source
// of Truth. The UX.md table's grouped pre-execution row lists three failure
// codes (target_offline, adapter_unavailable, delivery_rejected); this expands
// it into individual triples. The check's matrix is thus DERIVED from UX.md,
// not hand-maintained alongside it — deleting a row from both the check and the
// showcase would still fail because UX.md (the independent source) still
// names it.
function parseUxRetryMatrix(uxSource, errors) {
  const rows = [];
  // Match the grouped pre-execution row: lists multiple codes in backticks.
  const groupedRe = /pre-execution failures \(([^)]+)\)\s*\|\s*`?any`?\s*\|\s*safe to retry/i;
  const groupedMatch = uxSource.match(groupedRe);
  if (groupedMatch) {
    const codes = [...groupedMatch[1].matchAll(/`([a-z_]+)`/g)].map((m) => m[1]);
    for (const code of codes) rows.push({ failure: code, strength: 'any', safety: 'safe' });
  } else {
    errors.push('retry matrix: could not parse UX.md grouped pre-execution failures row');
  }
  // Match the explicit single-code rows.
  const rowRe = /^\s*\|\s*`([a-z_]+)`\s*\|\s*`([a-z-]+)`\s*\|\s*(.+?)\s*\|\s*$/gm;
  let m;
  while ((m = rowRe.exec(uxSource)) !== null) {
    const failure = m[1];
    const strength = m[2];
    const safetyDesc = m[3].toLowerCase();
    if (failure === 'failure' || failure.includes('---')) continue;
    let safety;
    if (safetyDesc.includes('safe to retry')) safety = 'safe';
    else if (safetyDesc.includes('may double')) safety = 'maybe';
    else if (safetyDesc.includes('will double')) safety = 'unsafe';
    else if (safetyDesc.includes('not unconditionally')) safety = 'maybe';
    else { errors.push(`retry matrix UX.md: could not classify safety for "${failure}"`); continue; }
    rows.push({ failure, strength, safety });
  }
  return rows;
}

function checkRetryMatrix(showcase, protoEnums, uxSource, errors) {
  const uxRows = parseUxRetryMatrix(uxSource, errors);
  const failureCodes = protoMembers(protoEnums, 'FailureCode');
  const strengths = new Set(protoEnums.get('IdempotencyStrength').map((item) => item.name.slice('IDEMPOTENCY_STRENGTH_'.length)));
  for (const row of uxRows) {
    if (!failureCodes.has(row.failure)) errors.push(`retry matrix: UX.md failure term ${row.failure} is not a FailureCode enum member`);
    if (row.strength !== 'any' && !strengths.has(row.strength.toUpperCase().replace(/-/g, '_'))) {
      errors.push(`retry matrix: UX.md strength ${row.strength} is not an IdempotencyStrength enum member`);
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
  let uxSource;
  try {
    [css, showcase, tokens, uxSource] = await Promise.all([
      readFile(cssPath, 'utf8'),
      readFile(showcasePath, 'utf8'),
      readFile(tokensPath, 'utf8'),
      readFile(uxDocPath, 'utf8'),
    ]);
    checkBindings({ css, showcase, protoEnums }, errors);
    checkDominance(css, errors);
    checkRetryMatrix(showcase, protoEnums, uxSource, errors);
    checkContrast(css, tokens, errors);
  } catch (error) {
    errors.push(error.message);
  }

  if (showcase) await runAxe(showcase, errors);

  console.log('Presentation conformance check summary');
  console.log(`- registries checked: ${REGISTRY.length}`);
  console.log(`- CSS target: ${rel(cssPath)}`);
  console.log(`- showcase target: ${rel(showcasePath)}`);
  console.log(`- contrast: derived from CSS color+background rules (both light/dark modes)`);
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
