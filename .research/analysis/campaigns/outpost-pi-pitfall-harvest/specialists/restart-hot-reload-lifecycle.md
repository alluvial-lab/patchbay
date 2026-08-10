---
provenance: agent-synthesis
updated: 2026-08-09
---

# Restart and hot-reload lifecycle pitfalls

## Scope and terms

This brief treats an adapter-code upgrade as a change to the code that mediates between a control plane and an agent runtime. It distinguishes:

- **restart-as-continuation**: replace the runtime process, then resume the existing agent session;
- **restart-fresh**: replace the runtime process, then deliberately create a new agent session;
- **session replacement**: change the SDK session inside a process without necessarily replacing the process.

The distinction matters because the source system eventually used separate handshakes for continuation restart and fresh restart, and because the available handshake depends on which process manager owns the runtime.[restart-fresh-session-ea6b5fd]{1}[restart-fresh-session-ea6b5fd]{2}

## Findings at a glance

1. `/reload` was not a reliable adapter-code upgrade boundary for the ESM extension; a new process was required to load fresh `dist/` code.[restart-session-note-20260731]{1}
2. A settlement event was useful for choosing *when to begin* restart, but it was neither a lock nor an end-to-end delivery acknowledgment.[restart-hot-reload-feature]{8}[restart-hot-reload-code-ec2908c]{3}
3. The successful local protocol required process-incarnation identity, exclusive claim, ingress quiescence, an authoritative idle recheck, graceful shutdown, and explicit restart intent.[restart-hot-reload-feature]{7}[restart-hot-reload-feature]{8}[restart-hot-reload-feature]{9}
4. Exact process fencing regressed when terminal ownership forced the wrapper to run Pi in the foreground: the wrapper changed from matching one child PID to consuming any restart marker in a shared directory.[restart-wrapper-foreground-regression]{1}[restart-wrapper-foreground-regression]{2}[restart-wrapper-foreground-regression]{3}
5. External fleet restart accumulated advisory status checks, PID hunting, deadlines, SIGKILL escalation, sleeps, and retries; these mitigated observed failures but did not make the operation atomic.[restart-herdr-restart-arc]{2}[restart-herdr-restart-arc]{3}[restart-herdr-restart-arc]{4}[restart-herdr-restart-arc]{5}

## Pitfall harvest

### P1. Treating in-process reload as an adapter-code upgrade

**Failure mode.** The operator rebuilds `dist/`, invokes `/reload`, and continues running old ESM extension code.[restart-session-note-20260731]{1}

**Root cause.** The recorded loader path used native dynamic import, whose already-loaded ESM module remained cached for the process lifetime; disabling the CommonJS module cache did not invalidate it.[restart-session-note-20260731]{1}

**Patchbay relevance.** `{inferred: recommends}` Adapter-code upgrade should be a typed process-replacement operation, not an alias for SDK session reload. Completion should refer to a successor incarnation that reports the intended adapter build identity, not merely to receipt of a reload command.[restart-hot-reload-feature]{9}

### P2. Restarting from a per-turn event plus a timer

**Failure mode.** A queued follow-up or newly arriving prompt begins after `turn_end`, then an already-armed timer kills the new turn.[restart-hot-reload-feature]{1}

**Root cause.** The old design drained queued work before arming a fixed 500 ms restart timer; the timer was detached from the ownership and state of the later turn.[restart-hot-reload-feature]{1}

**Patchbay relevance.** `{inferred: recommends}` A restart request should move through an explicit quiescing state and be committed only after an authoritative idle observation made under the same incarnation fence.[restart-hot-reload-feature]{8}

### P3. Treating `agent_settled` as a lock or flush acknowledgment

**Failure mode.** New ingress can race the restart handler, or final outbound frames can remain locally queued when shutdown begins.[restart-hot-reload-feature]{4}[restart-hot-reload-feature]{8}

**Root cause.** Settlement followed the agent continuation loop, but it did not exclude a new run and did not acknowledge delivery through the owner channel and relay.[restart-hot-reload-feature]{8}[restart-hot-reload-code-ec2908c]{3}

