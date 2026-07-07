## Session bank — 2026-07-07 (extension-seams, two design features, the [prose] audit, and the retroactive design-gate audit)

**This is the reboot point.** A fresh context can pick up here. This session
took `feature-extension-seams-non-foreclosure`, `feature-lease-scope-decision`,
and `feature-idempotency-ambiguous-execution` from drafting to done — and then
the operator's question about the `[prose]` tag's reliability triggered a full
misroute audit, the retirement of the `[prose]` routing tag project-wide, and a
retroactive design-gate audit of four foundational decisions. Board now at 26/29
features done across two epics.

## What this session accomplished

### Three foundation-hardening features → `done`

1. **`feature-extension-seams-non-foreclosure` → done.** The non-foreclosure
   discipline: a three-way classification vocabulary (committed v0 / reserved
   seam / explicitly rejected) codified into `docs/SPEC.md` (Non-foreclosure
   discipline section), `AGENTS.md` (Extension pressure-test checklist), and
   `docs/PROTOCOL.md` (Extension seams registry — a 38-row cross-cutting index
   across all 12 scope areas, tagged C/R/X). Genuinely `[prose]` (consolidates
   already-settled classifications; the black-box test passed honestly from
   fresh context). **Lesson relearned:** I auto-advanced it to `done` on
   subagent review alone; the operator correctly pushed back that authority
   artifacts need operator sign-off. Reopened, operator reviewed the four
   judgment calls (codification shape, v0-only labeling framing, the lease
   tracking note, parked-ideas stance), ratified, advanced. The cross-model
   review caught one real misroute tendency (an invented "offline queued intent
   as first-class Operation" classification row) — fixed by removing it.

2. **`feature-lease-scope-decision` → done.** Operator-confirmed design: defer
   leases from v0 (no v0 use case — single-operator, single-core, no
   contention); keep a stated-normative precondition model in VERIFICATION;
   soften PROTOCOL's bare safety guarantee into a precondition gated on a
   future fencing model (epochs/tokens not designed now). Four tightly-coupled
   doc edits. Clean cross-model review (Approve, no findings). The forward-
   reference the extension-seams sweep left open is now closed; the registry
   row tracks `feature-lease-scope-decision`'s resolution.

3. **`feature-idempotency-ambiguous-execution` → done.** Operator-confirmed
   design (Q1–Q4): represent execution ambiguity as `execution_outcome_unknown`
   (failure-vocabulary term → `failed`, NOT a new CommandState); scope idempotency-
   key dedup per-target; reject payload-mismatched retries (`validation_failed`);
   pin key retention to at-least-until-terminal. The cross-model review caught
   THREE real foundation-doc contradictions (the value of deep review): (1) my
   retention wording broke the checked `RetryAfterTerminalReturnsExisting`; (2)
   a stale "command id OR idempotency key" rule contradicted the new contract;
   (3) UX overclaimed plain `failed` as safe-to-retry. All fixed; a second
   independent re-review confirmed clean resolution + formal-model alignment
   (per-target scoping is consistent with `command_lifecycle.qnt`'s `appliedKeys`
   abstraction).

### The `[prose]` misroute audit and tag retirement

The operator asked whether the remaining `[prose]`-tagged drafting features
would fast-lane or were misroutes. Applying the black-box test honestly: BOTH
were misroutes (`feature-formal-model-realignment` has open design questions +
pre-mortem; `feature-observability-operator-admin` has v0-in/out + diagnostic-
surface + redaction decisions). Stripped both.

This triggered a full audit of every item that ever carried `[prose]` (16
items). **Finding: 56% misroute rate** (9/16: 7 caught previously + 2 this
session). Root cause: the tag was applied by deliverable format ("produces
docs") not work-nature ("choosing between approaches"). 2 were genuine prose
(`feature-bank-formal-methods-skills`, `feature-extension-seams-non-foreclosure`).
4 slipped through to `done` still tagged — the foundational decisions.

**Decision: retire `[prose]` entirely.** Updated `.work/CONVENTIONS.md`
(removed `[prose]` from tag list; replaced the prose-black-box-test-as-routing-
gate with a work-nature note applied *inside* feature-design Phase 4.5 — same
test, never structurally skipped), `AGENTS.md` (routing row sends all design-
bearing work to feature-design; no prose-author lane), and `epic-foundation-
hardening` (lane-routing note updated). Legacy `[prose]` tags on done items are
inert; new items won't get the tag.

### Retroactive design-gate audit (`epic-retroactive-design-gate-audit` → done)

Filed a new epic with 4 child audit features, one per slipped-through found-
ational decision (`feature-v0-walking-skeleton`, `feature-command-state-ssot`,
`feature-persistence-snapshot-model`, `feature-security-threat-model`). Ran all
four in parallel as fresh-context design-gate auditors (`openai-codex/gpt-5.5`).
Full design-gate-equivalent per child (alternatives evaluation + faulty-
assumption hunt + propagation check; NO pre-mortem per operator direction).

**Results: all four hold. Zero faulty-assumption-found. Zero corrective actions.
Zero dependents re-opened.**

| Audit | Verdict | Note |
|---|---|---|
| v0-walking-skeleton (13 deps) | holds | clean; weak initial-command-set already re-resolved by later harness research |
| command-state-ssot (8 deps) | holds-with-caveats | `accepted→completed` tension already owned by formal-model-realignment |
| persistence-snapshot (2 deps) | holds-with-caveats | Ports & Adapters boundary genuinely holds; crash-recovery draftness is known |
| security-threat-model (3 deps) | holds-with-caveats | malicious-adapter posture is conscious v0, not accidental |

### The operator-consent gap (the most important reframe of the session)

After the audits returned "holds," the operator pushed back: *the question
wasn't whether the decisions were faulty — it was whether the agent made design
decisions during prose-implement that should have surfaced for operator
consideration.* This exposed a distinction I had collapsed:

- **"Holds"** = the decisions are *defensible* and *non-faulty* (what the audit
  verdicts said).
- **The operator's actual question** = were there decisions the agent made
  during prose-implement that the design gate's Phase 4.5 would have surfaced
  to the *operator*?

Those are different. A decision can be defensible AND still be one the operator
should have been asked. "Holds" is necessary but not sufficient for operator
authority on foundational decisions; the consent question is separate and must
be answered explicitly.

**Resolution:** Pulled out the two Category-C decisions (agent-made during
prose-implement/review on questions the gate would have surfaced):
1. `SubmissionOutcome` enum (command-state review fix) — a new pre-acceptance
   vocabulary. Operator ratified (a) separate enum.
2. Adapter snapshot capability tiers (persistence review fix) — already had
   retroactive ratification via `feature-session-identity-adapter-contract`;
   operator confirmed (a) three tiers.

Both landed decisions stand; the procedural gap (no question gate at first
introduction) is closed by explicit ratification.

## The lesson to carry forward

**A clean audit verdict is necessary but not sufficient for operator authority
on foundational decisions.** "Defensible" ≠ "consulted." When a feature slipped
the design gate, the gap is procedural (operator wasn't asked), and closing it
requires surfacing the agent-made decisions for explicit ratification — even
when the reasoning was sound. The `[prose]` retirement prevents future procedural
gaps; the ratification closed the residual debt on the two that slipped through.

## Board state at session end

**26/29 features done.** Two epics: `epic-foundation-hardening` (implementing)
and `epic-retroactive-design-gate-audit` (done).

### Remaining drafting features (all deps met; all now correctly routed)

- `feature-formal-model-realignment` — `[verification, protocol, foundation]`
  (stripped `[prose]` this session). Designs the model-realignment plan: VR2
  metadata schema choice, V1 model approach choice (strengthen
  command_lifecycle.qnt in place vs new operation_lifecycle.qnt), 4 new
  stated-normative model arcs. Routes through **feature-design**. Coordinate
  with the command-state audit's `accepted→completed` finding.
- `feature-observability-operator-admin` — `[foundation]` (stripped `[prose]`
  this session). v0-in/out scope decision, diagnostic-surface design,
  redaction-rules decision. Routes through **feature-design**.
- `feature-research-v0-stack-tooling` — `[research]`. Routes to research-
  orchestrator.

## Where a fresh context picks up

The board is clean and the `[prose]` debt is fully closed (past: audited +
ratified; present: 2 misroutes fixed; future: tag retired). The next pickups
are the 3 drafting features above. Two are design features (formal-model-
realignment is the most complex — model-approach choices with regression risk
to 7 checked properties); one is a research engagement. The operator prefers
being consulted on design questions (Phase 4.5 interactive, not autopilot-
resolved) — especially after this session's lesson about operator consent.

## Key files touched this session

- Foundation docs: `docs/SPEC.md` (non-foreclosure discipline), `AGENTS.md`
  (extension pressure-test checklist + routing-row `[prose]` retirement),
  `docs/PROTOCOL.md` (extension seams registry, lease precondition softening,
  idempotency boundary-scoping + `execution_outcome_unknown` + dedup rules),
  `docs/VERIFICATION.md` (lease cross-ref, idempotency boundary scope),
  `docs/UX.md` (retry-safety matrix), `docs/GLOSSARY.md` (lease note).
- Conventions: `.work/CONVENTIONS.md` (`[prose]` retired; work-nature test
  moved into feature-design Phase 4.5).
- New epic + 4 audit features: `epic-retroactive-design-gate-audit` + children.

## Commits (chronological)

`0d38b16` implement extension-seams → `197ddaf` review → `a0ddd97` reopen
(operator pushback) → `ead7bb7` done (operator sign-off) → `fe5695c`/`ba154ba`/
`fb5ed7f` lease-scope design/implement/done → `01531a5`/`60784e0`/`f918b38`/
`ad2d848` idempotency design/implement/fix-blockers/done → `a49a52e` strip
[prose] from 2 misroutes → `7d6bec0` retire [prose] tag → `845f521` scope audit
epic → `b3507a0`/`29477cf`/`ecdd05f`/`15885d2` four audits → `bf05c72` audit
epic done → `3210b95` operator ratification of 2 decisions.
