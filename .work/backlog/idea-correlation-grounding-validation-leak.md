---
id: idea-correlation-grounding-validation-leak
created: 2026-07-30
updated: 2026-07-30
tags: [protocol, security, validation]
---

# Correlation grounding: schema_ref + AttentionRequired is not a validation engine

Surfaced during combined-surface vision review (round 5, both reviewers).
Independent of the vision's fate — it's a correctness finding about reusing
existing protocol vocabulary for purposes it doesn't support.

## The finding

The combined-surface vision proposed "minimal honest correlation grounding"
for linking a commissioned Operation to a work ledger item: a `schema_ref`'d
instruct-payload schema carrying the work item id, plus pane-side validation
flagging dangling/stale references as attention — citing Patchbay's own
`AttentionRequired` machinery as existing infrastructure "for exactly this."

Both reviewers verified this leaks, in three ways:

1. **`AttentionRequired` does not exist "for exactly this."** It is a signaling
   record (`observations.proto:82-87`): `(target_scope, state, correlations,
   revision_lsn)` — the mechanism by which adapters/actors *signal* attention
   on targets they own. It is a reporting primitive. Nothing in it scans
   instruct payloads, resolves work-item ids against a git ledger, or detects
   staleness. All of that logic would be new code in the pane — the one
   component that was unbuilt.
2. **`schema_ref` is an uninterpreted string.** `common.proto:120-124`:
   `PayloadEnvelope.schema_ref` is a bare `string`; the only place the protocol
   gives it semantics is `ResponseContract` for elicitations. Nothing validates
   an instruct payload against its `schema_ref` at the core. General Operation
   validation does not inspect payload content, content type, or `schema_ref`
   (`core/src/acceptance/pipeline.rs:411-478`). A mistyped or drifted
   `schema_ref` value itself fails silently.
3. **It cannot detect the doc's own headline failure case.** The cross-repo
   torn-state case (Operation `completed`, ledger item in patchbay's `.work/`,
   code in token-commune's repo, no atomic commit) requires the pane to know
   what "shipped" means in a *different repository*: which remote, which
   branch, deployed where, behind which release process. That is not pane-side
   validation of a dangling reference; it is a per-repo release-state
   integration — another adapter-shaped thing.

## Why it matters

A partial validator that validates only the cheap half (deleted/renamed item id
in one repo) is **arguably worse than none** — it manufactures exactly the
durable false confidence it was built to prevent. The cockpit shows `completed`
next to a green ledger item, now *with a validation badge that only validates
the cheap half*. This violates the project's own Fail Fast principle
(AGENTS.md: "unknown input is validated at system boundaries").

## What a real grounding would require (if ever built)

An honest external-reference contract containing at least:

- ledger kind and version;
- ledger-instance identity;
- repository or authority locator;
- work-item ID;
- observed item revision;
- optional intended code repository and artifact/commit/release reference.

Plus: who validates it, when validation runs and reruns, what "dangling,"
"stale," and "mismatched" mean, whether attention is durable core state /
adapter Observation / local UI decoration, and how every surface avoids
presenting command completion as work completion.

## Source

Combined-surface vision review round 5 (GPT finding 3 + Kimi finding 3),
against `contracts/proto/patchbay/common.proto:120-124`,
`contracts/proto/patchbay/observations.proto:82-87`,
`core/src/acceptance/pipeline.rs:411-478`, and `docs/PROTOCOL.md`.
