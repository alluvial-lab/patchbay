---
id: backlog-core-generation-persistence
tags: [protocol, foundation]
created: 2026-07-27
---

# Core generation: wire-present but unset; rejection unimplemented

Docs-audit finding (2026-07-27): GLOSSARY/PROTOCOL claimed core generation is
assigned on restart and used to reject prior-incarnation snapshots/events.
Reality: snapshot materialization emits `core_generation: None`
(`server/src/state.rs:253-258`). Prose corrected to wire-present/reserved;
the capability itself (persistence + cross-incarnation validation) is open
v0.1+ work. It matters for the rejected-snapshot safety story once cores can
restart with ambiguous state sources (federation-adjacent).
