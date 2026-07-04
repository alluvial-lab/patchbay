---
source_handle: cursor-cli-output
fetched: 2026-07-03
source_url: https://cursor.com/docs/cli/reference/output-format
provenance: source-direct
---

# Per-source attestation: Cursor CLI output format

## Structural metadata

- Publisher/site: Cursor Documentation.
- Page title: "Output format | Cursor Docs".
- Canonical URL observed in fetched page metadata: `https://cursor.com/docs/cli/reference/output-format`.
- Page description observed in fetched metadata: "Control output formatting for Cursor CLI commands and responses."
- Internal headings observed: "JSON format", "Stream JSON format", "Event types", "System initialization", "User message", "Assistant message", "Tool call events", "Tool call types", "Terminal result", "Text format", "Notes".

## Paraphrased summary

The page documents CLI output schemas for agent execution. It has a final JSON result mode, a stream-json NDJSON event mode, and a text mode. The stream format exposes system initialization, user input, assistant messages, tool-call start/completion events, and terminal result events.

## Key passages

1. Under "JSON format", the page states that JSON output "emits a single JSON object ... when the run completes successfully" and that "Deltas and tool events are not emitted; text is aggregated into the final result."

2. The success response example includes `{ "type": "result", "subtype": "success", "is_error": false, ... "result": "<full assistant text>", "session_id": "<uuid>", "request_id": "<optional request id>" }`.

3. Under "Stream JSON format", the page states that stream-json "emits newline-delimited JSON (NDJSON)" and that each line is "a single JSON object representing an event during execution"; it also says the format outputs "one line per assistant message" and the stream ends with a terminal event on success.

4. Under "Streaming partial output", the page states that `--stream-partial-output` with `--output-format stream-json` emits text as generated "in small chunks, with multiple assistant events per message."

5. Under "Event types", the page states that system initialization is "Emitted once at the beginning of each session" and gives fields including `session_id`, `model`, and `permissionMode`.

6. Under "User message", the page states: "Contains the user's input prompt" and gives an event with `type: "user"` and a user message payload.

7. Under "Assistant message", the page states: "Emitted once per complete assistant message (between tool calls). Each event contains the full text of that message segment" and gives an event with `type: "assistant"`.

8. Under "Tool call events", the page states: "Tool calls are tracked with start and completion events" and gives `type: "tool_call"`, `subtype: "started"` and `subtype: "completed"` examples with a `call_id`.

9. Under "Terminal result", the page states that the final event on successful completion has `type: "result"`, `subtype: "success"`, duration fields, `result`, and `session_id`.

10. Under "Text format", the page states that text output "provides only the final assistant message without any intermediate progress updates or tool call summaries."
