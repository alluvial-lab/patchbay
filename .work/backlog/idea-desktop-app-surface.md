---
id: idea-desktop-app-surface
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
tags: [ux, surface, extensibility]
---

# Parked idea: native desktop app as a future control surface

A native desktop app (macOS/Windows/Linux) is a plausible future Patchbay control surface, analogous to how a native Expo mobile app is already a reserved seam. v0 ships the responsive web cockpit first plus CLI; a desktop app is not v0. But the foundation should not foreclose it.

## Context

- VISION/SPEC already list "desktop" alongside web/CLI/mobile/notifications as future human control surfaces, and the shared TypeScript operator domain is explicitly designed so multiple surfaces reuse the same delivery/reconnect/session-state logic.
- The v0 two-process topology (`docs/ARCHITECTURE.md` "V0 process topology") makes a desktop app a natural later addition: it would be another control-surface client of the TS web server (or, depending on the web↔core seam design, possibly a direct client of the core), reusing the operator domain.
- Like the Expo app, a desktop app could add native affordances when they become load-bearing: system tray / dock integration, global hotkeys, native notifications, richer local cache, offline queued intent, OS-level window state.

## Why parked

- v0 is deliberately narrow: web cockpit + CLI, one operator, single-authority core.
- A desktop app is breadth, not a v0 requirement. Building it now would broaden the surface set without a concrete need.
- The extensibility discipline (`feature-extension-seams-non-foreclosure`) is the right place to ensure desktop is a reserved seam, not a v0 obligation.

## What this idea should influence

- `feature-extension-seams-non-foreclosure` should treat "additional human control surfaces (desktop, mobile, notifications, approval UIs)" as a reserved extension seam and ensure capability/registry design does not assume web+CLI only.
- The shared TypeScript operator domain should stay surface-neutral so a desktop shell (e.g. Tauri/Electron wrapping the web app, or a native shell reusing the operator domain) can adopt it later without forking protocol semantics.
- The web↔core protocol seam design (`feature-web-core-protocol-seam`) should not assume the browser is the only client of the web server; a desktop client is a peer control surface.
- Native desktop affordances (tray, hotkeys, native notifications, offline queue) stay out of the core protocol and belong to the desktop surface when it is built.

## Keep parked until

The operator identifies a concrete need for desktop-native affordances that the responsive web cockpit cannot satisfy (e.g. always-on system presence, global hotkeys, deep OS integration). At that point, scope it as a feature and route through design.
