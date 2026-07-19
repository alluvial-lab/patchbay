#!/usr/bin/env node
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, '../..');
const designDir = path.join(repoRoot, '.mockups', 'design-system');
const check = path.join(scriptDir, 'check-presentation.mjs');

function runFixture(dir) {
  const result = spawnSync(process.execPath, [check], {
    cwd: repoRoot,
    encoding: 'utf8',
    env: {
      ...process.env,
      PRESENTATION_CSS_PATH: path.join(dir, 'components.css'),
      PRESENTATION_SHOWCASE_PATH: path.join(dir, 'components.html'),
      PRESENTATION_TOKENS_PATH: path.join(dir, 'tokens.css'),
      PRESENTATION_SKIP_TRACE: '1',
    },
  });
  return `${result.stdout}\n${result.stderr}`;
}

const fixture = await mkdtemp(path.join(os.tmpdir(), 'patchbay-presentation-'));
try {
  for (const file of ['components.css', 'components.html', 'tokens.css']) {
    await writeFile(path.join(fixture, file), await readFile(path.join(designDir, file)));
  }

  let css = await readFile(path.join(fixture, 'components.css'), 'utf8');
  css = css.replace('.elicitation-card--declined {', '.elicitation-card--declined-missing {');
  await writeFile(path.join(fixture, 'components.css'), css);
  const missingBindingOutput = runFixture(fixture);
  if (!/missing CSS binding \.elicitation-card--declined/.test(missingBindingOutput)) {
    throw new Error('meta-test expected a missing elicitation CSS binding failure');
  }

  css = await readFile(path.join(designDir, 'components.css'), 'utf8');
  await writeFile(path.join(fixture, 'components.css'), css);
  let tokens = await readFile(path.join(fixture, 'tokens.css'), 'utf8');
  let inverseTokenCount = 0;
  tokens = tokens.replace(/--color-text-inverse:\s*#[0-9a-f]{6};/g, (declaration) => {
    inverseTokenCount += 1;
    return inverseTokenCount === 1 ? '--color-text-inverse: #2a2218;' : '--color-text-inverse: #f0e6d2;';
  });
  await writeFile(path.join(fixture, 'tokens.css'), tokens);
  const toastContrastOutput = runFixture(fixture);
  if (!/toast\/inverse surface text/.test(toastContrastOutput)) {
    throw new Error('meta-test expected the invisible-toast contrast failure');
  }

  console.log('Presentation check meta-tests passed (missing binding and invisible-toast fixtures rejected).');
} finally {
  await rm(fixture, { recursive: true, force: true });
}
