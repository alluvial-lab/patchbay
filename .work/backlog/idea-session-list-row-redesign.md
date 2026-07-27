---
id: idea-session-list-row-redesign
tags: [ui, cockpit, idea]
created: 2026-07-27
---

# Session-list row: unclear hierarchy + path wrapping

Dogfooding report (2026-07-27): each row reads "<model> · <name> · <cwd>"
with no visual hierarchy — the operator can only *infer* the first item is
the model and can't tell what "patchbay" refers to (session name? project?).
The cwd wraps after the first "/", mangling the path.

Fix direction: explicit labeled/typographic hierarchy (name primary, model as
badge, cwd secondary with nowrap+ellipsis or basename+tooltip), and activity
state kept visible. Mobile impact is HIGH: at ≤760px the session list is the
home screen. Mockup-first per convention when picked up.
