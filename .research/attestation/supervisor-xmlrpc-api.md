---
source_handle: supervisor-xmlrpc-api
fetched: 2026-07-07
source_url: https://supervisord.org/api.html
provenance: source-direct
---

# Attestation: Supervisor XML-RPC API

## Structural metadata

- Publisher/site: Supervisor documentation (`supervisord.org`).
- Page title observed: XML-RPC API Documentation.
- Source kind: programmatic control API reference for `supervisord`.

## Paraphrased summary

Supervisor exposes an XML-RPC API for checking daemon state, checking process state, starting/stopping/signaling processes and groups, and reading process metadata. The API state values gate permitted operations when the supervisor is shutting down, restarting, or fatal.

## Key passages

1. **Daemon state read.** `getState()` returns the current state of supervisord as a struct with integer `statecode` and string `statename`, representing what Supervisor believes is its operational state. Source anchor: lines 189-204.

2. **State gates method calls.** The API docs say clients can use `getState()` both for information and to ensure intended method calls will be permitted. Source anchor: lines 195-201.

3. **Fatal and shutdown method behavior.** Once Supervisor enters `FATAL`, it can never return without restart, and all future methods except `shutdown()` and `restart()` fail with `FATAL_STATE`; in `SHUTDOWN` or `RESTARTING`, all method calls are ignored and possible return values are undefined. Source anchor: lines 239-247.

4. **Process info shape.** `getProcessInfo(name)` returns a struct including `name`, `group`, `start`, `stop`, `now`, numeric `state`, string `statename`, `spawnerr`, `exitstatus`, `logfile`, `stdout_logfile`, `stderr_logfile`, `pid`, and `description`. Source anchor: lines 375-447.

5. **All-process snapshot.** `getAllProcessInfo()` returns an array of process status structs with the same fields as `getProcessInfo`; an empty process table returns an empty array. Source anchor: lines 494-500.

6. **Start/stop controls.** `startProcess(name, wait=True)` starts a process; `startProcessGroup` starts all in a group; `stopProcess(name, wait=True)` stops a process; `stopProcessGroup` stops a group. Source anchor: lines 512-550.

7. **Signal controls.** `signalProcess(name, signal)` sends an arbitrary UNIX signal to a process, and `signalProcessGroup(name, signal)` sends one to all processes in a group. Source anchor: lines 564-575.
