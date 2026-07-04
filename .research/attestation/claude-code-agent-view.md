---
source_handle: claude-code-agent-view
fetched: 2026-07-03
source_url: https://code.claude.com/docs/en/agent-view.md
provenance: source-direct
---

# Attestation: Claude Code agent view

## Paraphrased summary

Agent view is a terminal UI and shell-command family for dispatching, monitoring, replying to, attaching to, stopping, removing, respawning, and inspecting multiple background Claude Code sessions. Background sessions are managed by a per-user supervisor process, are separate Claude Code processes, may isolate file edits into worktrees, and persist state/transcripts on disk.

## Key passages

1. **Definition.** Agent view, opened with `claude agents`, shows background sessions grouped by state, lets operators dispatch new sessions, watch what they need, reply, attach, and leave sessions running without a terminal attached. Source anchor: lines 5-17.

2. **Dispatch semantics.** Typing a prompt in agent view starts a new background session; typing another prompt starts another session rather than sending a follow-up to the first. Source anchor: lines 48-53.

3. **Peek/reply and attach.** Selecting a row and pressing Space opens a peek panel with recent output or question; operators can type a reply without leaving agent view, while Enter/right-arrow attaches to the full conversation. Source anchor: lines 57-61 and 165-181.

4. **Session states.** Rows distinguish states such as Needs input, Idle, Working, Done, Failed, and Stopped; Needs input means Claude waits on a question or permission decision. Source anchor: lines 107-118.

5. **Background durability.** Background sessions do not need any terminal open; a separate supervisor process runs them so closing agent view or shell does not stop dispatched work. Source anchor: lines 132-136.

6. **Keyboard controls.** Agent view shortcuts include Enter to attach/dispatch, Space to peek, Shift+Enter to dispatch and attach, Ctrl+T pin/unpin, Ctrl+R rename, Ctrl+X stop/delete, and Shift+arrow reorder. Source anchor: lines 231-249.

7. **Dispatch controls.** Agent-view dispatch can target a custom subagent with first word or `@agent`, target a repo with `@repo`, dispatch slash commands/skills, or run shell background jobs with `!`; `/model` in agent view changes dispatch model for subsequent sessions. Source anchor: lines 254-283 and 406-424.

8. **Background from session/shell.** `/background` or `/bg` moves the current conversation into a background session; shell `claude --bg` starts a session directly in the background, and the command prints management commands (`claude agents`, `attach`, `logs`, `stop`). Source anchor: lines 299-355.

9. **Stop/remove/respawn shell commands.** Shell management commands include `claude agents --json`, `claude attach <id>`, `claude logs <id>`, `claude stop`/`kill`, `claude respawn`, `claude rm`, and `claude daemon status/stop`. Source anchor: lines 473-489.

10. **Supervisor.** Background sessions are hosted by a per-user supervisor process, separate from terminal and agent view; it starts automatically, keeps a pre-warmed worker, assigns workers to sessions, and applies each session's directory/settings/credentials. Source anchor: lines 491-505.

11. **Process lifetime.** Each background session is its own Claude Code process managed by the supervisor; active, input-waiting, or attached sessions keep their process running, idle sessions may be stopped after about an hour, and attach/peek/reply starts a fresh process from transcript/state when needed. Source anchor: lines 505-507.

12. **Storage.** State is stored under the Claude Code config directory, including supervisor log, roster, per-job `state.json`, and scratch directories; each background session gets `CLAUDE_JOB_DIR`. Source anchor: lines 529-540.

13. **Worktree isolation.** Background sessions start in the working directory and, before editing, move into an isolated git worktree under `.claude/worktrees/` unless already in a worktree, configured otherwise, or outside git. Source anchor: lines 374-400.
