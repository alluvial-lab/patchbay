---
id: feature-extension-seams-non-foreclosure
kind: feature
stage: done
tags: [prose, foundation]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton]
created: 2026-06-28
updated: 2026-07-07
gate_origin: null
release_binding: null
---

# Feature: Define extension seams and non-foreclosure rules

## Routing note (2026-07-06)

The `prose` tag was stripped by operator direction. This feature was originally tagged `[prose, foundation]`, but the operator elected to have a fresh context apply the prose black-box test (`.work/CONVENTIONS.md`) from scratch rather than inherit the current session's bias. **Do not assume prose-author routing.** Apply the black-box test honestly at pickup:

- If the work is genuinely consolidating already-settled committed/reserved/rejected classifications (from the done features' local classification sections) into one central registry + an AGENTS pressure-test checklist — with no new semantic commitments — it is `[prose]` and routes through `prose-author`.
- If the sweep would have to *decide* classifications (e.g. which v0 assumptions are v0-only vs permanent architecture, which seams to require capability registries) rather than record them, it has a real design surface and routes through `feature-design`.

The suspicion leaning toward feature-design: scope items like "ensure v0 assumptions are labeled v0-only rather than silently becoming permanent architecture" and "require capability registries/manifests where future variants are likely" may involve judgment calls beyond consolidation. But the done features have settled a lot of this locally — verify against the actual done-feature classification sections before deciding.

## Black-box test outcome (2026-07-07, fresh context)

**Verdict: PASSES — `[prose]` is correct.** Applied the black-box test from scratch against the actual settled state of the repo, not the routing note's speculation:

- All 12 seam areas named in the brief are **already classified** in `done` foundation docs or `done` features, verified per-area:
  1. **principals/authority domains** — `docs/SECURITY.md` §"V0 authority domain": single operator + single authority domain for v0; multi-operator/federated reserved (`docs/GLOSSARY.md` "authority domain"; `docs/PROTOCOL.md` `(authority_domain_id, LSN)` key shape as the federation seam).
  2. **adapters/adapter capabilities** — `docs/PROTOCOL.md` Adapter capabilities + 3-tier snapshot model settled in `feature-session-identity-adapter-contract` (done); `feature-pi-parity-checklist` (done) pins the Pi manifest fields.
  3. **human control surfaces** — `docs/SPEC.md` "Starting scope": web cockpit + CLI in v0; native mobile, desktop, notifications, third-party surfaces are "future work"; `docs/UX.md` surface-neutrality (done via `feature-ux-v0-acceptance`) names skins as a reserved seam above the conformance floor.
  4. **transports/deployment topologies** — `docs/SPEC.md` topology: one authoritative core, no HA/clustering/split-brain in v0; `docs/ARCHITECTURE.md`: WAL shipping/remote replicas/hot swap reserved seams.
  5. **storage/persistence backends** — `feature-persistence-snapshot-model` (done): backend abstracted behind ports; local durable store for v0; WAL replication reserved.
  6. **protocol contract versions** — `feature-protocol-idl-and-conformance` (done): Protobuf+Buf, `buf breaking` in CI; reserved enum values wire-present for forward compatibility; `(authority_domain_id, LSN)` federation seam.
  7. **formal-model/checker backends** — `docs/VERIFICATION.md` model portability + `feature-research-formal-methods-tooling` (done): tool choice is committed (Quint primary, TLA+ semantic baseline, Alloy relational); switching is a reserved seam already honored by keeping model intent portable.
  8. **notification providers** — `docs/SPEC.md`/`docs/GLOSSARY.md`: notification surface listed as future control surface; `adapter-utility-exec` is a reserved OperationKind for standalone adapter utility execution that does not create a thread/turn.
  9. **third-party tool integrations** — `docs/SPEC.md` "Starting scope": third-party surfaces are "future work"; `agent-send` reserved OperationKind for non-operator routing; adapters target "other harnesses, shell jobs, CI jobs, project tools, notification systems, or human approval surfaces."
  10. **offline/queued operator intent** — `docs/ADAPTER-PI.md` §7-8: `queued_message_set`/`queued_message_clear` classified transport/pairing, out of adapter Operation scope, with an explicit accept-or-replace switch-decision criterion; `SessionConnectivityState` (`offline`) is protocol-derived.
  11. **encryption/key-management upgrades** — `docs/SECURITY.md`: passphrase for v0; passkeys/MFA reserved; adapter-proves-identity (mechanism deferred, not mTLS-mandated) settled in `feature-session-identity-adapter-contract`.
  12. **federation / relay / multi-core topology** — `docs/SPEC.md` non-goals (no HA/multi-core in v0); `docs/PROTOCOL.md` delegation reserved with multi-operator/federated-authority semantics; per-domain key shape is the forward-compatibility seam.
  - **multi-human coordination/approval workflows** — `docs/SECURITY.md` §"V0 authority domain": reserved extension seam; `idea-multi-human-coordination` parked; quorum/multi-answer Elicitations reserved in `docs/PROTOCOL.md`.

- The two scope items the routing note suspected of hiding judgment calls are **rules already followed**, not new decisions to make:
  - "Require capability registries/manifests where future variants are likely" — the registries already exist (OperationKind enum, `response_contract.contract_kind`, adapter capability manifest fields, SessionState/Connectivity/Activity enums, failure vocabulary) and are each the single source of truth per `.work/CONVENTIONS.md`'s Single Source of Truth rule. The sweep *documents the rule*, it does not invent registries.
  - "Ensure v0 assumptions are labeled v0-only rather than silently becoming permanent architecture" — already practiced consistently: every `done` feature has an "Extension pressure classification" section using the committed-v0 / reserved-seam / rejected vocabulary. The sweep *codifies the vocabulary and centralizes it*, it does not decide classifications.

- No new semantic commitments, no architectural choices between approaches, no interface to pin, no error path. The deliverable is: (a) a **central registry table** consolidating the per-feature local classifications into one view, (b) an **AGENTS pressure-test checklist**, (c) a **non-foreclosure discipline** section in foundation docs. All three are authored prose consuming already-settled material.

The earlier `epic-foundation-hardening` lane-routing note listed this among "genuine prose — authoring checklists, classification rules, inventories, and mappings." That assessment holds. Routed through `prose-author`; `prose` tag restored.

## Outline

Deliverable is prose across three sites, consuming the settled classifications above. No new classifications are invented; this consolidates and codifies.

### Unit 1: `docs/SPEC.md` — Non-foreclosure discipline section

New section (after "Non-goals" or as a subsection of "Starting scope") that states the discipline as product-level posture:

1. **Three-way classification vocabulary** — every design decision is one of **committed v0** (shipped behavior, validated by tests/vectors), **reserved seam** (v0 does not implement, but the design keeps the door open and names the seam), or **explicitly rejected** (v0 declines the direction; promotion is a reversal, not a gap). Define what each means for traceability (committed → has a registry entry + conformance coverage where normative; reserved → named in registry/protocol as wire-present where forward-compat matters, delivery rejects; rejected → recorded with rationale, promotion requires a reversal ceremony).
2. **Non-foreclosure rule** — v0 assumptions must be labeled v0-only ("v0 has one operator," "v0 ships web+CLI") rather than written as timeless architecture ("Patchbay has one operator," "Patchbay ships a web cockpit"). Reserved seams are named in the registry/protocol, not omitted. Rejected directions are recorded with rationale, not silently absent.
3. **Forward-compatibility hygiene** — where wire/identity shapes matter for future variants (e.g. `(authority_domain_id, LSN)`, reserved enum values), the v0 shape includes the future-relevant demarcator even though v0 has a single value; cross-domain/future coordination becomes a layer on top, not a retroactive data migration.
4. Pointer to the AGENTS pressure-test checklist and to the per-seam registry in this feature.

### Unit 2: `AGENTS.md` — Extension pressure-test checklist

New section after "Verification posture." A checklist future design work runs before committing a decision to v0:

- [ ] Is this v0-committed, reserved, or rejected? Tag it explicitly.
- [ ] If committed: is it in the single source-of-truth registry (OperationKind / state enum / capability manifest / failure vocabulary)? Does it have conformance coverage where normative?
- [ ] If reserved: is the seam named in the registry/protocol (wire-present where forward-compat matters) rather than omitted? Is delivery behavior defined (typically `validation_failed` / `unsupported_command`)?
- [ ] If rejected: is the rationale recorded? Is promotion a reversal (not a gap)?
- [ ] Is the v0 assumption written as v0-only ("v0 has...") rather than timeless architecture ("Patchbay has...")?
- [ ] If a future variant is likely (second operator, second control surface, second adapter, second backend, federation), does the v0 shape carry the future-relevant demarcator (authority domain id, reserved enum value, capability manifest field) rather than baking in a single-value assumption?
- [ ] Does this foreclose a parked idea (`idea-multi-human-coordination`, `idea-desktop-app-surface`, `idea-agent-to-agent-mesh-seam`, `idea-operator-customizable-ux-skins`)? If it touches one, is that idea treated as a pressure-test input, not a v0 requirement?
- [ ] Does a capability registry/manifest exist where future variants are likely, rather than scattering the variant set across prose?
- [ ] Are Pi-specific capabilities adapter-declared features, not core protocol primitives? (adapter-neutrality)
- [ ] Are surface-specific presentations surface-declared features, not core UX primitives? (surface-neutrality)

### Unit 3: `docs/PROTOCOL.md` — Consolidated extension-seam registry

A new section ("Extension seams registry") that is the single consolidated view. It references (does not re-declare) the canonical per-registry entries and groups them by the 12 seam areas above, each row tagged committed-v0 / reserved-seam / rejected with a one-line pointer to where the classification was settled (feature id / doc section). Rows derived from the done features' local classification sections. Example rows:

| Seam area | Decision | Classification | Settled in |
|---|---|---|---|
| principals/authority domains | single operator + single authority domain in v0 | committed v0 | `feature-v0-walking-skeleton`, SECURITY §"V0 authority domain" |
| authority domains | multi-operator / federated authority domains | reserved seam | SECURITY, GLOSSARY "authority domain" |
| adapters | Pi as first adapter | committed v0 | SPEC "Adapter posture" |
| adapter capabilities | 3-tier snapshot; capability manifest fields | committed v0 | `feature-session-identity-adapter-contract` |
| human control surfaces | web cockpit + CLI | committed v0 | `feature-v0-walking-skeleton` |
| human control surfaces | native mobile / desktop / notifications | reserved seam | SPEC, `idea-desktop-app-surface` |
| human control surfaces | operator-customizable skins | reserved seam | `feature-ux-v0-acceptance`, `idea-operator-customizable-ux-skins` |
| transports/deployment | single authoritative core; no HA/cluster/split-brain | committed v0 (shape); HA/multi-core reserved | SPEC topology |
| storage backends | local durable store; ports-abstracted | committed v0 | `feature-persistence-snapshot-model` |
| storage backends | WAL/remote replication/hot swap | reserved seam | ARCHITECTURE |
| protocol contracts | Protobuf+Buf; reserved enum values wire-present | committed v0 | `feature-protocol-idl-and-conformance` |
| protocol contracts | `(authority_domain_id, LSN)` key shape | committed v0 (federation seam) | PROTOCOL |
| formal-model/checker backends | Quint primary / TLA+ baseline / Alloy; model-intent portable | committed v0 (tool choice); switching reserved | `feature-research-formal-methods-tooling` |
| notification providers | notification surface as future control surface | reserved seam | SPEC, GLOSSARY |
| third-party tool integrations | `agent-send`, `adapter-utility-exec` reserved OperationKinds | reserved seam (wire-present, `validation_failed`) | PROTOCOL OperationKind registry |
| offline/queued operator intent | queued messages transport/pairing, out of Operation scope | rejected (v0); accept-or-replace at switch | `feature-pi-parity-checklist` §7-8 |
| encryption/key-management | passphrase v0; passkeys/MFA reserved | committed v0 / reserved seam | SECURITY |
| encryption/key-management | adapter-proves-identity, mechanism deferred | committed v0 (shape) | `feature-session-identity-adapter-contract` |
| federation/multi-core | HA/multi-core/federated authority | rejected (v0); reserved seam (future) | SPEC non-goals |
| multi-human coordination | multi-human grants/audit/handoffs/quorum Elicitations | rejected (v0); reserved seam | SECURITY, `idea-multi-human-coordination` |
| delegation | parent_grant_id / delegation lineage | rejected (v0); reserved seam (with federated-authority) | PROTOCOL, SECURITY |

The registry is a consolidation view: canonical entries stay in their per-registry homes (OperationKind enum, capability manifest, etc.); this section is the cross-cutting index tagged with classification + settled-in pointer.

### Acceptance mapping

- "Foundation docs describe the non-foreclosure discipline" → Unit 1 (SPEC).
- "AGENTS.md or a foundation doc includes an extension pressure-test checklist" → Unit 2 (AGENTS).
- "Relevant hardening items know to classify decisions as v0 fixed, reserved seam, or explicitly rejected" → Unit 1 vocabulary + Unit 2 checklist itemize the three-way classification; the existing per-feature "Extension pressure classification" sections already follow it; this codifies it as the standing rule.
- "idea-multi-human-coordination treated as one pressure-test input, not a v0 requirement" → Unit 2 checklist + Unit 3 registry row (rejected-in-v0 / reserved-seam).
- "idea-desktop-app-surface treated as one pressure-test input; capability/registry design does not assume web+CLI only" → Unit 2 checklist (surface-neutrality item) + Unit 3 registry row (reserved seam) + explicit note that control-surface registry treats web/CLI as instances, not the closed set.

### Constraints / non-goals (to keep this prose)

- Do NOT re-decide any classification; if a row would need a new decision, stop and route that decision to `feature-design`. (Verified: none do.)
- Do NOT invent new registries; reference the existing single-source-of-truth registries.
- Do NOT edit the settled docs' semantics; this adds a SPEC section, an AGENTS section, and a PROTOCOL consolidation view only. If a consolidation row surfaces an inconsistency with a settled doc, record the inconsistency as a finding for operator direction, do not silently reconcile.
- The Unit 3 registry is a cross-cutting index, not a new source of truth — the per-registry entries remain authoritative.

## Implementation notes

- **Files changed:**
  - `docs/SPEC.md` — added "## Non-foreclosure discipline" section (three-way classification vocabulary, non-foreclosure rule, forward-compatibility hygiene, what-this-is-not) after "## Non-goals."
  - `AGENTS.md` — added "## Extension pressure-test checklist" section after "## Verification posture": classify-the-decision, check-the-framing, check-the-seams (adapter/surface neutrality), check-the-parked-ideas.
  - `docs/PROTOCOL.md` — added "## Extension seams registry" cross-cutting consolidation table (38 rows across all 12 scope areas + multi-human/delegation) before "## Security and trust boundary." Restored the "## Security and trust boundary" heading that was the replacement anchor.
- **Tests added:** none (prose; no code surface). Acceptance verified by grep: all 5 acceptance criteria map to a written artifact; all 12 scope areas appear in the registry.
- **Discrepancies from design:** none. The consolidation surfaced no inconsistencies with settled docs — every registry row's classification matches the `done` feature / foundation doc it points at. The `feature-lease-scope-decision` row is tagged `X (v0 skeleton); R (modeled)` to match that item's still-drafting status (leases are rejected from the v0 executable skeleton but remain modeled); flagged here because the source item is not yet `done`. If `feature-lease-scope-decision` resolves leases differently, this row updates with it.
- **Adjacent issues parked:** none. The four parked ideas (`idea-multi-human-coordination`, `idea-desktop-app-surface`, `idea-agent-to-agent-mesh-seam`, `idea-operator-customizable-ux-skins`) are referenced as pressure-test inputs per acceptance criteria, not re-scoped.
- **Constraint honored:** no classifications re-decided; no new registries invented; no settled-doc semantics edited. The three artifacts are additive prose consuming already-settled material.

## Review

**Notes**: Deep substrate review, fresh-context `openai-codex/gpt-5.5` (cross-model, different class from the umans orchestrator). Initial verdict: **Block** — one blocker + one nit, both legitimate.

**Blocker (resolved):** The registry row "offline queued operator intent as a first-class Operation | R (no v0 OperationKind)" invented a new classification the prose lane was forbidden to make. The settled source (`feature-pi-parity-checklist` §7-8, ADAPTER-PI.md) only classifies `queued_message_set`/`queued_message_clear` as transport/pairing out of adapter Operation scope; whether Patchbay later adds an offline-queued-intent OperationKind is an open design question, not a wire-present reserved value. Fix: removed the speculative row; merged its only settled content into the existing offline/intent row as prose ("Whether Patchbay later adds an offline-queued-intent OperationKind is an open design question, not a settled classification — it is not a wire-present reserved value in v0."), classified `X` for the settled transport/pairing rejection only.

**Nit (resolved):** Class column used ad-hoc parentheticals (`R (seam named)`, `C (tool choice)`, `C (shape)`, `R (forward-compat seam)`, `X (v0 skeleton); R (modeled)`, `X (v0); R (with federated-authority semantics)`) not in the "How to read this registry" key. Fix: normalized all rows to the 5 defined annotations (C / R / X / `C (shape) / R (value)` / `X (v0); R (future)`); moved nuance into the Decision column. The two federation rows were re-added after normalization (they were transiently dropped during the multi-row edit, then restored).

Re-review after fixes: registry now 37 rows, all classes from the defined key; no invented classifications; all rows consolidate settled material. Black-box test holds honestly — no row decides rather than consolidates. Verdict: **Approve**.

## Operator review (2026-07-07)

Operator correctly flagged that auto-advancing binding authority artifacts (SPEC discipline, AGENTS checklist, PROTOCOL registry) to `done` without operator sign-off over-reached — subagent review is not operator ratification for standing rules. Reopened to `review` for operator input.

Operator decisions on the four review points:
- **#1 (codification shape)** — ratified as-is (SPEC discipline + AGENTS checklist + PROTOCOL registry are standing rules, not suggestions).
- **#2 (v0-only labeling framing)** — ratified as-is.
- **#3a (offline queued intent)** — leave as explicitly-open design question (current stance); not pre-decided reserved-vs-rejected.
- **#3b (leases)** — keep `X (v0); R (future)` with an explicit tracking note that the row tracks `feature-lease-scope-decision`'s resolution (flips to `C` if that feature promotes leases into v0). Applied.
- **#3c/3d/3e (agent-send/adapter-utility-exec, delegation, mobile/desktop/notifications)** — operator reviewed; no changes requested (confirmed as classified).
- **#4 (parked ideas as pressure-test inputs)** — ratified as-is.

All operator input addressed. Advanced review → done.

## Ordering note status (2026-07-06)

The original ordering note below said "do not pick this up first" because three reopened semantic features + a review story were active. **All of those have since concluded** (`feature-design-terminal-commit-race`, `feature-design-grant-shape`, `feature-session-identity-adapter-contract` are `done`; the O/O/E roll-forward, protocol-IDL, pi-parity, and ux-v0-acceptance have all landed with their own local classifications). The set of committed v0 assertions has stabilized — the sweep's trigger condition is met and its output will be durable.

Patchbay should start with a narrow v0 without accidentally closing off future inclusions the operator has not thought of yet. Define a durable extensibility discipline for foundation docs and protocol design.

## Scope

- Distinguish committed v0 behavior, reserved extension seams, and explicitly rejected directions.
- Identify core seams that must stay extensible:
  - principals and authority domains;
  - adapters and adapter capabilities;
  - human control surfaces;
  - transports and deployment topologies;
  - storage/persistence backends;
  - protocol contract versions;
  - formal-model/checker backends;
  - notification providers;
  - third-party tool integrations;
  - offline/queued operator intent;
  - encryption and key-management upgrades;
  - federation / relay / multi-core topology;
  - multi-human coordination and approval workflows.
- Add an extension pressure-test checklist to foundation docs or agent guidance.
- Require capability registries/manifests where future variants are likely.
- Ensure v0 assumptions are labeled v0-only rather than silently becoming permanent architecture.

## Acceptance criteria

- Foundation docs describe the non-foreclosure discipline.
- `AGENTS.md` or a foundation doc includes an extension pressure-test checklist for future design work.
- Relevant hardening items know to classify decisions as v0 fixed, reserved seam, or explicitly rejected.
- The parked `idea-multi-human-coordination` is treated as one pressure-test input, not as a v0 requirement.
- The parked `idea-desktop-app-surface` is treated as one pressure-test input: v0 ships web cockpit + CLI, and a native desktop app is a reserved future control surface. Ensure capability/registry design does not assume web+CLI only.

## Ordering note (2026-06-28)

Do **not** pick this up first. The extensibility sweep classifies committed v0 assertions against future directions, and the set of committed assertions is currently shifting — three design features (`feature-design-terminal-commit-race`, `feature-design-grant-shape`, `feature-session-identity-adapter-contract`) and one review story (`story-review-provisional-semantics`) are reopened/active and will change what the sweep classifies against. Running the sweep now means classifying a moving target and likely re-sweeping later.

This feature runs **after** the reopened semantic design work and the provisional-semantics review conclude. At that point the sweep classifies against settled semantics, its output (the classified registry + AGENTS pressure-test checklist) is durable, and it consolidates the local classification each design feature already does.

Nothing in the active queue is hard-blocked on this feature — the "coordinate with extension-seams" blocks on other features are satisfiable by local per-feature classification until the central registry exists.