**Mitigation that held locally.** The handler stayed synchronous, set an ingress gate, rechecked `ctx.isIdle()`, then used graceful SIGTERM; code comments narrowed the guarantee to bounded local drain and reconnect rehydration rather than end-to-end receipt.[restart-hot-reload-code-ec2908c]{3}[restart-hot-reload-code-ec2908c]{4}

**Patchbay relevance.** `{inferred: recommends}` Separate three concepts in the lifecycle contract: **settled notification**, **exclusive quiescence**, and **delivery durability**. None should imply the others.[restart-hot-reload-feature]{10}

### P4. An old lifecycle action killing a successor

**Failure mode.** `/reload`, `/new`, or resume disposes one session, a successor starts and resets a shared flag, and an old timer then kills the successor.[restart-hot-reload-feature]{5}

**Root cause.** The delayed action checked mutable module state rather than an immutable generation/incarnation token captured when the action was scheduled.[restart-hot-reload-feature]{5}

**Patchbay relevance.** `{inferred: recommends}` Every delayed stop/restart callback should carry the target incarnation and fail closed if current ownership differs. A mutable boolean such as `disposed` is not a generation fence.[restart-hot-reload-code-ec2908c]{7}

### P5. Machine-global or room-scoped restart intent cross-firing

**Failure mode.** One runtime consumes another runtime's request, or multiple runtimes all decide they won the same request.[restart-hot-reload-feature]{2}

**Root cause.** The first protocol used a machine-global sentinel and check-then-unlink. A later proposal incorrectly treated POSIX rename-over-destination as exclusive claim.[restart-hot-reload-feature]{2}[restart-session-note-20260731]{2}

**Mitigation that held locally.** Arming became PID-scoped and nonce-bound; the handler used exclusive creation of a claim file and rejected nonce mismatch or stale requests.[restart-hot-reload-code-ec2908c]{1}[restart-hot-reload-code-ec2908c]{2}[restart-hot-reload-code-ec2908c]{3}

**Patchbay relevance.** `{inferred: recommends}` Restart authority should key on a stable logical session plus a unique runtime incarnation. PID may be a local locator, but the nonce/incarnation is the fence against reuse and cross-process consumption.[restart-hot-reload-feature]{7}

### P6. Terminal correctness weakening lifecycle fencing

**Failure mode.** Capturing a TUI child PID by backgrounding detaches it from the terminal and makes it exit; running it correctly in the foreground removes the shell's exact child-PID correlation, after which a foreign marker can authorize relaunch.[restart-wrapper-foreground-regression]{1}[restart-wrapper-foreground-regression]{3}

**Root cause.** Terminal ownership and process supervision were designed in one shell loop, so fixing the TUI execution model silently weakened the restart-intent fence.[restart-wrapper-foreground-regression]{2}[restart-wrapper-foreground-regression]{3}

**Current gap.** The extension comment still says the wrapper validates the marker against its child PID, while the same commit's wrapper scans for any marker.[restart-fresh-session-ea6b5fd]{7}

**Patchbay relevance.** `{inferred: recommends}` Do not derive incarnation correlation from shell job-control behavior. Put correlation in a supervisor/adapter protocol that can preserve foreground terminal ownership while retaining a stable child handle.[restart-wrapper-foreground-regression]{4}

### P7. Ambiguous exit semantics

**Failure mode.** Restart-on-every-exit-zero makes normal `/quit` relaunch immediately; without a manager, an intentional process exit strands the remote operator.[restart-hot-reload-feature]{3}

**Root cause.** Clean exit, continuation restart, and fresh restart were initially overloaded onto exit status without a typed manager handshake.[restart-hot-reload-feature]{3}

**Mitigation that held locally.** Hot reload used exit zero plus a durable marker for continuation; fresh session used exit 42 for one launch without `--continue`; unmanaged agents rejected fresh restart.[restart-fresh-session-ea6b5fd]{1}[restart-fresh-session-ea6b5fd]{2}

**Patchbay relevance.** `{inferred: recommends}` Model `stop`, `restart_as_continuation`, and `restart_fresh` as distinct operation kinds with declared adapter capabilities and separate conformance vectors.[restart-fresh-session-ea6b5fd]{6}

### P8. Losing input in the restart presence window

**Failure mode.** The app believes a prompt was sent while the destination disappears; reconnect history can recover output but cannot manufacture input the runtime never accepted.[restart-hot-reload-feature]{6}[restart-hot-reload-code-ec2908c]{4}

