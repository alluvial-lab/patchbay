---
source_handle: cursor-cloud-agents-api
fetched: 2026-07-03
source_url: https://cursor.com/docs/cloud-agent/api/endpoints
provenance: source-direct
---

# Per-source attestation: Cursor Cloud Agents API

## Structural metadata

- Publisher/site: Cursor Documentation.
- Page title: "Cloud Agents API | Cursor Docs".
- Canonical URL observed in fetched page metadata: `https://cursor.com/docs/cloud-agent/api/endpoints`.
- Page description observed in fetched metadata: "Create and manage Cursor Cloud Agents programmatically with the run-based REST API."
- Internal headings/endpoints observed include: "Create An Agent", "List Agents", "Get An Agent", "Create A Run", "List Runs", "Get A Run", "Stream A Run", "Cancel A Run", "Archive An Agent", "Unarchive An Agent", "Delete An Agent Permanently", and worker-token/private-worker sections.

## Paraphrased summary

The page documents a public beta v1 REST API for Cloud Agents. It separates durable agent resources from per-prompt runs. It supports creating, listing, reading, archiving, unarchiving, and deleting agents; creating/listing/reading/cancelling runs; and streaming run events over SSE.

## Key passages

1. Near the top, the page states: "The Cloud Agents API v1 is in public beta. APIs may change before general availability."

2. The page states: "The Cloud Agents API lets you programmatically launch and manage cloud agents that work on your repositories."

3. The migration note states: "This API splits work into a durable agent plus per-prompt runs, replacing the flatter v0 surface."

4. Under "Create An Agent", the page documents `POST /v1/agents` and states: "Create a Cloud Agent and immediately enqueue its initial run. The response returns both the durable [agent] and the initial run." The request body includes `prompt`, with `prompt.text` described as "The instruction text for the agent."

5. Under "Get An Agent", the page states: "Retrieve durable metadata for an agent. Execution status lives on runs — fetch latestRunId and call Get A Run to read run state."

6. Under "Create A Run", the page states: "Send a follow-up prompt to an existing active agent. The new run uses the agent's current conversation and workspace state." It also states: "Only one run can be active per agent" and that calling while another run is `CREATING` or `RUNNING` returns `409 agent_busy`; the user should wait or cancel it.

7. Under "Get A Run", the page states: "Retrieve status, timestamps, and (for terminal runs) the final result, duration, and pushed branches for a specific run." It describes the `result` field as "Final assistant reply text for a terminated run."

8. Under "Stream A Run", the page states: "Stream Server-Sent Events (SSE) for one run. The stream is scoped to the requested run and does not replay prior runs." It lists event types including status, assistant, tool_call, interaction_update, error, done, and heartbeat/terminal events in the surrounding text.

9. Under tool-call stream payloads, the page states that tool_call events use a stable envelope with `callId`, `name`, `status`, optional `args`, optional `result`, and optional `truncated` flags.

10. Under "Resuming a stream", the page states that most events include an id line and that reconnects can use `Last-Event-ID`; it says the event id must belong to the requested run or the request returns `400 invalid_last_event_id`.

11. Under "Cancel A Run", the page states: "Cancel the active run for an agent. Cancellation is terminal — the run transitions to [CANCELLED] and cannot be resumed. To continue the conversation, create a new run on the same agent." It also states that non-cancellable runs return `409 run_not_cancellable`.

12. Under "Archive An Agent", the page states: "Archive an agent. Archived agents remain readable but cannot accept new runs until unarchived. Use this for reversible 'soft delete' flows."

13. Under "Delete An Agent Permanently", the page states: "Permanently delete an agent. This action is irreversible. Use Archive An Agent for reversible removal."

14. In request/response examples, the page shows model configuration in agent creation (`model.id`, `params`), repository starting refs, MCP server configuration, and `envVars` described as session-scoped encrypted variables injected into the agent shell and deleted with the agent.

15. Near the authentication/API overview area, the page states: "View the full OpenAPI specification for detailed schemas and examples" and links `/docs-static/cloud-agents-openapi.yaml`.
