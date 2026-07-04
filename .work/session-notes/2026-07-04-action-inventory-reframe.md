
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

