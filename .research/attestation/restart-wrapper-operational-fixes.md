---
source_handle: restart-wrapper-operational-fixes
fetched: 2026-08-09
source_path: /home/agent/projects/outpost_pi git commits 4e2386d4f7c5f5e9eb687933a1003a3e8dfe49c2, 8bc44f442da3b09596a02a1de603aab4007296c6, d1773edd692a6e33757a853e3a93d25d11d3db7b, 126d44f003f9dccfe8c292ee6fef5f301a87051a, aa856aba5491a72b0e4767024c1469c6e93c0ab8
provenance: source-direct
---

# Attestation: operational fixes following hot-reload implementation

## Structural metadata

- Source type: five corrective commits
- Paths: `scripts/pi-restart-loop.sh`, `scripts/hot-reload.sh`, `pi-extension/src/index.ts`
- Scope used here: launch environment, process discovery, stale state, and successor startup

## Paraphrased summary

After the reviewed hot-reload implementation landed, operational use exposed missing PATH setup, TUI foreground requirements, an incorrect immediate-parent assumption, stale PID-scoped files, and a fresh-process relay startup gate that prevented reconnection.

## Key passages

### {1} Launch environment

Anchor: `4e2386d` commit message and diff.

Tmux/systemd contexts did not reliably source `.bashrc`, so `pi` was missing from PATH. The wrapper exited immediately and the tmux session died. The fix prepended `$HOME/.local/bin`.

### {2} TUI foreground ownership

Anchor: `8bc44f4` commit message and diff.

Backgrounding Pi to capture its PID detached the TUI from the terminal and made it exit immediately. The fix ran Pi in the foreground and replaced exact marker lookup with a glob.

### {3} Ancestor discovery

Anchor: `d1773ed` commit message and diff.

The arming shell's immediate parent could be an intermediate bash/subshell rather than Pi. The fix walked the process ancestry until it found a matching `.runtime-self-<PID>` identity.

### {4} Stale state after restart

Anchor: `126d44f` commit message and diff.

The original startup sweep removed only `.runtime-self-<PID>`. Hot reload left `.claimed-<old-PID>` and `.restart-marker-<old-PID>` files behind, so the sweep was extended to all three prefixes.

### {5} Successor did not restore relay availability

Anchor: `aa856ab` commit message and diff.

On a fresh process `_disposed` began false, causing `ensureStarted` to return early and leaving the relay disconnected after every hot-reload restart. The fix keyed startup on `_state === "started"` instead, so fresh and replacement processes initiate relay startup unless already connected.
