---
source_handle: aider-io
fetched: 2026-07-03
source_url: file:///tmp/aider/aider/io.py
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: Input/output and interaction surfaces

## Core findings
- `InputOutput` is the primary operator-facing IO surface and exposes:
  - `user_input()`, `tool_output()`, `tool_warning()`, `tool_error()`, `assistant_output()`, `ai_output()`, `confirm_ask()`, `prompt_ask()`, `get_input()`, `append_chat_history()`.
- `get_input()` uses `prompt_toolkit` with keybindings and `interrupt_input()` support (Ctrl-Z for suspend, Enter/Alt-Enter for multiline, file/clipboard watcher hooks).
- `confirm_ask()` supports binary/tri-state confirmations and persists prompt responses in chat output.
- Interrupt hook `interrupt_input()` stores partially typed text, marks interruption, and exits prompt session to return control.
- File and LLM history logging support via configured `chat_history_file` and `llm_history_file`.

## Evidence snippets
1) Method definitions for command/input/output primitives above.
2) `interrupt_input()` sets `prompt_session.app.exit()` after storing prompt text.
3) `get_input()` attaches bindings and returns typed input; in interruption path can process file-watcher command output.
4) Output functions emit messages directly to console and optionally update markdown history.