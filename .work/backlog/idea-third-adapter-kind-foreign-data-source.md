---
id: idea-third-adapter-kind-foreign-data-source
created: 2026-07-30
updated: 2026-07-30
tags: [protocol, adapter, seam]
---

# Third adapter kind: foreign data source Patchbay reads out of git

Surfaced during combined-surface vision review (round 5, both reviewers). This
finding is independent of the vision's fate — it's a real protocol-seam gap.

## The finding

Patchbay's adapter ontology has two target categories:

1. **Runtime sessions** (the Pi adapter) — you deliver `instruct`/`cancel`/
   `interrupt` Operations against them; they declare OperationKinds, stream
   transcripts, report Observations.
2. **Operational resources** (the token-commune adapter, per
   `epic-agent-operations-resource-plane`) — admitted only when they
   "materially affect agent capability/availability or require operator action
   to keep agent work operating"; arbitrary service telemetry is explicitly out
   of bounds (SPEC.md:40).

A **git-backed Markdown work ledger** (Workbench, the agile-workflow substrate,
a JIRA mirror) is neither. You do not `instruct` a ledger; it has no
OperationKinds to declare; its `AdapterCapability` manifest would be empty in
every field the proto defines. It is not an operational resource under SPEC's
own admission rule — a work item's stage doesn't govern agent availability.

So if Patchbay ever renders an external work ledger in the cockpit, it needs a
**third category** — a projection-source contract for foreign data sources
Patchbay reads out of git (or elsewhere), with its own authority boundary,
schema versioning, snapshot semantics, and reconciliation model. This seam kind
does not exist in the protocol, and `epic-agent-operations-resource-plane`
explicitly leaves "exact wire registries and presentation extension mechanics"
for future design.

## Why it matters

Calling a ledger an "adapter instance" (as the combined-surface vision did)
borrows Patchbay's adapter-neutrality identity for continuity it hasn't earned.
The honest category is a new projection-source contract — a seam *kind* the
protocol doesn't have, requiring the extension-pressure ceremony (registry,
classification, conformance vectors) before it's real.

## Open design questions (for when pressure materializes)

- Canonical ledger-instance and item identity (item id alone is insufficient
  across repos, clones, branches, renamed items).
- Item revision/cursor semantics.
- The minimum common projection (if any) vs. adapter-specific UI modules.
- Whether adapter-specific UI code is allowed (the current architecture has
  only reserved a plugin mechanism).
- Failure and stale-state behavior.
- Why this does not turn Patchbay into a workflow substrate (which SPEC.md:94
  excludes).
- Whether the generic seam is real adapter-neutrality or a rename of "render a
  Markdown file" — the genericity claim is untestable without a third consumer.

## Source

Combined-surface vision review round 5 (GPT + Kimi K3), against
`contracts/proto/patchbay/adapter.proto`, `docs/SPEC.md:40,94,189`,
`docs/ARCHITECTURE.md:33-46`, and
`.work/active/epics/epic-agent-operations-resource-plane.md`.
