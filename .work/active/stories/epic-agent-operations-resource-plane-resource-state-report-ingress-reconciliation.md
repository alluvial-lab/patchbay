---
id: epic-agent-operations-resource-plane-resource-state-report-ingress-reconciliation
kind: story
stage: done
tags: [adapter, protocol, storage]
parent: epic-agent-operations-resource-plane-resource-state
depends_on: [epic-agent-operations-resource-plane-resource-state-projection-replay, epic-agent-operations-resource-plane-capability-manifest-core-admission]
release_binding: null
gate_origin: null
created: 2026-08-03
updated: 2026-08-04
---

# Ingest authenticated resource reports and reconcile reconnects

## Checkpoint

Add the typed `ResourceReport` branch to adapter ingress, translate the generated
wire report into the validated core report, derive one atomic normalized
resource-state event, and fold it only after durable append. Implement reconnect
and adapter-loss degradation against per-view authoritative/partial/none
semantics while authenticating the exact adapter id and current adapter
generation.

## Acceptance evidence

- Malformed, cross-adapter, stale-generation, undeclared/mixed-kind,
  overclaimed/unknown-tier, schema-mismatched resource/projection, and invalid-
  payload reports reject before append or projection mutation.
- Authoritative snapshot reports upsert listed resources and tombstone omitted
  active members; partial reports update listed members and stale omissions;
  none reports carry no reconstructed members and stale cached state.
- Live delta reports mutate only explicitly named resources. Atomic replacement
  tombstones the old exact identity and registers its distinct same-adapter
  replacement in one durable event.
- Adapter disconnect marks every active owned resource stale in the same
  audited batch as session degradation; a fenced old delivery stream is inert.
- Partial append/fold failures recover by rebuilding the projection from the
  durable log rather than inventing in-memory success.
- Production ingress consumes the sibling manifest's exact
  `resource_capability` / `validate_resource_projection` API; it adds no
  fallback manifest fields or capability-derived authority.

## Ordering constraints

Consumes the resource projection/replay checkpoint. The parallel capability-manifest checkpoint owns declarations; this checkpoint
consumes its admission API, while retaining ownership only of state/tier fold
semantics and adding no manifest fields.

## Implementation notes

Added authenticated typed `ResourceReport` ingress under the shared
`CoreDecisionGate`. The server binds adapter id/generation to the current
attachment, requires exact manifest kind admission, enforces equal-or-weaker
snapshot tiers, and checks both payload/projection envelopes through
`validate_resource_projection` before normalization. The core normalizer emits
one stable-ordered `RESOURCE_STATE` event per valid report, derives
snapshot omissions by authoritative/partial/none semantics, keeps delta
omissions inert, validates atomic replacement, fences stale adapter generations,
and folds only after durable append. A fold failure rebuilds from the committed
log before the projection can be reused.

Adapter-generation advance stales unreported active records before installing
new evidence. Abnormal disconnect now composes session and resource degradation
sources into one existing `append_batch_audited` transaction with one
`ADAPTER_DETACHED` audit, then rebuilds both projections. Stream epoch/token
fences still make an obsolete disconnect inert.

Core tests cover all three completeness branches, delta omission, replacement,
new-generation degradation, stale-generation rejection, duplicate/malformed
reports, and durable replay. Server evidence covers authenticated manifest-bound
ingress, overclaimed tier/schema rejection without resource-state append,
durable projection, and resource staleness after stream loss.

Checkpoint verification: focused core resource tests and adapter-service tests
passed; `cargo check --workspace` and
`cargo clippy --workspace --all-targets -- -D warnings` passed.
