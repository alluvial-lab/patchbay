---
source_handle: aider-main
fetched: 2026-07-03
source_url: file:///tmp/aider/aider/main.py
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: aider main-session orchestration and lifecycle

## Core findings
- Main entry flow checks `args.gui` and invokes `launch_gui(argv)` for browser mode; otherwise proceeds with terminal session setup and `main` loop.
- Lifecycle telemetry/events are emitted with `analytics.event()`, including:
  - `launched`, `gui session`, and many `exit` reasons (`Streamlit not installed`, `exit flag set`, `Completed --message`, `Completed main CLI coder.run`, etc.).
- CLI session path is: create/get parser -> parse args -> build `Commands` and `Coder` -> optional `copy_paste`, optional `--load` file replay, optional one-shot `--message`/`--message-file` (then return), optional `--exit` (return), otherwise loop `coder.run()`.
- Compact-like behavior exists via `restore_chat_history` passed into `Coder.create`; chat history is then restored before interactive loop if enabled.
- `--load` is executed before interactive start and processes command file lines.
- `--gui/--browser` is a separate Streamlit-backed client mode (via `launch_gui`), not a separate in-product “agent instance manager” command.
- `launch_gui` uses `streamlit web.cli.main` with generated `st_args` and `--` followed by original args.

## Evidence snippets
1) `main` branch checks `if args.gui and not return_coder` and calls `launch_gui`.
2) Post-build branch evaluates `args.show_prompts`, `args.lint`, `args.test`, `args.commit`, `args.show_repo_map`, `args.apply`, `args.apply_clipboard_edits`, `args.show_release_notes`, `args.load`, `args.message`, `args.message_file`, `args.exit` then enters `while True: coder.run()`.
3) `analytics.event("exit", reason=...)` at many control points and session events.
4) `Coder.create(..., restore_chat_history=args.restore_chat_history, ...)` and command loop wrapper `if args.message: ...` / `if args.message_file: ...`.