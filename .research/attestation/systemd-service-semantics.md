---
source_handle: systemd-service-semantics
fetched: 2026-07-07
source_url: https://www.freedesktop.org/software/systemd/man/latest/systemd.service.html
provenance: source-direct
---

# Attestation: systemd service unit semantics

## Structural metadata

- Publisher/site: freedesktop.org systemd manual pages.
- Page title observed: `systemd.service — Service unit configuration`, systemd 261.1.
- Source kind: service-unit configuration reference.

## Paraphrased summary

A systemd service unit describes a process controlled and supervised by systemd. The service reference defines how systemd decides a service has started, how failures and timeouts are interpreted, and how restart policy is configured. Service readiness may be immediate, process-exec-based, parent-fork-based, D-Bus-name-based, or explicit notification-based.

## Key passages

1. **Service unit definition.** The description says a unit ending in `.service` encodes information about a process controlled and supervised by systemd. Source anchor: lines 23-25.

2. **Startup-completion types.** `Type=` configures the mechanism by which the service notifies the manager that startup has finished, with values such as `simple`, `exec`, `forking`, `oneshot`, `dbus`, `notify`, `notify-reload`, and `idle`. Source anchor: lines 89-92.

3. **`simple` can report start success before exec failure.** For `Type=simple`, the manager considers the unit started immediately after fork, before process attributes are configured or `execve()` invokes the service binary; the manual notes `systemctl start` may report success even when the binary cannot be invoked. Source anchor: lines 92-106.

4. **`exec` delays success until exec succeeds.** For `Type=exec`, the manager considers the unit started after the main service binary has executed and `systemctl start` reports failure when the binary cannot be invoked. Source anchor: lines 106-115.

5. **`notify` lets service code decide readiness.** For `Type=notify`, the service is expected to send `READY=1`; the manual says this lets service program code precisely schedule when to consider startup successful and proceed with follow-up units. Source anchor: lines 147-151 and 181-186.

6. **Command sequencing and failure.** `ExecStart=` commands are executed when the service is started; for non-`oneshot` services exactly one command is required. If multiple configured commands are used and one fails without `-`, later lines are not executed and the unit is considered failed. Source anchor: lines 235-245.

7. **Startup timeout failure.** `TimeoutStartSec=` configures how long to wait for startup; if a daemon does not signal startup completion within the time, the service is considered failed and shut down again, subject to the configured failure mode. Source anchor: lines 367-374.

8. **Restart policy vocabulary.** `Restart=` configures whether a service restarts when its process exits, is killed, or reaches a timeout. It accepts `no`, `on-success`, `on-failure`, `on-abnormal`, `on-watchdog`, `on-abort`, or `always`. Source anchor: lines 489-500.

9. **Failure restart conditions.** `Restart=on-failure` restarts when the process exits nonzero, is terminated by signal, an operation times out, or the watchdog timeout triggers; `always` restarts regardless of clean/unclean exit, signal, or timeout, with `oneshot` restrictions. Source anchor: lines 507-519.

10. **Restart exceptions and rate limiting.** Service restart is prevented for configured prevent statuses and administrative `systemctl stop`; restart is subject to unit start-rate limiting with `StartLimitIntervalSec=` and `StartLimitBurst=`. Source anchor: lines 520-529.
