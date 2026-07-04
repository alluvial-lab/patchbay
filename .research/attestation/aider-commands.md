---
source_handle: aider-commands
fetched: 2026-07-03
source_url: file:///tmp/aider/aider/commands.py
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: aider in-chat command surface and dispatch

## Core findings
- Commands are prefixed by `/` or `!`: `is_command()` checks first char in `'/!'`.
- `run()` resolves command names to `cmd_<name>` methods; unknown commands report errors; ambiguous commands are rejected.
- On command execution, it emits command events:
  - `command_run` for `!`-prefixed shell passthrough.
  - `command_<command>` for each resolved slash command.
- Command handlers include `/add`, `/drop`, `/clear`, `/reset`, `/chat-mode`, `/model`, `/editor-model`, `/weak-model`, `/ask`, `/code`, `/architect`, `/context`, `/ok`, `/run` (alias to shell run), `/git`, `/exit`, `/quit`, `/undo`, `/commit`, `/lint`, `/test`, `/save`, `/load`, `/map`, `/map-refresh`, `/tokens`, `/settings`, `/voice`, `/paste`, `/copy`, `/report`, `/think-tokens`, `/reasoning-effort`, `/copy-context`, `/help`, etc. (covered by `cmd_` method names and generated docs table.)
- `/run` executes shell via `run_cmd`, then (by default) asks user to add output; shell output can be piped back into chat.
- `/git` executes raw git command and outputs to operator UI only (`tool_output`, output excluded from chat).
- `/apply` CLI path exists in main; `/model` family switches models via `SwitchCoder` events.
- No command methods for `spawn`/`retire` were observed.

## Evidence snippets
1) `is_command`, `matching_commands`, and `run()` dynamic mapping to `cmd_<name>`.
2) `run()` calls `self.coder.event("command_run")` and `self.coder.event(f"command_{command}")`.
3) Command method set includes `_` names from `cmd_add` through `cmd_copy_context` (multiple durable/ephemeral control verbs).
4) `cmd_run` and `cmd_git` bodies show shell/git execution behavior and output routing.