**Root cause.** Presence, transport acceptance, runtime acceptance, and durable command acknowledgment were not one atomic event.[restart-hot-reload-feature]{6}

**Mitigation limit.** The quiescing gate returned a recoverable delivery error rather than promising local replay, leaving resend to the app.[restart-session-note-20260731]{4}[restart-hot-reload-feature]{10}

**Patchbay relevance.** `{inferred: recommends}` Restart should expose quiescing/restarting presence and command acceptance should be idempotent or durably acknowledged across incarnation change.[restart-hot-reload-feature]{6}

### P9. Declaring restart complete before the successor is usable

**Failure mode.** A successor process starts but its relay does not auto-connect, so the mobile control surface remains unavailable.[restart-session-note-20260731]{6}

**Root cause.** Startup gating reused a disposal flag whose fresh-process initial value meant "do nothing."[restart-wrapper-operational-fixes]{5}

**Patchbay relevance.** `{inferred: recommends}` Restart completion should be a successor readiness condition—adapter loaded, session selected, control channel attached, and current incarnation published—not merely old-process exit or child spawn.[restart-wrapper-operational-fixes]{5}

### P10. External PID hunting as a restart API

**Failure mode.** Injected `/quit` never reaches the TUI; a relaunch is refused as pane busy; later starts race pane-idle detection and sometimes do not stick.[restart-herdr-restart-arc]{2}[restart-herdr-restart-arc]{5}

**Root cause.** The process manager lacked an owned restart primitive, so an operations script composed stale pane metadata, `foreground_processes[0]`, POSIX signals, polling, sleeps, and retry.[restart-herdr-restart-arc]{3}[restart-herdr-restart-arc]{4}

**Patchbay relevance.** `{inferred: recommends}` Adapter restart should be invoked through an adapter-owned lifecycle port. External PID lookup may diagnose, but should not be the authority boundary.[restart-herdr-restart-arc]{3}

### P11. Hidden launch-environment assumptions

**Failure mode.** The wrapper cannot resolve `pi` under tmux/systemd, or the arming helper identifies an intermediate shell rather than the runtime.[restart-wrapper-operational-fixes]{1}[restart-wrapper-operational-fixes]{3}

**Root cause.** Interactive shell PATH initialization and immediate-parent topology were treated as stable deployment contracts.[restart-wrapper-operational-fixes]{1}[restart-wrapper-operational-fixes]{3}

**Patchbay relevance.** `{inferred: recommends}` Spawn specifications should carry an explicit executable, environment, working directory, terminal mode, and manager-owned incarnation identity.[restart-wrapper-operational-fixes]{1}

### P12. Restart debris and asynchronous teardown

**Failure mode.** PID-scoped claims and markers accumulate after restart; separately, a test deletes its state directory while socket startup is still pending, causing a late `chmod` ENOENT even though every assertion passed.[restart-wrapper-operational-fixes]{4}[restart-enoent-race]{1}[restart-enoent-race]{2}

**Root cause.** Restart state and spawned asynchronous resources did not share a joined lifetime with their owning test/runtime generation.[restart-enoent-race]{3}

**Patchbay relevance.** `{inferred: recommends}` Every incarnation should own a resource scope whose shutdown can be awaited or cancelled before storage cleanup; verification should treat unhandled asynchronous errors as failure even when assertions are green.[restart-enoent-race]{4}

### P13. Reusing direct process exit for restart-fresh

**Failure mode.** The fresh-session branch acknowledges, resets local projection state, waits 100 ms, and calls `process.exit(42)` rather than running the graceful signal/drain path.[restart-fresh-session-ea6b5fd]{3}

**Root cause.** The process-manager handshake encoded session selection in an exit code, while acknowledgment and shutdown durability remained a fixed-delay convention.[restart-fresh-session-ea6b5fd]{1}[restart-fresh-session-ea6b5fd]{3}

**Patchbay relevance.** `{inferred: recommends}` Session-selection intent should be handed to the manager before a common graceful shutdown path; restart-fresh should not need weaker drain semantics than restart-as-continuation.[restart-hot-reload-feature]{9}

## Seam decisions

### Decisions that survived review

