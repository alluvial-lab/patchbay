## Session bank — 2026-07-06 (Pi parity checklist, end-to-end)

**This is the reboot point.** A fresh context can pick up here. This session
took `feature-pi-parity-checklist` from drafting to done through the full
lifecycle — and the path through it is itself the most valuable artifact: a
prose-misroute catch, a load-bearing semantic question inverted by going to
source, and two cross-model review passes (design + implementation) that both
surfaced real defects. Board now at 18/25 features done.

## What this session accomplished

### `feature-pi-parity-checklist` → `done`

The Pi adapter v0 parity checklist and migration floor. Ships as a new
dedicated adapter doc `docs/ADAPTER-PI.md` + a one-line forward-reference in
`docs/ARCHITECTURE.md` ("Pi-first migration path"). It consumes the settled
canonical registries (`OperationKind`, adapter capability manifest, session
identity tuple, snapshot tiers — all in `docs/PROTOCOL.md`) and maps Pi's
grounded action surface onto them. It pins no new core protocol semantics.

**Full lifecycle, six commits:**

1. **Prose-author misroute caught and reversed** (`d9f3d1c`). The feature was
   tagged `[prose, adapter, foundation]` at `stage: drafting`. The prose black-
   box test passed on a shallow first reading (all dependent registries were
   settled in `done` features). The operator stopped the advance and asked
   "should this be going through the prose-author shortcut?" Re-applied
   honestly, the test fails: the scope line "Mapping from Pi session metadata
   to Patchbay session identity" hides a genuine semantic classification with
   real verification consequences. Stripped `prose`, logged the misroute with
   the four design questions named, returned without advancing. **This is the
   same misroute pattern that hit `feature-session-identity-adapter-contract`
   (2026-06-28) and that prompted the 2026-07-06 project-wide codification of
   the prose black-box test.** The catch here, one day later, validates that
   codification.
2. **Feature-design** (`f98057f`). Resolved four design questions interactively
   against the remote_pi **source** (`/home/agent/projects/remote_pi/pi-extension/`),
   not just the `.research/attestation/pi-extension.md` summary. The attestation
   summarized the Pi action surface; the source surfaced that the load-bearing
   question was `session_new` and inverted the initial intuition (see "The
   load-bearing decision" below).
3. **Advisory design review** (`060a97b`). Fresh-context cross-model deep review
   on `openai-codex/gpt-5.5` (high thinking). Verdict: Approve with comments.
   Three important findings + two nits, all applied to the design body
   in-stride (manifest column not consistently manifest-shaped; `pair_request`
   lane inconsistency; replacement-window/multi-surface-fan-out/`/reload`
   risks not explicit; `session_new != spawn` phrasing; generated Rust name
   mismatch).
4. **Implement** (`45e7ef4`). Wrote `docs/ADAPTER-PI.md` (9 sections) +
   Architecture forward-reference. Inline stride (prose/adapter doc, no
   coordination → inline, not the orchestrator). Self-verified all 6 Unit-1
   acceptance criteria + 2 Unit-2 criteria + the document-consistency checks.
5. **Substrate deep review — Block → fixed** (`d411b87`). Fresh-context
   cross-model deep review on `gpt-5.5`. Verdict: **Block**. One real blocker +
   two important findings, all correct (verified against canonical sources):
   - **B1 (blocker, single-source-of-truth violation):** the doc said the
     `question` `response_contract` was "reserved pending promotion." Canonically
     (`docs/PROTOCOL.md:157`, `:331`), `elicitation-response` is committed v0
     for `question` contracts and `question` is a committed contract kind;
     reserved contracts are `freeform`/`secret`/`function_result`/
     `file_attachment`/`structured_schema`/`service_request`. Fixed: rewrote
     §4/§7/§9 to frame `elicitation-response` + `question` as committed core,
     with the Pi adapter's lack of a non-approval question wire type as an
     **adapter-support limitation** (declares non-approval `question`
     Elicitations unsupported at delivery via `unsupported_command` until
     promoted) — a reserved adapter-level seam, not a reclassification of the
     core contract.
   - **I1 (important):** §2 outbound inventory omitted `model_select`/
     `thinking_level_select` (attested reconfiguration hooks). Added to §2 + §5.
   - **I2 (important):** manifest column used prose shorthands as "actual
     fields," but generated `AdapterCapability` fields are `streaming_support`/
     `snapshot_support`/`cancellation_support`/`session_replacement_support`.
     §4 intro now lists the actual generated field names and labels the
     table's shorter names as prose shorthand.
6. **Confirmatory review → done** (`9bbefeb`). Fresh-context cross-model
   confirmatory pass on `gpt-5.5`. Verdict: Approve. All three fixes confirmed
   against canonical sources; 7/7 regression check pass; zero findings at any
   level. Advanced to `stage: done`.

### The load-bearing decision: `session_new` is a replacement, not a `/clear`

This is the moment that justified the feature-design lane and is the most
transferable lesson of the session.

The Pi `session_new` wire action resets the session's conversation. The
operator's intuition (and the attestation summary's framing) was that it's
like a `/clear` on other harnesses — same session handle, wipe the transcript
in place. **That intuition is dangerously incorrect.** Investigating the
remote_pi source instead of the attestation inverted it:

