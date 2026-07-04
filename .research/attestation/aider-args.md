---
source_handle: aider-args
fetched: 2026-07-03
source_url: file:///tmp/aider/aider/args.py
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: aider CLI argument surface

## Core findings
- `get_parser()` defines flags for:
  - one-shot model message modes: `--message/--msg/-m` and `--message-file/-f` (disable chat mode), `--exit`, `--show-prompts`, `--show-repo-map`, `--show-release-notes`, `--apply`, `--apply-clipboard-edits`.
  - lifecycle/session-related flags: `--message`, `--message-file`, `--exit`, `--gui/--browser`, `--copy-paste`, `--load`, `--chat-history-file`, `--restore-chat-history`, `--input-history-file`, `--git`, `--auto-commits`, `--dirty-commits`, `--version`, `--commit`, `--test`, `--lint`.
  - control/confirmation: `--yes-always`.
  - mode selectors: `--chat-mode`, `--architect`, `--model`, `--editor-model`, `--weak-model`, `--edit-format`.
  - shell execution/testing flags: `--lint-cmd`, `--auto-lint`, `--test-cmd`, `--auto-test`, plus command-related settings `--shell`-related completion settings and `--test`/`--lint` execution flags.
  - watch/copy side features: `--watch-files` and `--copy-paste`.
- `--yes` appears in docs only as a prefix of `--yes-always` (BooleanOptionalAction generates opposite `--no-...`), while config migration helper still warns on old `yes:` in config files (indicating historical naming in config docs)

## Evidence snippets
1) Argument definitions around CLI modes include one-shot message/apply/exit flags.
2) History/session arguments include chat-history and restore options.
3) Model and settings flags show session/model-control actions.
4) Non-durable sidecar-related flags are `--gui/--browser` and watcher/copy-paste flags, not explicit process-manager commands.