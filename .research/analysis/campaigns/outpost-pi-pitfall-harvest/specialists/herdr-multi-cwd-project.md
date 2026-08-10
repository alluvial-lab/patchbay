---
campaign: outpost-pi-pitfall-harvest
facet: herdr-multi-cwd-project
provenance: agent-synthesis
updated: 2026-08-09
---

# Multi-cwd headless project management through Herdr

This brief examines an installation that ran twelve Pi processes in separate project working directories under Herdr, with one process launched through a custom restart wrapper and the others launched as Herdr-managed agents.[herdr-session-20260731]{4}[herdr-session-20260731]{6} `{inferred:taxonomy}` It treats “project,” cwd, workspace, pane, agent process, and restart owner as separate identities because the operational history shows failures when scripts inferred one from another.[herdr-cwd-migration]{6}[herdr-restart-bulk-fix]{1}[herdr-ancestor-fix]{1}

## Disconfirming analysis

1. **The PTY incident does not show that Herdr itself stalls detached agents.** The recorded stall belonged to a code-server-owned pseudo-terminal; tmux and then Herdr were adopted precisely so a server-side multiplexer would continue draining terminal output.[herdr-session-20260731]{1}[herdr-session-20260731]{3} `{inferred:scope}` The supported conclusion is that a headless adapter needs an independently drained process terminal, not that every terminal container has this failure.
2. **`pane.send_text` is not uniformly unusable.** The setup path used it to start a wrapper at a shell prompt, and the session note identified it as an integration surface.[herdr-setup-pane-fix]{4}[herdr-session-20260731]{5} The narrower adverse result is that injected `/quit` and literal `C-c` did not control an already-running Pi TUI during validation.[herdr-restart-signal-fix]{1}
3. **Cwd-derived identity worked during ordinary operation.** The observed identity break occurred when the checkout basename changed; the source does not establish that every stable-cwd deployment loses identity.[herdr-cwd-migration]{6} `{inferred:scope}` The design concern is relocation and aliasing, not cwd use as adapter metadata.
4. **The restart timing defects are bounded to the observed Herdr 0.7.5 integration and script.** Lowercase-name validation and pane-idle races were recorded by a bulk run, but no cross-version evidence was fetched.[herdr-restart-bulk-fix]{1}[herdr-restart-bulk-fix]{2} `{confidence:bounded}`
5. **The shell-script residue is integration debt, not evidence about Herdr core quality.** The stale `/quit` description and unused polling helper remained in the outpost_pi restart script after the PID-signaling correction.[herdr-restart-signal-fix]{5} `{inferred:scope}`

## Pitfalls

### 1. “Headless” still has a backpressure-bearing terminal

A browser-hosted terminal stopped draining its PTY master when the client went away; a blocking stdout write then froze the single-threaded Pi process even though its relay WebSocket remained connected.[herdr-session-20260731]{1}[herdr-session-20260731]{2} `{inferred:design}` A headless host therefore cannot equate “process continues to exist” with “process continues to make progress.”

**Patchbay relevance.** `{inferred:design}` Adapter conformance should distinguish process existence, terminal-drain liveness, and agent responsiveness. A project/cwd capability that launches a TUI should declare who owns and drains its PTY after all human surfaces detach; reachability should not be inferred from a live process or transport alone.[herdr-session-20260731]{1}[herdr-session-20260731]{2}

### 2. A cwd is mutable location, not durable project identity

Relocating one checkout required a session outside the moving directory, carefully ordered edits to trust, extension, and shell configuration, regeneration of path-bearing build state, and re-pairing because both agent name and room ID changed with `basename(cwd)`.[herdr-cwd-migration]{1}[herdr-cwd-migration]{2}[herdr-cwd-migration]{4}[herdr-cwd-migration]{6}

**Patchbay relevance.** `{inferred:design}` A durable `ProjectId` should not be recomputed from cwd. The adapter may expose cwd as mutable launch metadata and may map it to Herdr workspace or pane identifiers, but identity-preserving relocation should be explicit and testable.[herdr-cwd-migration]{5}[herdr-cwd-migration]{6}

### 3. Terminal text injection is not lifecycle control

The first restart algorithm sent `/quit` and a literal `C-c` through the pane, then attempted a new agent start.[herdr-restart-initial]{4} Validation showed that `/quit` never reached the live TUI as intended, literal `C-c` was not an interrupt, and relaunch failed with `agent_pane_busy`; the correction queried pane process information and signaled the foreground PID.[herdr-restart-signal-fix]{1}[herdr-restart-signal-fix]{2}

**Patchbay relevance.** `{inferred:design}` `send_input`, `request_graceful_shutdown`, `terminate_process`, and `restart_session` should be distinct adapter operations. A successful write to a PTY is not an acknowledgment that an application command was parsed or that the process exited.[herdr-restart-signal-fix]{1}[herdr-restart-signal-fix]{3}

