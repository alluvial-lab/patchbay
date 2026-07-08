---
source_handle: systemd-systemctl
fetched: 2026-07-07
source_url: https://www.freedesktop.org/software/systemd/man/latest/systemctl.html
provenance: source-direct
---

# Attestation: systemctl command/status interface

## Structural metadata

- Publisher/site: freedesktop.org systemd manual pages.
- Page title observed: `systemctl` manual, systemd 261.1.
- Source kind: command reference for controlling and inspecting systemd units, jobs, and manager state.

## Paraphrased summary

`systemctl` is the operator/admin interface for listing units, inspecting unit/job properties, and issuing lifecycle actions such as start/stop/restart. It exposes both human-readable status and computer-parsable property views. Unit state is runtime state, while prior invocations are recovered through journald rather than through the current `status` output.

## Key passages

1. **Unit listing includes active, pending-job, and failed units.** The `list-units` description says only units that are active, have pending jobs, or have failed are shown by default, with `--all` changing that behavior. Source anchor: lines 29-34.

2. **Active-state vocabulary.** The `list-units` section defines the ACTIVE column as a general unit state including `active`, `inactive`, `activating`, `deactivating`, `maintenance`, `refreshing`, `reloading`, and `failed`; it says SUB is unit-type-specific and the full set may change across releases. Source anchor: lines 57-65.

3. **Status is human-oriented; show is machine-oriented.** The `status` command is described as human-readable, while the manual says to use `show` when computer-parsable output is required. Source anchor: lines 116-130 and 174-180.

4. **Runtime status is current or most recent invocation only.** The `status` command note says it displays runtime status about the current invocation, or most recent invocation if still in memory; earlier invocations and prior boots may be retrieved via `journalctl --unit=`. Source anchor: lines 130-137.

5. **`show` exposes units, jobs, and manager properties.** The `show` command description says it shows properties of units, jobs, or the manager, with unit name selecting unit properties and job ID selecting job properties. Source anchor: lines 174-180.

6. **Failed-state cause goes to logs.** The status output explanation says the failed state is entered when a service failed by crash, error exit, or timeout, and that the cause is logged for later reference. Source anchor: lines 168-174.
