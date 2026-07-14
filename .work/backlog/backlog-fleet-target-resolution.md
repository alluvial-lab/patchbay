---
id: backlog-fleet-target-resolution
kind: feature
stage: backlog
tags: [protocol, foundation]
parent: null
depends_on: []
release_binding: null
gate_origin: null
created: 2026-07-13
updated: 2026-07-14
---

# Backlog: OperationKind-aware target resolution (fleet spawn path)

## Source
Authority design review (R5 decision, revision 2). The existing `SessionRegistry`-backed `TargetResolver` rejects fleet spawn targets.

## Finding
After the grant check, the acceptance `submit` pipeline always calls `TargetResolver::resolve`. The existing `SessionRegistry` impl requires `adapter_id` + `runtime_session_id` (an existing session). A fleet/supervisor spawn Operation targets a scope where the session does NOT yet exist (the spawn creates it), so `resolve` returns `TargetNotFound`. **Authority can correctly authorize a fleet spawn and acceptance still rejects it at target resolution.**

This is an acceptance/sessions gap, not authority's: target resolution needs to be OperationKind-aware. Spawn needs a fleet/supervisor resolution path that does not require an existing runtime session (it targets the fleet, not a session); existing-session operations continue through `SessionRegistry`.

## Direction
Add OperationKind-aware target resolution: a `TargetResolver` (or a dispatching resolver) that routes `Spawn` to a fleet/supervisor resolution path (succeeds for valid fleet scopes) and other kinds to the existing `SessionRegistry` path. Scope as an acceptance or sessions feature. Until this lands, the spawn end-to-end path is blocked (authority's GrantCheck + grant model are still valuable and testable independently).

## Priority
Required for the spawn end-to-end path to work. Not blocking for the authority feature's GrantCheck/grant-model/proptests, but blocking for the vertical-slice descendant-grant reactor to be exercised live. Should land before v0.1.0 ships if spawn is in v0.1.0 scope (SPEC.md line 76 confirms fleet spawn authority IS in v0.1.0 scope).
