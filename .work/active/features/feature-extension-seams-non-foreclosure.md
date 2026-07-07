---
id: feature-extension-seams-non-foreclosure
kind: feature
stage: drafting
tags: [foundation]
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

