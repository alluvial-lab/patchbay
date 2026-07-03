---
id: idea-harvest-remote-pi-app-design
created: 2026-07-02
updated: 2026-07-03
tags: [ux, adapter, pi]
---

# Harvest remote_pi's app design (reimplemented in Patchbay's architecture)

remote_pi's Flutter app (`app/`) is clean-architecture with a strict Ports &
Adapters domain layer (`app/lib/domain/CLAUDE.md`): domain depends on nothing,
all other layers depend on domain — exactly Patchbay's stated principle. Three
designs are directly harvestable as *patterns* (reimplement in TS/web, do not
port Dart/Flutter):

## 1. Session state model — `app/lib/domain/session_state.dart`

- `UserMsgStatus { pending, confirmed, failed }` — a rebroadcast lifecycle:
  `pending` = sent over WS but Pi hasn't echoed; `confirmed` = Pi rebroadcast
  (or came from history/another device); `failed` = 15s elapsed without echo,
  user can retry.
- `TranscriptTurnView` with `AppTurnStatus { idle, working, awaitingTool,
  streaming, done, error, stale }` — the `stale` state and the
  pending→confirmed→failed discipline are precisely the "stale-state honesty"
  and "session liveness vs command delivery" separation that
  `feature-ux-v0-acceptance` and `docs/UX.md` call for. remote_pi already
  solved the UX state machine Patchbay is about to design.

## 2. Transcript projection seam — `app/lib/domain/transcript/transcript_projection.dart`

Three-layer projection:
- `RoomTurnProjection` — transport/room state
- `TranscriptTurnView` — app projection
- `AppTurnProjection` — UI projection

This is the concrete realization of "UI state is never authoritative; reconnect
paths reconcile against core snapshots" (`.agents/rules/`). Directly portable
as a pattern, even reimplemented in TS.

## 3. Protocol codegen tool — `tools/protocol-codegen/`

Manifest → IR → code generator with profiles and families. Patchbay's contract
toolchain (Buf/prost/Protobuf-ES) is different, but the *design* of a
manifest-driven, multi-target codegen with a checked-in IR is the same shape —
useful as a reference for how remote_pi solved the generated-contracts problem
Patchbay is now designing in `feature-protocol-idl-and-conformance`.

## What does NOT harvest

- Flutter/Dart UI widgets, `go_router`, `ChangeNotifier` viewmodels —
  framework-specific. Patchbay v0 is responsive web (single-file HTML/CSS/JS
  per the ux-ui-design skills). Reimplement in TS/web, do not port Dart.

## When to pick up

Item 1+2 directly inform `feature-ux-v0-acceptance` (the v0 screen/state
design) — pick up when that feature's design pass runs. Item 3 informs
`feature-protocol-idl-and-conformance` Q3 (buf generation) as a reference
design. Reference the app at `/home/agent/projects/remote_pi/app/`.
