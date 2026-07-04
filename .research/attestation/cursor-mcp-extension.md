---
source_handle: cursor-mcp-extension
fetched: 2026-07-03
source_url: https://cursor.com/docs/mcp
provenance: source-direct
---

# Per-source attestation: Cursor MCP documentation

## Structural metadata

- Publisher/site: Cursor Documentation.
- Page title: "Model Context Protocol (MCP) | Cursor Docs".
- Canonical URL observed in fetched page metadata: `https://cursor.com/docs/mcp`.
- Page description observed in fetched metadata: "Connect Cursor to external tools and data sources using Model Context Protocol (MCP). Install servers, configure authentication, and integrate with databases, APIs, and third-party services."
- Internal headings observed include: "What is MCP?", "How it works", "Protocol and extension support", "Installing MCP servers", "Using the Extension API", "Enterprise admin controls", "Using MCP in chat", "Tool approval", "Run Mode", "Tool response", and "Security considerations".

## Paraphrased summary

The page documents Cursor's MCP support for connecting the agent/chat surface to external tools and data sources. It covers local command, HTTP/SSE, OAuth, configuration locations, enterprise allowlists/network controls, chat usage, default tool approval, and tool-result display.

## Key passages

1. Under "What is MCP?", the page states that MCP "enables Cursor to connect to external tools and data sources" and that servers are installed/managed from the Customize surface or `mcp.json`.

2. Under "Why use MCP?", the page states: "MCP connects Cursor to external systems and data. Instead of explaining your project structure repeatedly, integrate directly with your tools."

3. Under "How it works", the page states: "MCP servers expose capabilities through the protocol, connecting Cursor to external tools or data sources" and that Cursor supports transport methods including local command/STDIO and remote HTTP/SSE examples.

4. Under "Protocol and extension support", the page lists "Tools" as supported and describes the Apps extension as "Interactive UI views returned by MCP tools".

5. Under "Using the Extension API", the page states: "For programmatic MCP server registration, Cursor provides an extension API that allows dynamic configuration without modifying [mcp.json] files. This is particularly useful for enterprise environments and automated setup workflows." It links to an extension API reference and names `vscode.cursor.mcp.registerServer()`.

6. Under "MCP Allowlist", the page states that enterprise admins can control which MCP servers users may run from the Cursor dashboard; it says command entries approve local MCP servers by command pattern, URL entries approve remote HTTP/SSE servers by URL pattern, and tool allowlists restrict which tools from an approved server can run automatically.

7. Under "Using MCP in chat", the page states: "Cursor automatically uses MCP tools listed under Available Tools when relevant. This includes Plan Mode. Ask for a specific tool by name or describe what you need. Enable or disable MCP servers from [Customize] in the sidebar."

8. Under "Tool approval", the page states: "Cursor asks for approval before using MCP tools by default. Click the arrow next to the tool name to see arguments."

9. Under "Run Mode", the page states: "MCP follows the same Run Modes as terminal commands. For example, in Auto-review mode, allowlisted MCP tools run immediately and everything else is routed through the classifier."

10. Under "Tool response", the page states: "Cursor shows the response in chat with expandable views of arguments and responses."

11. Under "Images as context", the page states that MCP servers can return images and that "Cursor attaches returned images to the chat. If the model supports images, it analyzes them."

12. Under "Security considerations", the page warns: "Remember that MCP servers can access external services and execute code on your behalf. Always understand what a server does before installation."
