---
source_handle: gnu-screen-session-persistence
fetched: 2026-07-07
source_url: https://www.gnu.org/software/screen/manual/screen.html
provenance: source-direct
---

# Attestation: GNU Screen session persistence and attach model

## Structural metadata

- Publisher/site: GNU project manual.
- Page title observed: GNU Screen manual.
- Source kind: terminal multiplexer manual.

## Paraphrased summary

GNU Screen creates persistent terminal sessions whose programs continue running while windows are hidden or the session is detached. Its command-line options support listing, reattaching, forced detach/reattach, detached creation, multiuser attach, and remote command/query behavior.

## Key passages

1. **Programs continue detached.** The manual says programs continue to run when their window is not visible and even when the whole screen session is detached from the user’s terminal. Source anchor: lines 86-88.

2. **Detach another session.** `-d`/`-D` do not start screen; they detach a screen session running elsewhere, with `-d` equivalent to typing `C-a d` in the controlling terminal and `-D` as power detach. Source anchor: lines 213-219.

3. **Reattach combinations.** Combined `-d`/`-D` with `-r`/`-R` can reattach, detach first, create if absent, and in `-D -R` attach here-and-now by reattaching or creating as necessary. Source anchor: lines 220-247.

4. **List sessions and status labels.** `-ls`/`-list` prints session identification strings; sessions marked `detached` can be resumed with `screen -r`, `attached` are running with a controlling terminal, `multi` means multiuser, and `unreachable` may be different-host or dead. Source anchor: lines 288-302.

5. **Detached creation.** `-d -m` starts screen in detached mode, creating a session without attaching, which the manual says is useful for system startup scripts. Source anchor: lines 321-325.

6. **Authentication/multiuser hints.** `-P` turns authentication on for attach, and `-r sessionowner/...` connects to another user's multiuser screen session using that user's session directory. Source anchor: lines 352-354 and 387-396.

7. **Remote query.** `-Q` lets some commands be queried from a remote session and returns responses on stdout. Source anchor: lines 369-372.
