# Vision: Patchbay as the durable hub — adapter seams for work, resources, and multi-host surfaces

> **This is a vision document for discussion, critique, and buy-in — not a
> committed plan or a release scope.** It describes a direction the
> sole-maintainer of Patchbay and the co-owners of Workbench and token-commune
> find compelling. Nothing here is authorized work. Specific commitments, when
> they come, will run through the substrate, the foundation docs, and the
> extension-seam ceremony the project already requires. Where this vision
> contradicts the current `SPEC.md` / `ARCHITECTURE.md` / `PROTOCOL.md`
> contract, those docs are authoritative and this vision is a proposal to
> change them — not a silent revision.

## The determination

Patchbay, Workbench, and token-commune are co-owned, but the value the
combined surface would deliver was always Patchbay's value: its durable
delivery contract (intent accepted before delivery, a typed failure
vocabulary, dispatch idempotency) viewed alongside git-backed work state and
pooled fuel. Four rounds of adversarial review converged on the same finding
from different angles: the "three-product family" framing doesn't hold up,
because the composition was always Patchbay's delivery contract displayed next
to other things. The delivery contract *is* Patchbay.

The honest repositioning: **Patchbay is the durable hub; Workbench and
token-commune are the first two adapter instances, not co-equal substrates.**
Patchbay drives toward the adapter seams that let any git-backed ledger and any
operational resource project into the cockpit. Co-ownership is a convenience —
the seam contracts can co-evolve without negotiation latency — but it is not a
structural requirement. The seams are generic, so the value survives
independent ownership.

This matches Patchbay's existing identity (adapter-neutral core, formally
specified) and where sole-maintainer control actually sits. It is narrower than
"three-product family," but it is the framing that survives scrutiny.

## The four seams Patchbay drives toward

Each is independently valuable; each is a promotion or projection of an
existing seam, not a new product.

### 1. The public client API promotion

Today `patchbay.ControlService` is an **internal** boundary — the web server is
an authenticated principal that owns login, CSRF, session-evidence forwarding,
and Origin checks; the core is loopback-only (`ARCHITECTURE.md:167,174-175`).
The cockpit is already a webview client over this boundary (browser → web
server → core, via Protobuf/Connect).

Promoting it to a **public client API** requires a dedicated auth-topology
design: separating public client methods from core administration, defining
browser/extension-host/CLI auth profiles, and specifying CORS/CSP/webview
messaging/SecretStorage behavior. That design does not exist yet. This is the
seam that makes Patchbay reachable from multiple hosts — the standalone cockpit,
an IDE extension, and the CLI all become clients of one protocol.

This is a security-architecture change, not webview packaging. It is the real
cost the IDE-extension path carries, and it is named here rather than hidden.

### 2. A generic external-work-ledger projection seam

Not Workbench-specific. Any git-backed ledger (Workbench, the agile-workflow
substrate Patchbay currently runs, a JIRA mirror) projects into the cockpit
via an adapter-declared, versioned contract. The pane renders the ledger
alongside the delivery contract; Patchbay does not mutate the ledger and does
not own work state (Option B — git owns work-state durability, atomicity,
concurrency, rollback).

This is the seam that delivers "view the ledger alongside the delivery
contract." It is adapter-neutral, which is Patchbay's identity — a generic seam
fits the product; a Workbench-specific coupling does not. Workbench becomes one
adapter instance; the current agile-workflow substrate could be another.

### 3. The token-commune resource adapter

Already designed (`epic-token-commune-observer` + `epic-token-commune-control-attention`,
both at `stage: drafting`). The second reference adapter — the first
operational-resource adapter, proving the adapter boundary across resource
shapes (token-commune is a quota pool, not a session/transcript). This is the
seam that delivers "pooled fuel viewed alongside the work."

token-commune is an independent product that plugs into Patchbay via this seam.
Co-ownership lets the gateway's external API and the adapter contract
co-evolve, but the adapter consumes only the external API — LLM traffic never
enters Patchbay.

### 4. The minimal honest correlation grounding

The correlation between a commissioned Operation and a work item cannot be an
unvalidated string in an opaque payload — that produces durable false
confidence (the cockpit shows "delivered, completed" next to a stale item, with
an audit trail). The minimal honest grounding: a `schema_ref`'d instruct-payload
schema carrying the work item id, plus **pane-side validation that flags
dangling/stale references as attention** (Patchbay's own `AttentionRequired`
machinery exists for exactly this). Keeps Option B cheap while making drift
*visible* instead of silent — which is what the project's Fail Fast principle
demands.

