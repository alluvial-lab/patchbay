## Session bank — 2026-07-06 (O/O/E roll-forward through protocol-IDL)

**This is the reboot point.** A fresh context can pick up here. This session
completed two features and a convention, and left the board at 17/25 done.

## What this session accomplished

### 1. `feature-operator-presence-and-action-inventory` → `done`

The O/O/E (Operation/Observation/Elicitation) frame roll-forward into the 8
foundation docs. Full workflow: implement → 3-axis deep adversarial review
(gpt-5.5, cross-model from umans orchestrator) → 2 amendment cycles → final
re-review (READY) → substrate review (Phase 1 completeness, Approve with
comments) → done.

Key things that happened in this arc:
- **umans exception** (operator-authorized): implementer ran on
  `umans/umans-glm-5.2` for its 400K context window. **Exception is now cast off**
  — back to standard codex routing. The 400K justification no longer holds
  (corpus is ~75-95K tokens; gpt-5.5's 272K is sufficient).
- **`drive` → `instruct` rename** (operator-authorized): the OperationKind for
  "send prompt/input/steering content" was renamed to disambiguate from
  `Command`/`CommandState` (the retained checked-lifecycle term). The operator
  caught that a 07-04 decision to "drop drive" had been silently superseded
  during the frame shift and never recorded.
- **`feature-formal-model-realignment` filed** (stage: drafting): the model-side
  follow-on the doc honesty pass exposed. Three classes of misalignment: VR2
  (`@promotion` metadata drift — 16 seed properties still say `checked-normative`
  when docs now classify them `checked-model`); V1 follow-on
  (`command_lifecycle.qnt` doesn't verify the transition adjacency the docs now
  mark stated-normative); new stated-normative properties needing models
  (Elicitation, subscription, spawn authority, response→Elicitation correlation).
- Deep-review defects fixed: V1-V3/V4, G1-G3, C1-C2, nits, then VR1/VR3/VR4 +
  nits. All verified against primary sources (model files, research corpus).
- Final stage: `done` (not archived — parent epic still implementing).

### 2. Prose black-box test convention codified

After misrouting `feature-foundation-doc-completeness-gaps` as `[prose]`
(P1-P3 are semantic/architectural commitments, not prose polishing), codified
the epic's lane-routing discipline project-wide in `.work/CONVENTIONS.md`. The
rule: if a docs-only deliverable involves choosing between approaches, pinning
a semantic model, or making an architectural commitment, route to
`feature-design`, not `prose-author`. Origin recorded.

### 3. `feature-foundation-doc-completeness-gaps` → `done`

Closed the 3 P1-P3 completeness findings from the action-inventory review:
- P1: OperationKind registry gained a `Lifecycle notes` column (ported from
  design body §4, which the roll-forward had dropped when applying to PROTOCOL).
- P2: Elicitation terminal notification/reconciliation mechanism made explicit
  (terminal transitions ride the subscription stream; cursor/snapshot reconcile;
  late second answers rejected as stale, forcing resync).
- P3: Spawn descendant grant field list made concrete (normal grant instance,
  allowed OperationKinds explicitly enumerated — excludes spawn/attach, no
  parent_grant_id, (c) inherit-from-spawning-grant noted as reserved).

Routed through feature-design (not prose-author) per the new convention.
Single-pass implement + standard-lane review (Approve, zero findings).

### 4. `feature-protocol-idl-and-conformance` → `done`

The first artifact-producing feature under the epic. Created `contracts/`:
- **.proto package** (7 files: common, operations, observations, elicitations,
  sessions, authority, adapter) mapping all 16 PROTOCOL.md registries. Reserved
  kinds/contracts wire-present as named `RESERVED_*` enum values (forward-
  compatible without making them validatable). Payloads opaque `bytes` +
  `PayloadContentType`.
- **Generated Rust crate** (`patchbay-contracts`, prost) + **generated TS
  package** (`@patchbay/contracts`, Protobuf-ES). Both build. `buf generate`
  wired. Generated code committed.
- **12 conformance vectors** (JSON with structured envelope: property_id,
  promotion_status=draft, proto_fields_constrained, input, expected_outcome).
- **Traceability script** (`check-vectors.mjs`): validates vectors against
  property registry, generates VERIFICATION.md traceability table.
- **Generated-code drift check** (`check-generated-drift.mjs`): `buf generate`
  + `git diff --exit-code` — enforces the Generated Contracts principle.

Design decisions (locked with operator): Q1(c) spine+envelopes/opaque
payloads; Q2(b) package split; Q3(a) JSON vectors; Q4(b) document+wire-up
generation (the "easy win" — proves .proto is generatable, not just documented).

Review: deep-lane gpt-5.5 fresh-context. Initial Request changes (3 findings:
failure-vector operation_state contradiction with PROTOCOL.md pre-acceptance-
refusal semantics; reply-correlation mis-typed as Elicitation response instead
of Reply/Observation; missing drift check). All fixed inline; re-review READY.
Operator confirmed single fresh-context adversarial pass sufficient to advance
(two-phase convergence loop deferred — change surface smaller than O/O/E
roll-forward, and the single pass caught real defects).

## Board state at end of session

`epic-foundation-hardening` (stage: implementing): **17/25 features done**.

### Done this session
- `feature-operator-presence-and-action-inventory`
- `feature-foundation-doc-completeness-gaps`
- `feature-protocol-idl-and-conformance` (+ 4 child stories)

### Other done features (15) — the foundation core
`feature-v0-walking-skeleton`, `feature-command-state-ssot`,
`feature-design-grant-shape`, `feature-design-terminal-commit-race`,
`feature-persistence-snapshot-model`, `feature-security-threat-model`,
`feature-session-identity-adapter-contract`, `feature-verification-contract-authority`,
`feature-formal-model-seed`, `feature-research-contract-tooling`,
`feature-research-formal-methods-tooling`, `feature-research-harness-action-surfaces`
(the D2 research that spawned the reframe), `feature-research-web-control-security`,
`feature-bank-formal-methods-skills`, + all 7 stories.

### Drafting (8) — what's left
- `feature-formal-model-realignment` — model-side follow-on to the O/O/E roll-
  forward (VR2 metadata, V1 transition-adjacency gap, new stated-normative
  models). Needs a design pass first (open questions: metadata schema,
  strengthen-in-place vs. new model, authoring order). Depends on the
  action-inventory feature + formal-model-seed (both done).
- `feature-extension-seams-non-foreclosure` — extension seams + non-foreclosure
  rules. Depends on v0-walking-skeleton.
- `feature-idempotency-ambiguous-execution` — `maybe_executed` state,
  idempotency-key semantics. Depends on command-state-ssot + session-identity-
  adapter-contract.
- `feature-lease-scope-decision` — leases in/out of v0, fencing. Depends on
  v0-walking-skeleton + security-threat-model.
- `feature-observability-operator-admin` — operator/admin observability.
  Depends on v0-walking-skeleton + persistence-snapshot-model.
- `feature-pi-parity-checklist` — Pi migration + parity checklist. Depends on
  v0-walking-skeleton + session-identity-adapter-contract + (now-done)
  operator-presence-and-action-inventory.
- `feature-research-v0-stack-tooling` — v0 stack/tooling picks (research).
  Depends on research-contract-tooling.
- `feature-ux-v0-acceptance` — v0 web cockpit UX acceptance criteria. Depends
  on v0-walking-skeleton + command-state-ssot + (now-done) operator-presence.

### Backlog (13) — parked
Notably `idea-multi-human-coordination`, `idea-desktop-app-surface`,
`idea-agent-to-agent-mesh-seam`, plus research-handoff candidates.

## Next logical feature (recommendation)

**`feature-pi-parity-checklist`** or **`feature-ux-v0-acceptance`** — both were
unblocked by the action-inventory feature landing at done (it's in their
depends_on), and both are checklist/criteria features (lower-risk, close out
the "what does v0 look like?" picture before diving into the heavier semantic
features). Of the two, `feature-pi-parity-checklist` directly consumes the
OperationKind registry we just stabilized and would validate that the
rolled-forward foundation is actually derivable for adapter parity — a good
forcing function, similar to how protocol-idl tested derivability.

If the operator wants the model-side work next, `feature-formal-model-realignment`
is the direct follow-on to this session's O/O/E work, but it needs a design
pass first (heavier lift).

The heavier semantic features (`feature-idempotency-ambiguous-execution`,
`feature-lease-scope-decision`) are independent of the O/O/E arc and can be
picked up in any order; they're about lifecycle/authority details, not the
action inventory.

## Key files (reboot reference)

- Foundation docs (authoritative): `docs/{VISION,ARCHITECTURE,PROTOCOL,
  VERIFICATION,GLOSSARY,UX,SECURITY,SPEC}.md`
- Contracts (new): `contracts/` (proto, rust, ts, vectors, scripts)
- Formal models (need realignment): `specs/seed/*.qnt`, `*.als`
- Substrate: `.work/active/features/`, `.work/active/stories/`,
  `.work/active/epics/epic-foundation-hardening.md`
- Conventions: `.work/CONVENTIONS.md` (now includes prose black-box test)
- Session notes: `.work/session-notes/` (this file + prior banks)

## Routing discipline reminders for fresh context

- **umans exception is OFF.** Standard codex routing. Implementers and reviewers
  on `openai-codex/gpt-5.5` (or spark for light work).
- **Prose black-box test** is now in `.work/CONVENTIONS.md` — apply to every
  `[prose]` candidate before routing. Semantic commitments → feature-design.
- **Fresh-context adversarial review** is the gate for stage advancement. For
  substantial features, prefer the deep-review skill's two-phase order
  (completeness + adversarial as separate passes); for smaller features a
  single fresh-context adversarial pass is acceptable per operator judgment.
- The `contracts/` artifacts are committed; `cargo build` + `npm run build` +
  `check-vectors.mjs` + `check:drift` all pass and should be re-verified on
  pickup if the contracts are touched.
