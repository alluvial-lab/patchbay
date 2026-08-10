---
source_handle: keyring-tests
fetched: 2026-08-09
source_path: git:/home/agent/projects/outpost_pi@ea6b5fd7ee5e15e86de4db98f162f4eab7a70ef8:pi-extension/src/pairing/storage.test.ts
provenance: source-direct
---
# Source attestation: identity storage tests

Paraphrased summary: Tests cover retry recovery to the original keyring key, refusal to regenerate on persistent core-keyring failure with no file, returning an existing file key during persistent keyring failure, and explicit file-identity opt-in. The tests do not establish that a keyring-success identity is mirrored to the file store, nor that a present file matches a known owner pairing.

## Key passages

- Test title: "transient read failure recovers via retry → uses keyring entry, no file written"
- Test title: "persistent failure on a core-keyring platform with no file → throws (refuses to regen)"
- Test title: "persistent failure but identity.json already exists → returns the FILE key (never throws, never regen)"
- Test title: `OUTPOST_PI_ALLOW_FILE_IDENTITY=1 opts into file identity even on a core-keyring platform`
- The first test explicitly asserts `existsSync(_IDENTITY_FILE_FOR_TEST)).toBe(false)` after recovering the keyring entry.
