---
id: feature-operator-presence-and-action-inventory
kind: feature
stage: drafting
tags: [foundation, protocol, adapter]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-command-state-ssot, feature-research-harness-action-surfaces]
created: 2026-07-02
updated: 2026-07-04
gate_origin: null
release_binding: null
---

# Feature: Sharpen operator-presence positioning and derive the operator↔harness↔agent action inventory

## Brief

Patchbay's foundation docs specify, exhaustively, the *states* an accepted action passes through (`CommandState`, `SubmissionOutcome`, session axes, failure vocabulary). They do **not** specify the *action set* — what an operator can actually do to a running agent session, and which of those actions patchbay's core must durably carry vs. which are payload the agent/harness interprets. Every downstream design (`feature-protocol-idl-and-conformance`'s `.proto` registry, `feature-pi-parity-checklist`, `feature-ux-v0-acceptance`) has been reaching for this unstated input and deriving contracts from state machines + a SPEC bullet instead.

This feature sharpens patchbay's positioning thesis — what patchbay *should be* — and derives the complete operator↔harness↔agent action inventory as the completeness proof that patchbay can be the primary control surface (no bridge machine, no direct-harness fallback). It is a foundation-sharpening + inventory item: the positioning gets concretized, and the inventory becomes the action layer the protocol contracts derive from.

## The gap, precisely

1. **No action inventory.** PROTOCOL enumerates states exhaustively but never the actions that produce them. `docs/UX.md` §Send intent hand-waves past the enumeration with one overloaded bullet ("a prompt, command, approval, cancel, or other adapter-supported action"). The contract feature (Q4) was trying to derive a registry from the wrong end of the chain.

2. **Topology hedge contradicts the value prop.** `docs/ARCHITECTURE.md` hedges: "v0 may colocate them on one host for simplicity," with split deployment a reserved seam. The operator pain this project exists to solve (agents on a VM, must keep workstation up as bridge) is *not addressed* by colocation — it requires the core to be a reachable fixed point that neither operator machines nor agent machines are load-bearing for.

3. **"Command" is overloaded three ways**, stacking a patchbay definition on incumbent usage: (a) harness slash-commands (`/agile-workflow:review`) — payload the agent interprets, not a patchbay concept; (b) patchbay `Command` — "operator intent that may cause external action, requires a grant, has a `CommandState` lifecycle"; (c) the conflation of "lifecycle-bearing drive action" with "umbrella for all operator requests." remote_pi's actual wire protocol avoids the word entirely (it uses `user_message`, `approve_tool`, `cancel`, `session_sync`...).

4. **Remote spawn has prior art but is harness-specific.** The initial framing held that remote spawn (cold-start a fresh agent instance from a remote device) was unserved by all harnesses; the reconciled survey (informed by the operator's prior `~/SNC/.research/` corpus) **corrected this**: Claude Code Remote Control `--spawn worktree --capacity N`, OpenCode `serve`, Claude Dispatch, Codex thread creation, Cursor Cloud Agents, and Antigravity managed environments all expose spawn-as-operator-action. The operator deployed the Claude Code path (`docs/ops/remote-agent-piloting.md` — a systemd unit). What remains genuinely novel for patchbay is **harness-agnostic + durable/authority-bearing spawn** — the same spawn capability across Pi, Codex, etc. (not just within one vendor's ecosystem), wrapped in grant/LSN/snapshot semantics. The original pain stands: spawn a fresh *Pi* on the VM from the phone is unserved (Claude's path works for Claude, not Pi).

## Strategic decisions

- **D1 — Topology principle (commitment).** The coordination core is a **network-reachable fixed point**; operator surfaces and agent/harness machines are *both reconnecting clients* of it. Neither side is load-bearing for the other — the operator's machine can be off while agents run, and an agent machine can be off while the operator controls others. V0 colocating-on-one-host remains a *deployment convenience*, not the architecture. This replaces the current "may colocate for simplicity" hedge with the explicit reachability principle. (Grounded in the operator's actual pain: the workstation must stay up as a bridge only because the harness attachment is terminal-coupled; a reachable fixed-point core dissolves that coupling on both sides.)

- **D2 — Harness/tooling grounding: WIDE survey (research engagement).** The inventory is grounded in a **wide survey of operator/agent/harness action surfaces** across multiple harnesses and tooling — not Pi + 2–3 for triangulation. The goal is to understand the full surface of available operator/agent/harness actions so the inventory is genuinely harness-agnostic, not Pi-shaped. This must be a **grounded research engagement with attested sources** (reading actual harness SDK/extension surfaces, docs, and wire types — the way remote_pi's `pi-extension` source grounded the Pi surface), not asserted from familiarity. The prior design pass asserted Claude Code and Codex-class control surfaces from general knowledge; that is the exact "mechanism claim not grounded empirically" failure the formal-model gloss audit caught and is rejected. The survey net is wide: Pi, Claude Code, Codex-class, and other agent CLIs/harnesses/tooling in the operator's ecosystem (Aider, OpenCode, Continue, Cursor, etc.) — the research engagement scopes the final set. See Research prerequisite below.

- **D3 — Spawn/attach is a first-class action class.** "Bring a new agent/harness instance into existence (spawn), or connect a control surface to a session that exists (attach)" are named as distinct action classes in the inventory — spawn operates on the **fleet** (creating/retiring agent instances); attach operates on the **connection** (joining an existing session, reconciling state). The survey corrected an initial misreading: spawn **has prior art** across harnesses (Claude Code Remote Control `--spawn`, OpenCode `serve`, Codex threads, Cursor Cloud Agents, Antigravity managed, Claude Dispatch — the operator deployed the Claude Code path per `~/SNC/docs/ops/remote-agent-piloting.md`). Patchbay's novelty is **harness-agnostic + durable/authority-bearing spawn** (grant-checked, LSN-tracked, snapshot-recoverable spawn across harnesses), not the spawn primitive itself. Whether v0 *implements* spawn or defers it to a reserved seam is a design-pass decision; attach is a v0 commitment (you can't operate without it).

- **D4 — Normative vs. reference: UNDECIDED (open).** Whether the inventory *becomes* the normative action registry that `feature-protocol-idl-and-conformance`'s `.proto` derives from (normative — the contract feature inherits it; its Q4 dissolves) or is a grounding reference that *informs* the contract feature's registry decision (reference — the contract feature keeps wire-type authority) is **undecided**. This will be resolved **after** the research grounds the inventory — a normative commitment on an ungrounded inventory would reproduce the muddle this feature exists to fix. The decision depends on how clean and stable the surveyed action surface turns out to be.

## Design input — vocabulary (surfaced in conversation; design inherits, but survey may revise)

These three were surfaced and provisionally resolved during scoping; the design pass treats them as starting inputs, but the wide harness survey (D2) revised them (see Research prerequisite below for the corrected findings):

- **Rename `Command` → `Intent`: deferred** to avoid re-opening the done seed formal models (`command_lifecycle.qnt`, the checked `CommandDurability` property) mid-arc. For v0, **glossary-carve** instead: keep `Command` as patchbay's term but explicitly distinguish it from harness slash-commands in `docs/GLOSSARY.md`. The rename can be a separate decision once the inventory lands.
- **"Prompt" is payload content, not a protocol type.** The text the operator sends to drive the agent is the *payload* of a drive-action, not a message type. No `Prompt` message. **Survey confirmed.**
- **`Message` dropped for v0 (provisional, survey-refined).** The survey split this into two questions: (Q-A) no harness exposes a generic *operator-originated* no-grant `Message` command (drops for v0 — "operator drives, agent replies" is universal on the operator side); (Q-B) several harnesses expose *agent-originated* question/elicitation reply paths (Claude `AskUserQuestion`, Codex tool user-input requests, OpenCode `question.asked`, Antigravity `ASK_QUESTION`) — real and common, a separate modeling question the design inherits (likely a Request variant). The `TypedCorrelation` formal-model amendment narrows operator-originated correlation to the command space but must still accommodate agent-originated question/elicitation replies as a typed reference target.
- **Action spine: spawn/attach/operate/receive/payload (survey-refined).** The initial six-class sketch (drive/request/query/result/payload/provision) is revised to five primitives: **spawn** (provision, renamed per the operator's spawn-vs-pilot framing), **attach** (connect to an existing session — split out from drive, which silently assumed attachment; grounded in OpenCode serve→client-attach, Claude connect-from-mobile, Pi pair+sync), **operate** (the cluster of drive/request/query within an attached session), **receive** (agent→operator output, renamed from result), **payload** (content carried by operate).

## Scope

- **Positioning sharpening (VISION + ARCHITECTURE roll-forward at implementation):** elevate "machine-independent durable operator presence" + "harness-agnostic control" + "core as reachable fixed point; operators and agents both reconnecting clients" from implied to the central thesis.
- **Action inventory:** enumerate the full operator↔harness↔agent action set, grounded in the wide harness/tooling survey (now done — `.research/analysis/campaigns/harness-action-surfaces/parent.md`). Classify each action by structural shape using the survey-confirmed five-primitive spine: **spawn / attach / operate / receive / payload** (operate sub-clusters into drive/request/query).
- **Completeness proof:** demonstrate that the core+adapter can carry every operator action so no direct-harness fallback (and thus no bridge machine) is ever needed.
- **Vocabulary application:** apply glossary-carve, prompt-as-payload, and Message-drop to PROTOCOL/UX/SPEC/GLOSSARY as part of the foundation roll-forward (subject to survey revision per Design input above).

## Acceptance criteria

- `docs/VISION.md` and `docs/ARCHITECTURE.md` state the reachability principle: the core is a network-reachable fixed point; operator surfaces and agent/harness machines are both reconnecting clients; neither side is load-bearing for the other. The v0 colocate-on-one-host convenience is labeled as deployment convenience, not architecture.
- A foundation doc (or `docs/PROTOCOL.md` section) enumerates the operator↔harness↔agent action inventory, classified by structural shape, grounded in the wide harness/tooling survey (not asserted).
- The spawn/attach action classes are named and classified (spawn = fleet-level; attach = connection-level), with spawn's v0-commit vs reserved-seam disposition decided and documented (extension-pressure test).
- `docs/GLOSSARY.md` distinguishes patchbay `Command` from harness slash-commands.
- Vocabulary decisions (prompt-as-payload, Message-drop) are applied to PROTOCOL/VERIFICATION consistent with survey findings.
- D4 (normative vs. reference) is resolved and documented; `feature-protocol-idl-and-conformance`'s relationship to this inventory is stated accordingly.

## Research prerequisite (DONE — grounded via the wide survey)

The design pass is **unblocked**: the wide harness/tooling survey (D2) is grounded in attested sources across 7 harnesses (Pi + Claude Code + Codex + Cursor + OpenCode + Aider + Antigravity), reconciled with the operator's prior `~/SNC/.research/` corpus (which the initial engagement missed). Findings live at `.research/analysis/campaigns/harness-action-surfaces/parent.md` (cross-synthesis) + 34 per-source attestations + 4 cross-corpus pointer-attestations to the SNC corpus. The survey established:
- The five-primitive spine (spawn/attach/operate/receive/payload) survives across all 7 harnesses.
- Spawn has prior art (Claude `--spawn`, OpenCode `serve`, Dispatch, Codex threads, Cursor/Antigravity cloud); patchbay's novelty is harness-agnostic + durable/authority-bearing spawn.
- The Message question is split (Q-A operator-originated drops for v0; Q-B agent-originated question/elicitation is real and must be modeled).
- D4 (normative vs. reference) can now resolve from evidence: the convergence is strong enough to support normative.

## Workflow note (operator-requested sequencing)

The operator has explicitly requested this item run: **design → adversarial review of the design → implement (edit foundation docs) → deep adversarial review of the result.** The design pass should produce a reviewable design body; an adversarial cross-model review gate runs before implementation proceeds to editing VISION/ARCHITECTURE/PROTOCOL/VERIFICATION/GLOSSARY; a deep review validates the rolled-forward foundation. This is stricter than the default feature-design → implement → review path, befitting a foundation item.

## Relationships

- **`feature-protocol-idl-and-conformance`** (drafting): its relationship to this inventory depends on D4 (normative = depends on this; reference = informed by this). Either way the contract feature's Q4 ("command kinds") is reshaped by this inventory.
- **`feature-pi-parity-checklist`** (drafting) benefits: Pi parity becomes "does the Pi adapter cover these actions," against an already-defined action set.
- **`feature-ux-v0-acceptance`** (drafting) benefits: the UX flows derive from the action inventory.
- **`feature-extension-seams-non-foreclosure`** (drafting, coordination input — not a hard `depends_on`): the spawn action class's v0-vs-seam disposition uses this feature's extension-pressure classification discipline.

## Extension pressure test

- Classify each action as committed v0 behavior, reserved extension seam, or explicitly rejected direction. The spawn class is the primary candidate for reserved-seam (it requires an agent-side supervisor capability and a privileged process-spawning trust boundary). Attach is a v0 commitment. Avoid encoding v0 assumptions (e.g., "agent instances always already exist") as permanent architecture.

## Parked related ideas

- `idea-harvest-remote-pi-extension-as-adapter` — the Pi adapter can harvest remote_pi's Pi-facing session/transcript know-how.
- `idea-harvest-remote-pi-app-design` — the app's session-state model and transcript projection seam inform the UX action surface.
- `idea-agent-to-agent-mesh-seam` — the local agent mesh (deferred, separate seam; depends on extension-seams classification).
