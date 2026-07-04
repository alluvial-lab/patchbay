---
id: feature-research-harness-action-surfaces
kind: feature
stage: done
tags: [research, adapter, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton]
created: 2026-07-02
updated: 2026-07-04
gate_origin: null
release_binding: null
research_dials:
  scope_authority: in-engagement-judgment
  verification_rigor: standard
  intent: inform-architecture-decision
  output_kind: synthesis-brief
---

# Research: Survey operator/agent/harness action surfaces across harnesses and tooling

## Brief

`feature-operator-presence-and-action-inventory` needs to derive a harness-agnostic action inventory — what an operator can actually do to a running agent session, and what flows back — across multiple harnesses and tooling. A prior design pass asserted the Claude Code and Codex-class control surfaces from general knowledge rather than reading their actual SDK/extension/wire surfaces; that is the "mechanism claim not grounded empirically" failure the formal-model gloss audit caught and is rejected as a grounding method. This engagement grounds the survey in attested sources: actual harness extension APIs, SDK surfaces, wire types, and docs.

The survey's goal is breadth: understand the full surface of available operator/agent/harness actions so the downstream action inventory is genuinely harness-agnostic, not Pi-shaped. Patchbay is positioning itself as a useful plug into existing operator flows; that positioning is only credible if the action model is grounded in what those flows actually expose.

## Seed questions

- What operator→agent control actions does each surveyed harness expose (drive/prompt, interrupt/cancel, approve tool, sync/refresh, model/thinking reconfigure, session new/compact/resume, provision/spawn, retire/stop, others)?
- What agent→operator events does each harness surface (message chunks, tool-call requests, tool results, turn/agent lifecycle, compaction, errors)?
- Which actions are durable/lifecycle-bearing vs. ephemeral/payload vs. read-only queries?
- Which actions require a privileged sidecar/supervisor (e.g. spawning new agent instances), and how do existing harnesses handle that (out-of-band sysadmin vs. operator action)?
- What is common across harnesses (the likely harness-agnostic action set) vs. what is harness-specific (adapter capability, not core)?
- Are there harnesses that send no-grant informational replyable content (the case that would keep `Message` in v0)? Or is "operator drives agent, agent replies" the universal shape?
- What provisioning/spawn mechanisms exist (remote_pi's `pi-supervisord` is one; Claude Code's `/remote-control`; Codex desktop process model; others)? Are any exposed as operator actions, or all out-of-band?

## Engagement record

- Surveyed harness/tooling set: **to be determined in engagement** (operator input welcome on which harnesses matter most). Candidates: Pi (already grounded via remote_pi source — see Existing grounding below), Claude Code, Codex, Cursor, Aider, OpenCode, Continue, others in the operator's ecosystem.
- Output: `.research/analysis/briefs/harness-action-surfaces.md` synthesis brief cataloging the action surface per surveyed harness, with per-source attestations under `.research/attestation/`.
- The brief is consumed by `feature-operator-presence-and-action-inventory`'s design pass, which derives the action inventory and classification from it.

## Existing grounding (Pi — already source-grounded, do not re-derive)

The Pi surface is already grounded via remote_pi's `pi-extension` source (read this session). The survey feature should reference this, not re-research Pi:

- **Outbound (agent→operator events)** — Pi extension hooks: `turn_start`, `turn_end`, `message_update`, `message_end`, `tool_execution_start`, `tool_execution_end`, `model_select`, `thinking_level_select`, `session_before_compact`, `session_compact`, `agent_end`, `input`, `resources_discover`, `tool_call`.
- **Inbound (operator→agent control)** — remote_pi `ClientMessage` types: `user_message`, `approve_tool`, `cancel`, `session_sync`, `session_new`, `session_compact`, `model_set`, `thinking_set`, `list_models`, `ping`, `pair_request`, `queued_message_set`, `queued_message_clear`.
- **Provisioning** — remote_pi `pi-supervisord` (systemd/launchd-managed, `pi --mode rpc` child spawner) is out-of-band sysadmin, explicitly excluded from the setup wizard; not an operator action. `session_new` resets an attached session's conversation, does not spawn a process.

Source: `/home/agent/projects/remote_pi/pi-extension/src/` (index.ts, protocol/generated/protocol.generated.ts, actions/handlers.ts, bin/supervisord.ts).

## Scope