| Seam | Decision | Patchbay pressure |
|---|---|---|
| Adapter upgrade boundary | Replace the process to load new ESM adapter code.[restart-session-note-20260731]{1} | Require successor adapter-build evidence before operation completion. |
| Safe point | Begin from settlement, but add synchronous quiescence and an idle recheck.[restart-hot-reload-feature]{8} | Specify settlement, quiescence, and readiness separately. |
| Incarnation fence | PID-scoped request plus process nonce; exclusive claim with `O_EXCL`.[restart-hot-reload-code-ec2908c]{2}[restart-hot-reload-code-ec2908c]{3} | Carry an adapter-neutral incarnation id; treat PID as adapter-local metadata. |
| Shutdown | Write restart intent before graceful SIGTERM.[restart-hot-reload-feature]{9} | Persist operation intent before terminating the old incarnation. |
| Ingress during quiesce | Return recoverable error, not a replay promise the exiting runtime cannot honor.[restart-hot-reload-feature]{10} | Make acceptance/durability explicit in protocol responses. |
| Operation semantics | Separate continuation restart from fresh restart and reject unsupported manager modes.[restart-fresh-session-ea6b5fd]{1}[restart-fresh-session-ea6b5fd]{2} | Put support in the adapter capability manifest. |

### Decisions taken and later regretted or weakened

| Decision | Why it failed or weakened | Relationship |
|---|---|---|
| `turn_end` plus fixed delay | Follow-up work could start before the timer fired.[restart-hot-reload-feature]{1} | Replaced by settlement plus quiescing. |
| Machine-global sentinel | Multiple runtimes could consume or act on one request.[restart-hot-reload-feature]{2} | Replaced by process incarnation scoping. |
| Rename as exclusive claim | Rename-over-destination allowed multiple apparent winners.[restart-session-note-20260731]{2} | Replaced by exclusive create. |
| Restart on exit zero | Normal quit and restart were indistinguishable.[restart-hot-reload-feature]{3} | Replaced by explicit marker. |
| Exact child-PID marker via background job | Correct fencing broke TUI terminal ownership.[restart-wrapper-foreground-regression]{1}[restart-wrapper-foreground-regression]{2} | Replaced by a glob, re-opening cross-wrapper consumption. |
| Fixed-delay direct exit for fresh restart | It does not share the graceful hot-reload shutdown path.[restart-fresh-session-ea6b5fd]{3} | Still present; typed operation intent should be moved ahead of graceful shutdown. |

## Gaps discovered

1. **Exact marker correlation is currently absent.** The wrapper consumes any marker, its test covers one wrapper/one marker, and the extension comment claims stronger validation than exists.[restart-fresh-session-ea6b5fd]{4}[restart-fresh-session-ea6b5fd]{5}[restart-fresh-session-ea6b5fd]{7}
2. **Restart readiness has no single end-to-end acknowledgment.** Observed fixes separately handled process exit, pane idleness, retry, PATH, foreground terminal ownership, and relay auto-start.[restart-herdr-restart-arc]{5}[restart-wrapper-operational-fixes]{1}[restart-wrapper-operational-fixes]{2}[restart-wrapper-operational-fixes]{5}
3. **External fleet restart remains advisory.** Status comes from a pane-list snapshot and PID selection uses the first reported foreground process; the sources do not show an atomic "still idle and still this incarnation" check at signal time.[restart-herdr-restart-arc]{3}[restart-herdr-restart-arc]{4} `{inferred: identifies}`
4. **Input durability across restart remains client-driven.** The old process returns a recoverable error if it sees the message, but a message lost between relay acceptance and Pi receipt still lacks a source-attested durable replay mechanism.[restart-hot-reload-feature]{6}[restart-hot-reload-code-ec2908c]{4}
5. **Restart-fresh support varies by manager.** Eleven Herdr-managed agents were explicitly left outside the wrapper contract and therefore fail safely rather than restart fresh.[restart-fresh-session-ea6b5fd]{6}
6. **Graceful semantics diverge by operation.** Hot reload uses marker plus SIGTERM; restart-fresh uses a 100 ms delay plus direct exit.[restart-hot-reload-feature]{9}[restart-fresh-session-ea6b5fd]{3}
7. **Lifecycle tests can leave asynchronous failures after green assertions.** The parked restart-sweep race demonstrates that teardown/join behavior needs explicit verification.[restart-enoent-race]{1}[restart-enoent-race]{3}

