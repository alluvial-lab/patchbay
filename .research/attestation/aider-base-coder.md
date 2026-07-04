---
source_handle: aider-base-coder
fetched: 2026-07-03
source_url: file:///tmp/aider/aider/coders/base_coder.py
provenance: source-direct
substrate_confidence: source-direct
---

# Attestation: aider message send, output, and interaction events

## Core findings
- `run()` has interactive loop with `get_input()` unless `with_message` is supplied.
- Input can be preprocessed by `preproc_user_input`: if command-like prefix, it is delegated to `Commands.run`; otherwise treated as user content.
- `send_message()` emits lifecycle `message_send_starting` event before provider calls and emits `message_send_exception` on unexpected send errors.
- Streaming path in `show_send_output_stream()` iterates completion chunks (`for chunk in completion`), concatenates chunk content/reasoning, and updates output incrementally via `live_incremental_response()`.
- Non-streaming path handles full completion and renders via `assistant_output`.
- `send()` logs LLM traffic in history (`io.log_llm_history("TO LLM")` and `"LLM RESPONSE"`) and pushes final assistant output through `io.ai_output`.
- Keyboard interruption handling:
  - first Ctrl-C logs warning and requires second Ctrl-C within ~2s to exit.
  - interruption during send/message send sets user/assistant placeholder messages and marks interruption in chat context.
- The coder also includes chat history compaction behavior (`summarize_start()` and `summarize_worker()`) used when `summarizer.too_big(done_messages)`.
- `exit` events are emitted with reasons in multiple call sites (e.g., `Control-C`, message handling branches).

## Evidence snippets
1) `run()` / `run_one()` orchestration and `while message:` flow with `send_message`.
2) `send_message` beginning calls `self.event("message_send_starting")`.
3) `show_send_output_stream` chunk loop and `live_incremental_response` calls.
4) `send()` finally block logs LLM response with `io.log_llm_history("LLM RESPONSE", ...)`.
5) `keyboard_interrupt()` double-press logic and `self.event("exit", reason="Control-C")`.
6) `summarize_start()` condition and `summarize_worker` thread kickoff.