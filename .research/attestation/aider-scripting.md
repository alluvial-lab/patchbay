---
source_handle: aider-scripting
fetched: 2026-07-03
source_url: file:///tmp/aider/aider/website/docs/scripting.md
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: Aider external scripting surfaces

## Core findings
- Docs describe command-line scripting via:
  - `--message` (single instruction)
  - `--message-file`
  - `--yes`
  - `--commit`, `--run`, `--help`-related options, and `--auto-commits`, `--dirty-commits`, `--dry-run`.
- Docs also describe a Python API (noted as not officially supported): instantiate `Coder` and call `coder.run(...)`, optionally pass custom `InputOutput`.
- This indicates both CLI one-shot and API object-style control styles are documented, but CLI is primary documented operator surface.

## Evidence snippets
1) Scripting section states `--message` and `--message-file` send one instruction and exit.
2) `--yes` shown as shorthand confirmation option in docs.
3) Python usage example with `Coder.create(...)` and `coder.run(...)`.