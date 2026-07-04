---
source_handle: codex-typescript-sdk
fetched: 2026-07-03
source_url: https://github.com/openai/codex/tree/main/sdk/typescript
provenance: source-direct
---

# Per-source attestation: codex-typescript-sdk

## Structural metadata

- Source kind: TypeScript SDK README and source files in the OpenAI Codex repository.
- Local fetched copies read:
  - `/tmp/codex-src/sdk/typescript/README.md`
  - `/tmp/codex-src/sdk/typescript/src/events.ts`
  - `/tmp/codex-src/sdk/typescript/src/thread.ts`
- Scope observed: TypeScript SDK runtime model, streaming events, thread resume, working-directory/env/config controls, event union and thread methods.

## Paraphrased source summary

The TypeScript SDK embeds Codex by spawning the `codex` CLI from `@openai/codex` and exchanging JSONL events over stdin/stdout. It has a `Codex` client that starts or resumes threads and `Thread` methods that run turns or stream events. The public event union includes thread started, turn started/completed/failed, item started/updated/completed, and error events. `run()` buffers events into a final response/items/usage result; `runStreamed()` exposes an async event generator.

## Key passages with source-internal anchors

1. The README says the SDK wraps and spawns the CLI and exchanges JSONL over stdio. Anchor: `README.md` line 5.

> The TypeScript SDK wraps the `codex` CLI from `@openai/codex`. It spawns the CLI and exchanges JSONL events over stdin/stdout.

2. `run()` is presented as repeated turn execution on a thread. Anchors: `README.md` lines 20-28.

> const codex = new Codex();
> const thread = codex.startThread();
> const turn = await thread.run("Diagnose the test failure and propose a fix");
> ...
> Call `run()` repeatedly on the same `Thread` instance to continue that conversation.

3. `runStreamed()` returns structured events for intermediate tool calls, streaming responses, and file change notifications. Anchors: `README.md` lines 36-47.

> `run()` buffers events until the turn finishes. To react to intermediate progress—tool calls, streaming responses, and file change notifications—use `runStreamed()` instead, which returns an async generator of structured events.

4. Threads are persisted in `~/.codex/sessions`, and `resumeThread()` reconstructs a lost thread object. Anchors: `README.md` lines 100-106.

> Threads are persisted in `~/.codex/sessions`. If you lose the in-memory `Thread` object, reconstruct it with `resumeThread()` and keep going.

5. The SDK supports working directory controls and optional git repo check skipping. Anchors: `README.md` lines 110-117.

> Codex runs in the current working directory by default. To avoid unrecoverable errors, Codex requires the working directory to be a Git repository. You can skip the Git repository check by passing the `skipGitRepoCheck` option when creating a thread.

6. The SDK allows host environment control and config overrides. Anchors: `README.md` lines 119-145.

> By default, the Codex CLI inherits the Node.js process environment. Provide the optional `env` parameter when instantiating the `Codex` client to fully control which variables the CLI receives—useful for sandboxed hosts like Electron apps.
> ...
> Use the `config` option to provide additional Codex CLI configuration overrides.

7. The TypeScript event union defines thread started, turn started, turn completed, turn failed, item started/updated/completed, and error events. Anchors: `src/events.ts` lines 5-66.

> export type ThreadStartedEvent = { type: "thread.started"; thread_id: string; };
> export type TurnStartedEvent = { type: "turn.started"; };
> export type TurnCompletedEvent = { type: "turn.completed"; usage: Usage; };
> export type TurnFailedEvent = { type: "turn.failed"; error: ThreadError; };
> export type ItemStartedEvent = { type: "item.started"; item: ThreadItem; };
> export type ItemUpdatedEvent = { type: "item.updated"; item: ThreadItem; };
> export type ItemCompletedEvent = { type: "item.completed"; item: ThreadItem; };
> export type ThreadErrorEvent = { type: "error"; message: string; };

8. `Thread.runStreamed` calls the exec adapter with input, threadId, images, model, sandbox mode, working directory, reasoning effort, abort signal, web-search/network/access/approval options, and additional directories; it updates thread id when `thread.started` arrives. Anchors: `src/thread.ts` lines 53-95.

> async runStreamed(input: Input, turnOptions: TurnOptions = {}): Promise<StreamedTurn> { ... }
> ...
> const generator = this._exec.run({
>   input: prompt,
>   ...
>   threadId: this._id,
>   images,
>   model: options?.model,
>   sandboxMode: options?.sandboxMode,
>   workingDirectory: options?.workingDirectory,
>   ...
>   signal: turnOptions.signal,
>   networkAccessEnabled: options?.networkAccessEnabled,
>   webSearchMode: options?.webSearchMode,
>   webSearchEnabled: options?.webSearchEnabled,
>   approvalPolicy: options?.approvalPolicy,
>   additionalDirectories: options?.additionalDirectories,
> });
> ...
> if (parsed.type === "thread.started") {
>   this._id = parsed.thread_id;
> }

9. `Thread.run` collects `item.completed` items, derives final response from `agent_message`, collects usage from `turn.completed`, and throws on `turn.failed`. Anchors: `src/thread.ts` lines 101-123.

> if (event.type === "item.completed") {
>   if (event.item.type === "agent_message") {
>     finalResponse = event.item.text;
>   }
>   items.push(event.item);
> } else if (event.type === "turn.completed") {
>   usage = event.usage;
> } else if (event.type === "turn.failed") {
>   turnFailure = event.error;
>   break;
> }
