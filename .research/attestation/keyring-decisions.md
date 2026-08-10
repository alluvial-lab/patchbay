---
source_handle: keyring-decisions
fetched: 2026-08-09
source_path: git:/home/agent/projects/outpost_pi@fafcb92103cb596619868981a34d5362633e0807:docs/DECISIONS.md
provenance: source-direct
---
# Source attestation: identity and authority decisions

Paraphrased summary: Outpost-Pi records persistent pairing, identity as a public key, Ed25519 authentication, one Pi key per PC, hardware-change re-pairing, Owner-signed membership, and relay non-adjudication of membership.

## Key passages

- "**Persistent, not ephemeral** | Peers saved in `peers.json` (PC) + Keychain/Keystore (mobile). Pair-once, reconnect-forever."
- "**Identity = pubkey** | No username. Relay auth via Ed25519 challenge-response."
- "**Ed25519 everywhere for identity** | Owner-key signs `mesh_versions`; Pi-key authenticates to relay and signs cross-PC envelopes"
- "**One Pi-key per PC** | Hardware change = re-pairing. No Pi-key migration"
- "**Relay never decides membership** | It forwards ... and verifies signatures, but never adjudicates who is in the mesh."
- "**Anti-rollback membership** | Monotonic version + signature prevents relay/attacker from regressing the mesh."
