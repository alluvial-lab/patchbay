---
id: feature-operator-presence-and-action-inventory
kind: feature
stage: drafting
tags: [foundation, protocol, adapter]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton, feature-command-state-ssot]
created: 2026-07-02
updated: 2026-07-03
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

4. **Remote provisioning is an unserved action class.** None of Pi, Claude Code, or remote_pi lets an operator *remotely spawn a new agent/harness instance* without direct machine access. remote_pi's `session_new` only resets an already-attached session's conversation (`ctx.newSession()`); it does not spawn a process. remote_pi's `pi-supervisord` (systemd/launchd-managed process spawner) is gated behind `/remote-pi install` as out-of-band sysadmin, explicitly excluded from the setup wizard. So "start a fresh agent on the VM from my phone" is impossible in all three systems — it forces a fallback to direct machine access, which is exactly the bridge-machine problem patchbay exists to dissolve.

## Strategic decisions

- **D1 — Topology principle (commitment).** The coordination core is a **network-reachable fixed point**; operator surfaces and agent/harness machines are *both reconnecting clients* of it. Neither side is load-bearing for the other — the operator's machine can be off while agents run, and an agent machine can be off while the operator controls others. V0 colocating-on-one-host remains a *deployment convenience*, not the architecture. This replaces the current "may colocate for simplicity" hedge with the explicit reachability principle. (Grounded in the operator's actual pain: the workstation must stay up as a bridge only because the harness attachment is terminal-coupled; a reachable fixed-point core dissolves that coupling on both sides.)

- **D2 — Harness/tooling grounding: WIDE survey (research engagement).** The inventory is grounded in a **wide survey of operator/agent/harness action surfaces** across multiple harnesses and tooling — not Pi + 2–3 for triangulation. The goal is to understand the full surface of available operator/agent/harness actions so the inventory is genuinely harness-agnostic, not Pi-shaped. This must be a **grounded research engagement with attested sources** (reading actual harness SDK/extension surfaces, docs, and wire types — the way remote_pi's `pi-extension` source grounded the Pi surface), not asserted from familiarity. The prior design pass asserted Claude Code and Codex-class control surfaces from general knowledge; that is the exact "mechanism claim not grounded empirically" failure the formal-model gloss audit caught and is rejected. The survey net is wide: Pi, Claude Code, Codex-class, and other agent CLIs/harnesses/tooling in the operator's ecosystem (Aider, OpenCode, Continue, Cursor, etc.) — the research engagement scopes the final set. See Research prerequisite below.

- **D3 — Provision/retire is a first-class action class.** "Bring a new agent/harness instance into existence on a target machine, or retire one, without direct machine access" is named as a distinct action class in the inventory — it operates on the **fleet**, not on a session, and requires an agent-side supervisor capability. Whether v0 *implements* it or defers it to a reserved seam is a design-pass decision (the extension-pressure test decides); the point is that the architecture must not silently assume "agent instances always already exist," which the current adapter-attaches-to-existing-harness framing does.

- **D4 — Normative vs. reference: UNDECIDED (open).** Whether the inventory *becomes* the normative action registry that `feature-protocol-idl-and-conformance`'s `.proto` derives from (normative — the contract feature inherits it; its Q4 dissolves) or is a grounding reference that *informs* the contract feature's registry decision (reference — the contract feature keeps wire-type authority) is **undecided**. This will be resolved **after** the research grounds the inventory — a normative commitment on an ungrounded inventory would reproduce the muddle this feature exists to fix. The decision depends on how clean and stable the surveyed action surface turns out to be.

## Design input — vocabulary (surfaced in conversation; design inherits, but survey may revise)

These three were surfaced and provisionally resolved during scoping; the design pass treats them as starting inputs, but the wide harness survey (D2) may revise them — e.g. if the survey finds harnesses that genuinely send no-grant informational replyable content, `Message` may need to stay for v0:

- **Rename `Command` → `Intent`: deferred** to avoid re-opening the done seed formal models (`command_lifecycle.qnt`, the checked `CommandDurability` property) mid-arc. For v0, **glossary-carve** instead: keep `Command` as patchbay's term but explicitly distinguish it from harness slash-commands in `docs/GLOSSARY.md`. The rename can be a separate decision once the inventory lands.
- **"Prompt" is payload content, not a protocol type.** The text the operator sends to drive the agent is the *payload* of a drive-action, not a message type. No `Prompt` message.
- **`Message` dropped for v0 (provisional).** PROTOCOL's `Message` ("informational, no-grant, replyable") is structurally justified but has no v0 exercise. Dropping it for v0 simplifies the `TypedCorrelation` checked property to one target space; it returns if/when a real need appears. (Note: this amends a checked formal-model property — `reply_correlation.qnt`'s `TypedCorrelation` — and must be applied as a model edit when the design lands. Flagged for the design pass; the survey informs whether this is safe.)

## Scope

