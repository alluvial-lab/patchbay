---
source_handle: herdr-session-20260731
fetched: 2026-08-09
source_path: c659d52aee0905e45ecb323db431346048eed624:.work/session-notes/2026-07-31-hot-reload-pty-stall-herdr-migration.md
provenance: source-direct
---

## Summary

Session note recording a code-server PTY stall diagnosis, migration through tmux to Herdr, operation of twelve Pi processes across distinct project working directories, and remaining process-management gaps.

## Key passages

1. Under “Pty-stall diagnosis,” the note states: “pi (PID 3699086) had stdin/stdout/stderr wired to `/dev/pts/1`, a pseudo-terminal whose master end was owned by **code-server's terminal host**,” and says a stopped PTY drain caused a blocking stdout write to freeze the process.
2. The empirical evidence is recorded as “a 2-hour gap (22:12 → 00:19) with ZERO working updates while pi's WebSocket to the relay stayed connected,” followed by Pi resuming before the app reconnected.
3. The stated mitigation is: “Initial fix: run pi under tmux (whose server drains the pty independently of any client). Later migrated to Herdr.”
4. Under “Herdr migration,” the note says: “The operator runs 12 pi sessions across different project cwds and needs state visibility + remote access.”
5. The Herdr exploration records the socket operations `pane.send_text`, `agent.start`, and `agent.wait`, native Pi session restore, and the gap: “no auto-restart/respawn (wrapper still needed).”
6. The setup records relay configuration for all twelve projects, `herdr-setup.sh`, `herdr-start-agents.sh`, all twelve Pi processes under Herdr, and a split launch model: outpost_pi under a wrapper while the others are managed agents.
7. The note records the decision “PID-scoped state, not room-scoped: multi-pi is real and common,” with each process reading its own `.hot-reload-armed-<PID>`.
8. Open items include an app-side room/project switcher for access to all twelve Pis and shell arming robustness, wrapper filesystem validation, subprocess tests, and command error reporting.

## Structural metadata

- Artifact type: operational session note
- Commit: `c659d52`
- Recorded session date: 2026-07-31
- Commit date: 2026-08-03
- Relevant sections: “Arc 2: Pty-stall diagnosis,” “Arc 5: Herdr migration,” “Key decisions,” “Operational state at session end,” “Open items”
