---
source_handle: tmux-session-persistence
fetched: 2026-07-07
source_url: https://man7.org/linux/man-pages/man1/tmux.1.html
provenance: source-direct
---

# Attestation: tmux session persistence and attach model

## Structural metadata

- Publisher/site: man7.org Linux manual pages.
- Page title observed: `tmux(1)`.
- Source kind: terminal multiplexer command manual.

## Paraphrased summary

`tmux` manages persistent terminal sessions through a server process. Clients can detach, reattach, list, target, and control sessions/windows/panes. Its persistence and reconnect model is terminal-oriented: sessions preserve pseudo-terminals across disconnects rather than recording semantic command lifecycles.

## Key passages

1. **Session definition.** The manual says a session is a collection of pseudo-terminals under tmux management; a session has one or more windows, windows contain panes, and each pane is a separate pseudo-terminal. Source anchor: lines 81-90.

2. **Multiple clients and server.** Any number of tmux instances may connect to the same session, and all sessions are managed by a single server; server and clients are separate processes communicating through a socket in `/tmp`. Source anchor: lines 91-105.

3. **Persistence after disconnect.** Each session is persistent and survives accidental disconnection such as SSH timeout or intentional detaching; it can later be reattached with `tmux attach`. Source anchor: lines 95-100.

4. **Control mode exists.** The `-C` option starts tmux in control mode, pointing readers to the CONTROL MODE section. Source anchor: lines 112-113.

5. **Command target model.** Most commands accept target-client/session/window/pane arguments; target-session resolution can use session ID, exact name, prefix, glob, or default current/most-recent session, with multiple matches producing an error. Source anchor: lines 500-533.

6. **List sessions as addressing aid.** The target-session section references names as listed by `list-sessions`, and command targets can specify session/window/pane by structured forms such as `session:window.pane`. Source anchor: lines 513-552.