## Contradictions

| Position A | Position B | Relationship |
|---|---|---|
| The session note calls `agent_settled` the correct restart boundary because continuation work has drained.[restart-session-note-20260731]{3} | The feature/code state that settlement is neither an exclusive lock nor an end-to-end flush acknowledgment.[restart-hot-reload-feature]{8}[restart-hot-reload-code-ec2908c]{3} | `qualifies`: correct observation point, insufficient commit fence. |
| Reviewed wrapper behavior required the exact exited child PID's marker.[restart-hot-reload-code-ec2908c]{5} | Foreground TUI repair deliberately changed the wrapper to accept any recent marker.[restart-wrapper-foreground-regression]{3} | `contradicts`: exact correlation was removed to satisfy terminal ownership. |
| Current extension comment says the wrapper validates the marker against its child PID.[restart-fresh-session-ea6b5fd]{7} | Current wrapper scans all markers and chooses the first.[restart-fresh-session-ea6b5fd]{4} | `contradicts`: documentation overstates the live fence. |
| Hot-reload review rejected direct `process.exit` because it bypassed graceful lifecycle.[restart-hot-reload-feature]{9} | Restart-fresh later schedules `process.exit(42)` after 100 ms.[restart-fresh-session-ea6b5fd]{3} | `tension`: distinct operation semantics, shared shutdown durability concern. |
| Feature intent sought manager-backed restart without stranding the operator.[restart-hot-reload-feature]{3} | Herdr-managed agents remained outside the wrapper contract and reject restart-fresh.[restart-fresh-session-ea6b5fd]{6} | `qualifies`: safety preserved by capability rejection, coverage incomplete. |

## Disconfirming analysis

- **Against “restart is always required”:** the fetched corpus specifically establishes failure to reload this ESM extension build, not a universal property of every adapter or loader. No fetched source demonstrated fresh `dist/` uptake through `/reload`; Patchbay should therefore make the boundary adapter-capability-driven while treating process replacement as required for this Pi adapter.[restart-session-note-20260731]{1}
- **Against “settlement solves the race”:** the design review and implementation comments explicitly disconfirm lock and flush interpretations. The surviving claim is narrower: settlement is a useful trigger before a separate quiescing/idle/claim protocol.[restart-hot-reload-feature]{8}[restart-hot-reload-code-ec2908c]{3}
- **Against “PID-scoped filenames solve cross-process restart”:** extension-side arming and claim are PID-scoped, but wrapper-side consumption is not. The current single-wrapper test does not disconfirm the foreign-marker race.[restart-fresh-session-ea6b5fd]{4}[restart-fresh-session-ea6b5fd]{5}
- **Against “SIGTERM is sufficient”:** the Herdr script still needed polling, SIGKILL escalation, settling, and retry, while successor relay startup required a separate correction. Signal delivery alone was not operation completion.[restart-herdr-restart-arc]{3}[restart-herdr-restart-arc]{5}[restart-wrapper-operational-fixes]{5}
- **Against “all restart operations share graceful lifecycle”:** restart-fresh is a counterexample in the current implementation because it uses direct exit after a fixed acknowledgment delay.[restart-fresh-session-ea6b5fd]{3}
- **Against “green tests prove clean lifecycle”:** the restart-sweep record reports all assertions passing alongside a nonzero process exit from late asynchronous ENOENT.[restart-enoent-race]{1}[restart-enoent-race]{3}

## Revisit if

- Pi's extension loader gains a source-attested cache-busting or versioned-import contract that demonstrably loads rebuilt ESM code without process replacement.
- The adapter exposes an owned restart API with incarnation id, quiesce acknowledgment, session-selection intent, and successor-ready acknowledgment.
- The wrapper regains exact incarnation correlation without backgrounding the TUI, or is replaced by a supervisor that owns the foreground terminal and child handle.
- Command acceptance becomes durable/idempotent across restart, removing the client-resend gap.
- Herdr-managed and wrapper-managed agents converge on one declared lifecycle capability contract.
- Restart-sweep tests join or cancel all socket/supervisor tasks before deleting state directories.
