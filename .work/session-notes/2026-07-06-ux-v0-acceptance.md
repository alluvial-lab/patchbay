## Session bank — 2026-07-06 (UX v0 acceptance, surface-neutrality reframe)

**This is the reboot point.** A fresh context can pick up here. This session
took `feature-ux-v0-acceptance` from drafting to done — and, like the pi-parity
session before it, the path through it is the valuable artifact: an operator
reframe that shifted the deliverable from "specify one web cockpit" to "define
what any conformant UX must minimally entail," a design-review Block on a
single-source-of-truth violation caught *before* implementation, and a clean
substrate Approve. Board now at 19/25 features done.

## What this session accomplished

### `feature-ux-v0-acceptance` → `done`

The v0 web cockpit UX acceptance criteria — restructured into a **surface-
neutral conformance floor** + the v0 web cockpit as its first conformant
instance. Ships as a restructured `docs/UX.md` + a one-line cross-reference in
`docs/ARCHITECTURE.md`. The floor is registry-derived (references
`docs/PROTOCOL.md` by section; does not re-declare the registries).

**Full lifecycle, four commits:**

1. **Feature-design** (`5ed6a68`). The operator reframed the deliverable
   mid-design: not "specify one web cockpit" but "define what *any* conformant
   UX must minimally entail." This named **surface-neutrality** as a principle
   symmetric to adapter-neutrality (surface-specific presentation is a
   surface-declared feature, not a core UX primitive), with the v0 web cockpit
   as the first conformant *instance* and operator-customizable skins/layouts
   ("Codex-style vs Claude-style vs CLI") as a reserved seam above the floor.
   The operator also asked whether a UX component layer should be defined for
   different surfaces to consume — answer: yes, named as the **shared
   presentation-component layer** (refining `docs/ARCHITECTURE.md:152`'s
   "presentation model"), the seam that binds canonical protocol states to
   skin-able presentable primitives. Implementation deferred; the seam is
   named, not built (symmetric to how pi-parity named supervisord-control
   `spawn` without building it).
