---
source_handle: restart-session-note-20260731
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi@c659d52aee0905e45ecb323db431346048eed624:.work/session-notes/2026-07-31-hot-reload-pty-stall-herdr-migration.md
provenance: source-direct
---

# Attestation: 2026-07-31 hot-reload, PTY stall, and Herdr migration session note

## Structural metadata

- Source type: repository session note
- Commit: `c659d52aee0905e45ecb323db431346048eed624`
- Internal structure: Summary; five arcs; key decisions; operational state; commits; open items
- Scope used here: process-restart rationale, design evolution, observed post-restart failures, terminal/process-manager constraints, and unresolved lifecycle work

## Paraphrased summary

The note records an operational session in which ESM extension reload behavior led to a process-restart hot-reload design, repeated cross-model review changed the fencing protocol, fresh-process relay startup failed, and terminal/process-manager behavior forced changes from code-server PTYs through tmux to Herdr. It distinguishes an agent-settled notification from an exclusive lock and lists remaining delivery and process-manager gaps.

## Key passages

### {1} Process restart as the code-upgrade boundary

Anchor: `## Arc 3: Extension hot-reload via process restart` → `### The problem`.

> "pi's `/reload` does NOT re-import a `type: module` (ESM) extension."
>
> "A full process restart is the only way to load new `dist/` code."

The note attributes the behavior to jiti's native dynamic import path and the runtime ESM cache.

### {2} Rejected restart protocols

Anchor: `## Arc 3` → `### Design evolution (3 rounds of cross-model review)`.

> "v1 (inline, rejected): turn_end + machine-global sentinel + rename claim + exit-42 + process.exit."
>
> "v2 design (rejected): agent_settled + PID-scoped sentinel + rename claim + exit-42."
>
> "v2 revised (approved): agent_settled + PID-scoped identity/nonce + O_EXCL claim + quiescing gate + ctx.isIdle() recheck + graceful SIGTERM + marker handshake."

### {3} Settlement is not exclusive and shutdown needs an ingress gate

Anchor: `## Arc 3` → `### Final architecture`; repeated under `## Key decisions`.

> "agent_settled is the correct restart boundary: it fires after ALL turns/retries/compactions/queued-followups settle, but is NOT an exclusive lock (ctx.isIdle() recheck + quiescing gate needed)."

The final architecture also says the handler is synchronous and rejects new messages while quiescing.

### {4} Delivery semantics during restart

Anchor: `## Arc 3` → `### Final architecture`; `## Open items`.

> "Quiescing gate: rejects new messages as recoverable `delivery_error` (NOT `delivery_pending` — the process is exiting, replay is impossible)."
>
> "The `delivery_error` during restart window — app should resend on recoverable error (future app improvement)."

### {5} Graceful signal and marker handshake

Anchor: `## Arc 3` → `### Final architecture`.

> "Graceful shutdown: `process.kill(pid, \"SIGTERM\")` → full session_shutdown path (working=false, relay drain). NOT process.exit."
>
> "Wrapper handshake: exit 0 + `.restart-marker-<child-PID>` → relaunch; exit 0 no marker → stop; non-zero → stop"

### {6} Fresh-process startup was a separate failure mode

Anchor: `## Arc 4: Relay auto-start fix`.

> "After every hot-reload restart (or any fresh pi process), the relay didn't auto-connect."
>
> "The `ensureStarted` gate used `_disposed` (starts `false` on a fresh module), so it returned early and the relay never started."

The recorded fix changes the gate to relay `_state` and starts unless already started.

### {7} Terminal substrate can freeze or invalidate wrapper mechanics

Anchor: `## Arc 2: Pty-stall diagnosis`; `## Arc 5: Herdr migration`.

The note records a Pi TUI whose event loop froze when code-server stopped draining its PTY master, then reports Herdr's missing auto-restart/respawn as a reason the wrapper remained necessary.

### {8} Explicitly parked lifecycle gaps

Anchor: `## Open items`.

The feature left four important findings parked: shell arming robustness, wrapper filesystem validation, subprocess tests, and command error reporting. The note separately leaves app resend behavior during the restart window open.
