---
source_handle: jupyter-kernel-messaging
fetched: 2026-07-07
source_url: https://jupyter-client.readthedocs.io/en/latest/messaging.html
provenance: source-direct
---

# Attestation: Jupyter kernel messaging protocol

## Structural metadata

- Publisher/site: Jupyter Client documentation on Read the Docs.
- Page title observed: Messaging in Jupyter.
- Source kind: protocol documentation for frontend/kernel messaging.

## Paraphrased summary

The Jupyter messaging protocol defines messages with IDs, sessions, parent headers for correlating replies and side effects, request/reply flows, busy/idle status events, and execution replies. It is a frontend-kernel protocol centered on code execution and kernel status rather than on human authority over autonomous sessions.

## Key passages

1. **Message header IDs.** A message header includes `msg_id` (typically a UUID, unique per message), `session` (typically a UUID, unique per session), `username`, `date`, `msg_type`, and `version`. Source anchor: lines 539-543.

2. **Parent-header correlation.** When a message is the result of another message, such as side-effect output/status or direct reply, `parent_header` is a copy of the causing message's header; `_reply` messages must have a parent header. Source anchor: lines 589-596.

3. **Minimum kernel messages.** Kernels must implement execute and kernel-info messages along with associated busy and idle kernel-status messages. Source anchor: lines 656-660.

4. **Request/reply pattern.** The client sends an `<action>_request` such as `execute_request` on the shell socket; the kernel publishes `status: busy`, processes the request, sends the matching `<action>_reply`, and then publishes `status: idle` after associated IOPub messages. Source anchor: lines 813-823.

5. **Reply statuses.** Reply messages have a `status` field with values including `ok`, `error`, and deprecated `aborted`; the error case includes exception name/value/traceback fields. Source anchor: lines 824-850.

6. **Execute request body.** `execute_request` carries source code to be executed and options including `silent`, `store_history`, `user_expressions`, `allow_stdin`, and `stop_on_error`. Source anchor: lines 856-890.

7. **Execute completion.** Upon completing an execution request, the kernel always sends a reply with a status code and additional data depending on outcome. Source anchor: lines 922-925.

8. **Execution count.** Requests with `store_history=True` increment a kernel counter used in prompts and returned as `execution_count` in `execute_reply` and `execute_input` messages. Source anchor: lines 934-936.

9. **Kernel status messages.** The kernel-status section says the `status` message type is used by frontends to monitor the kernel status. Source anchor: lines 2013-2016.
