---
source_handle: cursor-run-modes
fetched: 2026-07-03
source_url: https://cursor.com/docs/agent/security/run-modes
provenance: source-direct
---

# Per-source attestation: Cursor Run Modes

## Structural metadata

- Publisher/site: Cursor Documentation.
- Page title: "Run Modes | Cursor Docs".
- Canonical URL observed in fetched page metadata: `https://cursor.com/docs/agent/security/run-modes`.
- Page description observed in fetched metadata: "Choose how Cursor Agent runs shell, MCP, and Fetch calls with Auto-review, allowlists, sandboxing, and enterprise controls."
- Internal headings observed: "Pick a mode", "How Auto-review works", "Configuring Auto-review", "Sandboxing", "Environment variables", "Network access", "Other protections", "Team controls", "Changelog".

## Paraphrased summary

The page describes Cursor's local-agent approval and execution policy. Run Modes determine when the local agent may run tool calls without asking, when shell commands are sandboxed, and when a classifier or human approval path is used. It distinguishes local agents from Cloud Agents, which do not use these Run Modes.

## Key passages

1. The introduction states: "Run Modes control how the Cursor agent runs tool calls, and when Cursor interrupts you for approval."

2. The page states: "Use them to decide how much autonomy the agent gets for shell commands, MCP tools, and Fetch calls. The safest useful setup for most people is Auto-review. It runs known-safe calls, sandboxes shell commands when it can, and asks a classifier to review anything else."

3. Under "Pick a mode", the page says the desktop application location is "Settings > Agents > Approvals & Execution".

4. In the mode table, the Auto-review row states: "Allowlisted calls run immediately. Other shell commands run in the sandbox when possible. Calls that do not use the sandbox go to the Auto-review classifier."

5. Under "How Auto-review works", the page states: "Auto-review applies to shell, MCP, and Fetch tool calls. Cursor checks each call in this order" and the diagram alt text says the classifier "can allow the call, ask the agent to take a different approach, or ask you to approve."

6. Under sandboxing, the page states: "A shell command 'can run in the sandbox' when it works under the sandbox's file and network limits. Commands that need full system access, like writes outside the workspace or privileged operations, can't be sandboxed, so they go to the classifier instead."

7. The Cloud Agents callout states: "Run Modes apply to local agents. Cloud Agents run inside their own dedicated machine, so the agent never asks you to approve an action."

8. Under team controls, the page states: "Admins can override which modes are available for their users, as well as configure the sandbox networking rules for terminal commands, and more. All of these settings are available in the web dashboard."