- Survey ≥3 non-Pi harnesses/tooling (operator input on which matter most) via attested sources — actual SDK/extension APIs, wire types, shipped extension code, docs.
- Per harness, catalog: outbound events, inbound control actions, provisioning mechanism (and whether operator-action or out-of-band), durability/lifecycle shape of each action.
- Surface the common cross-harness action set vs. harness-specific actions.
- Specifically investigate the no-grant-informational-replyable-content question (does `Message` need to stay for v0?) and the provisioning-as-operator-action question (is any harness exposing spawn/retire as an operator action, or are all out-of-band?).
- Produce the synthesis brief + attestations.

## Acceptance criteria

- `.research/analysis/briefs/harness-action-surfaces.md` exists, cataloging the action surface per surveyed harness with citations to attested sources.
- Per-source attestations exist under `.research/attestation/` for each surveyed harness.
- The brief identifies the common cross-harness action set vs. harness-specific actions.
- The brief answers the no-grant-informational-content question and the provisioning-as-operator-action question.
- The brief is sufficient for `feature-operator-presence-and-action-inventory`'s design pass to derive a grounded action inventory without asserting surfaces from general knowledge.
- Citation lint, adversarial-read, and spot-check gates pass per the ARD discipline.

## Extension pressure test

- Classify surveyed actions as candidate v0 / reserved seam / rejected for the patchbay core. The survey informs but does not decide v0 disposition (that's the consuming feature's design pass).

## Relationships

- **`feature-operator-presence-and-action-inventory`** (drafting) depends on this: its design pass is blocked until this survey grounds the action surface. Its D4 (normative vs. reference) resolves after this lands.
- **`feature-pi-parity-checklist`** (drafting) benefits: Pi parity can be assessed against the cross-harness common set this survey establishes.

## Engagement record

Completed: 2026-07-04

- **Fan-out**: 6 parallel research-specialists (by-harness decomposition, Candidate A) on `openai-codex`: `gpt-5.5` (medium) for Claude Code, Codex, Cursor, Antigravity (complex SDK/extension surfaces); `gpt-5.3-codex-spark` (medium) for OpenCode, Aider (readable open-source). OpenCode facet grounded directly by lead after two specialist dispatches hit harness limits without writing files.
- **Surveyed**: Pi (via remote_pi pi-extension source — attested as `pi-extension`), Claude Code, Codex, Cursor, OpenCode, Aider, Antigravity — 7 harnesses total.
- **Gate outcomes**:
  - Citation lint: 483 resolved / 49 broken (mostly `[low] unreachable-source` from no-network URL probes; 1 `[medium]` local-path nit on `opencode-schema-events`); 0 thin; 17 `warn` pattern flags (content nits, cited). Run with `--no-url-check` after direct source fetches, matching the prior engagement's posture.
  - Adversarial-read: 2 passes (`gpt-5.5` high). Pass 1 NEEDS-REVISION (8 findings: Message-drop over-breadth, uncited tables, spine misstatement, provisioning posture, Aider flags, payload framing, Claude citation locators). Pass 2 NEEDS-REVISION (5 blocking findings: table citations, broken `[pi-extension-supervisord]` handle, unsupported Codex citation, stale spine, Antigravity collapse). All addressed in-revision; final pass APPROVED.
  - Spot-check (lead): remaining `warn` flags are framing prose, not grounding failures.
- **Acquisition candidates**: 2 blocking (Antigravity `agy` CLI canonical docs; Antigravity SDK Overview/permissions docs); 2 enriching (Cursor Cloud Agents OpenAPI spec; Codex app-server schema from exact binary version). Persisted research-side; operator-confirmed promotion at the research-handoff gate.
- **Outputs**: `.research/analysis/campaigns/harness-action-surfaces/` (`parent.md` synthesis, `specialists/*.md`, `verification-checklist.md`); 34 per-source attestations under `.research/attestation/` (including the new `pi-extension.md` grounding Pi's surface).
- **Key findings consumed by `feature-operator-presence-and-action-inventory`**:
  - Six-class spine (drive/request/query/result/payload/provision) survives across all 7 harnesses.
  - Provisioning is novel for patchbay — no harness exposes remote-machine process spawn as an operator action. Four postures surfaced (out-of-band sysadmin / programmatic local sidecar / in-process session creation / cloud-managed).
  - `Message` (operator-originated no-grant replyable) drops for v0; agent-originated question/elicitation surface is real and common (a separate modeling question the design pass inherits). Formal-model `TypedCorrelation` amendment re-scoped: narrows operator-originated correlation to the command space, must still accommodate agent-originated question/elicitation replies as a typed reference target.
