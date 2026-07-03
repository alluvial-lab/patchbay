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

This feature sharpens patchbay's positioning thesis — what patchbay *should be* — and derives the complete operator↔harness↔agent action inventory as the completeness proof that patchbay can be the primary control surface (no bridge machine, no direct-harness fallback). It is a foundation-sharpening + inventory item: the positioning gets concretized, and the inventory becomes the normative action layer the protocol contracts derive from.

## The gap, precisely

1. **No action inventory.** PROTOCOL enumerates states exhaustively but never the actions that produce them. `docs/UX.md` §Send intent hand-waves past the enumeration with one overloaded bullet ("a prompt, command, approval, cancel, or other adapter-supported action"). The contract feature (Q4) was trying to derive a registry from the wrong end of the chain.

2. **Topology hedge contradicts the value prop.** `docs/ARCHITECTURE.md` hedges: "v0 may colocate them on one host for simplicity," with split deployment a reserved seam. The operator pain this project exists to solve (agents on a VM, must keep workstation up as bridge) is *not addressed* by colocation — it requires the core to be a reachable fixed point that neither operator machines nor agent machines are load-bearing for.

3. **"Command" is overloaded three ways**, stacking a patchbay definition on incumbent usage: (a) harness slash-commands (`/agile-workflow:review`) — payload the agent interprets, not a patchbay concept; (b) patchbay `Command` — "operator intent that may cause external action, requires a grant, has a `CommandState` lifecycle"; (c) the conflation of "lifecycle-bearing drive action" with "umbrella for all operator requests." remote_pi's actual wire protocol avoids the word entirely (it uses `user_message`, `approve_tool`, `cancel`, `session_sync`...).

4. **Remote provisioning is an unserved action class.** None of Pi, Claude Code, or remote_pi lets an operator *remotely spawn a new agent/harness instance* without direct machine access. remote_pi's `session_new` only resets an already-attached session's conversation (`ctx.newSession()`); it does not spawn a process. remote_pi's `pi-supervisord` (systemd/launchd-managed process spawner) is gated behind `/remote-pi install` as out-of-band sysadmin, explicitly excluded from the setup wizard. So "start a fresh agent on the VM from my phone" is impossible in all three systems — it forces a fallback to direct machine access, which is exactly the bridge-machine problem patchbay exists to dissolve.

## Strategic decisions

- **D1 — Topology principle (commitment).** The coordination core is a **network-reachable fixed point**; operator surfaces and agent/harness machines are *both reconnecting clients* of it. Neither side is load-bearing for the other — the operator's machine can be off while agents run, and an agent machine can be off while the operator controls others. V0 colocating-on-one-host remains a *deployment convenience*, not the architecture. This replaces the current "may colocate for simplicity" hedge with the explicit reachability principle. (Grounded in the operator's actual pain: the workstation must stay up as a bridge only because the harness attachment is terminal-coupled; a reachable fixed-point core dissolves that coupling on both sides.)

- **D2 — Harness grounding depth: Pi + 2–3 harnesses for triangulation.** The inventory is grounded in Pi (via remote_pi's already-discovered action surface) plus Claude Code (`/remote-control`) and a Codex-class desktop harness, to see what's common (drive/interrupt/approve/observe) vs. Pi-specific. This grounds the "harness-agnostic control" thesis rather than asserting it. A Pi-only inventory risks Pi-shaped assumptions leaking into the adapter-neutral core (the failure `.agents/rules/` warns against); a broad agent-CLI survey over-surveys before the action model is framed.

- **D3 — Provision/retire is a first-class action class.** "Bring a new agent/harness instance into existence on a target machine, or retire one, without direct machine access" is named as a distinct action class in the inventory — it operates on the **fleet**, not on a session, and requires an agent-side supervisor capability. Whether v0 *implements* it or defers it to a reserved seam is a design-pass decision (the extension-pressure test decides); the point is that the architecture must not silently assume "agent instances always already exist," which the current adapter-attaches-to-existing-harness framing does.

- **D4 — Normative, not reference.** The inventory *becomes* the action registry that `feature-protocol-idl-and-conformance`'s `.proto` derives from. The contract feature inherits the registry rather than re-deriving it; its Q4 ("command kinds") dissolves once this lands. This is the logical-prior relationship: the action layer precedes the contract layer, and deriving contracts from an absent registry is what caused the overload muddle.

## Design input — vocabulary (resolved in conversation; design inherits)

These three were surfaced and resolved during scoping; the design pass treats them as locked-in inputs:

- **Rename `Command` → `Intent`: deferred** to avoid re-opening the done seed formal models (`command_lifecycle.qnt`, the checked `CommandDurability` property) mid-arc. For v0, **glossary-carve** instead: keep `Command` as patchbay's term but explicitly distinguish it from harness slash-commands in `docs/GLOSSARY.md`. The rename can be a separate decision once the inventory lands and we see whether `Intent` is the right word in context.
- **"Prompt" is payload content, not a protocol type.** The text the operator sends to drive the agent is the *payload* of a drive-action, not a message type. No `Prompt` message, no GLOSSARY entry beyond "the content carried by a drive-action." "Prompt" is operator slang for content; it should not be promoted to a protocol noun.
- **`Message` dropped for v0.** PROTOCOL's `Message` ("informational, no-grant, replyable") is structurally justified but has no v0 exercise — if the only v0 operator action is driving the agent (content as a drive-action) and receiving replies, replies always correlate to a drive-action and the separate message id space is unused. Dropping it for v0 simplifies the `TypedCorrelation` checked property to one target space; it returns if/when a real "send informational replyable content without a grant" need appears. (Note: this amends a checked formal-model property — `reply_correlation.qnt`'s `TypedCorrelation` — and must be applied as a model edit when the design lands. Flagged for the design pass.)

