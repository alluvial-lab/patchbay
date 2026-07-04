---
source_handle: codex-python-sdk-api
fetched: 2026-07-03
source_url: https://github.com/openai/codex/blob/main/sdk/python/docs/api-reference.md
provenance: source-direct
---

# Per-source attestation: codex-python-sdk-api

## Structural metadata

- Source kind: Python SDK API reference in the OpenAI Codex repository.
- Local fetched copy read at: `/tmp/codex-src/sdk/python/docs/api-reference.md`.
- Scope observed: `Codex`/`AsyncCodex` public methods, thread methods, turn controls, streaming, sandbox presets, return fields, auth handles.

## Paraphrased source summary

The Python SDK provides sync and async clients for Codex workflows. The client can manage authentication/account state, start/list/resume/fork/archive/unarchive threads, list models, and create thread objects. Thread objects can run a turn, create a lower-level turn handle, read thread state, set a name, and compact. Turn handles expose `steer`, `interrupt`, `stream`, and `run`. Turns accept per-turn parameters including approval mode, cwd, effort, model, output schema, personality, sandbox, service tier, and summary. Results include final response, items, status, errors, timestamps, and token usage.

## Key passages with source-internal anchors

1. The API reference states the public surface and beta status. Anchors: lines 1-7.

> # OpenAI Codex Python SDK (Beta) - API Reference
>
> Public surface of `openai_codex` for Codex workflows.
>
> This SDK is in beta. Public APIs may change before `1.0`. Turn streams are routed by turn ID so one client can consume multiple active turns concurrently.

2. `Codex` sync methods include auth/account methods, thread lifecycle/list/read-like entrypoints, archive/unarchive, and model list. Anchors: lines 61-74.

> - `metadata -> InitializeResponse`
> - `close() -> None`
> - `login_api_key(api_key: str) -> None`
> - `login_chatgpt() -> ChatgptLoginHandle`
> - `login_chatgpt_device_code() -> DeviceCodeLoginHandle`
> - `account(*, refresh_token: bool = False) -> GetAccountResponse`
> - `logout() -> None`
> - `thread_start(...) -> Thread`
> - `thread_list(...) -> ThreadListResponse`
> - `thread_resume(thread_id: str, ...) -> Thread`
> - `thread_fork(thread_id: str, ...) -> Thread`
> - `thread_archive(thread_id: str) -> ThreadArchiveResponse`
> - `thread_unarchive(thread_id: str) -> Thread`
> - `models(*, include_hidden: bool = False) -> ModelListResponse`

3. Async methods mirror sync methods. Anchors: lines 101-114.

> - `metadata -> InitializeResponse`
> - `close() -> Awaitable[None]`
> ...
> - `thread_start(...) -> Awaitable[AsyncThread]`
> - `thread_list(...) -> Awaitable[ThreadListResponse]`
> - `thread_resume(thread_id: str, ...) -> Awaitable[AsyncThread]`
> - `thread_fork(thread_id: str, ...) -> Awaitable[AsyncThread]`
> - `thread_archive(thread_id: str) -> Awaitable[ThreadArchiveResponse]`
> - `thread_unarchive(thread_id: str) -> Awaitable[AsyncThread]`
> - `models(*, include_hidden: bool = False) -> Awaitable[ModelListResponse]`

4. Thread methods include `run`, `turn`, `read`, `set_name`, and `compact`. Anchors: lines 153-165.

> - `run(input: str | Input, *, approval_mode=None, cwd=None, effort=None, model=None, output_schema=None, personality=None, sandbox: Sandbox | None = None, service_tier=None, summary=None) -> TurnResult`
> - `turn(input: str | Input, *, approval_mode=None, cwd=None, effort=None, model=None, output_schema=None, personality=None, sandbox: Sandbox | None = None, service_tier=None, summary=None) -> TurnHandle`
> - `read(*, include_turns: bool = False) -> ThreadReadResponse`
> - `set_name(name: str) -> ThreadSetNameResponse`
> - `compact() -> ThreadCompactStartResponse`

5. Turn results include identifiers, status, error, timestamps, final response, items, and usage; final_response can be absent. Anchors: lines 171-182.

> - `id: str`
> - `status: TurnStatus`
> - `error: TurnError | None`
> - `started_at: int | None`
> - `completed_at: int | None`
> - `duration_ms: int | None`
> - `final_response: str | None`
> - `items: list[ThreadItem]`
> - `usage: ThreadTokenUsage | None`
>
> `final_response` is `None` when the turn finishes without a final-answer or phase-less assistant message item.

6. Low-level turn control is through `turn(...)`, and `TurnHandle` exposes steer, interrupt, stream, and run. Anchors: lines 184-215.

> Use `turn(...)` when you need low-level turn control (`stream()`, `steer()`, `interrupt()`) before collecting the turn result.
>
> - `steer(input: str | Input) -> TurnSteerResponse`
> - `interrupt() -> TurnInterruptResponse`
> - `stream() -> Iterator[Notification]`
> - `run() -> TurnResult`

7. Async turn handles expose the same controls and one client can stream multiple active turns concurrently. Anchors: lines 224-232.

> - `steer(input: str | Input) -> Awaitable[TurnSteerResponse]`
> - `interrupt() -> Awaitable[TurnInterruptResponse]`
> - `stream() -> AsyncIterator[Notification]`
> - `run() -> Awaitable[TurnResult]`
>
> - `stream()` and `run()` consume only notifications for their own turn ID
> - one `AsyncCodex` instance can stream multiple active turns concurrently

8. Sandbox presets distinguish read-only, workspace-write, and full-access modes. Anchors: lines 192-205.

> Use `sandbox=` consistently on thread lifecycle methods and turns:
> ...
> - `Sandbox.read_only`: read files without allowing writes.
> - `Sandbox.workspace_write`: the normal default for projects with a recorded trust decision; read files and write inside the workspace and configured writable roots.
> - `Sandbox.full_access`: run without filesystem access restrictions.
