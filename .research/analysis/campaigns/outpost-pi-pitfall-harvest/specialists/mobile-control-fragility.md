---
provenance: agent-synthesis
updated: 2026-08-09
campaign: outpost-pi-pitfall-harvest
facet: mobile-control-fragility
---

# Mobile control fragility: editor-seam driving and the dedicated-operations rescope

## Finding

The mobile `/new` failure was not primarily a missing button route. In the Pi SDK, ordinary lifecycle/event contexts do not carry the session-replacement capability: the SDK defines session control as part of `ExtensionCommandContext`, whose methods are marked safe only for user-initiated commands; the ordinary event context is a separate base context. [mobile-sdk-contract]{1} [mobile-sdk-contract]{2} [mobile-sdk-contract]{3} The extension therefore had no supported host operation to call when the mobile action arrived. [mobile-newctx]{1} [mobile-newctx]{2}

## Pitfalls: what not to do

### 1. Do not drive the TUI through an editor callback as a durable control plane

The editor seam was real but narrower than its first interpretation. A retained custom editor's `onSubmit("/new")` could reach the native parser in interactive TUI mode, but this was an indirect UI callback, not a supported command-invocation API. It was TUI-only, and `sendUserMessage` was not an alternative: the SDK explicitly disables command handling for that path, so slash text is delivered as prompt content rather than a native command. [mobile-editor-spike]{1} [mobile-editor-spike]{3} [mobile-sdk-contract]{4} [mobile-sdk-contract]{5} [mobile-editor-spike]{5}

The seam also replaced the active editor with a newly constructed custom component: the SDK copies callbacks and current text, not the full editor object/state. [mobile-sdk-editor]{1} [mobile-sdk-editor]{2} The review consequently found loss of full editor state/history across reload, possible conflicts with other custom-editor extensions, stale references after replacement, and a race in which programmatic submission could clobber text that a human was composing in the TUI. [mobile-review]{1} [mobile-review]{2} [mobile-review]{3} [mobile-review]{4}

**Disconfirming analysis.** The spike demonstrated a successful `/new` probe, so an editor seam can be a useful diagnostic or tightly pinned experimental bridge. That success does not establish mode coverage, non-interference, lifecycle stability, or a durable SDK contract. The later review explicitly limits any interim use to a curated, version-pinned, TUI-only experiment. [mobile-editor-spike]{2} [mobile-review]{7}

### 2. Do not infer completion from incidental lifecycle events

The proposed editor design attempted to turn `void` submission into success/error by observing lifecycle events. The review found no robust completion correlation across command kinds (no uniform acknowledgement signal). [mobile-review]{5}

**Disconfirming analysis.** A fresh `session_start` can be useful evidence for a session-replacement operation, but it cannot be treated as a universal acknowledgement for arbitrary commands without an operation-specific correlation contract. This is an `{inferred}` Patchbay protocol requirement from the observed failure mode, not a claim that every operation must synchronously complete.

### 3. Do not route every mobile `/` prefix into a generic command channel

The proposed composer routing had a dangerous fallback: Pi's prompt path did not reject unknown commands; an unknown slash-prefixed string could become a model prompt. This also left literal slash messages and attachment behavior ambiguous. [mobile-review]{6}

**Disconfirming analysis.** A picker or curated action can constrain the vocabulary, but a free-text picker still needs explicit validation and an unsupported-operation result. A slash-looking string must not silently change from user content to control input.

### 4. Do not retire the existing typed session operation merely to make the UI look generic

The rescope retained `session_new` as a dedicated typed operation rather than replacing it with arbitrary slash invocation; the architectural gap was in the SDK capability available to the handler, not the wire operation itself. [mobile-ops]{2} [mobile-ops]{3}

## Architectural gap

The missing seam was a host-owned, mode-capable operation gateway. `{inferred}` The extension API exposed no public method to obtain a command context, invoke an extension command without an LLM turn, or replace the active session runtime. [mobile-newctx]{3} The SDK restricts session-control methods to command contexts ("safe only for user-initiated commands"), while base-context operations such as compaction are available from ordinary extension contexts; the deadlock-avoidance motive is not source-attested. [mobile-ops]{2} [mobile-ops]{3]

This creates two distinct control paths that must not be conflated. `{inferred}`

- **Control via a TUI seam:** incidental, UI-owned, mode-limited, vulnerable to editor state and SDK implementation changes.
- **Control via Operations:** host-owned, typed per capability, explicit about supported modes, process ownership, acknowledgement, and session identity.

The second path is the converged model. `{inferred}` The cockpit already routes pure built-ins such as `/new` and `/compact` through dedicated methods/RPC rather than a generic slash-command injector. [mobile-ops]{1} [mobile-ops]{4} [mobile-ops]{5} [mobile-ops]{6}

## Seam decisions for Patchbay

