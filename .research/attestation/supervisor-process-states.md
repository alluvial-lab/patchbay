---
source_handle: supervisor-process-states
fetched: 2026-07-07
source_url: https://supervisord.org/subprocess.html
provenance: source-direct
---

# Attestation: Supervisor subprocess state machine

## Structural metadata

- Publisher/site: Supervisor documentation (`supervisord.org`).
- Page title observed: Subprocesses.
- Source kind: process-management documentation for programs controlled by `supervisord`.

## Paraphrased summary

Supervisor documents a finite state vocabulary for each controlled process and describes which transitions are administrative, automatic, or retry-driven. Its model distinguishes start requests, running processes, stop requests, too-fast startup failures, exited processes, fatal start failure, and unknown supervisor-internal state.

## Key passages

1. **State vocabulary.** The process-states section says a controlled process is in one of the listed states at any time and that clients may see those names in UI elements. The listed states are `STOPPED`, `STARTING`, `RUNNING`, `BACKOFF`, `STOPPING`, `EXITED`, `FATAL`, and `UNKNOWN`. Source anchor: lines 276-318.

2. **Start/running/backoff definitions.** `STARTING` means a process is starting due to a start request; `RUNNING` means it is running; `BACKOFF` means it entered `STARTING` but exited too quickly before `startsecs` elapsed. Source anchor: lines 286-298.

3. **Terminal-looking states differ.** `EXITED` is a process that exited from `RUNNING`, expectedly or unexpectedly; `FATAL` means the process could not be started successfully; `UNKNOWN` indicates a supervisord programming error. Source anchor: lines 304-317.

4. **Backoff retry loop.** When an autorestarting process is in `BACKOFF`, supervisord restarts it, switching between `STARTING` and `BACKOFF` until `startretries` is exceeded, then transitions to `FATAL`. Source anchor: lines 328-333.

5. **Exit autorestart.** An `EXITED` process automatically restarts according to conditional or unconditional autorestart configuration, and the number of `RUNNING`/`EXITED` transitions is not limited. Source anchor: lines 342-357.

6. **Fatal requires manual restart.** An autorestarted process will never be automatically restarted if it ends in `FATAL`; it must be manually restarted from that state. Source anchor: lines 358-360.

7. **Administrative stop.** A process transitions to `STOPPING` via administrative stop request and then to `STOPPED`; a process that cannot be stopped may remain `STOPPING` forever. Source anchor: lines 361-367.

8. **User-action transitions.** The documented transitions requiring user action include `FATAL -> STARTING` and `RUNNING -> STOPPING`; typically user-action transitions include `STOPPED -> STARTING` except autostart and `EXITED -> STARTING` except autorestart. Source anchor: lines 369-379.
