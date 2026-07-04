---
source_handle: cursor-extension-api
fetched: 2026-07-03
source_url: https://cursor.com/docs/extension-api
provenance: source-direct
---

# Per-source attestation: Cursor Extension API reference

## Structural metadata

- Publisher/site: Cursor Documentation.
- Page title: "Extension API reference | Cursor Docs".
- Canonical URL observed in fetched page metadata: `https://cursor.com/docs/extension-api`.
- Page description observed in fetched metadata: "Register MCP servers and plugin paths programmatically using Cursor's extension API. Build custom integrations and automate configuration for enterprise workflows."
- Internal headings observed include: "Type definitions", "MCP servers", `vscode.cursor.mcp.registerServer`, `vscode.cursor.mcp.unregisterServer`, "Plugin paths", `vscode.cursor.plugins.registerPath`, and `vscode.cursor.plugins.unregisterPath`.

## Paraphrased summary

The page documents a narrow extension API exposed under `vscode.cursor`. The API is for VS Code extensions running in Cursor to register/unregister MCP servers and plugin paths programmatically. It does not document a general operator-to-agent control API for messages, approvals, cancellation, or session management.

## Key passages

1. The introduction states: "Cursor exposes extension APIs under `vscode.cursor` for programmatic configuration. Use these APIs from VS Code extensions to register MCP servers and plugin paths without editing config files."

2. Under "MCP servers", the page states: "Register and manage MCP servers at runtime. This is useful for enterprise environments, onboarding tools, and automated setup workflows where editing `mcp.json` isn't practical."

3. Under `vscode.cursor.mcp.registerServer`, the page states: "Registers an MCP server" and gives the signature `vscode.cursor.mcp.registerServer(config: ExtMCPServerConfig): void`.

4. Under `vscode.cursor.mcp.unregisterServer`, the page states: "Unregisters a previously registered MCP server" and gives the signature `vscode.cursor.mcp.unregisterServer(serverName: string): void`.

5. The MCP examples include registering a remote server with `url` and `headers`, registering a local server with `command`, `args`, and `env`, and unregistering a server by name.

6. Under "Plugin paths", the page states: "Register additional plugin directories at runtime. Extensions can use this API to tell Cursor about plugin locations without requiring users to manually copy files to `~/.cursor/plugins/local/`."

7. Under `vscode.cursor.plugins.registerPath`, the page states: "Registers a directory path as a plugin source. Cursor loads any valid plugins found in the directory" and gives the signature `vscode.cursor.plugins.registerPath(path: string): void`.

8. Under `vscode.cursor.plugins.unregisterPath`, the page states: "Removes a previously registered plugin path" and gives the signature `vscode.cursor.plugins.unregisterPath(path: string): void`.
