---
id: idea-tlc-temporal-workaround
kind: backlog
created: 2026-07-01
updated: 2026-07-01
tags: [verification, foundation]
research_refs: []
---

# Backlog: Pursue TLC-checkable form for temporal properties (Apalache-temporal experimental risk)

Filed from the deep review of `feature-formal-model-seed`. All promoted temporal properties (TerminalFinality, GenerationMonotonic, LateGenerationInert, PreAppendTerminalChoice, LsnDeterminesTerminalWinner, RetryReusesIdAndKey, RetryAfterTerminalReturnsExisting) are checked via Apalache's default backend (`echo y | quint verify --temporal`), which warns its temporal support is "experimental and might give incorrect results." The `--backend tlc` path fails because the Quint→TLA+ compilation of `next()`-in-`always()` emits `[](\A cmd: x'=x)`, which TLC rejects with `[] followed by action not of form [A]_v` (TLA+ requires `[]` to wrap an action subscripted by vars).

This is a residual risk for safety-claiming properties. The current mitigation (all properties are `always(...)` safety, not `eventually` liveness) is the conservative end of Apalache's temporal support but is NOT a proof of correctness.

**Options to evaluate (when this is promoted):**
1. Hand-author or patch the emitted TLA+ into TLC-checkable `[A]_vars` form (the `[Next]_vars` subscript). Requires understanding the Quint emission and may break on Quint version bumps.
2. Restructure the Quint temporal properties to avoid `next()`-in-`always()` — e.g. encode as inductive invariants with explicit history variables (a state variable recording the prior value), checkable by Apalache's stable invariant path (not the experimental temporal path).
3. Cross-check the temporal properties via TLC on a hand-written TLA+ baseline for the most safety-critical ones (TerminalFinality, GenerationMonotonic) — accepts the Q4 drift concern but gets a second, stable checker.
4. Accept the experimental caveat and document it as a known verification gap pending Apalache temporal maturation.

Not blocking the seed feature's advancement once the self-defining-property blockers (story-fix-formal-model-genuine-checks) are fixed — but should be resolved before the properties are treated as durable product semantics in a release.
