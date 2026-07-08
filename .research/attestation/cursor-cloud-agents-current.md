---
source_handle: cursor-cloud-agents-current
fetched: 2026-07-07
source_url: https://cursor.com/docs/cloud-agent/api/endpoints
provenance: source-direct
---

# Attestation: Cursor Cloud Agents API

## Structural metadata

- Publisher/site: Cursor documentation.
- Page title observed in fetched page metadata/content: Cloud Agents API endpoints.
- Source kind: REST/SSE API documentation for Cursor Cloud Agents.

## Paraphrased summary

Cursor's Cloud Agents API is a public-beta REST API for launching and managing cloud agents against repositories. It separates durable agent resources from per-prompt runs, supports agent lifecycle operations, run lifecycle operations, run streaming over SSE, cancellation, archiving, unarchiving, deletion, and API authentication.

## Key passages

1. **Beta status.** The page states that Cloud Agents API v1 is in public beta and may change before general availability. Source anchor: page top/API overview.

2. **Purpose.** The page says the API lets clients programmatically launch and manage cloud agents that work on repositories. Source anchor: API overview.

3. **Durable agent plus runs.** The migration note says the API splits work into a durable agent plus per-prompt runs, replacing the flatter v0 surface. Source anchor: migration note.

4. **Create agent plus initial run.** `POST /v1/agents` creates a Cloud Agent and immediately enqueues its initial run; the response returns both the durable agent and initial run. Source anchor: Create An Agent section.

5. **Durable metadata vs execution status.** `Get An Agent` retrieves durable metadata and says execution status lives on runs, with `latestRunId` used to fetch run state. Source anchor: Get An Agent section.

6. **Follow-up runs and concurrency.** `Create A Run` sends a follow-up prompt to an existing active agent using current conversation/workspace state; only one run can be active per agent, and attempts during `CREATING`/`RUNNING` return `409 agent_busy`. Source anchor: Create A Run section.

7. **Run status and terminal result.** `Get A Run` retrieves status, timestamps, and for terminal runs final result, duration, and pushed branches; the `result` field is final assistant reply text for a terminated run. Source anchor: Get A Run section.

8. **Streaming.** `Stream A Run` streams SSE for one run and does not replay prior runs; event types include status, assistant, tool_call, interaction_update, error, done, and heartbeat/terminal events. Source anchor: Stream A Run section.

9. **SSE resume scope.** Reconnects can use `Last-Event-ID`; the event ID must belong to the requested run or the request returns `400 invalid_last_event_id`. Source anchor: Resuming a stream subsection.

10. **Cancellation.** `Cancel A Run` cancels the active run; cancellation is terminal, the run transitions to `CANCELLED`, cannot be resumed, and continuation requires creating a new run on the same agent. Source anchor: Cancel A Run section.

11. **Archive/delete.** `Archive An Agent` is a reversible soft delete that prevents new runs until unarchived; permanent delete is irreversible. Source anchor: Archive/Unarchive/Delete sections.

12. **Agent creation configuration.** Request/response examples include model configuration, repository starting refs, MCP server configuration, and encrypted session-scoped environment variables that are deleted with the agent. Source anchor: Create An Agent examples.
