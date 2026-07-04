---
source_handle: claude-code-remote-control
fetched: 2026-07-03
source_url: https://code.claude.com/docs/en/remote-control.md
provenance: source-direct
---

# Attestation: Claude Code Remote Control

## Paraphrased summary

Remote Control connects claude.ai/code or Claude mobile apps to a Claude Code session running locally. It can be started from CLI server mode, from an interactive session, from an existing session via slash command, or from VS Code. It supports multi-device messaging, session sync, remote session spawning modes, trusted-device enforcement, and push notifications. The local process remains necessary, and some commands are local-only.

## Key passages

1. **Definition and locality.** Remote Control connects claude.ai/code or Claude mobile apps to a Claude Code session running on the user’s machine; the session keeps running locally, using the local filesystem, MCP servers, tools, and project configuration, while web/mobile are windows into the local session. Source anchor: lines 13-21.

2. **Start modes.** Remote Control can start as `claude remote-control` server mode, as an interactive session using `claude --remote-control`/`--rc`, from an existing session using `/remote-control`/`/rc`, or from VS Code. Source anchor: lines 38-112.

3. **Server-mode spawn/capacity flags.** Server mode has `--spawn <mode>` values: `same-dir`, `worktree`, and `session`; `--capacity <N>` sets maximum concurrent sessions; `--create-session-in-dir` pre-creates a session in the current directory. Source anchor: lines 56-62.

4. **Interactive remote control.** `claude --remote-control` starts a full interactive terminal session that can also be controlled from claude.ai or the Claude app, and `/remote-control` in an existing session carries over current conversation history. Source anchor: lines 67-96.

5. **Sync across devices.** The overview says the conversation stays in sync across terminal, browser, and phone and messages can be sent from any connected surface. Source anchor: lines 17-18.

6. **Connection status and URLs.** Interactive terminal sessions show an `/rc active` footer indicator with session URL/QR code; VS Code shows a banner and posts the URL in conversation. Source anchor: lines 102-120.

7. **Auto-enable and instance count.** Remote Control only activates when explicitly run unless auto-connect is enabled in config; with auto-connect, each interactive Claude Code process registers one remote session, while server mode supports multiple concurrent sessions from one process. Source anchor: lines 143-147.

8. **Transport/security.** The local session makes outbound HTTPS requests only, registers with the Anthropic API, and polls for work; the server routes messages between web/mobile and local session over streaming, with TLS and short-lived credentials. Source anchor: lines 149-153.

9. **Trusted devices.** Trusted Devices is an org-wide setting requiring members to verify a device before viewing or steering Remote Control sessions; interaction requires an enrolled device and recent sign-in. Source anchor: lines 155-168.

10. **Push notifications.** When Remote Control is active, Claude can send mobile push notifications; `/config` enables proactive notifications when Claude decides and actions-required notifications for permission prompts and questions. Source anchor: lines 216-246.

11. **Limitations.** Outside server mode, each Claude Code instance supports one remote session; the local process must keep running; extended network outage can time out; and some commands are local-only. Remote-capable commands include text-output commands such as `/compact`, `/clear`, `/context`, `/usage`, `/exit`, `/usage-credits`, `/recap`, `/reload-plugins`, and `/mcp` summary/reconnect/enable/disable subcommands. Source anchor: lines 254-262.

12. **Remote Control vs web/cloud.** Remote Control runs on the local machine with local MCP/tools/project config; Claude Code on the web runs in Anthropic-managed cloud infrastructure. Source anchor: lines 210-212.
