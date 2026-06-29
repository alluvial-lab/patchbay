---
id: feature-command-state-ssot
kind: feature
stage: review
tags: [prose, protocol, foundation, verification]
parent: epic-foundation-hardening
depends_on: [feature-v0-walking-skeleton]
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
---

# Feature: Define canonical command, session, and failure state machines

Review found command, session, and presentation states duplicated across README, architecture, protocol, verification, and UX with divergent members. Consolidate these into one source of truth.

## Scope

- Core command lifecycle states and allowed transitions.
- Control-surface-local states such as draft/submitting.
- Session liveness states such as live, idle, working, stale, offline, unknown.
- Failure/outcome vocabulary across transport, acceptance, delivery, execution, and presentation.
- Cancellation, expiration, supersession, running, and completion race semantics.

## Outline

Target files:

- `docs/PROTOCOL.md` — canonical source for command lifecycle, local submission states, session state axes, failure vocabulary, transitions, and race semantics.
- `docs/UX.md` — presentation guidance that derives UI labels from protocol state instead of redefining enums.
- `docs/ARCHITECTURE.md` — point architecture planes at the protocol registry/source of truth rather than restating state members.
- `docs/VERIFICATION.md` — reference canonical protocol variables for formal model obligations.
- `docs/GLOSSARY.md` — define ambiguous state terms.
- `README.md` — avoid stale state-list drift by pointing readers to `docs/PROTOCOL.md` for canonical state names.

Authoring decisions:

- Treat `docs/PROTOCOL.md` as the current prose source of truth until generated IDL/schema registries exist.
- Split core command state from control-surface-local submission state; local `draft`/`submitting` must not become durable command states.
- Split session status into connectivity/freshness and activity axes, then let UX compose labels like “Live idle” or “Stale working”.
- Make failure vocabulary layer-aware so timeout/offline/denied/rejected/failed do not collapse into one ambiguous bucket.
- Classify state decisions as committed v0 behavior, reserved extension seams, or rejected directions in the protocol text.

## Acceptance criteria

- `docs/PROTOCOL.md` owns the canonical state machines.
- `docs/UX.md`, `docs/ARCHITECTURE.md`, and `docs/VERIFICATION.md` reference rather than redefine them.
- `docs/GLOSSARY.md` defines ambiguous terms such as `superseded`, `unknown`, and `running`.
- The state machines are concrete enough to become TLA+/Quint variables and generated TS/Rust enums.

## Extension pressure test

- Coordinate with `feature-extension-seams-non-foreclosure`: classify decisions as committed v0 behavior, reserved extension seam, or explicitly rejected direction. Avoid encoding v0 assumptions as permanent architecture unless intentionally rejected.

## Implementation notes

- Files changed: `docs/PROTOCOL.md`, `docs/UX.md`, `docs/ARCHITECTURE.md`, `docs/VERIFICATION.md`, `docs/GLOSSARY.md`, `docs/VISION.md`, `README.md`.
- Tests added: none; docs-only prose feature.
- Discrepancies from design: also touched `docs/VISION.md` to avoid retaining a stale enum-like state list in the vision questions.
- Adjacent issues parked: none.
- Verification: proofread changed sections; used `rg` to confirm prior duplicated enum-like state lists were removed or converted to protocol references; confirmed `docs/PROTOCOL.md` owns concrete `CommandState`, `LocalSubmissionState`, `SessionConnectivityState`, `SessionActivityState`, failure vocabulary, transitions, and extension-pressure classification.
