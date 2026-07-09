---
id: epic-foundation-hardening
kind: epic
stage: implementing
tags: [foundation]
depends_on: [story-fix-failurecode-execution-outcome-unknown]
parent: null
created: 2026-06-28
updated: 2026-07-09
gate_origin: null
release_binding: null
---

# Epic: Foundation hardening after adversarial review

Patchbay's initial docs establish the right product direction, but review found that implementation should not begin until the starting slice, protocol state machines, security model, persistence/snapshot semantics, and verification source-of-truth are sharpened.

This epic tracks the refinement program that converts the current foundation docs from a strong vision into an executable starting-state project.

## Review synthesis

Fresh-context top-down and bottom-up review converged on these concerns:

- Define a concrete v0 walking skeleton rather than an entire platform program.
- Consolidate command/session/failure state into one source of truth.
- Specify persistence, ordering, snapshots, and crash recovery before relying on durable acceptance.
- Define security principals, grants, device/browser identity, threat model, and revocation.
- Pin session identity/generation semantics so wrong-session prevention is implementable.
- Define adapter capability tiers and Pi parity without letting Pi become the core ontology.
- Decide how prose, formal models, generated contracts, and conformance vectors relate.
- Split UX presentation state into session liveness vs command delivery and make v0 screens actionable.
- Decide whether leases are v0 scope or deferred.

## Acceptance criteria

- Foundation docs define a buildable v0 slice with explicit exclusions.
- `docs/PROTOCOL.md` contains canonical state-machine and identity semantics rather than scattered enum-like lists.
- `docs/SECURITY.md` or equivalent defines v0 threat model and principal/grant posture.
- `docs/ARCHITECTURE.md` defines v0 persistence/topology/snapshot ordering assumptions.
- `docs/VERIFICATION.md` maps v0 models to normative artifacts and conformance checks.
- `docs/UX.md` has v0 cockpit acceptance criteria and separates session and command presentation states.
- Follow-on implementation can begin without inventing protocol semantics ad hoc.

## Lane routing discipline (2026-06-28)

A retrospective pass found that several `[prose]` features in this epic were misrouted — they discharged scope items involving genuine architectural/semantic choices through the collapsed prose-author lane, which skips the design gate, pre-mortem, and alternatives evaluation. The following have been retagged from `[prose]` to design features:

- `feature-session-identity-adapter-contract` — session generation semantics, adapter capability tiers.
- `feature-idempotency-ambiguous-execution` — `maybe_executed` state, idempotency-key semantics.
- `feature-lease-scope-decision` — leases in/out of v0, fencing design if in.
- `feature-ux-v0-acceptance` — screen inventory, navigation, timeline behavior.
- `feature-verification-contract-authority` — artifact authority order, generation targets.

Three reopened semantic decisions from already-done prose features were filed as explicit design features:

- `feature-design-terminal-commit-race` — the first-durable-terminal-commit-wins race rule.
- `feature-design-grant-shape` — grant field list and delegation seam.
- `feature-session-identity-adapter-contract` also carries the three-tier adapter snapshot model reopened from `feature-persistence-snapshot-model`.

### `[prose]` tag retired (2026-07-07)

A full audit of every item that ever carried `[prose]` found a 56% misroute rate (9 of 16: the 7 retagged above + `feature-foundation-doc-completeness-gaps` + `feature-pi-parity-checklist` caught 2026-07-06, + `feature-formal-model-realignment` and `feature-observability-operator-admin` caught 2026-07-07). Two were genuine prose (`feature-bank-formal-methods-skills`, `feature-extension-seams-non-foreclosure`). The remaining 4 slipped through to `done` still tagged `[prose]` — `feature-v0-walking-skeleton`, `feature-command-state-ssot`, `feature-persistence-snapshot-model`, `feature-security-threat-model` — and are under retroactive design-gate audit (epic `epic-retroactive-design-gate-audit`) because they are foundational decisions whose skipped alternatives/pre-mortem debt propagates downward.

The `[prose]` routing tag and the `prose-author` lane are retired project-wide (see `.work/CONVENTIONS.md`). All design-bearing work, including docs-only features, routes through `feature-design`; its Phase 4.5 applies the same work-nature test the old black-box gate did, but inside the design lane so the gate is never structurally skipped. Do not add `[prose]` to new items; legacy `[prose]` tags on done items are inert.

