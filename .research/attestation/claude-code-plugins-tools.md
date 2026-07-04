---
source_handle: claude-code-plugins-tools
fetched: 2026-07-03
source_url: https://code.claude.com/docs/en/agent-sdk/plugins.md ; https://code.claude.com/docs/en/agent-sdk/custom-tools
provenance: source-direct
---

# Attestation: Claude Agent SDK plugins and custom tools

## Paraphrased summary

The plugin documentation says the Agent SDK can load local plugins that add skills, agents, hooks, MCP servers, and commands. The custom-tools page says applications can define in-process MCP servers with tools, pass them to `query`, control availability and permissions through `tools`, `allowedTools`, and `disallowedTools`, and inspect tool-use blocks/results in messages.

## Key passages

1. **Plugin capabilities.** Plugins extend Claude Code through the Agent SDK with skills, agents, hooks, and MCP servers; legacy `commands/` directories are still supported but skills are preferred for new plugins. Source anchor: plugins page lines 7-24.

2. **Plugin loading.** SDK plugins load by providing local filesystem paths in options; `type` must be `local`, and plugins from marketplaces or remote repositories must be downloaded first. Source anchor: plugins page lines 26-78.

3. **Plugin verification in init message.** Loaded plugins appear in the system initialization message; examples read `message.plugins`, `message.slash_commands`, and namespaced plugin skills/commands. Source anchor: plugins page lines 81-134.

4. **Plugin skills invocation.** Plugin skills are namespaced by plugin name and can be invoked directly by sending `/plugin-name:skill-name` as the prompt. Source anchor: plugins page lines 141-176.

5. **Plugin directory structure.** A plugin directory can include `.claude-plugin/plugin.json`, `skills/`, legacy `commands/`, `agents/`, `hooks/hooks.json`, and `mcp-servers/`. Source anchor: plugins page lines 272-288.

6. **Custom tool definition and server.** A custom tool is defined by name, description, input schema, and handler; after definition, it is wrapped in a server with `createSdkMcpServer` / `create_sdk_mcp_server`, and that server runs in-process inside the application, not as a separate process. Source anchor: custom tools page lines 1285-1304.

7. **Passing custom tools to query.** The MCP server is passed through the `mcpServers` option; the server key becomes the `{server_name}` segment in fully-qualified tool names like `mcp__{server_name}__{tool_name}`, which can be listed in `allowedTools` to run without a permission prompt. Source anchor: custom tools page lines 1350-1374.

8. **Tool availability vs permission.** `tools` and bare-name `disallowedTools` affect availability (whether a tool appears in context), while `allowedTools` and scoped `disallowedTools` affect permission only; removing a built-in entirely requires omitting it from `tools` or using a bare disallowed name. Source anchor: custom tools page lines 1438-1448.

9. **Tool-use observability.** The custom-tools example notes that `AssistantMessage` objects contain the tool calls Claude made and the final `ResultMessage` text can be inspected. Source anchor: custom tools page lines 1643-1643.
