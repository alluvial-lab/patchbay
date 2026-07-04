---
source_handle: claude-code-cli
fetched: 2026-07-03
source_url: https://code.claude.com/docs/en/cli-reference.md
provenance: source-direct
---

# Attestation: Claude Code CLI reference

## Paraphrased summary

The CLI reference enumerates top-level Claude Code commands and flags. Relevant action surfaces include starting interactive or print-mode sessions, continuing/resuming sessions, opening agent view and managing background sessions, starting Remote Control, cloud/web/teleport entry points, permission/model/effort/session flags, partial and hook-event stream output, MCP login, plugin management, and session persistence controls.

## Key passages

1. **Session start and prompt forms.** CLI commands include `claude` for interactive sessions, `claude "query"` for an interactive session with initial prompt, and `claude -p "query"` or piped content for SDK/print-mode queries that exit. Source anchor: “CLI commands,” lines 15-18.

2. **Continue/resume commands.** Commands include `claude -c -p "query"` to continue via SDK and `claude -r "<session>" "query"` to resume by id or name. Source anchor: lines 20-21.

3. **Background session management commands.** Commands include `claude agents`, `attach`, `logs`, `stop`/`kill`, `respawn`, `rm`, `daemon status`, and `daemon stop --any`; these are linked to background-session agent view and supervisor management. Source anchor: lines 28-43.

4. **Remote Control command.** `claude remote-control` starts a Remote Control server to control Claude Code from claude.ai or the Claude app in server mode, with server-mode flags described on the Remote Control page. Source anchor: line 39.

5. **Model/permission/background flags.** CLI flags include `--agent`, `--agents`, `--allowedTools`, `--disallowedTools`, `--permission-mode`, `--model`, `--fallback-model`, `--effort`, `--bg`/`--background`, and `--dangerously-skip-permissions`. Source anchor: flags table lines 58-101.

6. **Session flags.** CLI flags include `--continue`, `--resume`, `--fork-session`, `--session-id`, `--name`, and `--no-session-persistence`. Source anchor: flags table lines 70-113.

7. **Stream and hook output flags.** CLI flags include `--include-hook-events` to include hook lifecycle events in stream-json output, `--include-partial-messages` to include partial streaming events, `--input-format`, `--output-format`, and `--replay-user-messages`. Source anchor: flags table lines 88-110.

8. **Remote/web flags.** CLI flags include `--remote-control`/`--rc` to start an interactive session with Remote Control enabled, `--cloud` to create a web session, and `--teleport` to resume a web session locally. Source anchor: flags table lines 70, 108-119.

9. **Settings/plugin/MCP flags.** CLI flags include `--settings`, `--plugin-dir`, `--plugin-url`, `--mcp-config`, `--strict-mcp-config`, and command `claude mcp login`; they alter settings/plugin/MCP surfaces for the invocation. Source anchor: command table line 35 and flags table lines 103-115.