- remote_pi's own code groups `session_new` with `fork`/`switch`/`reload` as
  **"session replacement"** (`index.ts:1148`, `:2232`, `peer_channel.ts:80`,
  `handlers.ts:209`).
- The SDK tears down the old `ExtensionContext` and marks it **permanently
  stale** — any later use throws `"stale after session replacement or reload"`
  via `assertActive()`. A `/clear` preserves the handle; Pi does the opposite
  (transcript event log rotated, old context unusable).
- The `story-session-replacement-harness` (done 2026-07-05) treats `/new`,
  `/resume`, `/fork` as one replacement bug-class and proves old-context-stale
  is the safety property.

That is exactly the session replacement that `session_generation.qnt` models
with a generation bump + tombstone, so late events/replies binding to the
pre-`new` context become `stale_event` audit records (`LateGenerationInert`)
rather than polluting the new conversation. The mapping: stable
`runtime_session_id` for the daemon slot (a `--continue` restart reuses the
session, not a replacement); `session_generation` bump on `session_new` and on
fresh-session restart (the `EXIT_DAEMON_FRESH_SESSION` path); **no** bump on
`session_compact` or `--continue` restart.

**Transferable lessons:**
1. The attestation summarized the action surface; the source surfaced the
   semantic classification. When a design question turns on *what a wire
   action semantically is*, go to source, not the summary.
