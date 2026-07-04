---
source_handle: claude-code-slash-commands
fetched: 2026-07-03
source_url: https://code.claude.com/docs/en/agent-sdk/slash-commands.md
provenance: source-direct
---

# Attestation: Claude Agent SDK slash commands

## Paraphrased summary

The SDK slash-command guide documents slash commands as prompt strings that control Claude Code sessions when they are dispatchable without an interactive terminal. The system init message lists available commands. The guide documents `/compact`, `/clear`, and custom commands/skills as SDK-usable prompt surfaces.

## Key passages

1. **Slash command dispatchability.** Slash commands control Claude Code sessions through prompts beginning with `/`; only commands that work without an interactive terminal are dispatchable through the SDK, and the `system/init` message lists available commands. Source anchor: lines 7-13.

2. **Sending slash commands.** Slash commands are sent by including them in the prompt string like regular text; commands acting on history, such as `/compact`, need prior messages and should be sent as a follow-up to the same conversation. Source anchor: lines 48-122.

3. **`/compact`.** `/compact` reduces conversation history by summarizing older messages; when compaction runs, a `compact_boundary` system message reports the result, and when there is not enough to compact the result text reports that without a boundary message. Source anchor: lines 127-207.

4. **`/clear`.** `/clear` resets conversation context so subsequent prompts start without prior conversation history; the previous conversation remains on disk and can be returned to via `resume`. Source anchor: lines 210-217.

5. **Custom commands.** Custom slash commands can be defined in `.claude/commands/` or, preferably, `.claude/skills/<name>/SKILL.md`; once defined they are available through the SDK and appear in `slash_commands`. Source anchor: lines 220-333.