2. **Advisory design review — Block → fixed** (`be34b16`). Fresh-context
   cross-model deep review on `gpt-5.5`. Verdict: **Block**. One real blocker
   + four important findings, all correct:
   - **B1 (blocker, single-source-of-truth):** the design told UX.md to list
     concrete `CommandState`/session-state members while saying "no re-
     declaration" — reintroducing the duplicate-enum drift
     `feature-command-state-ssot` was created to kill (`docs/UX.md` must
     *reference* protocol state machines, not redefine them). Fixed: floor
     obligations reference `docs/PROTOCOL.md` by section/anchor; UI labels
     marked non-authoritative.
   - **I1:** floor lacked authority/grant visibility ("who is allowed to
     control this?"). Added.
   - **I2:** Operation affordance coverage incomplete (omitted `spawn`/`attach`).
     Added: every committed OperationKind actionable or visibly unavailable.
   - **I3:** presentation-component seam overstated enforceability while
     deferring the enforcer. Rephrased as *future* structural enforcement;
     added the "first web cockpit needs the layer or a conformance-test
     substitute" risk.
   - **I4:** mockup pass misclassified "not v0" (the web cockpit *is* v0).
     Reclassified as a v0 surface-design follow-up.
   - Two nits (subscription-stream spelling; benchmark preservation).
3. **Implement** (`410cdb1`). Restructured `docs/UX.md` into three parts:
   surface-neutral conformance floor (11 obligations) → shared presentation-
   component layer (named seam, deferred) → v0 web cockpit (first conformant
   instance). Preserved benchmark/mobile-first/anti-patterns by relocation.
   Self-verified all 7+2 acceptance criteria; confirmed no inline registry
   enumeration (B1 fix landed).
4. **Substrate deep review → done** (`9981e7b`). Fresh-context cross-model
   deep review on `gpt-5.5`. Verdict: **Approve, zero findings** (no blockers,
   no important, no nits). Notably cleaner than the pi-parity arc (which
   caught a Blocker at this stage) — because the design-review Block caught
   the load-bearing SSOT issue *before* implementation, the implement stride
   wrote the corrected version directly. No confirmatory re-review warranted.

### The reframe that mattered

The operator's "different operators might want different styles of cockpit"
question reshaped this feature. The original brief assumed a single v0 web
cockpit to specify; the reframe made it "define what any conformant UX must
minimally entail" — surface-neutrality, symmetric to adapter-neutrality, with
skins as a reserved seam. **Transferable lesson: the same pluggable-above-a-
floor principle applies to both edges of the system** (adapters below the
protocol, surfaces above it). Naming the shared presentation-component seam
made the floor enforceable rather than aspirational — without it, "conformant
floor" is just a prose checklist each surface re-binds independently.

This is the second session in a row where an operator reframe mid-design
shifted the deliverable for the better (pi-parity: `/clear` → session
replacement; ux-v0: one cockpit → surface-neutral floor). Both were caught by
the design conversation, not by review — review then validated the reframed
design.

### Two review passes, different outcomes — and why

This arc and the pi-parity arc both ran two cross-model `gpt-5.5` passes
(design advisory + substrate deep), but the *distribution* of findings
differed in an instructive way:

- **pi-parity:** design review Approve-with-comments (structural/coverage);
  substrate review **Block** (single-source-of-truth: the `question`-contract
  reclassification). The load-bearing defect slipped past design review and
  was caught at implementation review.
- **ux-v0:** design review **Block** (single-source-of-truth: the registry
  re-declaration); substrate review clean Approve. The load-bearing defect
  was caught at design review, so implementation wrote the corrected version.

**Lesson: the two-pass discipline earns its cost, but *where* the defect lands
varies.** A clean substrate pass does NOT mean the design review was
redundant — it means the design review did its job and the implementation was
faithful. Dropping either pass on the assumption the other will catch it
would, over time, ship the single-source-of-truth violations. Keep both for
foundation work.

### One backlog idea parked

`idea-operator-customizable-ux-skins` — the "Codex-style vs Claude-style vs
CLI" skins/layouts direction. Parked (not promoted to active) because it's a
future product direction with no v0 trigger, and it depends on the deferred
shared presentation-component layer existing first. The other three reserved
follow-ups (v0 web cockpit mockup pass; presentation-component layer
implementation; UX conformance vector) were **not** parked — they're deferred
v0 obligations with named triggers, correctly recorded in the feature body
under "Reserved follow-up," to be promoted into an implementation roadmap
when v0 implementation work is scoped. Parking them would mislabel required
predecessors as "someday" ideas.

## Board state at end of session

`epic-foundation-hardening` (stage: implementing): **19/25 features done**.

### Done this session
- `feature-ux-v0-acceptance`

### Other done features (18) — the foundation core
`feature-v0-walking-skeleton`, `feature-command-state-ssot`,
`feature-design-grant-shape`, `feature-design-terminal-commit-race`,
`feature-persistence-snapshot-model`, `feature-security-threat-model`,
`feature-session-identity-adapter-contract`, `feature-verification-contract-authority`,
`feature-formal-model-seed`, `feature-research-contract-tooling`,
`feature-research-formal-methods-tooling`, `feature-research-harness-action-surfaces`,
`feature-research-web-control-security`, `feature-bank-formal-methods-skills`,
`feature-operator-presence-and-action-inventory`,
`feature-foundation-doc-completeness-gaps`, `feature-protocol-idl-and-conformance`
(+ 4 child stories), `feature-pi-parity-checklist`, + all 7 stories.

### Drafting (6) — what's left
- `feature-formal-model-realignment` — model-side follow-on to the O/O/E
  roll-forward. Needs a design pass first. Depends on action-inventory +
  formal-model-seed (both done).
- `feature-extension-seams-non-foreclosure` — extension seams + non-foreclosure
  rules. Depends on v0-walking-skeleton.
- `feature-idempotency-ambiguous-execution` — `maybe_executed` state,
  idempotency-key semantics. Depends on command-state-ssot + session-identity.
- `feature-lease-scope-decision` — leases in/out of v0, fencing. Depends on
  v0-walking-skeleton + security-threat-model.
- `feature-observability-operator-admin` — operator/admin observability.
  Depends on v0-walking-skeleton + persistence-snapshot-model.
- `feature-research-v0-stack-tooling` — v0 stack/tooling picks (research).
  Depends on research-contract-tooling.

### Backlog (14) — parked
Now includes `idea-operator-customizable-ux-skins` (new). Notably
`idea-multi-human-coordination`, `idea-desktop-app-surface`,
`idea-agent-to-agent-mesh-seam`, the two remote-pi harvest ideas, plus
research-handoff candidates.

## Next logical feature (recommendation)

The "what does v0 look like?" picture is now substantially closed: pi-parity
(done) + ux-v0-acceptance (done) + protocol-idl (done) + the O/O/E action
inventory (done) together define the v0 surface, adapter, contract, and UX
floor. The remaining drafting features are the heavier *semantic* ones:

- **`feature-formal-model-realignment`** is the direct follow-on to the O/O/E
  arc — model-side work (VR2 metadata drift, V1 transition-adjacency gap, new
  stated-normative models for Elicitation/subscription/spawn-authority). It
  needs a design pass first (heavier lift; open questions: metadata schema,
  strengthen-in-place vs. new model, authoring order). This is the right pick
  if the operator wants to close the verification gap the O/O/E roll-forward
  exposed.
- **`feature-extension-seams-non-foreclosure`** is lower-risk and consolidates
  the committed/reserved/rejected classifications each done feature has been
  doing locally (incl. the surface-neutrality seam this session named). Its
  ordering note says to run it after the reopened semantic work concluded —
  which it has. Good pick if the operator wants a consolidation pass.
- The independent semantic features (`feature-idempotency-ambiguous-execution`,
  `feature-lease-scope-decision`) are about lifecycle/authority details and
  can be picked up in any order.

## Reserved follow-ups filed in the feature body (not v0 active)

- **v0 web cockpit mockup pass** — v0 surface-design follow-up, required
  before web cockpit implementation (navigation pattern decided there). Not
  parked; it's a deferred v0 obligation with a named trigger, recorded in the
  feature body. Promote into an implementation roadmap when v0 implementation
  is scoped.
- **Shared presentation-component layer implementation** — the structural
  enforcement mechanism for the UX floor; build deferred. Same handling.
- **UX conformance vector/checklist** — test substitute if the component layer
  isn't built when the web cockpit starts. Conditional; same handling.
- **Operator-customizable skins/layouts** — parked as
  `idea-operator-customizable-ux-skins` (genuine future direction, no v0
  trigger, depends on the deferred component layer).

## Key files (reboot reference)

- Foundation docs (authoritative): `docs/{VISION,ARCHITECTURE,PROTOCOL,
  VERIFICATION,GLOSSARY,UX,SECURITY,SPEC}.md`
- **UX doc (restructured): `docs/UX.md`** — surface-neutral conformance floor
  (11 obligations) + shared presentation-component layer (named seam) + v0
  web cockpit (first conformant instance).
- Pi adapter parity checklist: `docs/ADAPTER-PI.md`
- Contracts: `contracts/` (proto, rust, ts, vectors, scripts)
- Formal models: `specs/seed/*.qnt`, `*.als` (need realignment — see
  `feature-formal-model-realignment`)
- Substrate: `.work/active/features/`, `.work/active/stories/`,
  `.work/active/epics/epic-foundation-hardening.md`
- Conventions: `.work/CONVENTIONS.md` (prose black-box test for lane routing)
- Backlog: `.work/backlog/` (now incl. `idea-operator-customizable-ux-skins`)
- Session notes: `.work/session-notes/` (this file + prior banks)

## Routing discipline reminders for fresh context

- **umans exception is OFF.** Standard codex routing. Implementers and
  reviewers on `openai-codex/gpt-5.5` (or spark for light work). The umans
  orchestrator dispatches cross-model review to `gpt-5.5` and never spawns
  `umans/*` subagents.
- **Prose black-box test** is in `.work/CONVENTIONS.md` — apply to every
  `[prose]` candidate before routing. Semantic commitments → feature-design.
- **Two cross-model passes (design advisory + substrate deep) earn their cost
  on foundation work.** A clean substrate pass does NOT mean the design review
  was redundant — it means the design review caught the load-bearing defect
  and the implementation was faithful. Dropping either pass ships
  single-source-of-truth violations over time. Keep both.
- **The SSOT rule is load-bearing for UX docs:** `docs/UX.md` *references*
  `docs/PROTOCOL.md` registries by section; it does NOT re-list the members.
  `feature-command-state-ssot` was created to kill duplicate-enum drift; both
  this session's design-review Block and the pi-parity substrate-review Block
  were single-source-of-truth violations of this kind.
- **Surface-neutrality is now a named principle** (symmetric to adapter-
  neutrality). Downstream surface-design work must honor it: the floor is
  behavioral + state-binding; visual design is surface-declared; skins are a
  reserved seam above the floor. The shared presentation-component layer is
  the future enforcement mechanism.
- **Deferred v0 obligations with named triggers stay in the feature body, not
  the backlog.** The backlog is for "someday" ideas; required predecessors
  (mockup pass, component layer, conformance vector) belong in the feature's
  "Reserved follow-up" until promoted into an implementation roadmap.
