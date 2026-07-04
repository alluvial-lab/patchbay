---
source_handle: cursor-agents-window-subagents
fetched: 2026-07-03
source_url: https://cursor.com/docs/agent/agents-window and https://cursor.com/docs/subagents
provenance: source-direct
---

# Per-source attestation: Cursor Agents Window and Subagents

## Structural metadata

- Publisher/site: Cursor Documentation.
- Fetched page titles: "Agents Window | Cursor Docs" and "Subagents | Cursor Docs".
- Canonical URLs observed in fetched page metadata: `https://cursor.com/docs/agent/agents-window` and `https://cursor.com/docs/subagents`.
- Agents Window description observed in metadata: "Use Cursor's agent-first workspace to manage agents across repos, worktrees, cloud, and remote environments."
- Subagents description observed in metadata: "Create specialized AI subagents for task-specific workflows and context management."

## Paraphrased summary

The Agents Window page documents an agent-first GUI workspace for managing multiple agents across environments. The Subagents page documents delegated agent instances created by Cursor Agent, including foreground/background modes, automatic and explicit invocation, parallel execution, cloud subagents, and custom subagent files.

## Key passages

1. The Agents Window introduction states: "The Agents Window is Cursor's agent-first interface. It provides a unified workspace to build with agents across repos and environments, including local, cloud, remote SSH, and more. It combines the power of parallel agents with the depth and control of a development environment."

2. The Agents Window page states: "You can switch back to the editor anytime, or have both open simultaneously."

3. Under "Features Available Only in the Agents Window", the page lists "Parallel agents" and says users can "run many parallel agents in the cloud (and work with them from your phone, web, Slack, GitHub, and Linear)."

4. The same section lists "Easier handoff between local and cloud" and says users can "quickly move an agent from cloud to local to iterate quickly, and move it back to the cloud so it keeps working on its own."

5. The same section lists "Cloud subagents" and says users can hand off a task to a cloud subagent with `/in-cloud`, or `/babysit` a PR, so long-running work runs on its own VM and branch while local work continues.

6. The same section lists "Worktrees" and says users can "run agents in isolated Git checkouts so each task has its own files and changes."

7. The Subagents introduction states: "Subagents are specialized AI assistants that Cursor's agent can delegate tasks to. Each subagent operates in its own context window, handles specific types of work, and returns its result to the parent agent."

8. Under "Foreground vs background", the Subagents page states that foreground subagents block until completion and background subagents return immediately and work independently.

9. Under "How subagents work", the page states that Agent can launch a subagent automatically, the subagent receives a prompt with necessary context, works autonomously, and returns a final message.

10. Under "Explicit invocation", the page states that a user can request a specific subagent with `/name` or natural language.

11. Under "Parallel execution", the page states: "Launch multiple subagents concurrently for maximum throughput" and "Agent sends multiple Task tool calls in a single message, so subagents run simultaneously."

12. Under "Cloud subagents", the page states that from a local agent session a user can hand off work to a cloud subagent that runs on its own VM and branch, leaving the local workspace clean and responsive.
