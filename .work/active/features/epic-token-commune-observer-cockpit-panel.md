---
id: epic-token-commune-observer-cockpit-panel
kind: feature
stage: drafting
tags: [adapter, ux]
parent: epic-token-commune-observer
depends_on: [epic-token-commune-observer-snapshot-mapping, epic-token-commune-observer-polling-ingestion]
release_binding: null
gate_origin: null
created: 2026-08-05
updated: 2026-08-05
---

# token-commune cockpit resource panel and CLI projection

## Brief

The **surface-declared** token-commune resource panel, composed above the
conformance floor using Patchbay's shared presentation primitives, plus CLI
text-table projections over the same metadata. This is the UI-bearing feature of
the epic and the one net-new screen surface.

It delivers: adapter-shaped domain projection (capacity gauges / pool cards /
member-draw meter / fingerprint-watchdog views) nested beneath the canonical
Patchbay wrapper — using a local known decoder/compositor for the manifest-bound
projection schema, never loading adapter-supplied renderer code; grant-gated
member/admin view affordances as local defense-in-depth (upstream has no
read-scope distinction today — any member key reads all metadata); honest
stale/unknown/partial presentation that never styles stale data as live; and CLI
query/inspect projections as text tables over the same metadata.

It does NOT cover mutations, approval cards, re-onboarding elicitations, or
admin command affordances — those belong to the `control-attention` epic and are
out of scope for the read-only observer.

## Epic context

- Parent epic: `epic-token-commune-observer`
- Position in epic: **presentation consumer** — consumes the flowing resource
  state + Observations from `snapshot-mapping` / `polling-ingestion`. The
  conformance evidence in `conformance` proves the presentation honesty.

## Simplification opportunity

- Reuse the shared presentation-component layer (`StateBadge`,
  `CommandTimeline`, resource projection decoders, the cockpit-composition
  primitives from the resource-plane epic) — Tier-1 floor affordances come free;
  only the token-commune-specific domain projection decoder + panel layout is
  new.
- Do not duplicate allocation/quota/role logic in Patchbay; the panel renders
  adapter-reported metadata only.

## Foundation references

- `docs/UX.md` — surface-declared affordances compose above the conformance
  floor; the presentation conformance check enforces the floor structurally.
- `docs/ARCHITECTURE.md` — adapter-shaped domain projections compose above, not
  instead of, the canonical wrapper; Patchbay does not load adapter-provided
  renderer code.
- `docs/SECURITY.md` — member/admin visibility is governed by both upstream
  credentials and Patchbay grants; only metadata flows.
- Blueprint: `web-cockpit/` (resource projection decoders, cockpit composition
  from the resource-plane epic); `cli/`.

## Mockups

Net-new surface — mockups are **pending** the epic-level UI alignment pass
(`/ux-ui-design:screens epic-token-commune-observer-cockpit-panel`, plus a flow
if the panel spans a multi-step journey). Inherit design-system tokens from
`.mockups/design-system/tokens.css`. See the parent epic's `## Mockups` section.
Feature-design falls back to producing them if the epic pass has not run.

<!-- The design pass fills in the panel composition, the domain-projection
decoder, grant-gating rules, and implementation units. -->
