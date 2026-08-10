---
source_handle: keyring-self-revoke
fetched: 2026-08-09
source_path: git:/home/agent/projects/outpost_pi@ea6b5fd7ee5e15e86de4db98f162f4eab7a70ef8:pi-extension/src/mesh/self_revoke.ts; git:/home/agent/projects/outpost_pi@ea6b5fd7ee5e15e86de4db98f162f4eab7a70ef8:pi-extension/src/extension/command_surface/pairing_coordinator.ts
provenance: source-direct
---
# Source attestation: self-revoke and user-visible eviction

Paraphrased summary: `SelfRevoke` periodically fetches each Owner's signed membership envelope, verifies it, compares member public-key bytes with the local Pi public key, and when absent removes the Owner peer and invokes `onRevoke`. `PairingCoordinator` starts this poller with the loaded keypair, detaches the affected Owner using `session_replaced`, refreshes pairing state, and sends an `outpost-pi:mesh-revoked` notification instructing re-pairing.

## Key passages

- "Background poller that watches each Owner's `mesh_versions` envelope"
- "If not a member → `storage.removePeer(ownerEpk)` and fire `onRevoke(ownerEpk)`"
- `SelfRevoke` compares decoded member public-key bytes against `myPubkey`.
- `PairingCoordinator` passes `myPubkey: edKp.publicKey` and calls `this.selfRevoke.start()`.
- The `onRevoke` callback calls `owners.detach(ownerEpk, "session_replaced")`, then sends `customType: "outpost-pi:mesh-revoked"`.
- The notification says: "The mobile app for this Owner removed this PC from the mesh. Re-pair via /outpost-pi pair if this was unexpected."
