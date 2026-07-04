---
title: Aider harness action-surface research
campaign: harness-action-surfaces
specialist: aider
compiled: 2026-07-03
---

## Research outcome (source-grounded)
Aider is primarily a **local interactive session tool** with a dual control surface:
- **Operator→agent (out-of-band controls):** CLI flags and in-chat slash commands.
- **Agent behavior:** message/chat handling, shell/tool operations, and model responses that are surfaced through local IO and optional analytics events.

Sources: `aider-args`, `aider-main`, `aider-commands`, `aider-base-coder`, `aider-io`, `aider-gui`, `aider-scripting`, `aider-usage-commands`.

## 1) Operator actions observed

### CLI actions (terminal/operator entry)
- One-shot invocation mode: `--message` / `--message-file` executes one instruction and exits. [aider-args]{1}
- Session/session-like control: `--exit`, `--load`, `--restore-chat-history`, `--chat-history-file`, `--message`, `--message-file`, `--gui/--browser`, `--commit`, `--test`, `--lint`, `--apply`, `--apply-clipboard-edits`, `--copy-paste`. [aider-args]{1}[aider-main]{2}
- Model/control switches: `--model`, `--edit-format`, `--weak-model`, `--editor-model`, `--architect`, `--show-model-warnings`, etc. [aider-args]{1}

### In-chat slash-command actions
- Command grammar is clear: input beginning `/` or `!` is command-mode; all slash commands map to `cmd_*` methods. [aider-commands]{3}[aider-commands]{4}
- `!` is a passthrough shell command alias (`command_run`). [aider-commands]{4}
- Command set includes durable state/session operations like `/load`, `/save`, `/add`, `/drop`, `/clear`, `/reset`, `/exit`, `/quit`, `/model`, `/editor-model`, `/weak-model`, `/settings`, `/help`, plus shell/git operations `/run`, `/test`, `/git`, and mode switches (`/ask`, `/code`, `/architect`, `/context`, `/chat-mode`). [aider-commands]{3}[aider-usage-commands]{3}
- `/run` and `/git` route command output to tool output; `/run` can append output to chat, `/git` output is shown to operator UI and excluded from chat context. [aider-commands]{5}

### Model/message vs command split
- Non-command lines are treated as chat messages and sent as user messages (`role="user"`) after preprocessing. [aider-base-coder]{3}
- Commands are not sent as normal user content; they are intercepted through the command dispatcher first. [aider-base-coder]{3}[aider-commands]{3}

## 2) Durable vs ephemeral/read-only classification (from observed surface)

### Mostly durable / stateful
- Session state mutation in filesystem: restore/add files to chat (`/add`, `/read-only`, `--restore-chat-history`, `--load`), model mode switches (`/model`, `/editor-model`, `/weak-model`, `--model`, startup flags), file edit application (`apply_updates` path) and commit/test/lint branches. [aider-commands]{4}[aider-main]{4}[aider-base-coder]{6}
- `--commit`, `/commit`, `/undo`, `/save`, `/load` explicitly persist/reconstruct state across runs. [aider-main]{3}[aider-commands]{3}[aider-main]{4}

### Ephemeral/read-only
- Introspection/utility actions: `/tokens`, `/ls`, `/map`, `/map-refresh`, `/settings`, `/help`, `/clear`, `/drop`, `/reset` (context/session reset within run), `/report`. [aider-commands]{3}[aider-usage-commands]{3}
- Command outputs and prompts are surfaced directly via tool output methods without guaranteed durable effects unless explicitly persisted (e.g., save/commit). [aider-commands]{5}[aider-io]{1}

## 3) Agent→operator event/output channels

- Local operator-facing outputs are `tool_output`, `tool_warning`, `tool_error`, `assistant_output`, and `ai_output` on `InputOutput`. [aider-io]{1}
- LLM response streaming is chunked in `show_send_output_stream`; chunks are rendered incrementally via `live_incremental_response`/`assistant_output`. [aider-base-coder]{4}
- Non-streaming responses still surface through `assistant_output`; after completion, `ai_output` logs the final assistant content. [aider-base-coder]{4}
- Lifecycle/events in code are mostly telemetry-style via `self.event(...)` wiring to Analytics (`analytics.event`): `message_send_starting`, `message_send_exception`, `command_run`, `command_<name>`, `exit`, `cli session`, etc.; there is no dedicated external event bus for downstream tools seen in core code. [aider-base-coder]{2}[aider-commands]{4}[aider-main]{4}

## 4) Interrupt/cancel and lifecycle
- Interrupt model is in-loop and recoverable: first Ctrl-C warns, second Ctrl-C in short window triggers exit with event reason `Control-C`. [aider-base-coder]{7}
- KeyboardInterrupt during send contributes a synthetic user/assistant interruption turn into message history (`^C KeyboardInterrupt`) before continuing. [aider-base-coder]{7}

## 5) Sidecar / supervisor requirement
- No explicit privileged sidecar/supervisor command/API for “agent process management” is present in core CLI/session code; control remains within the same process flow. [aider-main]{2}[aider-base-coder]{2}
- GUI path is Streamlit-launched (`launch_gui`), i.e., a UI runner wrapper, not a persistent orchestrator API for agent lifecycle. [aider-main]{1}[aider-gui]{2}

## 6) Spawn/retire/agent lifecycle instances
- No observed command flag/method pair for explicit `spawn` or `retire` of an agent instance in parser flags or command handlers. Actions are session startup/exit and in-session file/chat/model operations. [aider-args]{1}[aider-commands]{3}

## 7) Python API note
- Docs include a Python API path (`Coder.create(...); coder.run(...)`) as an internal/unsupported interface. [aider-scripting]{3}

## Disconfirming analysis
- Initial expectation: clear explicit event stream for downstream systems beyond tool text. The code instead sends operator-visible state changes through output methods and logs many actions via analytics events; it does not expose a formal structured message bus in the inspected surface. [aider-main]{4}[aider-base-coder]{8}
- Initial expectation: one explicit “compact” command. No direct command/flag named “compact/refresh” for session compression was found; only auto-summarization occurs in coder session internals when history grows (`summarize_start`) and explicit `/clear`/`/reset` for manual context reset. [aider-base-coder]{8}[aider-commands]{3}

## Contradictions
1. Documentation vs parser naming:
   - Docs/examples mention `--yes`; parser defines `--yes-always` as the documented option with deprecated/no-alias transition handling and config migration notes. [aider-args]{1}[aider-main]{1}
   - This creates mild naming drift for operators reading docs.

2. Docs present Python API as helper but label it unsupported/possibly unstable; core README/source position is CLI-first. [aider-scripting]{3}[aider-main]{4}

## Revisit if
- Aider’s release adds a stable external event protocol (structured callbacks or explicit message API).
- New CLI commands/flags introduce explicit instance lifecycle verbs (spawn, retain, retire, checkpoint, compact).
- Streaming event shape changes from plain `analytics.event` to machine-consumable lifecycle envelopes.
- Command dispatch stops using `cmd_*` conventions in `Commands`.