**Going forward:** when in doubt, prefer design — the design gate's cost is low; the cost of a semantic commitment made silently through prose is high.

## Review findings (2026-07-09)

Epic-level deep review (substrate mode, deep lane). All 27 children are `done` (27/27). Fresh-context cross-model reviewer `openai-codex/gpt-5.5` (high thinking) — a different model class from the umans orchestrator — ran both phases (completeness/complementary, then adversarial) in a single fresh-context pass; full multi-pass convergence was not reached (single endpoint pass). Host independently verified the load-bearing claims before classifying.

**Verdict**: Block (one blocker; one important; no nits). The epic is NOT advanced to `done`; it stays at `stage: implementing` until the blocker closes.

### Blocker

- **Generated `FailureCode` contract omits canonical `execution_outcome_unknown`** (`docs/PROTOCOL.md:356`, `contracts/proto/patchbay/operations.proto:71-85`). The failure vocabulary includes `execution_outcome_unknown` (added by `feature-idempotency-ambiguous-execution`, commit `60784e0`), `docs/UX.md:26-28` has a 5-row retry-safety table keyed on it × `idempotency_strength`, and `docs/VERIFICATION.md:193` references it as a presentation/audit signal — but the generated wire enum ends at `FAILURE_CODE_STALE_EVENT = 13` with no `FAILURE_CODE_EXECUTION_OUTCOME_UNKNOWN`. Generated contracts are a first-class epic deliverable (`feature-protocol-idl-and-conformance`), so prose/proto drift on a canonical term blocks epic completion. This is a temporal cross-child artifact no single child review could catch: the proto feature went `done` ~2026-07-06, the idempotency feature added the prose term ~2026-07-07.
  -> Item: `story-fix-failurecode-execution-outcome-unknown` (already filed; now bound as `parent: epic-foundation-hardening` + epic `depends_on` edge so the epic cannot advance until it closes). Story is `stage: drafting`; routes through `fix`/`implement` (single file + `buf generate` + drift check).

### Important

- **Automated drift checks do not catch prose-registry ↔ proto enum drift** (`.work/active/features/feature-protocol-idl-and-conformance.md:151`). All three contract checks (`check-vectors.mjs`, `check-generated-drift.mjs`, `check-models.mjs`) pass while the blocker drift exists: `check-generated-drift.mjs` only verifies gen-vs-proto, not proto-vs-prose. After the enum is fixed, add/extend a registry/proto consistency check for growing registries or this class recurs after any later prose registry edit.
  -> Item: `idea-proto-prose-registry-consistency-check` (backlog).

### Nits

- None.

### Acceptance criteria

6 of 7 epic acceptance criteria verified met with file:line evidence (v0 slice, canonical state machines, security threat model, persistence/topology, verification mapping, UX acceptance). The 7th — "follow-on implementation can begin without inventing protocol semantics ad hoc" — is unmet until the blocker closes (implementation would otherwise invent an ad hoc representation for a canonical failure term). All 9 review-synthesis concerns are covered by done child work. The sibling `epic-retroactive-design-gate-audit` (`done`) recorded zero faulty-assumption findings and does not change this epic's posture.

### Contract / formal-model check results

- `check-vectors.mjs`, `check-generated-drift.mjs`, `check-models.mjs`: all pass (and pass *despite* the blocker — see Important finding).
- `command_lifecycle.qnt` (`command_durability`, `boundary_dedup`, `no_accepted_to_completed`) and `session_generation.qnt` (`session_identity_tuple`, `labels_cannot_override_identity`, `generation_monotonic`): pass at documented step bounds. A full multi-property temporal command-lifecycle run timed out at 300s; full model-suite convergence not reached in this pass.

### Notes

Cross-child drift assessment beyond `execution_outcome_unknown`: none found. OperationKind, OperationState/CommandState, SubmissionOutcome/LocalSubmissionState, session connectivity/activity axes, ElicitationState, and response_contract.contract_kind all match between prose and proto (committed + reserved values). `UNSPECIFIED` proto values are wire-default encoding, not state drift. `docs/ADAPTER-PI.md` stays properly out of the core ontology (adapter-neutrality honored).
