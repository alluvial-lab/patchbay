## Session bank — 2026-07-08/09 (formal-model-realignment design + implement + review)

**This is the reboot point.** A fresh context can pick up here. This session took
`feature-formal-model-realignment` from `stage: drafting` through design,
adversarial review, interactive decisions, implementation, and per-story deep
review — 6 stories, all `done`. The feature itself is at `stage: implementing`
awaiting its feature-level deep review (final gate) before advancing to `done`.

## What this session accomplished

### 1. Design pass + adversarial review + interactive decisions (Q1–Q6)

The feature brings the seed formal models into agreement with the rolled-forward
foundation docs (O/O/E frame split `checked-normative` into `checked-model` vs
`checked-normative`). Three classes of misalignment: VR2 (16 `@promotion` blocks
still said `tier: checked-normative`), V1 (`command_lifecycle.qnt` didn't verify
the transition adjacency), and 4 new stated-normative arcs (Elicitation, Spawn,
Subscription, TypedCorrelation extension).

The design arc:
- **Initial design pass** (written into the feature body).
- **Adversarial review** (fresh-context `openai-codex/gpt-5.5` xhigh, cross-model):
  Block — 6 blockers (B1–B6), 5 importants (I1–I5), 3 nits.
- **Interactive decisions** — the brief's 4 open questions + 2 additional live
  decisions walked through one-by-one with the operator, each with options and
  tradeoffs, ratified:
  - **Q1 (Option 3)** — derive product tier entirely: drop the `tier` field from
    `@promotion` blocks; `check-models.mjs` computes tier from `status` + vector
    coverage. Removes the drift mechanism that allowed VR2.
  - **Q2 (Option A)** — strengthen `command_lifecycle.qnt` in place with the
    EXACT PROTOCOL transition table + a stutter-safe transition-INTO property.
    Regression gate mandatory.
  - **Q3 (Option 3)** — extend existing draft models: rich new
    `elicitation_lifecycle.qnt`; SA+SUB into `authority.qnt` reusing real grant
    tuples; TC extends `reply_correlation.qnt`.
  - **Q4 (Option α)** — sequential, no parallel fan-out.
  - **Q5 (Option I)** — author all four arcs; per-story review loops.
  - **Q6 (Option 1 + verification)** — promote VR4 with a real `quint verify` +
    mutation test stride.
- **Revision** incorporating all 6 decisions.
- **Re-review** (fresh-context): Block — 3 residual blockers (B1/B4/B5) where
  decisions were recorded but unit specs weren't fully updated + stale text.
- **Second revision** addressing all residuals.
- **Confirmation pass** (fresh-context): Confirmed — all 8 residuals fixed,
  internal consistency clean.

### 2. Process rules landed (the recurring gap that caused the papering-over)

Two force-loaded rules + one convention:
- **`.agents/rules/design-checkpoints.md`** — when a skill says "use structured
  question tool" and none is available (this harness has none; the example
  `question.ts` isn't installed), ask in plain text. Tool absence does NOT invoke
  autopilot judgment-mode. Prefer plain-text asks (portable across local pi and
  remote_pi). Root cause: agile-workflow skills ported from Claude Code assume a
  built-in question tool that pi doesn't ship.
- **`.agents/rules/implementation-ambiguity.md`** — under autopilot, mechanical/
  syntactic ambiguities resolve in-stride; semantic 50/50s (affect protocol
  semantics, safety claims, state-machine shape) append `## Blocker`, commit,
  stop. The test: "would a different reasonable implementer pick a different
  option and produce a materially different model?"
- **`.work/CONVENTIONS.md` review-lane rule** — `[verification]`-tagged stories
  route to the DEEP lane (not fast-lane), with a per-story convergence loop to
  nits. The seed arc is the evidence: every adversarial pass caught self-defining
  properties the implementation missed.

### 3. Implementation + per-story deep review (6 stories, sequential)

All 6 stories implemented by `openai-codex/gpt-5.5` (high) subagents; deep-lane
review run by the umans orchestrator inline (cross-model; dispatch-economical
under 429 pressure). The host reproduced every mutation test independently —
this caught 3 false claims.

| Story | Unit | Outcome | Convergence |
|---|---|---|---|
| traceability | TR+M | done | 2-pass (B1: check-models trusted self-declared model field) |
| adjacency | CL | done | 1-pass |
| elicitation | EL | done | 2-pass (B1: elicitation_correlation_typed self-defining) |
| spawn | SA | done | 2-pass (B1: FleetAuthorityForSpawn self-defining, B5 residual) |
| subscription | SUB | done | 1-pass (N3 split to subscription_authority.qnt) |
| typed-correlation | TC | done | 1-pass |