1. **Operations over keystrokes.** Patchbay's mobile surface should emit typed Operations, not synthetic TUI input. A Pi adapter may expose an operation only after it proves that the underlying SDK/process mode supports it. `{inferred}`
2. **Curated vocabulary over generic slash passthrough.** Treat native session control, safe context operations, and adapter-owned operations as separate registered capabilities. Unknown or unsupported requests return a structured `unsupported`/`validation_failed` result; they must not fall through to an agent prompt. `{inferred}`
3. **Process ownership is part of capability.** `/new` via restart-fresh is valid only when a daemon supervisor or restart wrapper owns the process. The implementation acknowledges, resets projection state, exits with the shared fresh-session code, and lets the owner relaunch once without `--continue`. [mobile-fresh]{1} [mobile-fresh]{3} [mobile-fresh]{5} [mobile-fresh]{7}
4. **Safe failure beats unsafe recycle.** Unmanaged interactive agents return `fresh_session_restart_unavailable` and do not exit. The eleven Herdr-managed agents were intentionally not migrated in this slice, so their limitation remains an explicit capability boundary rather than a hidden kill-and-resume risk. [mobile-fresh]{2} [mobile-fresh]{4}
5. **Separate dispatch acknowledgement from completion.** An `action_ok` for a process recycle means the request was accepted/handed to the owner; successor session identity and `session_start reason=new` are separate lifecycle evidence. `{inferred}`
6. **Do not make a mobile control surface a closed mirror of the TUI.** The failed command-picker plan surfaced — `{inferred}` — that extension command discovery does not imply native TUI command enumeration. Patchbay should expose adapter-declared operations, not promise a complete slash catalog. `{inferred}`

## Contradictions

| Source handles | Relationship | Side-by-side |
|---|---|---|
| `mobile-editor-spike` and `mobile-review` | Correction: capability probe versus durability assessment | The spike found a TUI-only `onSubmit` path that reached `/new`; the review found it was not a transparent, stable, correlated injection contract. [mobile-editor-spike]{2} [mobile-review]{2} [mobile-review]{5} |
| `mobile-review` and `mobile-fresh` | Reversal: proposed upstream/editor bridge versus shipped bounded path | The review recommended an upstream host-operation API, with only a curated experiment pending that API; the implementation instead delivered `/new` through a process-manager-owned restart handshake and refused unmanaged exits. [mobile-review]{7} [mobile-fresh]{1} [mobile-fresh]{2} |
| `mobile-ops` and initial editor-seam plan | Reversal: generic command invocation versus dedicated operations | The initial plan treated arbitrary slash commands and two mobile entry modes as the target; the rescope removed editor-seam stories and retained typed operations, with arbitrary unknown commands out of scope. [mobile-ops]{1} [mobile-ops]{3} |

## Disconfirming analysis

A dedicated-operation model does not eliminate all fragility: restart-fresh depends on a real process owner, and the current wrapper migration does not cover every managed agent. It also does not supply a general API for future native or extension commands. The evidence supports a narrower claim: it avoids making the TUI editor seam the architectural control boundary, and it makes unsupported modes explicit instead of silently injecting text or killing an unmanaged process. [mobile-fresh]{2} [mobile-fresh]{4}

## Revisit if

- Pi exposes a documented host-operation or submit-input API with mode support and correlated results; reassess whether any generic operation can be promoted from reserved to committed.
- All Patchbay adapter modes have a process owner capable of exactly-once fresh-session recycle; expand the `session_new` capability rather than reintroducing editor driving.
- A future surface needs extension commands; first verify each handler's required context and declare per-command capabilities instead of assuming the native slash catalog is callable.
- SDK upgrades change editor construction, callback wiring, or context lifecycles; treat any editor bridge as an experimental seam requiring a pinned compatibility test, never as the default mobile path.

## Source handles

- `mobile-newctx` → `.research/attestation/mobile-newctx.md` (outpost_pi `304b8b8`, `.work/backlog/backlog-mobile-new-button-newsession-no-command-ctx.md`)
- `mobile-editor-spike` → `.research/attestation/mobile-editor-spike.md` (outpost_pi `7373273`, `.work/backlog/backlog-mobile-new-button-newsession-no-command-ctx.md`)
- `mobile-review` → `.research/attestation/mobile-review.md` (outpost_pi `7ceb72f`, `.work/active/features/feature-mobile-slash-command-invocation.md`)
- `mobile-ops` → `.research/attestation/mobile-ops.md` (outpost_pi `32dd3a6`, `.work/active/features/feature-mobile-slash-command-invocation.md`; `cockpit/lib/app/cockpit/ui/widgets/agent_composer.dart`)
- `mobile-fresh` → `.research/attestation/mobile-fresh.md` (outpost_pi `ea6b5fd`, `pi-extension/src/index.ts`; `scripts/pi-restart-loop.sh`)
- `mobile-sdk-contract` → `.research/attestation/mobile-sdk-contract.md` (installed SDK 0.80.6, `pi-extension/node_modules/@earendil-works/pi-coding-agent/dist/core/{extensions/types.d.ts,extensions/runner.js,agent-session.js}`)
- `mobile-sdk-editor` → `.research/attestation/mobile-sdk-editor.md` (installed SDK 0.80.6, `pi-extension/node_modules/@earendil-works/pi-coding-agent/dist/modes/interactive/interactive-mode.js`)