### 4. Shell ancestry is not authoritative session ownership

The hot-reload tool initially assumed its immediate parent was Pi. In practice, an agent tool could insert bash or a subshell, forcing the script to walk the process ancestry until it found a validated, nonce-bearing `.runtime-self-<PID>` record.[herdr-ancestor-fix]{1}[herdr-ancestor-fix]{2}[herdr-ancestor-fix]{3}[herdr-ancestor-fix]{4}

**Patchbay relevance.** `{inferred:design}` PID discovery belongs in the adapter or launch supervisor that created the process container. Core project identity should not expose Unix ancestry as a portable invariant, and shell callers should receive an opaque operation/session target rather than rediscovering it from `PPID`.[herdr-ancestor-fix]{1}[herdr-ancestor-fix]{4}

### 5. Bulk restart compounds state, naming, and timing assumptions

A bulk run found three separate constraints: generated workspace IDs containing uppercase letters were invalid as agent names; restart could race pane-idle detection; and agents in active states needed to be deferred rather than interrupted.[herdr-restart-bulk-fix]{1}[herdr-restart-bulk-fix]{2}[herdr-restart-bulk-fix]{4} The script responded with lowercase conversion, timed retry, inter-agent delay, and an `idle|done` guard.[herdr-restart-bulk-fix]{3}

**Patchbay relevance.** `{inferred:design}` Project ID, workspace ID, pane ID, and agent name require separate typed fields. Lifecycle operations should carry an explicit state precondition and return a stateful outcome such as deferred, accepted, completed, or failed; fixed sleeps may remain an adapter workaround but should not define core completion semantics.[herdr-restart-bulk-fix]{1}[herdr-restart-bulk-fix]{2}[herdr-restart-bulk-fix]{4}

### 6. Process persistence and restart ownership split across layers

Herdr supplied persistent panes and agent/session visibility but did not auto-restart exited agents in this installation.[herdr-session-20260731]{5} One of the twelve projects therefore ran under `pi-restart-loop.sh`, while eleven used direct Herdr agent launch; later restart automation had to branch by cwd suffix and skip the workspace containing the controlling conversation by generated ID.[herdr-session-20260731]{6}[herdr-restart-initial]{2}[herdr-restart-initial]{3}

The wrapper itself exposed a terminal/process tradeoff: backgrounding Pi made PID matching easy but deprived the TUI of a terminal and caused an immediate clean exit; foregrounding restored terminal behavior but changed the wrapper from exact-child marker matching to a marker glob.[herdr-wrapper-tty-fix]{1}[herdr-wrapper-tty-fix]{3}[herdr-wrapper-tty-fix]{5}

**Patchbay relevance.** `{inferred:design}` An adapter capability manifest should state separately whether it preserves a process across detach, restores a session after process loss, respawns on exit, and can restart with or without a resume target. Patchbay should not infer those capabilities from the presence of a terminal workspace.[herdr-session-20260731]{5}[herdr-session-20260731]{6}

### 7. Ad hoc JSON and cwd parsing turns operations into schema archaeology

The initial setup looked up a newly created pane through `pane list --json` using the wrong field; the correction instead captured `result.root_pane.pane_id` from the create response.[herdr-setup-pane-fix]{1} The scripts also embedded a twelve-entry absolute-path registry, filtered panes by a `/home/agent/projects/` substring, selected wrapper behavior by cwd suffix, and collapsed “already exists” and “failed” into one skip path.[herdr-setup-pane-fix]{2}[herdr-setup-pane-fix]{3}[herdr-restart-initial]{3}

After shutdown changed from input injection to signaling, comments and dry-run prose still described `/quit`, and the old agent-list polling helper remained.[herdr-restart-signal-fix]{5} `{inferred:diagnosis}` This is the shell-script archaeology cost: operational truth was distributed across commit messages, JSON shape assumptions, path conventions, generated IDs, and stale comments.

**Patchbay relevance.** `{inferred:design}` The adapter should normalize provider responses once, validate the response shape at the boundary, preserve provider IDs opaquely, and emit structured failure categories. Project registration should be declarative data rather than control flow spread across setup, start, and restart scripts.[herdr-setup-pane-fix]{1}[herdr-setup-pane-fix]{2}[herdr-setup-pane-fix]{3}

## Seam decisions

