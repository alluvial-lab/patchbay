---
source_handle: herdr-cwd-migration
fetched: 2026-08-09
source_path: 45a56a2c0977539022a49169b1134542c6f5a512:.work/SESSION-NOTE-cwd-migration.md
provenance: source-direct
---

## Summary

Verified runbook for relocating a live checkout from `remote_pi` to `outpost_pi`, including session placement, path-update ordering, build-state regeneration, and identity changes caused by deriving names and room identifiers from the working-directory basename.

## Key passages

1. The runbook says: “A session running from inside `remote_pi` cannot `mv` its own cwd,” so a separate session one directory above the checkout was required.
2. It states that path-bearing configuration “must point at a path that **exists at the moment it is read**,” then gives different before/after ordering for trust configuration, extension settings, and shell environment.
3. The verified outside-repository path inventory includes Pi extension registration, trusted cwd, `PATH`, `PUB_CACHE`, and `FLUTTER_WORKSPACE`; repository-local configuration also carries an `agent_name` override.
4. Build regeneration removes and recreates Flutter `.dart_tool` and build directories and reinstalls/rebuilds the Pi extension because old absolute paths were embedded in build state.
5. Verification requires the extension to load from the new path, Flutter to resolve under the new checkout, Git remotes to remain correct, and the phone to be re-paired if its stored room still targets the old room.
6. Under “Behavioral consequence,” the note states: “The default agent name and room ID both derive from `basename(cwd)`, so they change `remote_pi` → `outpost_pi`,” and says the stored phone pairing then appears offline until re-paired.

## Structural metadata

- Artifact type: verified operational migration runbook
- Commit: `45a56a2`
- Verified date in source: 2026-07-15
- Commit date: 2026-07-15
- Relevant sections: “Why a separate session,” “Ordering rules,” “Current verified state,” “Execution steps,” “Behavioral consequence”
