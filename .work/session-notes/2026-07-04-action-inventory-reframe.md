
## Session bank — 2026-07-04 (operator-presence + action inventory reframe)

**Read this before continuing `feature-operator-presence-and-action-inventory`.**

### Where we are

The wide harness/tooling survey (`feature-research-harness-action-surfaces`, DONE,
grounded across 7 harnesses + reconciled with operator's `~/SNC/.research/` corpus)
produced a synthesis at `.research/analysis/campaigns/harness-action-surfaces/parent.md`.

The operator then surfaced a REFRAME that invalidates the synthesis's structure:
the synthesis is operator-centric (operator→agent projection). The actual goal is a
direction-agnostic MODEL of the {entity}↔{entity} interaction space, INDEPENDENT of
patchbay, that patchbay's foundation docs then CONSUME by deciding which
{initiator}→{recipient}:{shape} tuples to mediate durably.

### The reframe (the load-bearing insight)

- WRONG frame: "what actions does the OPERATOR take that flow through patchbay"
- RIGHT frame: "what {initiator}→{recipient}:{shape} interactions exist in the world,
  independent of patchbay; patchbay's role is a projection/decision over that model"

Entities observed: Operator, Agent (via harness). Reserved: peer operator, peer agent.
Directions observed: op→agent, agent→op, agent→agent (Antigravity triggers, remote_pi
mesh). Reserved: op→op.

### The proposed shape set (DIRECTION-AGNOSTIC)

| Shape | What it is |
|---|---|
| Spawn | bring an entity/session into existence |
| Attach | establish a connection between entities |
| Command | lifecycle-bearing intent content sent to an entity |
| Request | lifecycle-acting / gate / input-request |
| Query | read |
| Output | content/result flowing back to the initiator |

(Payload = field of Command, not a shape.)

### OPEN QUESTIONS for the next session

1. **#2 unpacking — is "Output" its own shape, or does it collapse into Command when
   the agent initiates?** Lean: Output is distinct (correlates back to a prior Command
   via TypedCorrelation; it's a reply, not new intent). Agent→operator splits into
   Output (correlated reply) + Request (agent-initiated question like AskUserQuestion).
   Resolve definitively.

2. **The shape set itself** — does {spawn, attach, command, request, query, output}
   hold once agent→agent and agent→op-initiated are first-class? The survey found
   Antigravity triggers (automated_trigger — agent→agent Command?) and remote_pi
   agent_send/agent_request (mesh). Pressure-test whether these fit or surface new shapes.

3. **Does "Command" stay the patchbay term?** We agreed to drop "drive" (coinage) and
   use patchbay's native `Command` (glossary-carved from slash-command). In the
   direction-agnostic model, "Command" still works (it's a shape, not operator-specific).
   Confirm.

4. **The synthesis must be restructured** to separate MODEL (direction-agnostic
   interaction space) from PROJECTION (which tuples patchbay mediates durably vs passes
   through vs reserves). Current synthesis is operator-centric; needs rewrite into:
   (a) the {entity}↔{entity}:{shape} model, grounded in the 7 harnesses + mesh;
   (b) a projection table (patchbay's design decision over the model, clearly labeled).

5. **The consuming feature's design pass** was about to run (design→adversarial review→
   implement→deep review, per operator request). It should NOT run until the reframe
   lands — the design depends on the model, and the model is mid-reframe.

### What's DONE and committed

- `feature-research-harness-action-surfaces` — DONE. Synthesis + 34 attestations +
  4 cross-corpus pointer-attestations to SNC. Adversarial-verified (3 passes, APPROVED).
  NOTE: the synthesis is operator-centric and needs the reframe in #4 — but its
  *evidence* (the per-harness action tables) is sound and reusable; only the
  *organization* changes.
- Spawn prior art corrected (Claude --spawn, OpenCode serve, Dispatch, etc.) — no
  longer claims "novel for patchbay"; patchbay novelty = harness-agnostic + durable/
  authority-bearing spawn.
- Message split into Q-A (operator-originated, drops for v0) + Q-B (agent-originated
  question/elicitation — real and common). In the reframe, Q-B becomes
  Agent→Operator:Request cleanly (no longer awkward).
- Vocabulary: glossary-carve Command (don't rename to Intent — protects formal models);
  prompt = payload content; Message drops for v0 (operator-originated).
- Spawn/attach/operate/receive spine → SUPERSEDED by the direction-agnostic reframe.
  Keep the evidence; restructure the organization.

### Key files

- Synthesis (operator-centric, needs reframe): `.research/analysis/campaigns/harness-action-surfaces/parent.md`
- Consuming feature: `.work/active/features/feature-operator-presence-and-action-inventory.md`
- SNC prior corpus: `~/SNC/.research/` (855 attestations; `remote-agent-operation-landscape.md`
  is the load-bearing prior brief — spawn-vs-pilot framing lives there)
- Deploy guide (spawn prior art): `~/SNC/docs/ops/remote-agent-piloting.md`

### Methodology note (for future engagements)

The substrate-check was scoped too narrowly to patchbay's `.research/` and missed the
operator's broader `~/SNC/.research/` corpus. For engagements touching harness/operator
tooling, the check should span the operator's whole research corpus. Not formalized as a
skill change pending operator decision.


## Update — message-centric reframe (refinement to the direction-agnostic model)

The operator sharpened the reframe further: "Command" couples the message to the
operator-as-authority role. The fundamental unit is **{sender} → {recipient} : Message{force}**,
direction-agnostic. Force = the speech-act distinction:

- **Directive** — "do this work" (= old "Command")
- **Assertive** — "here's a result/fact" (= old "Output")
- **Interrogative** — "I need input" (= old "Query"/AskUserQuestion)

The operator→agent→response PATTERN is one config (op sends directive, agent sends
assertive back). Not the only reality. This is why op→op and agent→agent didn't fit —
we'd hard-coded the op→agent direction into primitive names. In message-centric form
they fit trivially: just messages between entities.

### Refined shape set (candidate)

- **Spawn** — bring an entity/session into existence (connection-management)
- **Attach** — establish a connection between entities (connection-management)
- **Message** — content sent from one entity to another, with a FORCE:
  - directive (do this) / assertive (here's a result) / interrogative (need input)
  - (commissive, declarative — reserved?)
- (Payload = field of Message, not a primitive)

### Reconciliation with the "Message drop" decision

The drop was a SPECIFIC flavor (operator-originated no-grant informational replyable
= assertive op→agent message, rare/weird, no harness exercises it). Drop stands for
that flavor. The GENERAL Message primitive (content exchanged between entities, with
force) is the unifying abstraction — always alive, hidden under role-coupled names.

### Honest tension (for design pass)

Patchbay's protocol needs to distinguish "directive message with a CommandState
lifecycle" (TerminalFinality proves a property about it) from "assertive reply."
Resolution: MODEL is message-centric (one Message primitive + force); PROJECTION
(patchbay's protocol) distinguishes message sub-types (Command = directive message
whose execution has lifecycle tracking; Output = assertive reply; Query =
interrogative) based on force + execution semantics. Model stays clean; patchbay
makes distinctions as a design decision OVER the model.

### What this supersedes

- The spawn/attach/operate/receive spine → SUPERSEDED by {spawn, attach, Message+force}.
- Open question #2 (Output vs Command) → DISSOLVED: they're the same primitive
  (Message) with different forces. No longer a separate question.
- The "Command" term → still patchbay's protocol word (for the directive-message-with-
  lifecycle sub-type in the projection), but NOT a model-layer primitive.

### Next session should

1. Pressure-test the message-centric model: does {spawn, attach, Message+force} hold?
2. Resolve force set: directive/assertive/rogative sufficient, or need commissive/declarative?
3. Restructure the synthesis into (a) message-centric MODEL + (b) patchbay PROJECTION.
4. Then the consuming feature's design pass can run (design→review→implement→deep review).
