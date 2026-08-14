#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export PATH="${HOME}/.npm-global/bin:${PATH}"

if ! command -v quint >/dev/null 2>&1; then
  echo "formal checks require quint (expected on PATH or at ~/.npm-global/bin/quint)" >&2
  exit 1
fi

# First enforce the model-promotion registry and generated traceability contract.
node "${repo_root}/contracts/scripts/check-models.mjs"

passed=0
failed=0

record_pass() {
  passed=$((passed + 1))
  printf '[pass] %s\n' "$1"
}

record_failure() {
  failed=$((failed + 1))
  printf '[fail] %s\n' "$1" >&2
}

# Every Quint source must parse and typecheck, including draft models.
while IFS= read -r model; do
  relative="${model#"${repo_root}/"}"
  if (cd "$(dirname "${model}")" && quint compile "$(basename "${model}")" >/dev/null); then
    record_pass "typecheck ${relative}"
  else
    record_failure "typecheck ${relative}"
  fi
done < <(find "${repo_root}/specs/seed" -maxdepth 1 -type f -name '*.qnt' -print | LC_ALL=C sort)

# The @promotion blocks are the single source of truth for checked model
# invocations. Emit only promoted checks, then execute their allowlisted Quint
# commands from the model directory so the recorded basename invocations work.
promotions_file="$(mktemp)"
trap 'rm -f "${promotions_file}"' EXIT
node - "${repo_root}" >"${promotions_file}" <<'NODE'
const fs = require('node:fs');
const path = require('node:path');
const root = process.argv[2];
const seed = path.join(root, 'specs', 'seed');
const files = fs.readdirSync(seed).filter((name) => name.endsWith('.qnt')).sort();

for (const name of files) {
  const source = fs.readFileSync(path.join(seed, name), 'utf8');
  const blocks = source.match(/^\s*\/\/\s*@promotion\s*\{[\s\S]*?^\s*\/\/\s*\}\s*$/gm) ?? [];
  for (const block of blocks) {
    const fields = {};
    let current = null;
    for (const raw of block.split('\n')) {
      const line = raw.replace(/^\s*\/\/\s?/, '').trim();
      if (!line || line === '@promotion {' || line === '}') continue;
      const match = line.match(/^([A-Za-z_][A-Za-z0-9_-]*):\s*(.*)$/);
      if (match) {
        current = match[1];
        fields[current] = match[2].trim();
      } else if (current) {
        fields[current] = `${fields[current]} ${line}`.trim();
      }
    }
    if (fields.status === 'promoted') {
      for (const key of ['property', 'model', 'invocation']) {
        if (!fields[key] || fields[key].includes('\t') || fields[key].includes('\n')) {
          throw new Error(`${name}: promoted block has invalid ${key}`);
        }
      }
      process.stdout.write(`${fields.property}\t${fields.model}\t${fields.invocation}\n`);
    }
  }
}
NODE

while IFS=$'\t' read -r property model invocation; do
  case "${invocation}" in
    "quint verify "*|"echo y | quint verify "*) ;;
    *)
      echo "unsupported promoted model invocation for ${property}: ${invocation}" >&2
      exit 1
      ;;
  esac

  model_path="${repo_root}/${model}"
  if [[ ! -f "${model_path}" || "${model}" != specs/seed/*.qnt ]]; then
    echo "promoted model ${property} names an invalid model path: ${model}" >&2
    exit 1
  fi

  if (cd "$(dirname "${model_path}")" && bash -o pipefail -c "${invocation}"); then
    record_pass "verify ${property}"
  else
    record_failure "verify ${property}"
  fi
done <"${promotions_file}"

printf 'Formal model checks: %d passed, %d failed\n' "${passed}" "${failed}"
[[ "${failed}" -eq 0 ]]
