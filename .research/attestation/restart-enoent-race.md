---
source_handle: restart-enoent-race
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi@ac8c4c5866a5af625fc1d635e908e577d097ad06:.work/backlog/backlog-extension-hot-reload-restart-sweep-enoent-race.md
provenance: source-direct
---

# Attestation: restart-sweep ENOENT test lifecycle race

## Structural metadata

- Source type: parked defect record
- Commit: `ac8c4c5866a5af625fc1d635e908e577d097ad06`
- Path: `.work/backlog/backlog-extension-hot-reload-restart-sweep-enoent-race.md`
- Scope used here: asynchronous resource teardown and false-green verification

## Paraphrased summary

The record describes a full extension test run in which every test assertion passed but the process exited nonzero because asynchronous Unix-socket startup continued after a restart-sweep test removed its temporary directory. A late chmod of a socket path raised ENOENT. The issue was reproduced on the baseline and parked for a separate test-isolation fix.

## Key passages

### {1} False-green assertion result with nonzero suite

Anchor: `## Observation (2026-08-03)`.

The suite reported 55 files passed, 962 tests passed, 3 skipped, and one unhandled error; the overall command exited nonzero.

### {2} Concrete error

Anchor: error block.

> `ENOENT { syscall: 'chmod', path: '/tmp/pi-ext-restart-sweep-XXXX/.pi/remote/locks/YYYY.sock' }`

### {3} Root cause

Anchor: paragraphs after the error.

The restart-sweep test deleted its temporary directory before asynchronous Unix-socket startup in the leader-election/supervisor path completed, so late `chmod` touched a removed path.

### {4} Proposed lifecycle direction

Anchor: `## Direction`.

The record proposes waiting for or cancelling asynchronous socket lifecycle work before deleting the directory, then rerunning under concurrency. It presents an existence guard as another possible local mitigation.
