---
id: feature-operator-presence-and-action-inventory
kind: feature
stage: implementing
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

## Architectural choice

Patchbay is the **durable operator-presence layer**: a network-reachable coordination core that carries operator intent, authority, and recoverable state across whichever harness the operator chose and whichever machines host the agents or the operator. The core is a fixed point; operator surfaces and agent/harness machines are both reconnecting clients of it. This is the thesis that dissolves the bridge-machine problem on both sides: the operator's workstation need not stay up (the core is reachable from the phone), and an agent machine need not be the core's host (it reconnects as a client).

The inventory proves this thesis is complete: if the core+adapter can carry every operator action — including remote provisioning of new agent instances — then no direct-harness fallback is ever needed, and machine-coupling cannot re-enter through a missing action. An action the core cannot carry is a fallback to direct harness attachment, which is exactly the bridge-machine coupling patchbay exists to dissolve.

## The action inventory (grounded in Pi + Claude Code + Codex-class)

The inventory below is derived from three harnesses: Pi (via remote_pi's already-discovered wire types in `pi-extension/src/protocol/generated/protocol.generated.ts`), Claude Code (`/remote-control` + slash-commands), and Codex-class desktop harnesses. The triangulation surfaces what is common (drive/interrupt/approve/observe/provision) vs. what is harness-specific (compact, model-switch). Each action is classified by structural shape; the classification spine is the resolution to the "command" overload.

### Drive — lifecycle-bearing operator intent

Operator intent that may cause external action, requires a grant, and has a durable `CommandState` lifecycle (accepted → delivered → running → terminal). The terminal-race formal models (`command_lifecycle.qnt`: `TerminalFinality`, `LsnDeterminesTerminalWinner`, `PreAppendTerminalChoice`) prove this class's semantics. Carries prompt content as payload.

| Action | Pi wire type | Claude Code analog | Codex-class analog | v0? |
|---|---|---|---|---|
| Send prompt / drive the agent | `user_message` | send message in `/remote-control` | send message | committed v0 |

### Request — lifecycle-acting operator action

Operator action that acts *on* a drive-action's lifecycle — a terminal candidate competing for the drive-action's terminal commit, or a gate decision unblocking a stalled drive-action. Not itself a drive-action (no prompt payload, no separate drive lifecycle). Has its own durable record because it races with completion and must be arbitrated by LSN.

| Action | Pi wire type | Claude Code analog | Codex-class analog | v0? |
|---|---|---|---|---|
| Interrupt / cancel a running turn | `cancel` | Esc in `/remote-control` | cancel button | committed v0 |
| Approve a pending tool call | `approve_tool` | approve in tool-gate UI | approve in tool-gate UI | committed v0 |

### Query — read / observe (no durable lifecycle)

Operator or system request for state with no durable `CommandState` — the response is authoritative-snapshot-backed, not lifecycle-tracked. Read-only; does not race with anything.

| Action | Pi wire type | Claude Code analog | Codex-class analog | v0? |
|---|---|---|---|---|
| Sync / refresh session state | `session_sync` | reconnect snapshot | reconnect snapshot | committed v0 |
| List available models | `list_models` | `/model` list | model picker open | committed v0 |

### Result — agent→operator output (correlates back; not an operator action)

Agent output that correlates back to a prior drive-action or request by typed reference. Not an operator action at all — it is what the operator receives. Owned by the Reply/Event contracts; the `TypedCorrelation` property proves replies can't forge correlation across id spaces.

| Result | Pi wire type | v0? |
|---|---|---|
| Agent message chunk / final message | `agent_chunk`, `agent_message`, `agent_done` | committed v0 |
| Tool call request (for approval) | `tool_request` | committed v0 |
| Tool result | `tool_result` | committed v0 |
| Compaction event | `compaction` | committed v0 (informational) |

### Payload — content the agent/harness interprets (not a patchbay action type)

Content carried *inside* a drive-action that the agent or harness interprets. Not a patchbay protocol type — patchbay carries it, doesn't interpret it. This is where slash-commands live: `/agile-workflow:review` is content the agent's harness parses, not a patchbay action.

| Payload | What it is | v0? |
|---|---|---|
| Prompt text | the content of a drive-action | committed v0 (as payload) |
| Slash-command text | harness-interpreted content carried by a drive-action | committed v0 (as payload) |

### Provision — fleet-level action (structurally distinct; v0-vs-seam TBD)

Operator action that operates on the **fleet**, not on a session: bring a new agent/harness instance into existence on a target machine, or retire one, without direct machine access. Requires an agent-side supervisor capability (remote_pi's `pi-supervisord` is the closest existing analog, but it is out-of-band sysadmin, not an operator action). None of Pi, Claude Code, or remote_pi currently expose this as an operator action.

| Action | Closest existing analog | v0 disposition |
|---|---|---|
| Spawn a new agent instance on a target machine | remote_pi `pi-supervisord` (out-of-band) | **reserved seam** (see Pre-mortem) |
| Retire / stop an agent instance | remote_pi `daemon stop` (out-of-band) | **reserved seam** |

## Vocabulary application

The three decisions captured as design input apply as follows:

- **Glossary-carve `Command` (v0):** `docs/GLOSSARY.md` gets an entry distinguishing patchbay `Command` (the lifecycle-bearing drive-action class above) from harness slash-commands (payload the agent interprets). No rename in this feature; `Command` stays as patchbay's term to avoid re-opening `command_lifecycle.qnt` / `CommandDurability`. The rename `Command`→`Intent` is filed as a separate future decision.
- **Prompt = payload content:** `docs/PROTOCOL.md` and `docs/GLOSSARY.md` clarify that "prompt" is operator slang for the content carried by a drive-action, not a protocol type. No `Prompt` message.
- **Message dropped for v0:** `docs/PROTOCOL.md` §Messages/commands/replies is narrowed — the separate `Message` id space (no-grant, replyable, informational) is reserved, not v0. Replies correlate to a drive-action (Command) only. `specs/seed/reply_correlation.qnt`'s `TypedCorrelation` is amended to check correlation to the command space only (the message-space branch is removed from the checked property; the `messageIds`/`messageCorrelationOk` machinery stays in the model as a reserved seam, not a checked branch).

## Implementation Units

### Unit 1: Sharpen VISION.md positioning

**File**: `docs/VISION.md`

Edit "Why Patchbay exists" and add a positioning paragraph stating the reachability principle: the coordination core is a network-reachable fixed point; operator surfaces (phone, laptop, desktop, CLI) and agent/harness machines are *both reconnecting clients* of it; neither side is load-bearing for the other. The operator's machine can be off while agents run; an agent machine can be off while the operator controls others. This is the mechanism that dissolves the bridge-machine coupling.

"What Patchbay is" gains: "a durable operator-presence layer — accepted operator intent survives across operator-machine switches, agent-machine switches, and harness choices."

"What Patchbay is not" is unchanged (already excludes harness replacement, LLM orchestration, workflow substrate).

**Acceptance Criteria**:
- [ ] VISION states the reachability principle (core = fixed point; operators and agents both reconnecting clients; neither load-bearing).
- [ ] VISION frames patchbay as the durable operator-presence layer.
- [ ] "What Patchbay is not" unchanged (not a harness, not an orchestrator).

### Unit 2: Sharpen ARCHITECTURE.md topology

**File**: `docs/ARCHITECTURE.md`

**2a — Planes diagram (component view)**: add an explicit note that the core is a reachable fixed point and both human control surfaces and adapter/harness machines are clients of it. The existing diagram already shows this shape; add the principle in prose.

**2b — V0 process topology**: replace the hedge `"v0 may colocate them on one host for simplicity"` with the explicit principle: v0 colocation is a *deployment convenience*, not the architecture; the principle is that the core is a reachable fixed point and neither operator machines nor agent machines are load-bearing for control. Split deployment is supported by the principle from day one; v0 may colocate to reduce moving parts.

**2c — V0 component slice**: add the provision-action-class note — the v0 slice assumes existing agent instances; the provision action class (spawn/retire agent instances without direct machine access) is a reserved seam requiring an agent-side supervisor capability, not a v0 commitment. The architecture must not silently assume "agent instances always already exist."

**Acceptance Criteria**:
- [ ] ARCHITECTURE states the reachability principle in prose (component view).
- [ ] The v0-colocate hedge is replaced with the principle + convenience framing.
- [ ] The provision reserved-seam is noted in the v0 component slice.

### Unit 3: Add the action inventory to PROTOCOL.md

**File**: `docs/PROTOCOL.md`

Add a new top-level section `## Operator action inventory` (after the existing "Actors and endpoints" section, before "Sessions") containing the six-class classification (drive / request / query / result / payload / provision) with the grounded tables above. This section becomes the normative source the `.proto` registry derives from (D4). Cross-reference `docs/VERIFICATION.md`'s authority order (prose owns product intent + vocabulary naming; the inventory is prose authority for action classification).

Also narrow the existing "Messages, commands, and replies" section per the vocabulary decisions: "prompt" = payload content; `Message` id space reserved (not v0); replies correlate to a drive-action (Command) only for v0.

**Acceptance Criteria**:
- [ ] PROTOCOL has an `## Operator action inventory` section with the six-class classification.
- [ ] The inventory tables ground each action in Pi + Claude Code + Codex-class.
- [ ] "Prompt" is defined as payload content, not a protocol type.
- [ ] `Message` id space is marked reserved (not v0); replies correlate to Command for v0.

### Unit 4: Glossary-carve Command vs slash-command

**File**: `docs/GLOSSARY.md`

Add/extend a `## Command` entry distinguishing:
- **Patchbay Command** — the lifecycle-bearing drive-action class (requires grant, has `CommandState`, terminal-race semantics). Derives from the action inventory's Drive class.
- **Harness slash-command** (e.g. `/agile-workflow:review`) — operator-typed content the agent's harness interprets. It is *payload* carried by a Patchbay Command, not a Patchbay Command itself. Patchbay carries it; it does not interpret it.

Note the deferred rename: `Command`→`Intent` is a future decision once the inventory is in place and the right word is clearer in context; for v0, `Command` is retained to avoid re-opening the done formal models.

**Acceptance Criteria**:
- [ ] GLOSSARY distinguishes Patchbay Command from harness slash-command.
- [ ] The deferred rename is noted (not applied).

### Unit 5: Amend VERIFICATION.md + reply_correlation.qnt for Message-drop

**File**: `docs/VERIFICATION.md`, `specs/seed/reply_correlation.qnt`

**5a — VERIFICATION.md**: update the "Reply correlation" model-area properties and the checked-normative baseline to reflect that v0 replies correlate to a Command only; the Message id space is a reserved seam. The `TypedCorrelation` property text narrows from "a known command or message id" to "a known command id" for v0, with the message-id branch reserved.

**5b — reply_correlation.qnt**: amend the `TypedCorrelation` checked property so its independent oracle checks correlation to the command space only (the `messageIds` / `messageCorrelationOk` machinery stays in the model as a reserved seam, not a checked branch). Re-run the check (`quint verify reply_correlation.qnt --invariant typed_correlation --max-steps 12`) and confirm it still passes. Update the inline `@promotion` block's `semantics` field to reflect the v0 narrowing. Commit the updated `.emitted.tla`.

**Acceptance Criteria**:
- [ ] VERIFICATION's Reply-correlation properties reflect Command-only correlation for v0.
- [ ] `reply_correlation.qnt`'s `TypedCorrelation` amended and re-checked (passing).
- [ ] The `.emitted.tla` inspection artifact is regenerated.

### Unit 6: Update downstream feature references

**File**: `.work/active/features/feature-protocol-idl-and-conformance.md`

Add `feature-operator-presence-and-action-inventory` to `depends_on`. Note in the body that the `.proto` registry (the old Q4) now derives from this feature's action inventory (D4) — the contract feature inherits the classification rather than inventing it.

**Acceptance Criteria**:
- [ ] `feature-protocol-idl-and-conformance` declares the dependency.
- [ ] Its body notes the registry derives from the action inventory.

## Implementation Order

1. Unit 1 (VISION) + Unit 2 (ARCHITECTURE) — the positioning sharpening, one cohesive edit to the foundation docs.
2. Unit 3 (PROTOCOL inventory) — the normative action layer; depends on the positioning being sharpened.
3. Unit 4 (GLOSSARY carve) + Unit 5 (VERIFICATION + model amendment) — vocabulary and model consistency with Unit 3.
4. Unit 6 (downstream feature reference) — bookkeeping.

No child stories spawned. This is a single-stride foundation-doc design with tight cross-doc cohesion; stories would add overhead rather than useful parallelism. (Matches the `feature-verification-contract-authority` and `feature-session-identity-adapter-contract` precedent: cross-doc foundation design = single feature stride.)

## Testing

No implementation code. Verification by document + model consistency:
- `rg` confirms the reachability principle appears in VISION and ARCHITECTURE.
- `rg` confirms the action inventory section exists in PROTOCOL with all six classes.
- `rg` confirms GLOSSARY distinguishes Command from slash-command.
- `quint verify reply_correlation.qnt --invariant typed_correlation --max-steps 12` passes after the Message-drop amendment.
- `work-view` confirms the dependency edge from `feature-protocol-idl-and-conformance`.

## Risks

- **Message-drop amends a checked formal model.** Narrowing `TypedCorrelation` to Command-only correlation removes the message-id branch from the checked property. Risk: if the model edit is careless, the property could become vacuous or self-defining. Mitigation: apply the genuine-checking discipline from the seed-model arc — mutate the amended predicate and confirm the invariant still catches a broken correlation; re-run the check. This is exactly the discipline the feature's workflow (adversarial review → deep review) is designed to enforce.
- **Provision as reserved-seam may be wrong.** Classifying provision as a reserved seam (not v0) means the bridge-machine problem is *partially* dissolved for v0: the operator can't spawn new agents remotely, only control existing ones. If the operator's actual v0 need is "spawn from phone," this classification is wrong and provision must be v0. Mitigation: the adversarial review gate pressure-tests this; the disposition is reversible (promote the seam to v0 if review finds it load-bearing for the value prop).
- **The triangulation is shallow.** Grounding in Pi + Claude Code + Codex-class is 3 harnesses, not a survey. Risk: a common action in a 4th harness (e.g. Aider's `/undo`, Continue's inline-edit) is missed and the "completeness" claim is overconfident. Mitigation: the inventory is explicitly grounded in these 3, not claimed exhaustive across all harnesses; the extension-pressure test covers adding action classes as seams. Review should pressure-test whether 3 is enough for the positioning claim.
- **Rename Command→Intent deferred.** Keeping `Command` (glossary-carved) leaves the overloaded word in the protocol. Risk: continued conversational conflation with slash-commands. Mitigation: the glossary carve is the v0 defense; the rename is filed as a future decision and the inventory's Drive class gives the rename a concrete target word if/when it happens.

## Parked related ideas

- `idea-harvest-remote-pi-extension-as-adapter` — the Pi adapter can harvest remote_pi's Pi-facing session/transcript know-how.
- `idea-harvest-remote-pi-app-design` — the app's session-state model and transcript projection seam inform the UX action surface.
- `idea-agent-to-agent-mesh-seam` — the local agent mesh (deferred, separate seam; depends on extension-seams classification).
