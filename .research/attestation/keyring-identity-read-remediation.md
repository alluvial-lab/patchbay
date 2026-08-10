---
source_handle: keyring-identity-read-remediation
fetched: 2026-08-09
source_path: git:/home/agent/projects/outpost_pi@1c1bd2bf9ac0e2f4818b098a9aec1477be122dab:app/lib/pairing/owner_identity_bridge.dart; git:/home/agent/projects/outpost_pi@1c1bd2bf9ac0e2f4818b098a9aec1477be122dab:.work/releases/v0.3.0/gate-security-identity-store-fatal-read-rotates-owner-key.md
provenance: source-direct
---
# Source attestation: Owner identity read remediation

Paraphrased summary: A separate mobile Owner identity failure mode was documented and fixed: an identity-store read error had been treated as absent first-run state, allowing replacement Owner-key generation and persistence. The acceptance contract was changed so generation occurs only on a successful null load; sync-unavailable and platform failures are surfaced, and a second read prevents concurrent restoration from being overwritten.

## Key passages

- "boot() catches every IdentityStoreError and treats it like an absent first-run identity"
- "The bridge then generates and saves a replacement key"
- "Generate only when load() returns null"
- "A replacement Owner key is generated only when load() returns null (genuine first run) — never on an error path."
- "Added a second load before saving a generated identity so a concurrent restored identity wins."
