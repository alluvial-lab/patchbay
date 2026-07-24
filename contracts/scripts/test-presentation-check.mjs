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
  // Return status AND output so assertions can prove the check actually
  // FAILS (non-zero exit) on a broken fixture, not merely that it prints a
  // diagnostic. A check that prints failures but exits 0 (neutered exitCode)
  // would pass an output-only assertion — that is the self-defining-check
  // anti-pattern this meta-test exists to catch.
  return {
    status: result.status,
    stdout: result.stdout ?? '',
    stderr: result.stderr ?? '',
    output: `${result.stdout ?? ''}\n${result.stderr ?? ''}`,
  };
}

const fixture = await mkdtemp(path.join(os.tmpdir(), 'patchbay-presentation-'));
try {
  for (const file of ['components.css', 'components.html', 'tokens.css']) {
    await writeFile(path.join(fixture, file), await readFile(path.join(designDir, file)));
  }

  // POSITIVE control: unmodified fixture must exit 0 (proves the check
  // actually passes on conformant artifacts — guards against a check that
  // fails indiscriminately, which would also be unsound).
  const baseline = runFixture(fixture);
  if (baseline.status !== 0) {
    throw new Error(`meta-test baseline (unmodified) exited ${baseline.status}; expected 0. Output: ${baseline.output.slice(0, 500)}`);
  }

  // NEGATIVE fixture 1: missing elicitation CSS binding → must exit non-zero
  // AND print the diagnostic (status alone isn't enough; output alone isn't enough).
  let css = await readFile(path.join(fixture, 'components.css'), 'utf8');
  css = css.replace('.elicitation-card--declined {', '.elicitation-card--declined-missing {');
  await writeFile(path.join(fixture, 'components.css'), css);
  const missingBinding = runFixture(fixture);
  if (missingBinding.status === 0) {
    throw new Error('meta-test: missing-binding fixture exited 0; expected non-zero (check must FAIL on defect)');
  }
  if (!/missing CSS binding \.elicitation-card--declined/.test(missingBinding.output)) {
    throw new Error('meta-test: missing-binding fixture did not print the expected diagnostic');
  }

  // NEGATIVE fixture 2: missing locked icon CSS selector → must exit non-zero.
  css = await readFile(path.join(designDir, 'components.css'), 'utf8');
  css = css.replace('.icon {', '.icon-missing {');
  await writeFile(path.join(fixture, 'components.css'), css);
  const missingIconCss = runFixture(fixture);
  if (missingIconCss.status === 0) {
    throw new Error('meta-test: missing-icon-css fixture exited 0; expected non-zero');
  }
  if (!/project-unique primitive icon: missing uncommented CSS class selector/.test(missingIconCss.output)) {
    throw new Error('meta-test: missing-icon-css fixture did not print the expected diagnostic');
  }

  // NEGATIVE fixture 3: missing icon showcase usage → must exit non-zero.
  css = await readFile(path.join(designDir, 'components.css'), 'utf8');
  await writeFile(path.join(fixture, 'components.css'), css);
  let showcase = await readFile(path.join(fixture, 'components.html'), 'utf8');
  showcase = showcase.replaceAll('class="icon', 'class="icon-missing');
  await writeFile(path.join(fixture, 'components.html'), showcase);
  const missingIconShowcase = runFixture(fixture);
  if (missingIconShowcase.status === 0) {
    throw new Error('meta-test: missing-icon-showcase fixture exited 0; expected non-zero');
  }
  if (!/project-unique primitive icon: missing showcase element/.test(missingIconShowcase.output)) {
    throw new Error('meta-test: missing-icon-showcase fixture did not print the expected diagnostic');
  }

  // NEGATIVE fixture 4: invisible-toast contrast (1:1) → must exit non-zero
  // AND print the contrast diagnostic. This is the regression the check exists for.
  await writeFile(path.join(fixture, 'components.html'), await readFile(path.join(designDir, 'components.html')));
  css = await readFile(path.join(designDir, 'components.css'), 'utf8');
  await writeFile(path.join(fixture, 'components.css'), css);
  let tokens = await readFile(path.join(fixture, 'tokens.css'), 'utf8');
  let inverseTokenCount = 0;
  tokens = tokens.replace(/--color-text-inverse:\s*#[0-9a-f]{6};/g, (declaration) => {
    inverseTokenCount += 1;
    return inverseTokenCount === 1 ? '--color-text-inverse: #2a2218;' : '--color-text-inverse: #f0e6d2;';
  });
  await writeFile(path.join(fixture, 'tokens.css'), tokens);
  const toastContrast = runFixture(fixture);
  if (toastContrast.status === 0) {
    throw new Error('meta-test: invisible-toast fixture exited 0; expected non-zero (check must FAIL on defect)');
  }
  if (!/contrast.*\.toast:.*requires 4\.5/.test(toastContrast.output) && !/contrast.*inverse.*surface/.test(toastContrast.output)) {
    throw new Error('meta-test: invisible-toast fixture did not print the expected contrast diagnostic');
  }

  console.log('Presentation check meta-tests passed: baseline exits 0; missing-binding, icon CSS/showcase, and invisible-toast fixtures exit non-zero with diagnostics.');
} finally {
  await rm(fixture, { recursive: true, force: true });
}
