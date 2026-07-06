#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, '../..');
const contractsRoot = path.join(repoRoot, 'contracts');

// `buf` must be available on PATH. In this workspace README, npm global binaries
// live at ~/.npm-global/bin, so include that directory for sandbox parity while
// still honoring the caller's PATH first.
const npmGlobalBin = path.join(process.env.HOME ?? '', '.npm-global', 'bin');
const env = {
  ...process.env,
  PATH: [process.env.PATH, npmGlobalBin].filter(Boolean).join(path.delimiter),
};

function run(command, args, options) {
  const result = spawnSync(command, args, {
    stdio: 'inherit',
    env,
    ...options,
  });

  if (result.error) {
    console.error(`Failed to run ${command}: ${result.error.message}`);
    return 127;
  }

  return result.status ?? 1;
}

console.log('Checking generated contract drift: generated paths are clean before regeneration');
const preflightStatus = run('git', ['diff', '--exit-code', '--', 'contracts/rust/src/gen', 'contracts/ts/src/gen'], { cwd: repoRoot });
if (preflightStatus !== 0) {
  console.error('\nGenerated contract files already differ from HEAD. Commit or revert those changes before running the drift check.');
  process.exit(preflightStatus);
}

console.log('Checking generated contract drift: buf generate');
const generateStatus = run('buf', ['generate'], { cwd: contractsRoot });
if (generateStatus !== 0) process.exit(generateStatus);

console.log('Checking generated contract drift: git diff --exit-code -- contracts/rust/src/gen contracts/ts/src/gen');
const diffStatus = run('git', ['diff', '--exit-code', '--', 'contracts/rust/src/gen', 'contracts/ts/src/gen'], { cwd: repoRoot });
if (diffStatus !== 0) {
  console.error('\nGenerated contracts drift detected. Run `buf generate` from contracts/ and commit the updated generated files.');
}

process.exit(diffStatus);
