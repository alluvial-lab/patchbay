---
source_handle: keyring-storage
fetched: 2026-08-09
source_path: git:/home/agent/projects/outpost_pi@ea6b5fd7ee5e15e86de4db98f162f4eab7a70ef8:pi-extension/src/pairing/storage.ts
provenance: source-direct
---
# Source attestation: extension identity storage

Paraphrased summary: `getOrCreateEd25519Keypair` reads the `dev.outpostpi.pi` keyring entry first, retries thrown reads, then reads `~/.pi/remote/identity.json`. A present file is returned without regeneration. If the keyring is persistently unreadable and no file exists, macOS/Windows fail with `KeyringUnavailableError` unless explicitly opted into file identity; headless platforms generate and write a file identity. The keyring-success path returns an existing or freshly generated keypair without writing the file mirror. The file writer is only called by the fresh fallback path.

## Key passages

- "Resolution order: 1. Keyring service `dev.outpostpi.pi` ... 2. File `~/.pi/remote/identity.json` (use if present — never regenerate over an existing one) 3. Generate a fresh keypair"
- "A throw here means the keyring op FAILED"
- "The Outpost-Pi hard cutover deliberately does not inspect legacy remote-pi/keytar services."
- "If a file identity already exists ... use it, never regenerate."
- "On macOS/Windows ... we FAIL LOUD instead"
- The keyring-success path returns `_deserialize(existing)` or returns `fresh` after `backend.write(...)`; it does not call `_writeKeypairToFile`.
- `_writeKeypairToFile` is called in the final fallback branch after `console.warn(...)`, immediately before returning the generated `fresh` keypair.