- **Positioning sharpening (VISION + ARCHITECTURE roll-forward at implementation):** elevate "machine-independent durable operator presence" + "harness-agnostic control" + "core as reachable fixed point; operators and agents both reconnecting clients" from implied to the central thesis.
- **Action inventory:** enumerate the full operator↔harness↔agent action set, grounded in a wide harness/tooling survey (D2). Classify each action by structural shape. A prior design pass sketched a six-class spine (drive / request / query / result / payload / provision) derived from remote_pi's Pi surface; this sketch is a starting point, not a locked classification — the wide survey determines whether it survives, merges, or expands.
- **Completeness proof:** demonstrate that the core+adapter can carry every operator action so no direct-harness fallback (and thus no bridge machine) is ever needed.
- **Vocabulary application:** apply glossary-carve, prompt-as-payload, and Message-drop to PROTOCOL/UX/SPEC/GLOSSARY as part of the foundation roll-forward (subject to survey revision per Design input above).

## Acceptance criteria

- `docs/VISION.md` and `docs/ARCHITECTURE.md` state the reachability principle: the core is a network-reachable fixed point; operator surfaces and agent/harness machines are both reconnecting clients; neither side is load-bearing for the other. The v0 colocate-on-one-host convenience is labeled as deployment convenience, not architecture.
- A foundation doc (or `docs/PROTOCOL.md` section) enumerates the operator↔harness↔agent action inventory, classified by structural shape, grounded in the wide harness/tooling survey (not asserted).
- The provision/retire action class is named and classified (fleet-level, not session-level), with its v0-commit vs reserved-seam disposition decided and documented (extension-pressure test).
- `docs/GLOSSARY.md` distinguishes patchbay `Command` from harness slash-commands.
- Vocabulary decisions (prompt-as-payload, Message-drop) are applied to PROTOCOL/VERIFICATION consistent with survey findings.
- D4 (normative vs. reference) is resolved and documented; `feature-protocol-idl-and-conformance`'s relationship to this inventory is stated accordingly.

## Research prerequisite (blocks the design pass)

The design pass — and especially the inventory tables and the D4 resolution — **cannot proceed until the wide harness/tooling survey (D2) is grounded.** The prior design pass (commit `f23b377`, reverted) asserted Claude Code and Codex-class control surfaces from general knowledge rather than reading their actual SDK/extension/wire surfaces. That is rejected as a grounding method.

The survey should be a grounded research engagement with attested sources (reading actual harness extension APIs, SDK surfaces, docs, and wire types — the method that grounded the Pi surface via remote_pi's `pi-extension` source). It produces a `.research/` brief cataloging the operator/agent/harness action surface per surveyed harness, which this feature's design then derives the inventory from.

**Open structural question (for the operator):** should the wide harness/tooling survey be a **separate `[research]` feature** (routing through the agentic-research research-orchestrator, with `research_dials:`, producing a `.research/` brief — matching the repo's four existing research features) that this feature adds to `depends_on`, or folded into this feature as a research sub-engagement? The repo pattern favors separate; the operator has not yet decided.

## Workflow note (operator-requested sequencing)

The operator has explicitly requested this item run: **design → adversarial review of the design → implement (edit foundation docs) → deep adversarial review of the result.** The design pass should produce a reviewable design body; an adversarial cross-model review gate runs before implementation proceeds to editing VISION/ARCHITECTURE/PROTOCOL/VERIFICATION/GLOSSARY; a deep review validates the rolled-forward foundation. This is stricter than the default feature-design → implement → review path, befitting a foundation item.

## Relationships

- **`feature-protocol-idl-and-conformance`** (drafting): its relationship to this inventory depends on D4 (normative = depends on this; reference = informed by this). Either way the contract feature's Q4 ("command kinds") is reshaped by this inventory.
- **`feature-pi-parity-checklist`** (drafting) benefits: Pi parity becomes "does the Pi adapter cover these actions," against an already-defined action set.
- **`feature-ux-v0-acceptance`** (drafting) benefits: the UX flows derive from the action inventory.
- **`feature-extension-seams-non-foreclosure`** (drafting, coordination input — not a hard `depends_on`): the provision action class's v0-vs-seam disposition uses this feature's extension-pressure classification discipline.

## Extension pressure test

- Classify each action as committed v0 behavior, reserved extension seam, or explicitly rejected direction. The provision class is the primary candidate for reserved-seam. Avoid encoding v0 assumptions (e.g., "agent instances always already exist") as permanent architecture.

## Parked related ideas

- `idea-harvest-remote-pi-extension-as-adapter` — the Pi adapter can harvest remote_pi's Pi-facing session/transcript know-how.
- `idea-harvest-remote-pi-app-design` — the app's session-state model and transcript projection seam inform the UX action surface.
- `idea-agent-to-agent-mesh-seam` — the local agent mesh (deferred, separate seam; depends on extension-seams classification).
