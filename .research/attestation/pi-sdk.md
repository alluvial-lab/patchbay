---
source_handle: pi-sdk
fetched: 2026-08-08
source_path: /home/agent/.local/lib/node_modules/@earendil-works/pi-coding-agent/docs/sdk.md
provenance: source-direct
---

## Summary
Pi's SDK separates an AgentSession from AgentSessionRuntime. AgentSession owns one active logical session and its messages/events; AgentSessionRuntime owns replacing the active session and rebuilding cwd-bound runtime state. Replacement includes new session, switching, forking, cloning, and import. Consumers must re-subscribe after replacement and re-bind extensions.

## Key passages
- “Session replacement APIs such as new-session, resume, fork, and import live on AgentSessionRuntime, not on AgentSession.” (createAgentSession and AgentSessionRuntime)
- Runtime replacement rebuilds cwd-bound runtime state and changes `runtime.session`; event subscriptions attach to a specific AgentSession and must be re-established after replacement. (AgentSessionRuntime)
- `createAgentSession()` can use `SessionManager.continueRecent(cwd)` or `SessionManager.open(path)`; `SessionManager` provides persistent or in-memory storage. (Session Management)
- `AgentSession` exposes `sessionFile`, `sessionId`, messages, streaming state, and subscription; runtime replacement methods can throw, leaving error handling to the caller. (AgentSession and AgentSessionRuntime)