## Scope

- **Positioning sharpening (VISION + ARCHITECTURE roll-forward at implementation):** elevate "machine-independent durable operator presence" + "harness-agnostic control" + "core as reachable fixed point; operators and agents both reconnecting clients" from implied to the central thesis.
- **Action inventory:** enumerate the full operator↔harness↔agent action set, grounded in Pi + 2–3 harnesses (D2). Classify each action by structural shape:
  - **Drive** — lifecycle-bearing operator intent (carries prompt content; has `CommandState`/lifecycle; the terminal-race models prove its semantics).
  - **Request** — lifecycle-acting operator action that acts *on* a drive-action's lifecycle (interrupt, approve) — a terminal candidate or a gate decision, not itself a drive-action.
  - **Query** — read/observe (sync, refresh, list models) — no durable lifecycle.
  - **Result** — agent→operator output (replies, events) — correlates back, not an operator action.
  - **Payload** — content the agent/harness interprets (slash-commands, prompt text) — not a patchbay action type.
  - **Provision** — fleet-level action (spawn/retire agent instances on a target machine); structurally distinct from session-level actions (D3).
- **Completeness proof:** demonstrate that the core+adapter can carry every operator action so no direct-harness fallback (and thus no bridge machine) is ever needed. Any action the core can't carry forces a fallback to direct harness attachment, reproducing the machine-coupling patchbay exists to dissolve.
- **Vocabulary application:** apply the glossary-carve (Command vs slash-command), prompt-as-payload, and Message-drop to PROTOCOL/UX/SPEC/GLOSSARY as part of the foundation roll-forward.

## Acceptance criteria

- `docs/VISION.md` and `docs/ARCHITECTURE.md` state the reachability principle: the core is a network-reachable fixed point; operator surfaces and agent/harness machines are both reconnecting clients; neither side is load-bearing for the other. The v0 colocate-on-one-host convenience is labeled as deployment convenience, not architecture.
- A foundation doc (or `docs/PROTOCOL.md` section) enumerates the operator↔harness↔agent action inventory, classified by structural shape (drive / request / query / result / payload / provision), grounded in Pi + 2–3 harnesses.
- The provision/retire action class is named and classified (fleet-level, not session-level), with its v0-commit vs reserved-seam disposition decided and documented (extension-pressure test).
- `docs/GLOSSARY.md` distinguishes patchbay `Command` from harness slash-commands.
- `docs/PROTOCOL.md` and `docs/VERIFICATION.md` reflect prompt-as-payload and Message-dropped-for-v0 (with the `TypedCorrelation` model amendment applied).
- `feature-protocol-idl-and-conformance`'s registry (Q4) derives from this inventory rather than inventing it bottom-up.

## Workflow note (operator-requested sequencing)

The operator has explicitly requested this item run: **design → adversarial review of the design → implement (edit foundation docs) → deep adversarial review of the result.** The design pass should produce a reviewable design body; an adversarial cross-model review gate runs before implementation proceeds to editing VISION/ARCHITECTURE/PROTOCOL/VERIFICATION/GLOSSARY; a deep review validates the rolled-forward foundation. This is stricter than the default feature-design → implement → review path, befitting a foundation item.

## Relationships

- **`feature-protocol-idl-and-conformance`** (drafting) depends on this: its `.proto` registry derives from this inventory (D4). Its Q4 ("command kinds") dissolves once the action classification lands. The contract feature stays at `drafting` until this lands.
- **`feature-pi-parity-checklist`** (drafting) benefits from this: Pi parity becomes "does the Pi adapter cover these actions," against an already-defined action set, not a re-derivation.
- **`feature-ux-v0-acceptance`** (drafting) benefits: the UX flows derive from the action inventory rather than the overloaded §Send intent bullet.
- **`feature-extension-seams-non-foreclosure`** (drafting, coordination input — not a hard `depends_on` since it's still drafting): the provision action class's v0-vs-seam disposition uses this feature's extension-pressure classification discipline.

## Extension pressure test

- Classify each action as committed v0 behavior, reserved extension seam, or explicitly rejected direction. The provision class is the primary candidate for reserved-seam (it requires an agent-side supervisor capability and a privileged process-spawning trust boundary). Avoid encoding v0 assumptions (e.g., "agent instances always already exist") as permanent architecture.

## Parked related ideas

- `idea-harvest-remote-pi-extension-as-adapter` — the Pi adapter can harvest remote_pi's Pi-facing session/transcript know-how.
- `idea-harvest-remote-pi-app-design` — the app's session-state model and transcript projection seam inform the UX action surface.
- `idea-agent-to-agent-mesh-seam` — the local agent mesh (deferred, separate seam; depends on extension-seams classification).