This is not the typed external-reference variant to `TypedCorrelation` (which
is a closed `oneof` over five internal id spaces — no external-reference
variant, and adding one is a registry change that is *not* load-bearing for the
core loop). That variant remains an optional future rigor. The grounding here is
convention + validation, not typed composition.

## The affirmative case: the commissioned-work loop, honestly

The value is narrower than "typed composition" or "verified landing" — both
were false, and this vision does not claim them. What it claims:

**A durable delivery contract (intent + reported outcome + failure
vocabulary) viewed alongside git-backed work state and pooled fuel.**

The worked loop:

1. **Scope.** The operator opens the ledger pane (rendered in their IDE or
   standalone cockpit — seam #2) and scopes an outcome. The item is a Markdown
   record in `.work/` with a stable id, dependency edges, and acceptance
   evidence.
2. **Commission.** The operator authorizes an agent via a Patchbay `instruct`
   Operation whose payload references the work item id under a `schema_ref`'d
   schema (seam #4). The Operation is durably recorded before delivery —
   accepted→delivered, with idempotency and declared failure modes — and its
   `CommandId` is the durable handle for the commission.
3. **Allocate fuel.** The agent draws pooled hosted-quota from token-commune's
   OpenAI-compatible endpoint. The token-commune adapter (seam #3) reports
   capacity, contribution health, and draw as Observations the operator sees
   alongside the session.
4. **Observe durable delivery.** The agent's tool calls and outputs are
   source-authenticated Observations correlated to the accepted Operation by
   `CommandId` (the one typed correlation the protocol provides). The operator
   sees the delivery contract progressing: what was intended, whether the
   adapter reported completion, what failed and how.
5. **Work state lives in git.** The agent edits files with its own tools; git
   owns work-state durability. The ledger is Markdown in git, updated as today.
   Patchbay does not mutate the ledger. An interrupted agent hands off via the
   ledger + git history; the next session resumes from the same state.

## What this is, and what it is not

**This is:** co-location plus a durable delivery contract. The operator sees the
delivery contract, the work ledger, and the fuel pool visible together in one
cockpit, conventionally linked via the work item id. That is more than a browser
with three tabs (which has no durable delivery contract), and less than typed
composition (which would require a `TypedCorrelation` variant the protocol
does not have).

**This is not:** verified landing. Patchbay's `completed` state only means the
adapter reported a result without a failure code — it is a *durable,
source-authenticated claim* of landing, not verification. The vision does not
claim verification.

**This is not:** a three-product family where Workbench and token-commune are
co-equal first-class substrates. They are adapter instances. The seams are
generic; the value survives independent ownership.

## The honest comparison is against the incumbent, and the gap is narrower than claimed

The actual incumbent is not a browser with three tabs — it's agile-workflow plus
a chat harness, which **already durably records intent, delivery, and failure
in the git-backed ledger** (stage transitions, acceptance evidence, blocked
stages, gate-origin items — all in `.work/CONVENTIONS.md`). The vision's earlier
claim that the incumbent has "no durable record of intent, delivery, or
failure" was false, and is corrected here.

What the incumbent genuinely lacks is narrower: **the commission-boundary
lifecycle** — accept-before-deliver durability, a typed failure vocabulary at
the dispatch point, and idempotency on dispatch. And that gap only bites in
**async detached commissioning** — the operator fires agents across machines
and walks away; one silently never starts; the durable `accepted` record with
no `running` transition is the only place that failure is visible without
reading scrollback. That is a real value. It is also Patchbay's existing
standalone v0.1.0 pitch — it does not require the ledger pane or the fuel
panel. The pane and the fuel panel extend it; they do not create it.

This means the buy-in question is honest and narrow: **is the combined view
(the delivery contract alongside the ledger and fuel, in one cockpit) worth
building over the incumbent, given the incumbent already covers most of what
the vision originally claimed?** The vision does not assert yes; it poses the
question.

## What "works today" honestly means

- The **delivery-contract half** of the core loop exists today (shipped in
  v0.1.0: durable acceptance, idempotency, failure vocabulary, the
  `CommandState` registry).
- The **fuel panel** is two `stage: drafting` epics away
  (`epic-token-commune-observer` + its dependency
  `epic-agent-operations-resource-plane`, both drafting).
- The **ledger pane** — the combined surface's entire UX delta — is unbuilt,
  undesigned, and unmocked, in a repo whose AGENTS.md mandates mockup-first for
  new surfaces. It is not "a renderer"; it is a new data source, authority
  boundary, security surface, reconciliation model, and host integration. Naming
  it "a renderer" minimizes the only component that carries the vision's value.
- The **public client API promotion** (seam #1) is unbuilt — today's
  `ControlService` is internal, and making it public is the auth-topology
  redesign the IDE-extension path depends on.

So: the delivery contract works today. The combined commissioned-work loop
does not. Buildability requires naming the work, not avoiding a registry
promotion.

## The cross-repo torn-state case (where Option B does not dissolve the problem)

"Git owns work-state durability, atomicity, concurrency, rollback — all
solved" is true **only for single-repo work state.** The vision's own headline
worked example is cross-repo: "land the token-commune observer adapter" — the
ledger item lives in the *patchbay* repo's `.work/`, the code being landed
lives in the *token-commune* repo. There is no atomic commit across the two
repositories; the torn state Option B claims to dissolve (Operation `completed`,
ledger not updated; or ledger updated, code reverted) reappears at the repo
boundary.

This is the common case for a multi-product operator, not an edge case. The
vision does not solve it; it names it as an open consequence of Option B that
the pane and the correlation grounding must surface (e.g., flag a
`completed` Operation against a ledger item whose repo's code hasn't shipped
as attention) rather than hide.

## Two promotions this vision depends on (both currently reserved)

Neither is load-bearing for the core loop above; both are *extensions* that add
value once the core loop is proven and pressure exists. Both are named reserved
seams, and both require the project's promotion ceremony (registry update,
pressure classification, formal models, conformance vectors) before becoming
product.

**1. Agents-as-principals.** Today `agent-send` is wire-present but rejected at
submission with `validation_failed`; `agent-send-reserved-validation` is a
draft conformance vector (named in VERIFICATION.md as a stated-normative
reservation, not yet checked). Making agents peers who originate Operations is
a *proposed promotion* requiring an OperationKind registry change, retirement
of that draft vector, new bounded models for agent→agent authority lineage and
grant-revocation ordering, and a scoped design for agent identity enrollment,
delegated grant shape, compromised-agent blast radius, and adapter-vs-agent
identity distinction. None of that design exists today. The core loop works
without it (operator→agent via `instruct`); agents-as-principals would extend
the loop (an agent commissions a sub-agent under a delegated grant).

**2. Multi-operator shared-deployment read visibility.** Today v0.1 has one
operator and one authority domain; `(authority_domain_id, LSN)` is a
constant-valued demarcator, not an access-control mechanism; presence-leak
prevention is a reserved seam. The protocol names an `authorized_filter` field
in the subscription state model (`ObservationSubscription`), but
`SubscribeRequest` on the wire carries only `authority_domain_id` and `cursor`
— the subscriber cannot express a filter and the server computes a fixed scope.
The multi-operator filtering design (per-operator evaluation, filtered snapshot
materialization, leak-freedom under reconnect, metadata side channels) does not
exist. The core loop works with a single operator; multi-operator read
visibility would extend it (a second operator observes the shared ledger and
delivery state). Full multi-human authority (delegation lineage, quorum
Elicitations, handoffs, responder-actor distinction) stays further out.

Under the project's own promotion rule, neither is eligible until a second
operator demonstrates pressure. The buy-in question is whether they are the
right *direction*, staged after the core loop proves demand — not whether to
build them now.

## Co-ownership: what it gives and what it costs

**Gives.** Co-ownership removes contract *negotiation* latency and lets the
seam contracts (the public client API, the ledger-projection contract, the
token-commune adapter contract) co-evolve under joint design.

**Costs.** Co-ownership also removes the independent party that catches gaps.
The honest-limits remedy — "a consumer of each contract that the contract owner
does not control" — is **vacuous as stated**, because all three products are
co-owned: no such consumer exists. Contract tests written by the owner test
what the owner *thinks* the contract says; it is a self-graded exam, and worse,
it *looks* like rigor — future drift will be pointed at the contract tests as
evidence of health.

A real independence mechanism would be one of: an external adapter author
consuming a seam; a published conformance suite with third-party runners; or a
frozen artifact versioned independently of all three repos with mutation
requiring ceremony. None exists today. The vision names this honestly rather
than prescribing a vacuous remedy: under co-ownership, the mitigations *reduce*
the referee problem rather than replace it, and a real independence mechanism
is an open question (see Q6).

## Workbench and token-commune as adapter instances

Under this framing:

- **Workbench** is an independent product that plugs into Patchbay via the
  generic ledger-projection seam (#2). It is one adapter instance among
  possible many. Co-ownership lets the seam contract co-evolve, but the seam is
  generic so Workbench is pluggable into any Patchbay, not just this one.
  Workbench is actually *more* valuable this way than as a co-equal substrate
  coupled to one Patchbay.
- **token-commune** is an independent product that plugs into Patchbay via the
  resource-adapter seam (#3, already designed). Same: co-ownership is
  convenience, not requirement.

This means adopting Workbench as "Patchbay-native" is not a goal of this
vision. The current agile-workflow substrate Patchbay runs could equally be an
adapter instance. The generic seam is the point; the specific ledger schema is
a swappable projection. (Workbench's simpler schema vs. agile-workflow's richer
stage/gate/release schema is a separate question, and the conversion cost —
losing the load-bearing `work-view` tooling and skill ecosystem mid-build — is
not paid by this vision.)

## The honest limits, collected

- **This is a vision, not a scope.** Several things it proposes contradict the
  current `SPEC.md` v1.0 contract (single operator), the `PROTOCOL.md`
  OperationKind registry (`agent-send` reserved), and the checked conformance
  suite. Those docs are authoritative until formally revised through the
  project's ceremony.
- **The seams and the pane have no substrate existence.** The public client API
  promotion, the generic ledger-projection seam, the ledger pane, and the
  correlation grounding have no `.work/` items, no designs, and no mockups — in
  a repo whose AGENTS.md mandates mockup-first for new surfaces. The
  token-commune adapter chain is real but at `stage: drafting`.
- **The "one public protocol" claim is a promotion, not a fact.** Today's
  `ControlService` is an internal boundary behind an authenticated web-server
  principal; making it public is a security-architecture change with real
  auth-topology work.
- **Multi-operator visibility is greenfield.** The subscribe wire path carries
  no filter; the multi-operator filtering design does not exist.
- **Distribution is unproven.** The demand question — who is the *second* user
  beyond the builder-operator — is open. The substrate's own answer (SPEC v1.0.0:
  additional self-hosting operators of *single-operator* Patchbay) is a demand
  hypothesis that requires neither promotion and arguably argues *against* the
  combined surface as the vehicle. The combined surface triples the surface
  area without tripling the demonstrated audience; the intersection of
  Patchbay + Workbench + token-commune users is n-of-1 by construction.
- **A legitimacy risk on the fuel.** Pooling subscription quota across a trust
  group sits in tension with upstream provider terms; a public product built on
  that fuel inherits the exposure. This vision does not resolve it.
- **Option B does not dissolve cross-repo torn state.** "All solved" is true
  only for single-repo work state; the cross-repo case (common for a
  multi-product operator) needs explicit attention, not a hand-wave.

## What we want from this document

Buy-in, pushback, and critique — not authorization. The questions, in priority
order:

1. **Is the combined view worth building over the incumbent?** The honest
   affirmative case is a durable delivery contract viewed alongside git-backed
   work state and pooled fuel — narrower than typed composition, but buildable.
   The incumbent (agile-workflow + chat harness) already durably records
   intent/delivery/failure in the ledger; what it lacks is the
   commission-boundary lifecycle, which only bites in async detached
   commissioning. Is that gap real enough to justify the pane and the seams?
2. **The four seams.** Are the public client API promotion, the generic
   ledger-projection seam, the token-commune resource adapter, and the
   correlation grounding the right set? Is anything missing, or is any
   mis-scoped (e.g., should the ledger seam be Workbench-specific rather than
   generic)?
3. **The two promotions (agents-as-principals, multi-operator read visibility).**
   Both are *extensions* to the core loop, not load-bearing for it. Under the
   project's own promotion rule, neither is eligible until a second operator
   demonstrates pressure. Are they the right *direction*, staged after the core
   loop proves demand?
4. **The IDE extension path.** Is Open VSX + OSS Code-family the right host
   strategy, or does the Open-VSX-only constraint cap the audience
   unacceptably? The core loop does not require the IDE extension; the
   standalone cockpit is sufficient. The IDE extension is a distribution bet,
   not a composition requirement.
5. **The generic ledger seam vs. Workbench-native.** Should the ledger seam be
   generic (any git-backed ledger as an adapter projection) or
   Workbench-specific? The vision argues generic, on adapter-neutrality
   grounds; the counter-argument is that a generic seam dilutes the
   Workbench-specific value. Which is right?
6. **Co-ownership discipline.** The prescribed independence mechanism is
   vacuous as stated (all three are co-owned). What real independence mechanism
   (an external adapter author, a published conformance suite with
   third-party runners, a frozen artifact versioned independently) could
   replace the missing referee?
7. **The demand question.** Who is the second user? The substrate's own answer
   (single-operator self-hosting) is a demand hypothesis requiring neither
   promotion. Is the combined surface the right vehicle for that demand, or
   does it consume v1.0.0 design attention without serving it?

The next step is not implementation. It is resolving these questions —
starting with #1, which decides whether the vision has a payoff worth its
costs — and, where the vision survives, running the specific promotions and
seam designs through the substrate and foundation-doc ceremony rather than
smuggling them in via a vision doc.
