## Session bank — 2026-07-04 (action-inventory frame adversarial review)

**Read this alongside `2026-07-04-action-inventory-reframe.md` before continuing
`feature-operator-presence-and-action-inventory`.**

### How this was produced

Fresh-context adversarial review dispatched on `openai-codex/gpt-5.5` (thinking high),
briefed to compare three frames as greenfield competitors (no incumbency bias either
direction), pressure-test six hypotheses, and surface alternatives none of the three
arrive at. The reviewer read SPEC/PROTOCOL/VERIFICATION/ARCHITECTURE/GLOSSARY + the
consuming feature + `command_lifecycle.qnt` + `reply_correlation.qnt`. It did **not**
read the `.research/` campaign artifacts (scoping gap on the orchestrator's part — see
open question #4).

The three frames:
- **A** — command-centric / role-coupled primitives (the current committed docs' implicit frame).
- **B** — message-centric / direction-agnostic with a force taxonomy (the proposed reframe).
- **C** — act/signal, direction as field (a third alternative surfaced in prior discussion).

### The operator's clarification on B's intent (LOAD-BEARING — resolves the inversion)

The operator clarified: **B was intended as a forward-flexibility / non-foreclosure
seam, not a normative model layer.** Because formal verification is being done up front,
the concern is that early formal models (which bind to v0 terms like `CommandState` /
`CommandDurability`) will *bake in* v0's command-centric naming and make post-v0 features
(agent→agent mesh, op→op coordination, agent-originated interactions) expensive to add
without renaming. B's purpose is to keep the vocabulary layer supple so non-v0
directions have a place to land.

This resolves the verification-inversion finding (B3) via the reviewer's "escape
hatch": B was never meant to be the checked layer. The formal models correctly bind to
the v0 projection (Command lifecycle); B is prose vocabulary that protects future
flexibility.

**BUT — B's *form* overclaims B's *intent*.** As stated in the reframe note and the
consuming feature body, B demotes `Command` to a "projection sub-type" of `Message+force`.
That demotion *would* require relabeling the checked formal properties
(`CommandDurability` → `DirectiveMessageDurability`?), which contradicts B's own goal
(don't force renaming). The cleaner way to achieve B's intent: **keep `Command` as the
v0 normative term the formal models bind to, and add `Message+force` /
`{sender}→{recipient}` as a *reserved vocabulary / extension seam* that future
projections can use without renaming `Command`.** B's intent is sound and survives;
B's form needs softening from "model the registry derives from" to "reserved vocabulary
future projections can extend into."

This maps directly onto `feature-extension-seams-non-foreclosure` (drafting): B's intent
is an instance of that feature's non-foreclosure discipline applied to the action
vocabulary. The home for B is extension-seam prose, not a restructuring of the v0 model.

### Verification-posture ranking (decisive axis, from the review)

1. Frame C (if tightened: broaden `Act` → authority-bearing `Operation`)
2. Frame A (best fit to current checked artifacts; weakness is product adequacy, not verification alignment)
3. Frame B (worst *unless constrained* — but B's intent, restated as extension seam, sidesteps the ranking entirely: it's not competing for the checked layer)

### What the review corrected in prior analysis

- `GrantAuthorityIsCommandKinds` is a checked-normative *obligation* (`VERIFICATION.md:30`, `:158`) but sits under stated-normative draft `authority.qnt` (`VERIFICATION.md:300`), not the promoted checked seed. **Any frame decision touching grant semantics must promote `authority.qnt` before grant behavior is treated as product.** Hidden dependency.
- C (as originally stated) has a real blind spot: status/snapshot refresh is authority-bearing (can disclose sensitive state, drives reconciliation) but not side-effecting. Under C's Act/Signal split, status query → grant-free Signal = unsafe. C needs "`Act` = authority-bearing operation" not "`Act` = side-effecting."
- A's blind spot is deeper than agent-interrogatives: no first-class model for *pending agent→operator elicitation* (opener, response authorization, timeout, cancellation, correlation). None of A/B/C model this.

### The alternative none of the three arrive at (strongest)

**Operation / Observation / Elicitation** — three primitives:
- `Operation` — any authority-bearing request (drive, cancel, interrupt, status/snapshot refresh, approvals, eventually spawn). Lifecycle + idempotency + grant + target. Formal models bind to `OperationState`.
- `Observation` — source-authenticated informational emission (output, events, replies, status facts). No operator grant, but requires source identity + correlation context.
- `Elicitation` — runtime-opened pending question/request to the operator. Its own id, opener, target context, allowed response kinds, timeout/cancellation, correlation. Formal models bind to `ElicitationState`.
- `Payload` — adapter-specific content carried.

Resolves C's gap (authority-bearing reads = Operations), B's gap (elicitation gets its own primitive with pending state), A's gap (agent-initiated interactions first-class) — while preserving verification posture (models bind directly to `OperationState` / `ElicitationState`, no unchecked layer).

(The review also surfaced two other alternatives — a registry-dimension frame and a ledger-record frame — and six common hidden assumptions across all three frames. See the full review below.)

### Open questions still gating the design pass

1. ~~Is B a normative model layer or prose vocabulary?~~ — **RESOLVED by operator**: B is forward-flexibility vocabulary, not a checked model. Soften B's form to match (extension seam, not model-restructure).
2. Should status/snapshot refresh be lifecycle-bearing + grant-checked, or should patchbay introduce a separate read/observe authority model? (Plain-speak below.)
3. Should v0 model agent→operator elicitation (AskUserQuestion-class) with its own pending state, or defer it? The survey says it's real and common across harnesses.
4. The `.research/` campaign was not read by this review pass. A second pass that reads `.research/analysis/campaigns/harness-action-surfaces/parent.md` + attestations is needed to validate cross-harness claims before the design pass rests on them.

### Plain-speak on open question #2 (status/snapshot refresh)

Right now the docs lump "ask the agent to do work" and "ask for a status update" into
the same bucket — both are "command kinds," both go through the full `CommandState`
lifecycle (accepted → delivered → running → completed), both need a grant.

A status check doesn't *do* anything to the agent (no side effect), but it does:
- need permission — you shouldn't be able to read sessions you don't own;
- can expose sensitive stuff — transcript content, what the agent is working on;
- drive reconciliation — the control surface uses status/snapshots to sync up after reconnect.

So the question is: does "what's the status of this session?" really need the full
command lifecycle (accepted/delivered/running/completed)? Probably not — a status poll
doesn't "run" on the agent. But it does need a grant (or some authorization). If we
split things into "side-effecting commands" vs. "everything else" and drop the grant
from "everything else," status checks lose their permission check — unsafe.

The Operation/Observation/Elicitation frame fixes this by making the split
"needs-authority" vs. "doesn't-need-authority" instead of "side-effecting" vs. "not."
Under that split, a status check is an `Operation` (needs a grant) even though it's
just a read, and it can have a lighter lifecycle than a side-effecting operation if
desired.

The real question for the operator: do you want status checks to go through the full
command lifecycle, or should reads have a lighter path that still requires a grant but
skips the lifecycle bookkeeping? That determines whether Frame C's gap actually matters
for v0.

---

## Full review (fresh-context, openai-codex/gpt-5.5, thinking high)

### 1. Grounding summary

Load-bearing verified facts:

- Patchbay currently defines protocol semantics around "durable operator intent, explicit authority, unambiguous target identity, and recoverable state" (`docs/PROTOCOL.md:3`).
- `docs/PROTOCOL.md` is explicitly the product-intent/vocabulary authority, not the final wire encoding (`docs/PROTOCOL.md:5`). `.proto` will own wire shape only; invariants remain formal-model authority (`docs/SPEC.md:79`, `docs/SPEC.md:85`; `docs/VERIFICATION.md:11-13`).
- Verification authority is question-type-layered, not globally ranked: formal models own invariants; prose owns product intent/vocabulary; `.proto` owns wire shape; implementation is never authority (`docs/VERIFICATION.md:11-17`).
- Current protocol has `Message`, `Command`, and `Reply`:
  - `Message`: information, may ask for a reply, no authority grant (`docs/PROTOCOL.md:49-51`).
  - `Command`: operator intent that may cause external action, requiring command id, idempotency key, target, authority grant, command kind, validation, expiration/cancellation semantics (`docs/PROTOCOL.md:53-63`).
  - `Reply`: typed correlation to prior message or command in same authority/session context (`docs/PROTOCOL.md:65-67`).
- The docs actually define **four** separate id spaces, not three: command id, message id, reply id, and event id/LSN (`docs/PROTOCOL.md:41-45`).
- `CommandState` is durable core state for accepted commands (`docs/PROTOCOL.md:73-75`) and is the thing modeled in `command_lifecycle.qnt`: `state <-> CommandState` (`specs/seed/command_lifecycle.qnt:7-8`).
- The command lifecycle model checks promoted properties for accepted-command durability, terminal finality, terminal race order, dedup, retry id/key stability, and retry-after-terminal behavior (`specs/seed/command_lifecycle.qnt:123-248`; summarized in `docs/VERIFICATION.md:287`).
- The reply correlation model checks `TypedCorrelation` over separate command/message/reply id spaces and same authority/session context (`specs/seed/reply_correlation.qnt:3-12`, `:18-21`, `:143-153`; summarized in `docs/VERIFICATION.md:289`).
- Authority is supposed to be command-kind-based: grants authorize command kinds (`docs/PROTOCOL.md:290-299`), grant checks happen before command acceptance (`docs/PROTOCOL.md:307`), and `GrantAuthorityIsCommandKinds` says grants are expressed only in canonical Patchbay command kinds (`docs/VERIFICATION.md:158`).
- Important correction: `GrantAuthorityIsCommandKinds` is listed as a checked-normative v0 authority-safety obligation (`docs/VERIFICATION.md:30`), but the seed table puts `authority.qnt`, including that property, under **stated-normative draft models**, not the promoted checked seed set (`docs/VERIFICATION.md:299-300`). So it is a required safety property, but not actually promoted/checked in the seed artifacts read here.
- The consuming feature says current docs have state machines but no complete action inventory (`.work/active/features/feature-operator-presence-and-action-inventory.md:24`), that "Command" is overloaded (`:28`), that agent-originated question/elicitation is real and must be modeled (`:48`, `:72`), and that the researched action spine is `spawn / attach / operate / receive / payload` (`:49`, `:54`, `:70`).

### 2. Per-frame analysis

#### Frame A — command-centric / role-coupled primitives

**What A gets right:**
- Best alignment with current checked artifacts. `command_lifecycle.qnt` binds directly to `CommandState` (`specs/seed/command_lifecycle.qnt:7-8`) and the promoted properties are explicitly command lifecycle properties (`docs/VERIFICATION.md:287`). A does not insert an unchecked abstraction above the checked object.
- Clear authority boundary. Commands require an authority grant (`docs/PROTOCOL.md:55-61`), and grants authorize command kinds (`docs/PROTOCOL.md:290-299`).
- Clear anti-forgery/correlation story. Separate id spaces and typed reply correlation are real in docs/models (`docs/PROTOCOL.md:41-45`, `:65-67`; `specs/seed/reply_correlation.qnt:143-153`).

**What A gets wrong that B/C get right:**
- Direction coupling does not fit agent-originated elicitation. Current `Reply` presupposes a prior message/command (`docs/PROTOCOL.md:65-67`), while the feature says agent-originated question/elicitation is real and must be modeled (`feature-operator-presence-and-action-inventory.md:48`, `:72`). B and C both make direction a field, which handles this more cleanly.
- "Command" is overloaded. The feature explicitly identifies three meanings: harness slash-command payload, Patchbay lifecycle-bearing command, and umbrella operator request (`feature-operator-presence-and-action-inventory.md:28`).
- SPEC already mixes action classes. Initial "command kinds" include send prompt, cancel/interrupt, request status/snapshot refresh, and receive correlated replies/events (`docs/SPEC.md:30`). That is not a clean command-kind taxonomy.

**Frame-specific blind spot:**
A's blind spot is not just naming; it lacks a first-class model for **pending agent-to-operator elicitation**: who opened it, what authorizes the response, whether it times out, whether the response is a command, and what it correlates to. Adding "agent message" may patch one case, but the deeper obligation is an elicitation/correlation state machine.

#### Frame B — message-centric / direction-agnostic with force taxonomy

**What B gets right:**
- Direction as data is a real improvement. Actors include operator, agent, adapter, daemon, service, and control surface (`docs/PROTOCOL.md:9`). A direction field is more general than primitive-name-implied direction.
- It notices the action-inventory gap. The consuming feature says the action set is missing and must be derived (`feature-operator-presence-and-action-inventory.md:24`, `:54`).
- Spawn/attach pressure is real. The feature says spawn/attach are distinct action classes and spawn has cross-harness prior art; Patchbay's novelty would be durable/authority-bearing harness-agnostic spawn (`feature-operator-presence-and-action-inventory.md:38`, `:70-71`).

**What B gets wrong that A/C get right:**
- Verification inversion risk is real if B's "Message+force" is model-authoritative. The checked command lifecycle model binds to `CommandState`, not to a `Message{force}` abstraction (`specs/seed/command_lifecycle.qnt:7-8`, `docs/VERIFICATION.md:287`). Verification authority belongs to formal models for invariants (`docs/VERIFICATION.md:11`), while prose owns naming/product intent (`docs/VERIFICATION.md:13`). If B calls `Message+force` the "grounding model" and makes `Command` a projection, it elevates an unchecked conceptual layer over the checked lifecycle.
- Force taxonomy is not what the current safety properties check. Current safety hinges on accepted-command durability, idempotency, terminal finality, authority, snapshots, and typed correlation (`docs/VERIFICATION.md:25-34`), not directive/assertive/interrogative classification.
- "Message is the unifying primitive" is overstated by B's own shape. If Spawn and Attach are peers of Message, then the top-level model is not message-centric; it is at least `{spawn, attach, message}`.

**Frame-specific blind spot:**
B risks letting linguistic force substitute for operational semantics. "Interrogative" does not answer whether a status/snapshot query requires authorization, durable state, replay, audit, idempotency, or snapshot reconciliation. Those are the load-bearing axes.

#### Frame C — Act/Signal, direction as field

**What C gets right:**
- Best greenfield preservation of verification posture. If `Act` is the renamed lifecycle-bearing thing currently called `Command`, the formal model can bind directly to `ActState`/`CommandState` without an unchecked layer.
- Direction field fixes A's structural direction problem. Agent-originated questions can be represented without inventing a role-coupled primitive per direction.
- Avoids B's force taxonomy. The current docs/models care about authority/lifecycle/durability, not Searle-style force categories.

**What C gets wrong that A/B get right:**
- Act/Signal is too binary unless "Act" includes authority-bearing reads. Status/snapshot refresh is in SPEC's initial command-kind list (`docs/SPEC.md:30`). If C defines Act as only side-effecting, then status/snapshot queries become Signals and may fall outside grant/authorization. That would be unsafe because status/snapshot can disclose sensitive state and drive reconciliation.
- Agent elicitation is under-modeled. A Signal "may ask for a reply" matches current Message prose (`docs/PROTOCOL.md:51`), but real elicitation needs pending state, response authorization, timeout/cancellation, and correlation. C does not specify that.
- Spawn may be underscoped. C keeps spawn/attach adapter-lifecycle scoped, but the feature says spawn is a first-class action class and likely normative after research (`feature-operator-presence-and-action-inventory.md:38`, `:70-73`). C must either define a fleet Act later or explicitly justify deferring spawn.

**Frame-specific blind spot:**
C misses a likely third category: **authority-bearing observation/request**. Not every authorized operation is side-effecting, and not every non-side-effecting interaction is safely grant-free.

### 3. Hypothesis verdicts

**1. Verification inversion in B — CONFIRMED, with an escape hatch.**
Confirmed if B means what the prompt says: `Message{force}` is a grounding model and `Command` is a projection. The actual checked lifecycle model binds to `CommandState` (`specs/seed/command_lifecycle.qnt:7-8`), and verification authority assigns invariants to formal models (`docs/VERIFICATION.md:11`). A new Message+force layer has no checked model in the artifacts read.
Escape hatch: if B explicitly demotes `Message+force` to prose/naming only, and lifecycle/authority invariants still bind directly to the lifecycle-bearing registry entry, the inversion dissolves. But then B is no longer a model layer from which the protocol registry derives; it is a vocabulary taxonomy.

**2. SPEC's v0 command-kind list force-mixes — CONFIRMED.**
SPEC lists "send message/prompt, cancel or interrupt, request status/snapshot refresh, and receive correlated replies/events" as initial command kinds (`docs/SPEC.md:30`). Under B's stated definition, only directive-message-with-lifecycle becomes Command. That would exclude or reclassify at least status/snapshot refresh and receive replies/events. So B changes v0 semantics unless it broadens "Command projection" beyond directives.

**3. Grant authority coverage gap — PARTIAL, but dangerous.**
Docs currently tie grants to command kinds (`docs/PROTOCOL.md:290-299`) and require grant checks before command acceptance (`docs/PROTOCOL.md:307`). If B narrows commands to directives, or C narrows Acts to side-effecting actions, then status/snapshot queries risk falling outside grant authority.
Agent-originated replies/output probably should not require an operator grant, but they still require source authentication and same-context correlation (`docs/PROTOCOL.md:65-67`; `specs/seed/reply_correlation.qnt:153`). Status/snapshot refresh is different: it is operator-initiated and can disclose/control authoritative state.
Also, the premise that `GrantAuthorityIsCommandKinds` is "checked" needs correction: it is a checked-normative obligation (`docs/VERIFICATION.md:30`, `:158`) but appears in the draft/stated `authority.qnt` row (`docs/VERIFICATION.md:300`), not the promoted checked seed list.

**4. B's "Message is unifying primitive" is overstated; spawn/attach force fleet authority — MOSTLY CONFIRMED, with scope caveat.**
Confirmed: B's own primitive set includes Spawn and Attach outside Message, so Message is not actually unifying. Current architecture already models adapter attach as lifecycle/registration, not as a message (`docs/ARCHITECTURE.md:31-35`; `docs/PROTOCOL.md:351-353`).
The fleet-spawn cost is real: durable/authority-bearing spawn needs a target/supervisor authority model before the runtime session exists. A and C avoid that cost in v0 by scoping spawn out.
Caveat: the feature says spawn has broad prior art and Patchbay's novelty may be durable, authority-bearing, harness-agnostic spawn (`feature-operator-presence-and-action-inventory.md:70-71`). So the cost may be unavoidable eventually; B's problem is forcing it into the core protocol before its authority/lifecycle model exists.

**5. B's force taxonomy is Searle residue; C's Act/Signal captures the load-bearing split — PARTIAL.**
The docs/models support the criticism: checked properties are about lifecycle, durability, idempotency, authority, snapshots, and correlation, not force taxonomy (`docs/VERIFICATION.md:25-34`).
But C's Act/Signal split is only sufficient if Signals can open a correlated pending elicitation, or C adds an `Elicitation`/`Question` primitive. Current Message prose says a message may ask for a reply (`docs/PROTOCOL.md:51`), and the feature says agent-originated question/elicitation is real (`feature-operator-presence-and-action-inventory.md:48`, `:72`). A bare Signal does not yet model the pending obligation.

**6. A cannot cleanly express agent→operator interrogatives — CONFIRMED; locally fixable but structurally pressuring.**
A's Reply requires a prior message/command (`docs/PROTOCOL.md:65-67`). The docs' id assignment also says command/message ids are client-generated in the operator domain (`docs/PROTOCOL.md:43`), which makes agent-originated questions awkward as Messages unless that rule changes.
This is locally fixable by adding an agent-originated `Question`/`Elicitation` primitive or by generalizing Message direction/id assignment. But if A keeps primitive names direction-coupled, the same issue will recur for future interaction shapes.

### 4. Alternatives none arrive at

**Alternative 1: Authority-bearing Operation / Observation / Elicitation**
Primitives:
- `Operation`: any authority-bearing request to core/adapter. Includes drive, cancel, interrupt, status/snapshot refresh, approvals, and eventually spawn. Has lifecycle, idempotency, grant kind, target, actor/session context.
- `Observation`: source-authenticated informational emission from adapter/core/agent. Includes output, events, replies, status facts. No operator grant, but requires source identity and correlation context.
- `Elicitation`: runtime-opened pending question/request to the operator. Has elicitation id, opener, target context, allowed response kinds, timeout/cancellation, and correlation.
- `Payload`: adapter/harness-specific content carried by an operation or observation.
Problem solved: separates authority-bearing reads from side effects, avoiding C's Act/Signal gap and B's force taxonomy. Formal models bind to `OperationState` and `ElicitationState`, not to linguistic force.

**Alternative 2: Registry-dimension frame**
No single top-level noun. Define an action registry where each entry declares dimensions: origin actor class; target scope; authority policy; lifecycle policy; durability/audit policy; idempotency policy; correlation policy; adapter capability mapping; payload schema. Instances are just `ActionRequest` and `ActionRecord`, validated against the registry.
Problem solved: avoids pretending "Command," "Message," "Act," or "force" is the universal abstraction. The registry is the model; formal properties attach to dimensions. This matches the project's Single Source of Truth posture better than hard-coded primitive families.

**Alternative 3: Ledger-record frame**
Primitives: `SubmittedRecord`; `AcceptedOperation`; `DeliveryRecord`; `RuntimeEmission`; `ElicitationOpened`; `ElicitationAnswered`; `AdapterAttached`; `SnapshotMaterialized`; `AuditRecord`. Each record kind has a state-machine/correlation schema. User-facing "command/message/reply" labels are projections over ledger records.
Problem solved: makes the durable log the semantic center. This aligns with LSN-first terminal ordering and snapshot reconciliation (`docs/PROTOCOL.md:249-261`) and avoids an unchecked conceptual layer above the formal objects.

### 5. Common hidden assumptions

All three frames appear to share assumptions a fourth frame could reject:
- There must be one unifying top-level interaction unit. The registry-dimension frame rejects this.
- Direction is a primary semantic axis. Source identity, target scope, authority policy, and correlation may matter more than human-readable direction.
- Grant authority only belongs to commands/acts. Status, snapshot, transcript, and inventory reads may need authorization even without side effects.
- Reply correlation is enough for agent-originated questions. Elicitation likely needs its own pending-state model.
- Spawn is either protocol or adapter-lifecycle. It may instead be a fleet/supervisor operation plane with separate authority and lifecycle.
- Product vocabulary can be chosen before action inventory. The feature demonstrates the inverse: missing inventory caused downstream registry ambiguity (`feature-operator-presence-and-action-inventory.md:24`, `:81`).

### 6. Verification-posture ranking

Decisive axis: formal models should bind to the thing whose invariants are checked; no unchecked conceptual layer should claim model authority.

1. **Frame C, if tightened.** Best greenfield posture because `Act` can be exactly the checked lifecycle-bearing object, direction becomes a field, and no conceptual force layer sits above the registry. But C must broaden Act to "authority-bearing operation," not only side-effecting act, or add a separate authorized `Observe`.
2. **Frame A.** Best fit to current artifacts: the existing checked model binds directly to `CommandState` (`specs/seed/command_lifecycle.qnt:7-8`; `docs/VERIFICATION.md:287`). Its weakness is product adequacy, not verification alignment.
3. **Frame B.** Worst unless constrained. As stated, it introduces `Message+force` as a grounding layer while the formal models check `CommandState` and typed correlation directly. If B demotes `Message+force` to naming prose, it can recover verification posture, but then its core reframe loses normative force.

Important caveat: authority safety is not fully promoted in the seed artifacts. `GrantAuthorityIsCommandKinds` is required by the checked-normative baseline (`docs/VERIFICATION.md:30`, `:158`) but listed under draft `authority.qnt` (`docs/VERIFICATION.md:300`). Any chosen frame needs that model promoted before grant semantics are treated as product behavior.

### 7. Open questions

- Is B intended as a normative model layer or just a prose vocabulary layer? That determines whether the verification inversion is fatal or cosmetic. *(Resolved by operator after the review — see header.)*
- Should status/snapshot refresh be lifecycle-bearing and grant-checked, or should Patchbay introduce a separate read/observe authority model?
- What exact lifecycle does agent-originated elicitation need: timeout, cancellation, allowed response kinds, grant binding, audit, replay?
- Does spawn enter v0 protocol, or remain a reserved fleet/supervisor seam? Current docs only define adapter attach lifecycle; the feature says spawn is likely normative but still needs disposition.
- The feature cites a grounded harness survey, but the reviewer did not read the `.research` campaign artifacts. To validate cross-harness claims, read `.research/analysis/campaigns/harness-action-surfaces/parent.md` and its attestations.
- How will command/action registry changes update the seed models, generated contracts, and conformance vectors together, per the semantic-change rule (`docs/VERIFICATION.md:236-258`)?
