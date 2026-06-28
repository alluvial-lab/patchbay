---
id: idea-multi-human-coordination
created: 2026-06-28
updated: 2026-06-28
gate_origin: null
release_binding: null
tags: [security, coordination]
---

Consider a stretch-goal architecture path where Patchbay scales beyond the current single-human-operator v0 framing into multiple humans in the loop.

Context from the originating discussion:

- Current docs intentionally target a single human operator for v0, but the architecture should not paint itself into a corner.
- Future multi-human coordination might use Git-like collaboration semantics and/or concepts from `nklisch/skills/agent-coordination`.
- Patchbay could expose extendable surfaces so third-party tools cover gaps instead of forcing every collaboration workflow into core.
- The design question is not to make multi-human operation v0, but to preserve extension seams for future grants, audit, handoff, review, coordination, and third-party control surfaces.

Keep this parked until the foundation-hardening work clarifies v0 authority domains, principals, grants, leases, and adapter surfaces.