**The recurring failure mode: self-defining invariants.** THREE of the 6 stories
(elicitation, spawn, + the traceability script) had invariants that re-used the
action's guard predicate — so breaking the guard left the invariant passing `[ok]`,
unable to detect a broken rule. Each was the same species the seed arc caught
(reply_correlation B1, CSRF B2). The fix pattern (uniform): make the action
**permissive** (record the bad behavior into state) + restructure the invariant
to an **independent oracle** (check raw state facts without calling the guard
helper). References: `reply_correlation.qnt` `recordedReplyIndependentOk`,
`elicitation_lifecycle.qnt` `recordedResponseIndependentOk`,
`authority.qnt` `acceptedSpawnHasRawFleetGrant`,
`subscription_authority.qnt` `acceptedSubscriptionHasRawGrant`.

**The implementers' mutation-test claims did NOT reproduce 3 times.** The host
caught each by independently running the mutation (break the action guard, not
the invariant). Lesson reinforced: for safety-claiming models, the orchestrator
must reproduce mutation tests — implementer reports are not sufficient evidence.

### 4. Genuine-checking proofs independently verified by host

For each promoted property, the host broke the action's guard and confirmed
`[violation]`:
- `no_accepted_to_completed` (CL): break `allowedTransition` → `[violation]` (7s)
- `elicitation_correlation_typed` (EL): break `responseMatchesTarget` domain → `[violation]` (10s)
- `fleet_authority_for_spawn` (SA): break `actionGrantAuthorizesSpawn` fleet-scope → `[violation]` (7s)
- `subscription_grant_checked` (SUB): break `actionGrantAuthorizesSubscription` scope → `[violation]` (5s)
- `typed_correlation` (TC): weaken `elicitationCorrelationOk` → `[violation]` (6s)
- `browser_local_state_not_authority` (VR4): break `serverAccepts` → `[violation]`

## Board state at end of session

`epic-foundation-hardening` (stage: implementing). **18 of 25 features done**
(was 17; `feature-formal-model-realignment` is at `stage: implementing` with all
6 child stories `done`, awaiting feature-level deep review).

### Done this session
- `story-formal-model-realignment-traceability`
- `story-formal-model-realignment-adjacency`
- `story-formal-model-realignment-elicitation`
- `story-formal-model-realignment-spawn`
- `story-formal-model-realignment-subscription`
- `story-formal-model-realignment-typed-correlation`

### The feature's remaining step
- **Feature-level deep review** (final gate): fresh-context pass over the whole
  realignment as a complete artifact. Cross-file tier consistency, mutation-test
  spot-checks across all arcs, VR4, N3 split consequence, overclaims. If clean,
  advance `feature-formal-model-realignment` to `stage: done`.

## Key files

- Feature: `.work/active/features/feature-formal-model-realignment.md`
- Stories: `.work/active/stories/story-formal-model-realignment-*.md` (6)
- New script (the tier authority): `contracts/scripts/check-models.mjs`
- New models: `specs/seed/elicitation_lifecycle.qnt`, `specs/seed/subscription_authority.qnt`
- Extended models: `specs/seed/command_lifecycle.qnt`, `specs/seed/reply_correlation.qnt`, `specs/seed/authority.qnt`, `specs/seed/csrf_browser.qnt`, all 7 seed `@promotion` blocks (tier field dropped)
- Rules: `.agents/rules/design-checkpoints.md`, `.agents/rules/implementation-ambiguity.md`
- Convention: `.work/CONVENTIONS.md` (verification-story deep-lane rule)

## Routing discipline reminders for fresh context

- **umans exception is OFF.** Standard codex routing. Implementers/reviewers on
  `openai-codex/gpt-5.5` (high); the umans orchestrator runs deep reviews inline
  (cross-model; dispatch-economical under rate limits).
- **Genuine-checking is the load-bearing gate.** Reproduce every mutation test
  independently. Implementer reports of "[violation]" are not sufficient — 3 of
  6 stories had false claims the host caught.
- **The `tier` field is gone from `@promotion` blocks.** Do not re-add it.
  `check-models.mjs` derives tier from `status` + vector coverage.
- **`[verification]` stories route to the deep lane** (not fast-lane) per
  `.work/CONVENTIONS.md`.
- **`quint`/`buf` PATH:** `/home/agent/.npm-global/bin` is not on PATH; prefix
  invocations with `export PATH="/home/agent/.npm-global/bin:$PATH"`.

## Provenance note

The operator requested this session run via autopilot with the explicit guard
that ambiguities must surface (no question tool → interactive prompts swallowed).
The two rules + convention landed at the start are what made the run safe:
implementation-ambiguity (semantic 50/50 → blocker, not judgment) and
verification-story deep-lane (per-story adversarial convergence, not fast-advance).
No blockers were surfaced during implementation — the design's Q1–Q6 decisions
pinned the semantic choices, and the implementers hit only mechanical issues
(Quint syntax, PATH, N3 state-space split) which they resolved in-stride. The
3 self-defining-invariant catches all came from the deep-review convergence loop,
not from implementer self-report — validating the convention.