2. The `/clear` analogy was the operator's first guess and the dangerous one.
   Naming-driven analogies ("new session" → "fresh/clear") can invert the
   actual semantics. The grouping comments in the source ("session
   replacement") were the disambiguator.
3. This is exactly the irreversible semantic commitment the prose black-box
   test exists to catch — encoding `session_new` as a same-generation clear
   through prose would have silently broken `GenerationMonotonic`/
   `LateGenerationInert` correlation.

### Two review passes, both with real findings

The feature went through two cross-model `gpt-5.5` fresh-context reviews
(design advisory, then substrate deep) plus a confirmatory pass. Both
substantive passes surfaced real defects the orchestrator (umans/GLM-5.2)
missed:

- **Design review**: manifest-column discipline, `pair_request` lane
  inconsistency, replacement-window risks — all structural/coverage gaps.
- **Substrate review**: the `question`-contract single-source-of-truth
  violation (a Blocker — the doc contradicted `docs/PROTOCOL.md`'s committed/
  reserved split), the omitted reconfiguration hooks, and the generated-field-
  name mismatch.

The confirmatory pass (scoped to the three fixes + a 7-point regression check)
came back clean Approve with zero findings — the right outcome for a fix
pass, and a signal that the fixes were surgical and complete rather than
over-reaching. **Lesson reinforced: two substantive passes (design +
implementation) earn their cost on foundation work; a single pass would have
shipped the `question`-contract contradiction.**

## Board state at end of session

`epic-foundation-hardening` (stage: implementing): **18/25 features done**.

### Done this session
- `feature-pi-parity-checklist`

### Other done features (17) — the foundation core
`feature-v0-walking-skeleton`, `feature-command-state-ssot`,
`feature-design-grant-shape`, `feature-design-terminal-commit-race`,
`feature-persistence-snapshot-model`, `feature-security-threat-model`,
`feature-session-identity-adapter-contract`, `feature-verification-contract-authority`,
`feature-formal-model-seed`, `feature-research-contract-tooling`,
`feature-research-formal-methods-tooling`, `feature-research-harness-action-surfaces`,
`feature-research-web-control-security`, `feature-bank-formal-methods-skills`,
`feature-operator-presence-and-action-inventory`,
`feature-foundation-doc-completeness-gaps`, `feature-protocol-idl-and-conformance`
(+ 4 child stories), + all 7 stories.

### Drafting (7) — what's left
- `feature-formal-model-realignment` — model-side follow-on to the O/O/E
  roll-forward (VR2 metadata, V1 transition-adjacency gap, new stated-normative
  models). Needs a design pass first. Depends on action-inventory + formal-
  model-seed (both done).
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
- `feature-ux-v0-acceptance` — v0 web cockpit UX acceptance criteria. Depends
  on v0-walking-skeleton + command-state-ssot + operator-presence.

### Backlog (13) — parked
Notably `idea-multi-human-coordination`, `idea-desktop-app-surface`,
`idea-agent-to-agent-mesh-seam`, plus research-handoff candidates.

## Next logical feature (recommendation)

**`feature-ux-v0-acceptance`** is now the strongest pick. It was the other
"checklist/criteria" feature unblocked by the action-inventory landing, and
`feature-pi-parity-checklist`'s §8 switch-decision checklist now references it
as a dependency (`feature-ux-v0-acceptance` must be met before the operator
can switch from Remote Pi to Patchbay). Closing it completes the "what does v0
look like?" picture and unblocks the Pi-parity migration decision end-to-end.

If the operator wants the model-side work next,
`feature-formal-model-realignment` is the direct follow-on to the O/O/E arc,
but it needs a design pass first (heavier lift; open questions: metadata
schema, strengthen-in-place vs. new model, authoring order).

The heavier semantic features (`feature-idempotency-ambiguous-execution`,
`feature-lease-scope-decision`, `feature-extension-seams-non-foreclosure`) are
independent of the O/O/E and Pi-parity arcs and can be picked up in any order.

## Reserved follow-ups filed in the feature body (not v0)

- **Harvest pass on remote_pi real-life behavior** — once the in-flight
  session-replacement / cross-session-leak / reconnect bugs close
  (`story-mobile-cross-session-history-leak` and kin in remote_pi's `.work/`),
  fold the debug evidence into a revision of the `docs/ADAPTER-PI.md`
  snapshot-tier and reconnect-parity sections (and possibly promote
  `research_refs:` bindings). Not bound to this feature now per operator
  direction 2026-07-06 — the in-flight bugs are not stable enough to cite as
  grounding; a harvest pass after they close is the disciplined way to fold
  real-life behavior in.
- **Supervisord-control `spawn` promotion** — a follow-on feature may add a
  small supervisor-RPC-backed spawn capability to the Pi adapter. Out of v0
  scope; `spawn` stays committed in the registry, Pi-adapter-unsupported at
  delivery.

## Key files (reboot reference)

- Foundation docs (authoritative): `docs/{VISION,ARCHITECTURE,PROTOCOL,
  VERIFICATION,GLOSSARY,UX,SECURITY,SPEC}.md`
- **Pi adapter parity checklist (new): `docs/ADAPTER-PI.md`** — the shipped
  artifact. 9 sections: purpose/scope, Remote Pi workflow inventory, session
  identity mapping, required v0 capabilities, discovery/send/stream/reconnect/
  status parity, commands-as-capabilities, unsupported/deferred features,
  migration-decision criteria, extension-pressure classification.
- Contracts: `contracts/` (proto, rust, ts, vectors, scripts)
- Formal models: `specs/seed/*.qnt`, `*.als` (need realignment — see
  `feature-formal-model-realignment`)
- Substrate: `.work/active/features/`, `.work/active/stories/`,
  `.work/active/epics/epic-foundation-hardening.md`
- Conventions: `.work/CONVENTIONS.md` (prose black-box test for lane routing)
- Grounding evidence: `.research/attestation/pi-extension.md` (Pi action
  surface); remote_pi source at `/home/agent/projects/remote_pi/pi-extension/`
- Session notes: `.work/session-notes/` (this file + prior banks)

## Routing discipline reminders for fresh context

- **umans exception is OFF.** Standard codex routing. Implementers and
  reviewers on `openai-codex/gpt-5.5` (or spark for light work). The umans
  orchestrator dispatches cross-model review to `gpt-5.5` and never spawns
  `umans/*` subagents (burns the 4-session gate).
- **Prose black-box test** is in `.work/CONVENTIONS.md` — apply to every
  `[prose]` candidate before routing. The catch this session (one day after
  codification) is the validation. Semantic commitments → feature-design.
- **Fresh-context cross-model review is the gate for stage advancement.** For
  foundation/feature work, two substantive passes (design advisory + substrate
  deep) earn their cost — this session's substrate review caught a Blocker the
  design review and the orchestrator both missed. A confirmatory scoped pass
  after fixes is the right close (not a full re-review).
- **When a design question turns on what a wire action semantically is, go to
  source, not the attestation summary.** The `session_new`-is-a-replacement
  classification came from remote_pi source comments and the session-
  replacement-harness safety property, not from the attestation.
- **`docs/ADAPTER-PI.md` consumes `docs/PROTOCOL.md` registries; it does not
  re-declare them.** If a registry value diverges, the canonical doc is
  correct and the adapter doc has a bug. The `question`-contract Blocker was
  exactly this class of drift.