1. **Stable core identity; mutable adapter location.** `{inferred:design}` Store a stable `ProjectId` in core. Put cwd, Herdr workspace ID, pane ID, launch command, and provider-specific process-container ID in adapter-owned bindings that can be replaced without changing project identity.[herdr-cwd-migration]{6}[herdr-setup-pane-fix]{1}
2. **Lifecycle control is not terminal input.** `{inferred:design}` Model text injection and process lifecycle as separate capabilities, and require lifecycle completion to be observed through process/session state rather than inferred from command delivery.[herdr-restart-signal-fix]{1}[herdr-restart-signal-fix]{2}
3. **Supervisor ownership is explicit.** `{inferred:design}` Record which component owns relaunch and resume policy for each session. Do not branch on cwd suffix to decide whether Herdr or a wrapper controls restart.[herdr-session-20260731]{5}[herdr-session-20260731]{6}[herdr-restart-initial]{3}
4. **Provider identifiers remain opaque and typed by role.** `{inferred:design}` Do not reuse workspace IDs as agent names or derive one by lowercasing the other; the observed validation failure shows that their grammars differ.[herdr-restart-bulk-fix]{1}[herdr-restart-bulk-fix]{5}
5. **Creation returns the authoritative binding.** `{inferred:design}` Capture workspace and root-pane identifiers from the create response, then persist them. Inventory/list APIs are reconciliation inputs, not a substitute for remembering the create result.[herdr-setup-pane-fix]{1}
6. **State-gated operations return structured outcomes.** `{inferred:design}` A restart request against a working agent should become a deferred or rejected operation, while an accepted restart should expose observed exit and successor readiness; a sleep-and-retry loop is adapter-local fallback behavior.[herdr-restart-bulk-fix]{2}[herdr-restart-bulk-fix]{3}[herdr-restart-bulk-fix]{4}
7. **Terminal-drain liveness is a declared hosting responsibility.** `{inferred:design}` A headless adapter should identify the durable PTY owner and provide a responsiveness signal independent of process and transport liveness.[herdr-session-20260731]{1}[herdr-session-20260731]{2}[herdr-session-20260731]{3}

## Gaps

- The fetched corpus contains a session-note account of the PTY diagnosis but not the raw PTY, process, and relay logs behind the two-hour timeline.[herdr-session-20260731]{1}[herdr-session-20260731]{2} `{confidence:bounded}`
- No pinned Herdr 0.7.5 response schema or CLI conformance transcript was fetched. The outpost_pi history proves that one field assumption failed, but it does not establish which response shapes are stable across Herdr versions.[herdr-setup-pane-fix]{1} `{confidence:bounded}`
- The operational note explicitly leaves shell arming robustness, wrapper filesystem validation, subprocess tests, and command error reporting open.[herdr-session-20260731]{8}
- The corpus records a need for a room/project switcher for twelve Pis but does not specify stable project identity, relocation behavior, or duplicate/nested-cwd handling for that surface.[herdr-session-20260731]{4}[herdr-session-20260731]{8} `{inferred:gap}`
- The foreground-wrapper correction changed exact-child restart-marker matching to a glob, but the fetched history does not include a multi-wrapper interference test for that revised handshake.[herdr-wrapper-tty-fix]{3}[herdr-wrapper-tty-fix]{5} `{confidence:bounded}`

## Contradictions

| Relation | Source A | Source B | Side-by-side account |
|---|---|---|---|
| `contradicts` | The initial restart commit describes pane-injected `/quit` plus `agent start --continue` as the graceful bulk-restart algorithm.[herdr-restart-initial]{1}[herdr-restart-initial]{4} | The next corrective commit says validation showed `/quit` never landed, literal `C-c` was not an interrupt, and the pane stayed busy.[herdr-restart-signal-fix]{1} | The first source records the intended control path; the second records an empirical failure of that path and replaces it with process signaling. |
| `qualifies` | The migration note names `pane.send_text` as a key Herdr integration surface.[herdr-session-20260731]{5} | Validation found it unsuitable for delivering shutdown control to a running Pi TUI.[herdr-restart-signal-fix]{1} | The API remained useful for sending shell commands to an idle pane, while its delivery semantics were insufficient as proof of TUI command execution.[herdr-setup-pane-fix]{4} |
| `tension` | The original wrapper bound restart authorization to the exact child PID so concurrent wrappers would not consume each other's intent.[herdr-wrapper-tty-fix]{1} [restart-wrapper-foreground-regression]{4} | The TTY correction required foreground execution and replaced exact-child lookup with the first matching restart-marker glob.[herdr-wrapper-tty-fix]{3}[herdr-wrapper-tty-fix]{5} | Terminal ownership and exact PID binding pulled the wrapper in different directions; the fetched source does not show a later test resolving multi-wrapper marker selection. |

## Revisit if

- Herdr exposes a stable, versioned lifecycle API that acknowledges application-level shutdown and restart, rather than only pane input and process inspection.
- A Herdr release adds declarative respawn/restart policy or changes session restoration semantics.
- Patchbay chooses cwd-derived project identity in any adapter or control surface.
- Patchbay introduces project relocation, nested projects, duplicate checkouts, or multiple sessions per cwd.
- Raw PTY/relay logs or a reproducible detached-output test become available.
- An integration test demonstrates multi-wrapper restart-marker isolation after foreground execution.
